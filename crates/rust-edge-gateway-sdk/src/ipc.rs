//! IPC protocol for communicating with the Rust Edge Gateway.
//!
//! Handlers communicate with the gateway using a simple length-prefixed JSON protocol
//! over stdin/stdout.
//!
//! # Handler Macros
//!
//! The SDK provides several macros for different handler patterns:
//!
//! ## `handler_loop!` - Synchronous handlers
//! ```ignore
//! fn handle(req: Request) -> Response {
//!     Response::ok(json!({"message": "Hello"}))
//! }
//! handler_loop!(handle);
//! ```
//!
//! ## `handler_loop_result!` - Handlers returning Result
//! ```ignore
//! fn handle(req: Request) -> Result<Response, HandlerError> {
//!     let data: MyInput = req.json()?;
//!     Ok(Response::ok(data))
//! }
//! handler_loop_result!(handle);
//! ```
//!
//! ## `handler_loop_async!` - Async handlers (requires `async` feature)
//! ```ignore
//! async fn handle(req: Request) -> Response {
//!     let data = fetch_from_db().await;
//!     Response::ok(data)
//! }
//! handler_loop_async!(handle);
//! ```

use crate::{Request, Response, HandlerError};
use serde::de::DeserializeOwned;
use std::io::{self, Read, Write};

/// Read a request from stdin (sent by the gateway)
pub fn read_request() -> Result<Request, HandlerError> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Read length prefix (4 bytes, big-endian)
    let mut len_buf = [0u8; 4];
    if handle.read_exact(&mut len_buf).is_err() {
        return Err(HandlerError::IpcError("Failed to read length prefix".into()));
    }

    let len = u32::from_be_bytes(len_buf) as usize;

    // Read the JSON payload
    let mut payload = vec![0u8; len];
    if handle.read_exact(&mut payload).is_err() {
        return Err(HandlerError::IpcError("Failed to read payload".into()));
    }

    // Parse the request
    serde_json::from_slice(&payload)
        .map_err(|e| HandlerError::IpcError(format!("Failed to parse request: {}", e)))
}

/// Send a response to stdout (received by the gateway)
pub fn send_response(response: Response) -> Result<(), HandlerError> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Serialize the response
    let payload = serde_json::to_vec(&response)
        .map_err(|e| HandlerError::IpcError(format!("Failed to serialize response: {}", e)))?;

    // Write length prefix
    let len = payload.len() as u32;
    handle.write_all(&len.to_be_bytes())
        .map_err(|e| HandlerError::IpcError(format!("Failed to write length: {}", e)))?;

    // Write payload
    handle.write_all(&payload)
        .map_err(|e| HandlerError::IpcError(format!("Failed to write payload: {}", e)))?;

    handle.flush()
        .map_err(|e| HandlerError::IpcError(format!("Failed to flush: {}", e)))?;

    Ok(())
}

/// Call a service through the gateway (for DB, Redis, etc.)
/// This sends a service request and waits for the response
pub fn call_service<T: DeserializeOwned>(request: serde_json::Value) -> Result<T, HandlerError> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    // Serialize the service request
    let payload = serde_json::to_vec(&request)
        .map_err(|e| HandlerError::IpcError(format!("Failed to serialize service request: {}", e)))?;

    // Write to stderr (service channel)
    let len = payload.len() as u32;
    handle.write_all(&len.to_be_bytes())
        .map_err(|e| HandlerError::IpcError(format!("Failed to write service request length: {}", e)))?;
    handle.write_all(&payload)
        .map_err(|e| HandlerError::IpcError(format!("Failed to write service request: {}", e)))?;
    handle.flush()
        .map_err(|e| HandlerError::IpcError(format!("Failed to flush service request: {}", e)))?;

    // Read response from stdin (interleaved with requests)
    // In practice, the gateway will handle this properly
    // For now, this is a placeholder

    // TODO: Implement proper bidirectional IPC
    Err(HandlerError::IpcError("Service calls not yet implemented".into()))
}

