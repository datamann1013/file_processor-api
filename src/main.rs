use async_trait::async_trait;
use file_processor_api::error_handler::{
    Actor, BufferManager, Component, DbClient, ErrorEvent, FileWriter, Handler, LogEvent, Severity,
};
use serde_json::json;
use uuid::Uuid;

// Dummy implementations for traits
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
        // your logic here…
        println!("Inserting error for msg_id={} evt={:?}", msg_id, evt);
        Ok(())
    }
    async fn replay_temp(&self, _lines: Vec<String>) -> Result<(), sqlx::Error> {
        println!("Replaying temp lines");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let handler = Handler::new(DummyWriter, DummyBuffer, DummyDb);

    let log_evt = LogEvent {
        event_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        message: "Test log".into(),
        context: json!({"key": "value"}),
        info_id: None,
        user_id: Some("user123".into()),
        session_id: Some("sess456".into()),
        request_id: Some("req789".into()),
    };
    handler.log_event(log_evt).await.unwrap();

    let err_evt = ErrorEvent {
        event_id: Uuid::new_v4(),
        message: "Test error".into(),
        context: json!({}),
        severity: Severity::EM,
        component: Component::H,
        actor: Actor::S,
        code: 0,
        stack_trace: None,
        timestamp: chrono::Utc::now(),
        user_id: Some("user123".into()),
        session_id: Some("sess456".into()),
        request_id: Some("req789".into()),
    };
    handler.log_error(err_evt).await.unwrap();

    let (infos, errors) = handler.snapshot().await;

    println!("Snapshots - infos: {:?}, errors: {:?}", infos, errors);
}
