//! Runtime module - Actor-based service runtime and handler loading (v2)
//!
//! This module provides the v2 architecture:
//! - Context API for handlers
//! - Actor-based services (database, cache, storage)
//! - Dynamic library handler loading with hot-swap
//! - Graceful handler draining for zero-downtime deployments
//! - Service lifecycle management
//! - Bundle deployment system

pub mod context;
pub mod services;
pub mod handler;
pub mod actor;
pub mod bundle;

pub use context::Context;
pub use services::Services;
pub use handler::HandlerRegistry;
pub use bundle::BundleManifest;