use async_trait::async_trait;
use std::io;

/// Abstracts JSONL writes and temp‑file writes for fallback
#[async_trait]
pub trait FileWriter: Send + Sync {
    /// Append a line to the current JSONL log, rotating if needed
    async fn write_jsonl(&self, line: &str) -> io::Result<()>;
    /// Write a fallback line to a temp file when DB fails
    async fn write_temp(&self, line: &str) -> io::Result<()>;
}  // uses async_trait for async in traits :contentReference[oaicite:2]{index=2}
