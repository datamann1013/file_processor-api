# file_processor_api

## Overview
Short description: a Rust gRPC service for secure, high‑performance file processing.

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
All operations will yield `Result<T, ApiError>` via a custom `thiserror`‑based error enum.

### CI Pipeline
GitHub Actions configured for build, test, Clippy, and security scans (Coverity, SonarCloud).

## Getting Started
Installation, WSL2 setup, RustRover project creation, and CI integration instructions.

## Contributing
Guidelines for code style, tests, and Pull Request process.

## License
Specify license.
