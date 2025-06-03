## Error Handler Module Documentation

### 1. Purpose and Scope
This document captures the agreed design for the **Error Handler** module of `file_processor_api`. It defines how errors, warnings, and informational messages are handled, logged locally, and persisted in PostgreSQL.

---

### 2. Severity Levels & Routing
- **Error Severe (ES):** logged immediately to DB and JSONL
- **Error Minor (EM):** logged to DB; buffered locally until next DB write
- **Warning Severe (WS):** logged to DB and JSONL 
- **Warning Minor (WM)**: events are **short-circuited** in the code: they never invoke any DB calls and only go to JSONL.
- **Info (I_):** logged to JSONL only

---

### 3. Error Code Structure
- **Runtime representation:** 8-character string, e.g., `ESCU1`
- **Database columns:**
    - `severity` (VARCHAR(2)): ES, EM, WS, WM, I_
    - `component` (CHAR(1)): C, H, E
    - `actor` (CHAR(1)): U, S, N
    - `code` (INTEGER): positive integer

---

### 4. Local JSONL Logging
```jsonl
{"timestamp":"2025-05-17T14:00:00Z","severity":"ES","component":"C","actor":"U","code":1,"message":"<text>","context":{...},"stack_trace":null,"related_logs":[]}
```

- Format: newline-delimited JSON (NDJSON)
- Rotation policy:
  - Rotate when file ≥ 1 MB or on restart 
  - Retain archives for 30 min, then delete 
  - Configurable via LOG_MAX_SIZE_MB and LOG_RETENTION_MIN env vars 
- Filename: Logfile_"Component"_"RFC3339".jsonl
  - e.g. Logfile_C_2025-05-17T14:00:00Z.jsonl

### 5. In-Memory Buffering & Atomic Snapshot
- Buffers: last 5 Info entries, last 10 Warnings, all Errors until persisted 
- Snapshot: on Severe Error, atomically write the new error plus buffered context to JSONL

1. Buffer the ErrorEvent
2. If severity == WM, write JSONL and return
3. insert_message into DB
4. insert_error into DB (fallback to write_temp on error)
5. write_jsonl

### 6. PostgreSQL Schema
```sql
CREATE TABLE error_codes (
  id         SERIAL PRIMARY KEY,
  severity   VARCHAR(2) NOT NULL,
  component  CHAR(1)    NOT NULL,
  actor      CHAR(1)    NOT NULL,
  code       INTEGER    NOT NULL,
  UNIQUE(severity, component, actor, code)
);

CREATE TABLE messages (
  id   UUID PRIMARY KEY,
  text TEXT    NOT NULL
);

CREATE TABLE errors (
  id           UUID PRIMARY KEY,
  timestamp    TIMESTAMPTZ NOT NULL,
  severity     VARCHAR(2)   NOT NULL,
  component    CHAR(1)      NOT NULL,
  actor        CHAR(1)      NOT NULL,
  code         INTEGER      NOT NULL,
  message_id   UUID         NOT NULL REFERENCES messages(id),
  context      JSONB        NOT NULL,
  stack_trace  JSONB,
  related_logs JSONB,
  occurrence   INTEGER      DEFAULT 1,
  FOREIGN KEY (severity, component, actor, code)
    REFERENCES error_codes(severity, component, actor, code)
);

CREATE TABLE blacklist (
  error_code_id INTEGER PRIMARY KEY REFERENCES error_codes(id),
  count         INTEGER NOT NULL
);

CREATE INDEX ON errors(timestamp);
CREATE INDEX ON errors(severity);
CREATE INDEX ON errors(code);
```
- Deduplication logic:
  - If occurrences < 3 → insert new row 
  - Else if similarity < ERR_DUP_THRESHOLD → insert new row 
  - Else → increment occurrence

### 7. Retention Strategy
- Primary: retain errors for 72 hrs uptime 
- Fallback: timestamp-based purge via scheduled DB job

### 8. Security & mTLS
- DB connections: mutual TLS using local cert.pem and key.pem 
- Future: at-rest encryption via pgcrypto

### 9. Configuration Variables

| Env Var             | Description                              | Default |
|---------------------|------------------------------------------|---------|
| LOG_MAX_SIZE_MB     | JSONL rotation size threshold (MB)       | 1       |
| LOG_RETENTION_MIN   | Rotated file retention (minutes)         | 30      |
| ERR_DUP_THRESHOLD   | Deduplication similarity threshold (0–1) | 0.5     |
| DB_HOST, DB_PORT    | PostgreSQL connection settings           |         |
| MTLS_CERT, MTLS_KEY | Paths to mTLS certificate and key        |         |

### 10. Implementation Notes
- WM short-circuit: returns immediately after JSONL write for Severity::WM. 
- Fallback logic: both insert_message and insert_error are wrapped to catch failures and call write_temp. 
- Enum comparability: Severity, Component, and Actor now derive PartialEq/Eq so you can do evt.severity == Severity::WM. 
- Strict testing: mocks require explicit expectations for each path (DB vs JSONL vs temp write).

### 11. Future Improvements
- Support external config files (TOML/YAML)
- Automate mTLS certificate rotation 
- Encrypt JSONL archives and database at rest 
- Integrate Prometheus/Grafana for metrics & alerts
- Add dynamic DAST scanning 
- Extend retention policies per severity 
- Develop web UI dashboard for log browsing 
- Offload JSONL archives to object storage
- Digital signatures or checksums for tamper/tampering detection. 
- Automated cleanup of old rotated files (only rotation is implemented). 
- Distributed/global rate limiting (current is per-process). 
- Deep, context-aware redaction of all possible sensitive data. 
- Integration with non-SMTP alerting systems. 
- Lock-free or sharded buffer management for extreme concurrency.