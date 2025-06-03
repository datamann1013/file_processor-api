use file_processor_api::api_connector::*;
use file_processor_api::error_handler::logger::ErrorLogger;
use file_processor_api::error_handler::{Actor, Component, ErrorEvent, Severity};
use mockall::mock;
use std::sync::Arc;
use uuid::Uuid;

mock! {
    pub ErrorLogger {}
    #[async_trait::async_trait]
    impl ErrorLogger for ErrorLogger {
        async fn log_error(&self, evt: ErrorEvent) -> Result<(), ()>;
    }
}

mock! {
    pub Handler {}
    #[async_trait::async_trait]
    impl Handler for Handler {
        async fn handle(&self, req: &[u8]) -> HandlerResult;
    }
}

#[tokio::test]
async fn test_unknown_service_logs_error_event() {
    let mut mock_error_logger = MockErrorLogger::new();
    mock_error_logger
        .expect_log_error()
        .times(1)
        .withf(|evt| {
            evt.component == Component::A && evt.severity == Severity::WM && evt.code == 1001
        })
        .returning(|_| Ok(()));

    let error_handler = Arc::new(mock_error_logger);
    let mut router = ApiRouter::new(error_handler.clone());

    let req = ApiRequest::new(ServiceId::Unknown(Uuid::new_v4()), vec![]);
    let result = router.route(&req).await;
    assert!(matches!(result, Err(ApiError::UnknownService)));
}

#[tokio::test]
async fn test_valid_service_does_not_log_error() {
    let mut mock_error_logger = MockErrorLogger::new();
    // Should not be called for valid service
    mock_error_logger.expect_log_error().times(0);

    let error_handler = Arc::new(mock_error_logger);
    let mut router = ApiRouter::new(error_handler.clone());

    let mut mock_handler = MockHandler::new();
    mock_handler
        .expect_handle()
        .times(1)
        .returning(|_| Ok(vec![1, 2, 3]));
    router.register_handler(ServiceId::Compression, Box::new(mock_handler));

    let req = ApiRequest::new(ServiceId::Compression, vec![0xAA]);
    let result = router.route(&req).await;
    assert_eq!(result.unwrap(), vec![1, 2, 3]);
}
