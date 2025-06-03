use crate::error_handler::types::{ErrorEvent, LogEvent};

/// Manages in‑memory circular buffers for Info, Warning, and Error context
#[async_trait::async_trait]
pub trait BufferManager: Send + Sync {
    async fn buffer_info(&self, event: &LogEvent);
    async fn buffer_warning(&self, event: &ErrorEvent);
    async fn buffer_error(&self, event: &ErrorEvent);
    /// Returns (info_buffer, warning_buffer) snapshots
    async fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>);
} // isolates side effects for unit testing
