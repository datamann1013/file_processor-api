use super::types::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, req: &[u8]) -> HandlerResult;
}

// Only use the Handler trait for all mocks and implementations

pub struct ApiRouter {
    handlers: Mutex<HashMap<ServiceId, Arc<dyn Handler>>>,
}

impl ApiRouter {
    pub fn new() -> Self {
        Self { handlers: Mutex::new(HashMap::new()) }
    }
    pub fn register_handler(&mut self, id: ServiceId, handler: Box<dyn Handler>) {
        self.handlers.lock().unwrap().insert(id, Arc::from(handler));
    }
    pub async fn route(&self, req: &ApiRequest) -> HandlerResult {
        let handlers = self.handlers.lock().unwrap();
        if let Some(handler) = handlers.get(&req.service_id) {
            handler.handle(&req.payload).await
        } else {
            Err(ApiError::UnknownService)
        }
    }
}

// Minimal ApiConnector for test compatibility
pub struct ApiConnector {
    router: ApiRouter,
}

impl ApiConnector {
    pub fn new(router: ApiRouter) -> Self {
        Self { router }
    }
    pub async fn handle_request(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        match self.router.route(&req).await {
            Ok(data) => Ok(ApiResponse { data, status: ApiStatus::Ok }),
            Err(e) => Err(e),
        }
    }
}
