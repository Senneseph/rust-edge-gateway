//! Rust Edge Gateway SDK - Types and utilities for writing Rust Edge Gateway handlers
//!
//! This crate provides the core types and traits that handlers use to interact
//! with the Rust Edge Gateway platform.

pub mod request;
pub mod response;
pub mod services;
pub mod storage;
pub mod ipc;
pub mod error;
pub mod sqlite;

pub mod prelude {
    //! Common imports for Rust Edge Gateway handlers
    pub use crate::request::Request;
    pub use crate::response::Response;
    pub use crate::services::*;
    pub use crate::storage::{Storage, StorageType};
    pub use crate::sqlite::SqliteClient;
    pub use crate::ipc::{read_request, send_response};
    pub use crate::error::HandlerError;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::{json, Value as JsonValue};
}

// Re-export key types at crate root
pub use request::Request;
pub use response::Response;
pub use error::HandlerError;
pub use storage::Storage;
pub use sqlite::SqliteClient;

