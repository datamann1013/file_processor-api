mod error_handler;

// src/main.rs
use file_processor_api::error_handler::{Handler, FileWriter, BufferManager, DbClient, LogEvent, ErrorEvent, Severity, Component, Actor};
use async_trait::async_trait;
use uuid::Uuid;
use serde_json::json;

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
impl BufferManager for DummyBuffer {
    fn buffer_info(&self, event: &LogEvent) {
        println!("Buffering info: {:?}", event);
    }
    fn buffer_warning(&self, event: &ErrorEvent) {
        println!("Buffering warning: {:?}", event);
    }


    fn buffer_error(&self, event: &ErrorEvent) {
        println!("Buffering error: {:?}", event);
    }
    fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>) {
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
    async fn insert_error(
        &self,
        evt: &ErrorEvent,
        msg_id: Uuid,
    ) -> Result<(), sqlx::Error> {
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
        message: "Test log".into(),
        context: json!({"key": "value"}),
        info_id: None,
    };
    handler.log_event(log_evt).await.unwrap();

    let err_evt = ErrorEvent {
        message: "Test error".into(),
        context: Default::default(),
        severity: Severity::EM,
        component: Component::H,
        actor: Actor::S,
        code: 0,
        stack_trace: None,
    };
    handler.log_error(err_evt).await.unwrap();

    let (infos, errors) = handler.snapshot().await;

    println!("Snapshots - infos: {:?}, errors: {:?}", infos, errors);
}
