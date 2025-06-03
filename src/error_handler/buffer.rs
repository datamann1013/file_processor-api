use crate::error_handler::types::{ErrorEvent, LogEvent};

/// Manages in‑memory circular buffers for Info, Warning, and Error context
pub trait BufferManager: Send + Sync {
    fn buffer_info(&self, event: &LogEvent);
    fn buffer_warning(&self, event: &ErrorEvent);
    fn buffer_error(&self, event: &ErrorEvent);
    /// Returns (info_buffer, warning_buffer) snapshots
    fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>);
} // isolates side effects for unit testing
