use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Severity levels for events: Error Severe, Error Minor, Warning Severe, Warning Minor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity { ES, EM, WS, WM }  // async_trait needed for async fn in traits 

/// Components of the system: Compression, Hashing, Encryption
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Component { C, H, E }  // directory‑based modules recommended for organization 

/// Actors responsible: User, Server, Network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor { U, S, N }

/// Simple wrapper for informational events
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEvent {
    pub message: String,
    pub context: Value,
    pub info_id: Option<String>,
}

/// Detailed wrapper for warnings & errors
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub severity: Severity,
    pub component: Component,
    pub actor: Actor,
    pub code: u32,
    pub message: String,
    pub context: Value,
    pub stack_trace: Option<Value>,
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

