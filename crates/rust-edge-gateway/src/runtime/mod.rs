//! Runtime module - Actor-based service runtime and handler loading
//!
//! This module provides:
//! - Context API for handlers
//! - Actor-based services (database, cache, storage)
//! - Dynamic library handler loading
//! - Service lifecycle management
//! - Bundle deployment system

pub mod context;
pub mod services;
pub mod handler;
pub mod actor;
pub mod bundle;

pub use context::Context;
pub use services::Services;
pub use handler::{HandlerRegistry, LoadedHandler};
pub use actor::{ActorHandle, ActorCommand};
pub use bundle::{BundleManifest, BundleDeployer, DeploymentResult};