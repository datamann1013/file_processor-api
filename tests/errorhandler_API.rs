                    // brings Handler, LogEvent, ErrorEvent, HandlerError into scope
use serde_json::json;
use uuid::Uuid;
use mockall::predicate::*;
use mockall::*;

// 1) Generate mocks for our traits:

mock! {
    pub FileWriter {}
    #[async_trait::async_trait]
    impl FileWriter for FileWriter {
        async fn write_jsonl(&self, line: &str) -> std::io::Result<()>;
        async fn write_temp(&self, line: &str) -> std::io::Result<()>;
    }
}

mock! {
    pub BufferManager {}
    impl BufferManager for BufferManager {
        fn buffer_info(&self, event: &LogEvent);
        fn buffer_warning(&self, event: &ErrorEvent);
        fn buffer_error(&self, event: &ErrorEvent);
        fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>);
    }
}

mock! {
    pub DbClient {}
    #[async_trait::async_trait]
    impl DbClient for DbClient {
        async fn insert_message(&self, text: &str) -> Result<Uuid, sqlx::Error>;
        async fn insert_error(
            &self,
            evt: &ErrorEvent,
            msg_id: Uuid
        ) -> Result<(), sqlx::Error>;
        async fn replay_temp(&self, lines: Vec<String>) -> Result<(), sqlx::Error>;
    }
}

// 2) Helper to produce a valid ErrorEvent
fn valid_error_event() -> ErrorEvent {
    ErrorEvent {
        severity: Severity::ES,
        component: Component::C,
        actor: Actor::U,
        code: 1,
        message: "valid".into(),
        context: json!({"key":"value"}),
        stack_trace: None,
    }
}

#[tokio::test]
async fn log_event_writes_info_only() {
    let mut fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let db = MockDbClient::new();

    // Expect exactly one JSONL write; DB never touched
    fw.expect_write_jsonl()
        .times(1)
        .with(eq("{\"message\":\"test\",\"context\":{},\"info_id\":\"INFO1\"}"))
        .returning(|_| Ok(()));
    db.expect_insert_error().never();

    let handler = Handler::new(fw, buf, db);
    let evt = LogEvent {
        message: "test".into(),
        context: json!({}),
        info_id: Some("INFO1".into()),
    };

    assert!(handler.log_event(evt).await.is_ok());
}

#[tokio::test]
async fn log_error_validation_fails_on_empty_message() {
    let fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let db = MockDbClient::new();

    let handler = Handler::new(fw, buf, db);
    let evt = ErrorEvent {
        severity: Severity::ES,
        component: Component::C,
        actor: Actor::U,
        code: 1,
        message: "".into(),
        context: json!({}),
        stack_trace: None,
    };

    let err = handler.log_error(evt).await.unwrap_err();
    match err {
        HandlerError::Validation(msg) => assert!(msg.contains("Empty message")),
        _ => panic!("Expected Validation error"),
    };
}

#[tokio::test]
async fn log_error_db_failure_falls_back_to_file() {
    let mut fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    // Simulate DB insert_error failing once
    db.expect_insert_message()
        .times(1)
        .returning(|_| Ok(Uuid::new_v4()));
    db.expect_insert_error()
        .times(1)
        .returning(|_, _| Err(sqlx::Error::RowNotFound));
    // On DB failure, a temp write must occur
    fw.expect_write_temp()
        .times(1)
        .returning(|_| Ok(()));

    let handler = Handler::new(fw, buf, db);
    let evt = valid_error_event();

    let res = handler.log_error(evt).await;
    assert!(matches!(res, Err(HandlerError::Db(_))));
}
