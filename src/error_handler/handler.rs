use crate::error_handler::types::{ErrorEvent, HandlerError, LogEvent};
use crate::error_handler::{BufferManager, DbClient, FileWriter, Severity};
use serde_json::to_string;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

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

use super::logger::ErrorLogger;

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
    pub async fn log_event(&self, mut evt: LogEvent) -> Result<(), HandlerError> {
        if evt.message.is_empty() {
            warn!("Validation failed: empty message in LogEvent");
            return Err(HandlerError::Validation("Empty message".into()));
        }
        // Sanitize and truncate message
        let max_len = 1024;
        evt.message = evt
            .message
            .replace(['\n', '\r', '\t'], " ")
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>();
        if evt.message.len() > max_len {
            evt.message = evt.message[..max_len].to_string();
        }
        // Redact sensitive data in context
        // (reuse ErrorEvent::redact_sensitive_data logic if needed)
        let line = to_string(&evt)?;
        if let Err(e) = self.file_writer.write_jsonl(&line).await {
            error!(error = ?e, "Failed to write LogEvent to JSONL");
            return Err(e.into());
        }
        info!(event_id = %evt.event_id, user_id=?evt.user_id, session_id=?evt.session_id, request_id=?evt.request_id, "LogEvent written to JSONL");
        Ok(())
    }

    /// Handle warnings & errors: buffer, conditional DB persistence, fallback, and JSONL
    pub async fn log_error(&self, mut evt: ErrorEvent) -> Result<(), HandlerError> {
        // 1. Validate
        if evt.message.is_empty() {
            warn!("Validation failed: empty message in ErrorEvent");
            return Err(HandlerError::Validation("Empty message".into()));
        }
        // Sanitize and truncate message
        evt.sanitize_and_truncate_message(1024);
        // Redact sensitive data
        evt.redact_sensitive_data();
        // 2. Buffer the event
        {
            let buf = self.buffer.lock().await;
            debug!(event_id = %evt.event_id, "Buffering error event");
            buf.buffer_error(&evt).await;
        }
        // 3. Warning Minor (WM): only JSONL, no DB calls
        if evt.severity == Severity::WM {
            let line = to_string(&evt)?;
            if let Err(e) = self.file_writer.write_jsonl(&line).await {
                error!(error = ?e, event_id = %evt.event_id, "Failed to write WM event to JSONL");
                return Err(e.into());
            }
            info!(event_id = %evt.event_id, user_id=?evt.user_id, session_id=?evt.session_id, request_id=?evt.request_id, "WM event written to JSONL");
            return Ok(());
        }
        // 4. For ES, EM, WS: insert message
        let msg_id = match self.db.insert_message(&evt.message).await {
            Ok(id) => {
                debug!(event_id = %evt.event_id, "Message inserted into DB");
                id
            }
            Err(e) => {
                error!(error = ?e, event_id = %evt.event_id, "DB insert_message failed, falling back to temp file");
                // Fallback: write full event to temp
                let fallback = to_string(&evt)?;
                if let Err(e2) = self.file_writer.write_temp(&fallback).await {
                    error!(error = ?e2, event_id = %evt.event_id, "Failed to write fallback temp file");
                }
                return Err(HandlerError::Db(e));
            }
        };
        // 5. Persist error or severe warning, fallback on DB error
        if let Err(e) = self.db.insert_error(&evt, msg_id).await {
            error!(error = ?e, event_id = %evt.event_id, "DB insert_error failed, falling back to temp file");
            let fallback = to_string(&evt)?;
            if let Err(e2) = self.file_writer.write_temp(&fallback).await {
                error!(error = ?e2, event_id = %evt.event_id, "Failed to write fallback temp file");
            }
            return Err(HandlerError::Db(e));
        }
        debug!(event_id = %evt.event_id, "Error event inserted into DB");
        // 6. Append to JSONL
        let line = to_string(&evt)?;
        if let Err(e) = self.file_writer.write_jsonl(&line).await {
            error!(error = ?e, event_id = %evt.event_id, "Failed to write error event to JSONL");
            return Err(e.into());
        }
        info!(event_id = %evt.event_id, user_id=?evt.user_id, session_id=?evt.session_id, request_id=?evt.request_id, "Error event written to JSONL");
        Ok(())
    }
    /// Returns the buffered snapshots of info and error events.
    pub async fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>) {
        let guard = self.buffer.lock().await;
        guard.snapshot().await
    }
} // spawns background rotation task separately

#[async_trait::async_trait]
impl<F, B, D> ErrorLogger for Handler<F, B, D>
where
    F: FileWriter + 'static,
    B: BufferManager + 'static,
    D: DbClient + 'static,
{
    async fn log_error(&self, evt: crate::error_handler::types::ErrorEvent) -> Result<(), ()> {
        Handler::log_error(self, evt).await.map_err(|_| ())
    }
}
