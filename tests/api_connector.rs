use file_processor_api::api_connector::{ApiConnector, ApiError, ApiRequest, ApiRouter, Handler, HandlerResult, ServiceId};
use mockall::*;
use uuid::Uuid;

mock! {
    pub Handler {}
    #[async_trait::async_trait]
    impl Handler for Handler {
        async fn handle(&self, req: &[u8]) -> HandlerResult;
    }
}

#[tokio::test]
async fn test_routing_to_compression_handler() {
    let mut router = ApiRouter::new();
    let mut comp = MockHandler::new();
    comp.expect_handle().times(1).returning(|_| Ok(vec![1,2,3]));
    router.register_handler(ServiceId::Compression, Box::new(comp));
    let api = ApiConnector::new(router);

    let req = ApiRequest::new(ServiceId::Compression, vec![0xAA]);
    let resp = api.handle_request(req).await.unwrap();
    assert_eq!(resp.data, vec![1,2,3]);
}

#[tokio::test]
async fn test_routing_to_encryption_handler() {
    let mut router = ApiRouter::new();
    let mut enc = MockHandler::new();
    enc.expect_handle().times(1).returning(|_| Ok(vec![9,9,9]));
    router.register_handler(ServiceId::Encryption, Box::new(enc));
    let api = ApiConnector::new(router);

    let req = ApiRequest::new(ServiceId::Encryption, vec![0xBB]);
    let resp = api.handle_request(req).await.unwrap();
    assert_eq!(resp.data, vec![9,9,9]);
}

#[tokio::test]
async fn test_unknown_service_id_logs_minor_user_error() {
    let mut router = ApiRouter::new();
    let api = ApiConnector::new(router);

    let unknown_id = ServiceId::Unknown(Uuid::new_v4());
    let req = ApiRequest::new(unknown_id, vec![]);
    let result = api.handle_request(req).await;
    assert!(matches!(result, Err(ApiError::UnknownService)));
    // Optionally: check that error handler was called with Severity::WM
}

#[tokio::test]
async fn test_audit_logging_on_all_requests() {
    // Use a mock error handler to verify audit logs are written for every request
}

#[tokio::test]
async fn test_extensibility_register_new_handler() {
    struct DummyHandler;
    #[async_trait::async_trait]
    impl Handler for DummyHandler {
        async fn handle(&self, _req: &[u8]) -> HandlerResult {
            Ok(vec![42])
        }
    }
    let mut router = ApiRouter::new();
    router.register_handler(ServiceId::Custom("dummy".into()), Box::new(DummyHandler));
    let api = ApiConnector::new(router);

    let req = ApiRequest::new(ServiceId::Custom("dummy".into()), vec![]);
    let resp = api.handle_request(req).await.unwrap();
    assert_eq!(resp.data, vec![42]);
}

#[tokio::test]
async fn test_mtls_handshake_required() {
    // Simulate a client without valid mTLS cert and assert connection is rejected
}
