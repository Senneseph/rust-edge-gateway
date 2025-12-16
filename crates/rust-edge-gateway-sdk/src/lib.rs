//! Rust Edge Gateway SDK - Types and utilities for writing Rust Edge Gateway handlers
//!
//! This crate provides the core types and traits that handlers use to interact
//! with the Rust Edge Gateway platform.
//!
//! # Handler Types
//!
//! ## V1 Handlers (Subprocess-based)
//! Use `handler_loop!` macros for handlers that run as separate processes.
//!
//! ## V2 Handlers (Dynamic Library)
//! Use `handler!` macros for handlers that are loaded as shared libraries.
//!
//! # Example (V2)
//!
//! ```ignore
//! use rust_edge_gateway_sdk::prelude::*;
//!
//! handler!(async fn my_handler(ctx: &Context, req: Request) -> Response {
//!     let id: i64 = req.path_param_as("id").unwrap_or(0);
//!     Response::ok(json!({"id": id}))
//! });
//! ```

pub mod request;
pub mod response;
pub mod services;
pub mod storage;
pub mod ipc;
pub mod error;
pub mod sqlite;
pub mod handler;

pub mod prelude {
    //! Common imports for Rust Edge Gateway handlers
    //!
    //! This module re-exports everything you need to write handlers:
    //! - Request and Response types
    //! - Service clients
    //! - Handler macros
    //! - Error types
    //! - Serialization helpers
    
    pub use crate::request::Request;
    pub use crate::response::Response;
    pub use crate::services::*;
    pub use crate::storage::{Storage, StorageType};
    pub use crate::sqlite::SqliteClient;
    pub use crate::ipc::{read_request, send_response};
    pub use crate::error::HandlerError;
    pub use crate::handler::{BoxFuture, HandlerFn, HandlerContext};
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::{json, Value as JsonValue};
    
    // Re-export v2 handler macros (new style)
    pub use crate::{handler, handler_result};
    
    // V1 handler macros are already exported from ipc module
    pub use crate::{handler_loop, handler_loop_result};
    
    #[cfg(feature = "async")]
    pub use crate::{handler_loop_async, handler_loop_async_result};
}

// Re-export key types at crate root
pub use request::Request;
pub use response::Response;
pub use error::HandlerError;
pub use storage::Storage;
pub use sqlite::SqliteClient;
pub use handler::{BoxFuture, HandlerFn};

