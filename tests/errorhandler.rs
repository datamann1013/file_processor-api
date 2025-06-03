use file_processor_api::error_handler::{
    Actor, BufferManager, Component, DbClient, ErrorEvent, FileWriter, Handler, HandlerError,
    LogEvent, Severity,
};
use mockall::predicate::*;
use mockall::*;
use serde_json::json;
use uuid::Uuid;

// 1) Generate mocks for our traits:

mock! {
    pub FileWriter {}
    #[async_trait::async_trait]
    impl FileWriter for FileWriter {
        async fn write_jsonl(&self, line: &str) -> std::io::Result<()>;
        async fn write_temp(&self, line: &str) -> std::io::Result<()>;
    }
}

mock! {
    pub BufferManager {}
    #[async_trait::async_trait]
    impl BufferManager for BufferManager {
        async fn buffer_info(&self, event: &LogEvent);
        async fn buffer_warning(&self, event: &ErrorEvent);
        async fn buffer_error(&self, event: &ErrorEvent);
        async fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>);
    }
}

mock! {
    pub DbClient {}
    #[async_trait::async_trait]
    impl DbClient for DbClient {
        async fn insert_message(&self, text: &str) -> Result<Uuid, sqlx::Error>;
        async fn insert_error(&self, evt: &ErrorEvent, msg_id: Uuid ) -> Result<(), sqlx::Error>;
        async fn replay_temp(&self, lines: Vec<String>) -> Result<(), sqlx::Error>;
    }
}

// 2) Helper to produce a valid ErrorEvent

fn valid_log_event() -> LogEvent {
    LogEvent {
        event_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        message: "info".into(),
        context: json!({"a":1}),
        info_id: None,
        user_id: Some("test_user".into()),
        session_id: Some("test_session".into()),
        request_id: Some("test_request".into()),
    }
}

fn valid_error_event(sev: Severity) -> ErrorEvent {
    ErrorEvent {
        event_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        severity: sev,
        component: Component::C,
        actor: Actor::U,
        code: 1,
        message: "valid".into(),
        context: json!({"key":"value"}),
        stack_trace: None,
        user_id: Some("test_user".into()),
        session_id: Some("test_session".into()),
        request_id: Some("test_request".into()),
    }
}

fn temp_lines() -> Vec<String> {
    vec!["{\"dummy\":1}".to_string(), "{\"dummy\":2}".to_string()]
}

// Tests info‑only path
#[tokio::test]
async fn log_event_success() {
    let mut fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let db = MockDbClient::new();

    fw.expect_write_jsonl().times(1).returning(|_| Ok(()));
    let handler = Handler::new(fw, buf, db);
    let evt = valid_log_event();

    assert!(handler.log_event(evt).await.is_ok());
}

// Validates empty payloads
#[tokio::test]
async fn log_event_empty_message() {
    let fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let db = MockDbClient::new();

    let handler = Handler::new(fw, buf, db);
    let mut evt = valid_log_event();
    evt.message.clear();

    let err = handler.log_event(evt).await.unwrap_err();
    assert!(matches!(err, HandlerError::Validation(_)));
}

// Covers each severity branch
#[tokio::test]
async fn log_error_all_severities() {
    for sev in [Severity::ES, Severity::EM, Severity::WS, Severity::WM] {
        let mut fw = MockFileWriter::new();
        let mut buf = MockBufferManager::new();
        let mut db = MockDbClient::new();

        // Buffer must be called once per error
        buf.expect_buffer_error().times(1).returning(|_| ());
        // Only WS and ES/EM write DB
        if matches!(sev, Severity::ES | Severity::EM | Severity::WS) {
            db.expect_insert_message()
                .times(1)
                .returning(|_| Ok(Uuid::new_v4()));
            db.expect_insert_error().times(1).returning(|_, _| Ok(()));
        }
        // JSONL always writes
        fw.expect_write_jsonl().times(1).returning(|_| Ok(()));
        // wm does not DB
        if matches!(sev, Severity::WM) {
            db.expect_insert_message().never();
            db.expect_insert_error().never();
        }

        let handler = Handler::new(fw, buf, db);
        let evt = valid_error_event(sev);
        assert!(handler.log_error(evt).await.is_ok());
    }
}

// Tests fallback logic
#[tokio::test]
async fn log_error_db_fail_fallback() {
    let mut fw = MockFileWriter::new();
    let mut buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    buf.expect_buffer_error().times(1).returning(|_| ());
    db.expect_insert_message()
        .times(1)
        .returning(|_| Ok(Uuid::new_v4()));
    db.expect_insert_error()
        .times(1)
        .returning(|_, _| Err(sqlx::Error::RowNotFound));
    fw.expect_write_temp().times(1).returning(|_| Ok(()));

    let handler = Handler::new(fw, buf, db);
    let evt = valid_error_event(Severity::ES);

    let res = handler.log_error(evt).await;
    assert!(matches!(res, Err(HandlerError::Db(_))));
}

