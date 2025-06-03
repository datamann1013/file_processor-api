use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ServiceId {
    Compression,
    Encryption,
    Hashing,
    Metadata,
    Custom(String),
    Unknown(Uuid),
}

#[derive(Clone, Debug)]
pub struct ApiRequest {
    pub service_id: ServiceId,
    pub payload: Vec<u8>,
    // user/session/request context fields could be added here
}

impl ApiRequest {
    pub fn new(service_id: ServiceId, payload: Vec<u8>) -> Self {
        Self { service_id, payload }
    }
}

#[derive(Clone, Debug)]
pub struct ApiResponse {
    pub data: Vec<u8>,
    pub status: ApiStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiStatus {
    Ok,
    Error(ApiError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    UnknownService,
    Unauthorized,
    Internal,
    // ...
}

pub type HandlerResult = Result<Vec<u8>, ApiError>;
