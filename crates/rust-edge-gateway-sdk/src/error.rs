//! Error types for Rust Edge Gateway handlers

use thiserror::Error;

/// Errors that can occur in a handler
#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("IPC error: {0}")]
    IpcError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFoundMessage(String),

    #[error("Not found")]
    NotFound,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl HandlerError {
    /// Convert the error to an HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            HandlerError::ValidationError(_) => 400,
            HandlerError::Unauthorized(_) => 401,
            HandlerError::NotFound | HandlerError::NotFoundMessage(_) => 404,
            HandlerError::ServiceUnavailable(_) => 503,
            _ => 500,
        }
    }

    /// Convert to a Response
    pub fn to_response(&self) -> crate::Response {
        crate::Response::json(
            self.status_code(),
            serde_json::json!({
                "error": self.to_string()
            }),
        )
    }
}

