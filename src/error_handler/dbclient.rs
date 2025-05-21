use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// Abstracts async DB operations: message lookup/insert, error insert, temp replay
#[async_trait]
pub trait DbClient: Send + Sync {
    async fn insert_message(&self, text: &str) -> Result<Uuid, sqlx::Error>;
    async fn insert_error(
        &self,
        evt: &crate::error_handler::types::ErrorEvent,
        msg_id: Uuid,
    ) -> Result<(), sqlx::Error>;
    async fn replay_temp(&self, lines: Vec<String>) -> Result<(), sqlx::Error>;
} // SQLx for async DB backed by mTLS connections 
