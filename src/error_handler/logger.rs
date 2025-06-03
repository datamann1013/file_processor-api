use super::types::ErrorEvent;

#[async_trait::async_trait]
pub trait ErrorLogger: Send + Sync {
    async fn log_error(&self, evt: ErrorEvent) -> Result<(), ()>;
}