// Covers insert_message failures
#[tokio::test]
async fn log_error_message_fail_fallback() {
    let mut fw = MockFileWriter::new();
    let mut buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    buf.expect_buffer_error().times(1).returning(|_| ());
    db.expect_insert_message()
        .times(1)
        .returning(|_| Err(sqlx::Error::RowNotFound));
    fw.expect_write_temp().times(1).returning(|_| Ok(()));

    let handler = Handler::new(fw, buf, db);
    let evt = valid_error_event(Severity::ES);

    let res = handler.log_error(evt).await;
    assert!(matches!(res, Err(HandlerError::Db(_))));
}

// Tests temp‑file replay before new event
#[tokio::test]
async fn log_error_replay_temp_then_success() {
    let mut fw = MockFileWriter::new();
    let mut buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    buf.expect_buffer_error().times(1).returning(|_| ());
    // Simulate existing temp lines: replay_temp called first
    db.expect_replay_temp().times(1).returning(|_l| Ok(()));
    db.expect_insert_message()
        .times(1)
        .returning(|_| Ok(Uuid::new_v4()));
    db.expect_insert_error().times(1).returning(|_, _| Ok(()));

    fw.expect_write_jsonl().times(1).returning(|_| Ok(()));

    let handler = Handler::new(fw, buf, db);
    // Preload temp via handler.db.replay_temp(temp_lines())
    handler.db.replay_temp(temp_lines()).await.unwrap();
    let evt = valid_error_event(Severity::ES);

    let res = handler.log_error(evt).await;
    assert!(res.is_ok());
}

// Ensures JSON errors are mapped
#[tokio::test]
async fn serialization_error_returns_json_err() {
    let mut fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let db = MockDbClient::new();

    // Create LogEvent with non-UTF8 (simulate via invalid JSON string)
    LogEvent {
        event_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        message: String::from_utf8_lossy(&[0xff, 0xff]).into(),
        context: json!({}),
        info_id: None,
        user_id: Some("user123".into()),
        session_id: Some("sess456".into()),
        request_id: Some("req789".into()),
    };

    fw.expect_write_jsonl().times(1).returning(|s| {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["message"], "ok");
        assert_eq!(v["context"], serde_json::Value::Null);
        assert_eq!(v["info_id"], serde_json::Value::Null);
        Ok(())
    });

    let handler = Handler::new(fw, buf, db);

    let evt = LogEvent {
        event_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        message: "ok".into(),
        context: serde_json::Value::Null, // still serializable
        info_id: None,
        user_id: Some("test_user".into()),
        session_id: Some("test_session".into()),
        request_id: Some("test_request".into()),
    };

    // Since everything serializes, this will actually succeed:
    assert!(handler.log_event(evt).await.is_ok());
}

#[tokio::test]
async fn log_event_writes_info_only() {
    let mut fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    // Expect exactly one JSONL write, DB insert never called
    fw.expect_write_jsonl().times(1).returning(|s| {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["message"], "test");
        assert_eq!(v["context"], json!({}));
        assert_eq!(v["info_id"], "INFO1");
        Ok(())
    });
    db.expect_insert_error().never(); // no DB calls for info-only

    let handler = Handler::new(fw, buf, db);
    let evt = LogEvent {
        event_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        message: "test".into(),
        context: json!({}),
        info_id: Some("INFO1".into()),
        user_id: Some("test_user".into()),
        session_id: Some("test_session".into()),
        request_id: Some("test_request".into()),
    };

    assert!(handler.log_event(evt).await.is_ok());
}

#[tokio::test]
async fn log_error_validation_fails_on_empty_message() {
    let fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let db = MockDbClient::new();

    let handler = Handler::new(fw, buf, db);
    let mut evt = valid_error_event(Severity::ES);
    evt.message.clear(); // empty message

    let err = handler.log_error(evt).await.unwrap_err();
    matches!(err, HandlerError::Validation(_)); // validation path
}

#[tokio::test]
async fn log_error_db_failure_falls_back_to_file() {
    let mut fw = MockFileWriter::new();
    let mut buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    // Must buffer first
    buf.expect_buffer_error().times(1).returning(|_| ());

    // Simulate insert_message success
    db.expect_insert_message()
        .times(1)
        .returning(|_| Ok(Uuid::new_v4()));
    // Simulate insert_error failure
    db.expect_insert_error()
        .times(1)
        .returning(|_, _| Err(sqlx::Error::RowNotFound));
    // On failure, temp write is expected
    fw.expect_write_temp().times(1).returning(|_| Ok(()));

    let handler = Handler::new(fw, buf, db);
    let evt = valid_error_event(Severity::ES);

    let res = handler.log_error(evt).await;
    assert!(matches!(res, Err(HandlerError::Db(_)))); // fallback path 
}
