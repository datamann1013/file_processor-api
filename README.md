

![File processor api](/documentation/images/README-header-image.png)


## Overview
About
File processor API is a Rust binary that exposes a gRPC endpoint secured by mutual TLS, ment for integration with C#, Java, and C++ backends. It processes file‑transform commands—encrypt/decrypt, compress/decompress, hash, and metadata extraction/modification—via asynchronous Rust runtimes. All logic is encapsulated in a centralized error handler.

```mermaid
flowchart TD
    subgraph Client
        A["gRPC Client"]
    end

    subgraph Server["file_processor_api"]
        direction TB
        B["Authentication \- mTLS"]
        C["gRPC Endpoint \- tonic"]
        D["Request Dispatcher"]
        E["File Processing Modules"]
        F["Centralized Error Handler"]
        G["Audit Logger"]
    end

    subgraph Storage["Filesystem / Streams"]
        H["Chunked Streams"]
        I["Metadata Store"]
    end

    A -->|"1\. Establish TLS"| B
    B -->|"2\. Send gRPC Request"| C
    C -->|"3\. Dispatch to Handler"| D
    D -->|"4a\. Encrypt / Compress / Hash / Metadata"| E
    D -->|"4b\. Stream Chunks"| H
    E -->|"5\. Errors & Results"| F
    F -->|"6\. Log Event"| G
    F -->|"7\. Respond"| C
    G -->|"8\. Persist Logs"| I
    H -->|"9\. Read/Write"| Storage

```

## Features Plan
### Compression and Decompression
Compression and decompression will be implemented using `zstd` and `flate2` crates.

### Encryption and Decryption
Encryption and decryption will be implemented via AES‑GCM (`openssl` or `rustls` crates).

### Hashing
Hashing will use `blake3` for fast, incremental digests.

### Metadata Handling
Metadata extraction and modification will leverage `std::fs::Metadata`.

### Chunked Streaming
Chunked streaming will use `tokio::io::AsyncReadExt`/`AsyncWriteExt` for zero‑copy.

### Logging and Auditing
Logging using `log` + `env_logger`; audit logs will record request IDs, timestamps, and outcomes.

### Centralized Error Handling
![Error Handler Overview](documentation/ErrorHandler.md)

### CI Pipeline
GitHub Actions configured for build, test, Clippy, and security scans (Coverity, SonarCloud).

## Project Goals Checklist
### Core Objectives
- Develop a Rust-based gRPC service (file_processor_api)
- Ensure secure client-server communication using mTLS 
- Implement modular file processing capabilities 
- Handle errors centrally and log audit trails 
- Manage file storage using chunked streams and metadata

### Deliverables
- Fully functional Rust gRPC service 
- Comprehensive documentation with visual aids 
- Clear project structure facilitating scalability

## License
Apache-2.0 license
