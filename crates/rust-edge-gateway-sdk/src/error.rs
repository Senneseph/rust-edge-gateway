//! Error types for Rust Edge Gateway handlers

use thiserror::Error;

/// Errors that can occur in a handler.
///
/// These errors automatically map to appropriate HTTP status codes
/// and can be converted to Response using `.to_response()` or
/// the `From<HandlerError>` implementation for `Response`.
///
/// # Example
/// ```ignore
/// fn handle(req: Request) -> Response {
///     match process(&req) {
///         Ok(data) => Response::ok(data),
///         Err(e) => e.into(), // Automatically converts to Response
///     }
/// }
/// ```
#[derive(Error, Debug)]
pub enum HandlerError {
    /// Bad request (400) - Invalid input, malformed JSON, missing fields
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Validation error (400) - Semantic validation failures
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Unauthorized (401) - Missing or invalid authentication
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Forbidden (403) - Authenticated but not authorized
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Not found (404) - Resource not found
    #[error("Not found")]
    NotFound,

    /// Not found with message (404)
    #[error("Not found: {0}")]
    NotFoundMessage(String),

    /// Method not allowed (405)
    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    /// Conflict (409) - Resource conflict (e.g., duplicate)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Payload too large (413)
    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    /// IPC error (500) - Internal communication failure
    #[error("IPC error: {0}")]
    IpcError(String),

    /// Serialization error (500)
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Database error (500)
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Redis error (500)
    #[error("Redis error: {0}")]
    RedisError(String),

    /// Storage error (500)
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Internal error (500) - Generic internal server error
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Internal error (500) - Alias for InternalError
    #[error("Internal error: {0}")]
    Internal(String),

    /// Service unavailable (503)
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl HandlerError {
    /// Convert the error to an HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            HandlerError::BadRequest(_) => 400,
            HandlerError::ValidationError(_) => 400,
            HandlerError::Unauthorized(_) => 401,
            HandlerError::Forbidden(_) => 403,
            HandlerError::NotFound | HandlerError::NotFoundMessage(_) => 404,
            HandlerError::MethodNotAllowed(_) => 405,
            HandlerError::Conflict(_) => 409,
            HandlerError::PayloadTooLarge(_) => 413,
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

/// Allows using `HandlerError` directly as a `Response` via `.into()` or `?` operator
impl From<HandlerError> for crate::Response {
    fn from(err: HandlerError) -> Self {
        err.to_response()
    }
}

