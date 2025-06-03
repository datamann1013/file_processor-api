use async_trait::async_trait;
use file_processor_api::error_handler::{
    Actor, BufferManager, Component, DbClient, ErrorEvent, FileWriter, Handler, LogEvent, Severity,
};
use file_processor_api::error_handler::logger::ErrorLogger;
use file_processor_api::api_connector::{
    ApiRouter, ApiConnector, Handler as ApiHandler, HandlerResult, ServiceId, ApiRequest, ApiResponse, ApiError
};
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;

// Dummy implementations for error handler traits
struct DummyWriter;
#[async_trait]
impl FileWriter for DummyWriter {
    async fn write_jsonl(&self, line: &str) -> std::io::Result<()> {
        println!("Writing JSONL: {}", line);
        Ok(())
    }
    async fn write_temp(&self, line: &str) -> std::io::Result<()> {
        println!("Writing temp line: {}", line);
        Ok(())
    }
}

struct DummyBuffer;
#[async_trait]
impl BufferManager for DummyBuffer {
    async fn buffer_info(&self, event: &LogEvent) {
        println!("Buffering info: {:?}", event);
    }
    async fn buffer_warning(&self, event: &ErrorEvent) {
        println!("Buffering warning: {:?}", event);
    }
    async fn buffer_error(&self, event: &ErrorEvent) {
        println!("Buffering error: {:?}", event);
    }
    async fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>) {
        (vec![], vec![])
    }
}

struct DummyDb;
#[async_trait]
impl DbClient for DummyDb {
    async fn insert_message(&self, _msg: &str) -> Result<Uuid, sqlx::Error> {
        println!("Inserting message into DB");
        Ok(Uuid::new_v4())
    }
    async fn insert_error(&self, evt: &ErrorEvent, msg_id: Uuid) -> Result<(), sqlx::Error> {
        println!("Inserting error for msg_id={} evt={:?}", msg_id, evt);
        Ok(())
    }
    async fn replay_temp(&self, _lines: Vec<String>) -> Result<(), sqlx::Error> {
        println!("Replaying temp lines");
        Ok(())
    }
}

// Dummy API handler for demonstration
struct DummyApiHandler;
#[async_trait]
impl ApiHandler for DummyApiHandler {
    async fn handle(&self, req: &[u8]) -> HandlerResult {
        println!("DummyApiHandler received: {:?}", req);
        Ok(vec![42, 43, 44])
    }
}

#[tokio::main]
async fn main() {
    // Set up error handler
    let handler = Handler::new(DummyWriter, DummyBuffer, DummyDb);
    let error_logger: Arc<dyn ErrorLogger + Send + Sync> = Arc::new(handler);

    // Set up API router and connector
    let mut router = ApiRouter::new(error_logger.clone());
    router.register_handler(ServiceId::Compression, Box::new(DummyApiHandler));
    let api = ApiConnector::new(router);

    // Use: valid service request
    let req = ApiRequest::new(ServiceId::Compression, vec![1, 2, 3]);
    match api.handle_request(req).await {
        Ok(ApiResponse { data, status }) => {
            println!("API Response: {:?}, Status: {:?}", data, status);
        }
        Err(e) => println!("API Error: {:?}", e),
    }

    // Use: unknown service request (should log error via error handler)
    let req = ApiRequest::new(ServiceId::Unknown(Uuid::new_v4()), vec![]);
    match api.handle_request(req).await {
        Ok(resp) => println!("Unexpected success: {:?}", resp),
        Err(e) => println!("API Error (expected unknown service): {:?}", e),
    }
}