/// Convenience macro for running a synchronous handler loop.
///
/// The handler function takes a `Request` and returns a `Response`.
///
/// # Example
/// ```ignore
/// use rust_edge_gateway_sdk::prelude::*;
///
/// fn handle(req: Request) -> Response {
///     Response::ok(json!({"path": req.path}))
/// }
///
/// handler_loop!(handle);
/// ```
#[macro_export]
macro_rules! handler_loop {
    ($handler:expr) => {
        fn main() {
            loop {
                match $crate::ipc::read_request() {
                    Ok(req) => {
                        let response = $handler(req);
                        if let Err(e) = $crate::ipc::send_response(response) {
                            eprintln!("Failed to send response: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read request: {}", e);
                        break;
                    }
                }
            }
        }
    };
}

/// Convenience macro for running a handler that returns `Result<Response, HandlerError>`.
///
/// Errors are automatically converted to HTTP responses using `HandlerError::to_response()`.
///
/// # Example
/// ```ignore
/// use rust_edge_gateway_sdk::prelude::*;
///
/// fn handle(req: Request) -> Result<Response, HandlerError> {
///     let data: MyInput = req.json()?;  // ? works naturally
///     let id = req.require_path_param::<i64>("id")?;
///     Ok(Response::ok(json!({"id": id, "data": data})))
/// }
///
/// handler_loop_result!(handle);
/// ```
#[macro_export]
macro_rules! handler_loop_result {
    ($handler:expr) => {
        fn main() {
            loop {
                match $crate::ipc::read_request() {
                    Ok(req) => {
                        let response = match $handler(req) {
                            Ok(resp) => resp,
                            Err(e) => e.to_response(),
                        };
                        if let Err(e) = $crate::ipc::send_response(response) {
                            eprintln!("Failed to send response: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read request: {}", e);
                        break;
                    }
                }
            }
        }
    };
}

/// Convenience macro for running an async handler loop.
///
/// This creates a Tokio runtime internally, so you don't need to manage it yourself.
/// Requires the `async` feature to be enabled.
///
/// # Cargo.toml
/// ```toml
/// [dependencies]
/// rust-edge-gateway-sdk = { version = "0.1", features = ["async"] }
/// ```
///
/// # Example
/// ```ignore
/// use rust_edge_gateway_sdk::prelude::*;
///
/// async fn handle(req: Request) -> Response {
///     let data = fetch_from_database().await;
///     Response::ok(data)
/// }
///
/// handler_loop_async!(handle);
/// ```
#[cfg(feature = "async")]
#[macro_export]
macro_rules! handler_loop_async {
    ($handler:expr) => {
        fn main() {
            // Create a single runtime for all requests
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime");

            loop {
                match $crate::ipc::read_request() {
                    Ok(req) => {
                        let response = rt.block_on($handler(req));
                        if let Err(e) = $crate::ipc::send_response(response) {
                            eprintln!("Failed to send response: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read request: {}", e);
                        break;
                    }
                }
            }
        }
    };
}

/// Convenience macro for running an async handler that returns `Result<Response, HandlerError>`.
///
/// Combines async support with automatic error conversion.
/// Requires the `async` feature to be enabled.
///
/// # Example
/// ```ignore
/// use rust_edge_gateway_sdk::prelude::*;
///
/// async fn handle(req: Request) -> Result<Response, HandlerError> {
///     let data: MyInput = req.json()?;
///     let result = database.insert(&data).await
///         .map_err(|e| HandlerError::DatabaseError(e.to_string()))?;
///     Ok(Response::created(result))
/// }
///
/// handler_loop_async_result!(handle);
/// ```
#[cfg(feature = "async")]
#[macro_export]
macro_rules! handler_loop_async_result {
    ($handler:expr) => {
        fn main() {
            // Create a single runtime for all requests
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime");

            loop {
                match $crate::ipc::read_request() {
                    Ok(req) => {
                        let response = match rt.block_on($handler(req)) {
                            Ok(resp) => resp,
                            Err(e) => e.to_response(),
                        };
                        if let Err(e) = $crate::ipc::send_response(response) {
                            eprintln!("Failed to send response: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read request: {}", e);
                        break;
                    }
                }
            }
        }
    };
}

