
#[tokio::test]
async fn log_event_writes_info_only() {
    let mut fw = MockFileWriter::new();
    let mut buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    // file_writer.write_jsonl called once, db.write never called
    fw.expect_write_jsonl().times(1).returning(|_s| Ok(()));
    db.expect_insert_error().never();

    let handler = Handler::new(fw, buf, db);
    let evt = LogEvent { message: "test".into(), context: json!({}), info_id: Some("INFO1".into()) };

    assert!(handler.log_event(evt).await.is_ok());
}

#[tokio::test]
async fn log_error_validation_fails_on_empty_message() {
    let handler = Handler::new(/* dummy mocks */);
    let evt = ErrorEvent { message: "".into(), /* ... */ };

    let err = handler.log_error(evt).await.unwrap_err();
    matches!(err, HandlerError::Validation(_));
}

#[tokio::test]
async fn log_error_db_failure_falls_back_to_file() {
    let mut fw = MockFileWriter::new();
    let buf = MockBufferManager::new();
    let mut db = MockDbClient::new();

    db.expect_insert_error().times(1).returning(|| Err(sqlx::Error::RowNotFound));
    fw.expect_write_temp().times(1).returning(|_s| Ok(()));

    let handler = Handler::new(fw, buf, db);
    let evt = valid_error_event();
    assert!(handler.log_error(evt).await.is_err());
}

// More tests: buffer rollover, deduplication logic, temp-file replay, etc.
