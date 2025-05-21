use crate::error_handler::types::{ErrorEvent, HandlerError, LogEvent};
use crate::error_handler::{BufferManager, DbClient, FileWriter, Severity};
use serde_json::to_string;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Central handler that routes events to buffer, JSONL, and DB
pub struct Handler<F, B, D>
where
    F: FileWriter + 'static,
    B: BufferManager + 'static,
    D: DbClient + 'static,
{
    file_writer: Arc<F>,
    pub(crate) buffer: Arc<Mutex<B>>,
    pub db: Arc<D>,
}

impl<F, B, D> Handler<F, B, D>
where
    F: FileWriter + 'static,
    B: BufferManager + 'static,
    D: DbClient + 'static,
{
    /// Create a new handler with injected dependencies
    pub fn new(file_writer: F, buffer: B, db: D) -> Self {
        Self {
            file_writer: Arc::new(file_writer),
            buffer: Arc::new(Mutex::new(buffer)),
            db: Arc::new(db),
        }
    }

    /// Handle an informational event: validate and write to JSONL only
    pub async fn log_event(&self, evt: LogEvent) -> Result<(), HandlerError> {
        if evt.message.is_empty() {
            return Err(HandlerError::Validation("Empty message".into()));
        }
        let line = to_string(&evt)?;
        self.file_writer.write_jsonl(&line).await?;
        Ok(())
    }

    /// Handle warnings & errors: buffer, conditional DB persistence, fallback, and JSONL
    pub async fn log_error(&self, evt: ErrorEvent) -> Result<(), HandlerError> {
        // 1. Validate
        if evt.message.is_empty() {
            return Err(HandlerError::Validation("Empty message".into()));
        }

        // 2. Buffer the event
        {
            let buf = self.buffer.lock().await;
            buf.buffer_error(&evt);
        }

        // 3. Warning Minor (WM): only JSONL, no DB calls
        if evt.severity == Severity::WM {
            let line = to_string(&evt)?;
            self.file_writer.write_jsonl(&line).await?;
            return Ok(());
        }

        // 4. For ES, EM, WS: insert message
        let msg_id = match self.db.insert_message(&evt.message).await {
            Ok(id) => id,
            Err(e) => {
                // Fallback: write full event to temp
                let fallback = to_string(&evt)?;
                self.file_writer.write_temp(&fallback).await?;
                return Err(HandlerError::Db(e));
            }
        };

        // 5. Persist error or severe warning, fallback on DB error
        if let Err(e) = self.db.insert_error(&evt, msg_id).await {
            let fallback = to_string(&evt)?;
            self.file_writer.write_temp(&fallback).await?;
            return Err(HandlerError::Db(e));
        }

        // 6. Append to JSONL
        let line = to_string(&evt)?;
        self.file_writer.write_jsonl(&line).await?;
        Ok(())
        
    }
    /// Returns the buffered snapshots of info and error events.
    pub async fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>) {
        let guard = self.buffer.lock().await;
        guard.snapshot()
    }
} // spawns background rotation task separately
