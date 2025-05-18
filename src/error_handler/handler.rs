use crate::error_handler::types::{ErrorEvent, HandlerError, LogEvent};
use crate::error_handler::{BufferManager, DbClient, FileWriter};
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
    buffer: Arc<Mutex<B>>,
    db: Arc<D>,
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

    /// Handle warnings & errors: buffer, DB persistence, fallback, and JSONL
    pub async fn log_error(&self, evt: ErrorEvent) -> Result<(), HandlerError> {
        if evt.message.is_empty() {
            return Err(HandlerError::Validation("Empty message".into()));
        }

        // Buffer the event
        {
            let mut buf = self.buffer.lock().await;
            buf.buffer_error(&evt);
        }

        // Replay any temp‑file entries first
        // (Implementation left to concrete DbClient)

        // Ensure message exists in DB, retrieving its UUID
        let msg_id = self.db.insert_message(&evt.message).await?;

        // Try inserting the error, fallback to temp file on failure
        if let Err(db_err) = self.db.insert_error(&evt, msg_id).await {
            let line = to_string(&evt)?;
            self.file_writer.write_temp(&line).await?;
            return Err(HandlerError::Db(db_err));
        }

        // Finally, append to JSONL
        let line = to_string(&evt)?;
        self.file_writer.write_jsonl(&line).await?;

        Ok(())
    }
}  // spawns background rotation task separately :contentReference[oaicite:5]{index=5}
