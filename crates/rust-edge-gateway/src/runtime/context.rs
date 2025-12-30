//! Context API for handlers
//!
//! The Context provides handlers with access to:
//! - Pre-established service connections
//! - Request metadata
//! - Runtime configuration
//!
//! There are two Context types:
//! - **RuntimeContext** (this file): Gateway's internal context with Services, RuntimeHandle, etc.
//! - **SdkContext** (SDK crate): What handlers receive, with trait objects for services.
//!
//! The gateway creates an SdkContext from RuntimeContext by wrapping service handles
//! in bridge implementations that use message-passing to communicate with service actors.

use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use rust_edge_gateway_sdk::Context as SdkContext;
use super::services::Services;
use super::services::minio_bridge::MinioClientBridge;

/// Request identifier for tracing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(String);

impl RequestId {
    /// Create a new random request ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    /// Create from a string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Runtime configuration snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Handler execution timeout in seconds
    pub handler_timeout_secs: u64,
    
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    
    /// Enable debug mode
    pub debug: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            handler_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            debug: false,
        }
    }
}

/// Runtime handle for spawning background tasks
#[derive(Clone)]
pub struct RuntimeHandle {
    inner: tokio::runtime::Handle,
}

impl RuntimeHandle {
    /// Create from current tokio runtime
    pub fn current() -> Self {
        Self {
            inner: tokio::runtime::Handle::current(),
        }
    }
    
    /// Spawn a background task
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn(future)
    }
    
    /// Spawn a blocking task
    pub fn spawn_blocking<F, R>(&self, f: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.inner.spawn_blocking(f)
    }
}

impl std::fmt::Debug for RuntimeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeHandle").finish()
    }
}

/// Context passed to every handler
///
/// This is the primary interface handlers use to access services and runtime information.
/// It is cheap to clone and should be passed by reference to handler functions.
///
/// # Example
///
/// ```ignore
/// async fn handle(ctx: &Context, req: Request) -> Response {
///     // Access database service
///     let users = ctx.services.require_db()?
///         .query("SELECT * FROM users", &[])
///         .await?;
///     
///     // Access cache
///     ctx.services.require_cache()?
///         .set("key", "value", Some(3600))
///         .await?;
///     
///     // Log with request ID
///     tracing::info!(request_id = %ctx.request_id, "Processed request");
///     
///     Response::ok(json!(users))
/// }
/// ```
#[derive(Clone)]
pub struct Context {
    /// Pre-established service connections
    pub services: Services,
    
    /// Runtime handle for spawning tasks
    pub runtime: RuntimeHandle,
    
    /// Unique request identifier
    pub request_id: RequestId,
    
    /// Configuration snapshot
    pub config: Arc<RuntimeConfig>,
}

impl Context {
    /// Create a new context for a request
    pub fn new(
        services: Services,
        runtime: RuntimeHandle,
        config: Arc<RuntimeConfig>,
    ) -> Self {
        Self {
            services,
            runtime,
            request_id: RequestId::new(),
            config,
        }
    }
    
    /// Create a context with a specific request ID
    pub fn with_request_id(
        services: Services,
        runtime: RuntimeHandle,
        config: Arc<RuntimeConfig>,
        request_id: RequestId,
    ) -> Self {
        Self {
            services,
            runtime,
            request_id,
            config,
        }
    }
    
    /// Create a child context (same services, new request ID)
    pub fn child(&self) -> Self {
        Self {
            services: self.services.clone(),
            runtime: self.runtime.clone(),
            request_id: RequestId::new(),
            config: self.config.clone(),
        }
    }
    
    /// Create a child context with a specific request ID
    pub fn child_with_id(&self, request_id: RequestId) -> Self {
        Self {
            services: self.services.clone(),
            runtime: self.runtime.clone(),
            request_id,
            config: self.config.clone(),
        }
    }
    
    /// Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        self.config.debug
    }
    
    /// Get handler timeout duration
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.handler_timeout_secs)
    }
    
    /// Spawn a background task
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("request_id", &self.request_id)
            .field("services", &self.services)
            .field("config", &self.config)
            .finish()
    }
}

/// Builder for creating Context instances
pub struct ContextBuilder {
    services: Services,
    runtime: Option<RuntimeHandle>,
    config: Option<Arc<RuntimeConfig>>,
    request_id: Option<RequestId>,
}

impl ContextBuilder {
    /// Create a new builder with the given services
    pub fn new(services: Services) -> Self {
        Self {
            services,
            runtime: None,
            config: None,
            request_id: None,
        }
    }
    
    /// Set the runtime handle
    pub fn runtime(mut self, runtime: RuntimeHandle) -> Self {
        self.runtime = Some(runtime);
        self
    }
    
    /// Set the configuration
    pub fn config(mut self, config: Arc<RuntimeConfig>) -> Self {
        self.config = Some(config);
        self
    }
    
    /// Set the request ID
    pub fn request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }
    
    /// Build the context
    pub fn build(self) -> Context {
        Context {
            services: self.services,
            runtime: self.runtime.unwrap_or_else(RuntimeHandle::current),
            request_id: self.request_id.unwrap_or_default(),
            config: self.config.unwrap_or_else(|| Arc::new(RuntimeConfig::default())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_request_id() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1, id2);
        
        let id3 = RequestId::from_string("test-id".to_string());
        assert_eq!(id3.as_str(), "test-id");
    }
    
    #[tokio::test]
    async fn test_context_builder() {
        let services = Services::new();
        let config = Arc::new(RuntimeConfig {
            handler_timeout_secs: 60,
            max_body_size: 1024,
            debug: true,
        });
        
        let ctx = ContextBuilder::new(services)
            .config(config.clone())
            .request_id(RequestId::from_string("req-123".to_string()))
            .build();
        
        assert_eq!(ctx.request_id.as_str(), "req-123");
        assert!(ctx.is_debug());
        assert_eq!(ctx.config.handler_timeout_secs, 60);
    }
    
    #[tokio::test]
    async fn test_context_child() {
        let services = Services::new();
        let ctx = ContextBuilder::new(services).build();
        
        let child = ctx.child();
        assert_ne!(ctx.request_id, child.request_id);
        
        let child_with_id = ctx.child_with_id(RequestId::from_string("child-123".to_string()));
        assert_eq!(child_with_id.request_id.as_str(), "child-123");
    }
}