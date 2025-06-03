use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Severity levels for events: Error Severe, Error Minor, Warning Severe, Warning Minor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    ES,
    EM,
    WS,
    WM,
} // async_trait needed for async fn in traits 

/// Components of the system: Compression, Hashing, Encryption
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Component {
    C,
    H,
    E,
} // directory‑based modules recommended for organization 

/// Actors responsible: User, Server, Network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    U,
    S,
    N,
}

/// Simple wrapper for informational events
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Clone)]
pub struct LogEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub context: Value,
    pub info_id: Option<String>,
    // Enhanced context fields
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
}

/// Detailed wrapper for warnings & errors
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Clone)]
pub struct ErrorEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: Severity,
    pub component: Component,
    pub actor: Actor,
    pub code: u32,
    pub message: String,
    pub context: Value,
    pub stack_trace: Option<Value>,
    // Enhanced context fields
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
}

impl ErrorEvent {
    /// Sanitize and truncate the message to prevent log injection and DoS
    pub fn sanitize_and_truncate_message(&mut self, max_len: usize) {
        let sanitized = self.message
            .replace(['\n', '\r', '\t'], " ")
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>();
        self.message = if sanitized.len() > max_len {
            sanitized[..max_len].to_string()
        } else {
            sanitized
        };
    }
    /// Redact sensitive fields in context and stack_trace
    pub fn redact_sensitive_data(&mut self) {
        // Example: redact fields named "password", "token", "secret"
        fn redact_value(val: &mut Value) {
            match val {
                Value::Object(map) => {
                    for (k, v) in map.iter_mut() {
                        if ["password", "token", "secret"].contains(&k.as_str()) {
                            *v = Value::String("***REDACTED***".to_string());
                        } else {
                            redact_value(v);
                        }
                    }
                }
                Value::Array(arr) => {
                    for v in arr.iter_mut() {
                        redact_value(v);
                    }
                }
                _ => {}
            }
        }
        redact_value(&mut self.context);
        if let Some(ref mut st) = self.stack_trace {
            redact_value(st);
        }
    }
}

/// Errors returned by the handler entrypoint
#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Validation failed: {0}")]
    Validation(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}
