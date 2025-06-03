use super::types::*;
use crate::error_handler::logger::ErrorLogger;
use crate::error_handler::{ErrorEvent, Severity, Component, Actor};
use serde_json::json;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, req: &[u8]) -> HandlerResult;
}

pub struct ApiRouter {
    handlers: Mutex<HashMap<ServiceId, Arc<dyn Handler>>>,
    error_handler: Arc<dyn ErrorLogger + Send + Sync>,
}

impl ApiRouter {
    pub fn new(error_handler: Arc<dyn ErrorLogger + Send + Sync>) -> Self {
        Self { handlers: Mutex::new(HashMap::new()), error_handler }
    }
    pub fn register_handler(&mut self, id: ServiceId, handler: Box<dyn Handler>) {
        self.handlers.lock().unwrap().insert(id, Arc::from(handler));
    }
    pub async fn route(&self, req: &ApiRequest) -> HandlerResult {
        let handlers = self.handlers.lock().unwrap();
        if let Some(handler) = handlers.get(&req.service_id) {
            handler.handle(&req.payload).await
        } else {
            // Log error event
            let error_event = ErrorEvent {
                event_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                severity: Severity::WM,
                component: Component::A,
                actor: Actor::S,
                code: 1001,
                message: "Unknown service requested".into(),
                context: json!({"service_id": format!("{:?}", req.service_id)}),
                stack_trace: None,
                user_id: None,
                session_id: None,
                request_id: None,
            };
            let _ = self.error_handler.log_error(error_event).await;
            Err(ApiError::UnknownService)
        }
    }
}

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
