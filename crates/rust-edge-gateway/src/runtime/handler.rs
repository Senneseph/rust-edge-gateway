//! Dynamic handler loading system
//!
//! Loads handler functions from dynamic libraries (.so/.dll) at runtime.
//! Supports hot-swapping handlers without gateway restart.
//! Provides graceful draining for zero-downtime deployments.
//!
//! # Architecture
//!
//! Handlers are compiled against the SDK which defines the Context type.
//! The gateway provides implementations of SDK service traits (MinioClient,
//! SqliteClient) that use message-passing to communicate with service actors.
//!
//! When a handler is called:
//! 1. Gateway creates an SDK Context with bridge implementations
//! 2. Handler receives the SDK Context and calls service methods
//! 3. Bridge implementations send messages to service actors
//! 4. Service actors process requests and return responses

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::pin::Pin;
use std::future::Future;

use libloading::{Library, Symbol};
use tokio::sync::RwLock;
use anyhow::{anyhow, Result};

use rust_edge_gateway_sdk::{Request, Response, Context as SdkContext};
use super::context::Context as RuntimeContext;

/// Type alias for the handler entry point function
///
/// All handlers must export a function with this signature:
/// ```ignore
/// #[no_mangle]
/// pub extern "C" fn handler_entry(ctx: &Context, req: Request) -> Response
/// ```
///
/// This is a synchronous signature for simplicity. Handlers that need async
/// operations should use tokio's block_on or similar.
///
/// The Context is the SDK's Context type, which provides access to service
/// providers via trait objects. The gateway creates an SDK Context populated
/// with bridge implementations that communicate with service actors.
pub type HandlerFn = unsafe extern "C" fn(&SdkContext, Request) -> Response;

/// A loaded handler with its library
pub struct LoadedHandler {
    /// The loaded library (must stay alive while handler is in use)
    _library: Library,

    /// The handler entry point (pub(crate) for use in execute_with_timeout)
    pub(crate) entry: HandlerFn,

    /// Path the library was loaded from
    pub path: PathBuf,

    /// When the handler was loaded
    pub loaded_at: Instant,

    /// Handler metadata
    pub metadata: HandlerMetadata,

    /// Active request count for graceful draining
    active_requests: AtomicU64,

    /// Whether this handler is draining (not accepting new requests)
    draining: AtomicBool,
}

/// Metadata about a handler
#[derive(Debug, Clone)]
pub struct HandlerMetadata {
    /// Handler name/ID
    pub name: String,

    /// Version (if available)
    pub version: Option<String>,

    /// Description (if available)
    pub description: Option<String>,
}

/// Guard that decrements active request count when dropped
pub struct RequestGuard {
    handler: Arc<LoadedHandler>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.handler.active_requests.fetch_sub(1, Ordering::SeqCst);
    }
}

impl LoadedHandler {
    /// Load a handler from a dynamic library
    ///
    /// # Safety
    /// This function loads and executes code from a dynamic library.
    /// The library must export a `handler_entry` function with the correct signature.
    pub unsafe fn load(path: &Path, name: &str) -> Result<Self> {
        // Load the library
        let library = Library::new(path)
            .map_err(|e| anyhow!("Failed to load library {:?}: {}", path, e))?;

        // Get the entry point symbol
        let entry: Symbol<HandlerFn> = library.get(b"handler_entry")
            .map_err(|e| anyhow!("Failed to find handler_entry symbol: {}", e))?;

        // Convert to raw function pointer (safe because library stays alive)
        let entry_fn: HandlerFn = *entry;

        Ok(Self {
            _library: library,
            entry: entry_fn,
            path: path.to_path_buf(),
            loaded_at: Instant::now(),
            metadata: HandlerMetadata {
                name: name.to_string(),
                version: None,
                description: None,
            },
            active_requests: AtomicU64::new(0),
            draining: AtomicBool::new(false),
        })
    }

    /// Execute the handler
    ///
    /// # Safety
    /// Calls into dynamically loaded code. The handler must be well-behaved.
    pub fn execute(&self, ctx: &SdkContext, req: Request) -> Response {
        // Call the handler entry point with SDK Context
        unsafe { (self.entry)(ctx, req) }
    }

    /// Increment active request count and return a guard
    pub fn acquire_request(self: &Arc<Self>) -> Option<RequestGuard> {
        // Don't accept new requests if draining
        if self.draining.load(Ordering::SeqCst) {
            return None;
        }
        self.active_requests.fetch_add(1, Ordering::SeqCst);
        Some(RequestGuard { handler: Arc::clone(self) })
    }

    /// Get the number of active requests
    pub fn active_request_count(&self) -> u64 {
        self.active_requests.load(Ordering::SeqCst)
    }

    /// Mark this handler as draining (no new requests accepted)
    pub fn start_draining(&self) {
        self.draining.store(true, Ordering::SeqCst);
        tracing::info!(
            handler = %self.metadata.name,
            active = self.active_request_count(),
            "Handler started draining"
        );
    }

    /// Check if handler is draining
    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    /// Check if handler has drained (no active requests)
    pub fn is_drained(&self) -> bool {
        self.draining.load(Ordering::SeqCst) && self.active_requests.load(Ordering::SeqCst) == 0
    }

    /// Get the age of this loaded handler
    pub fn age(&self) -> Duration {
        self.loaded_at.elapsed()
    }
}

// Safety: The handler function pointer is safe to send between threads
// because the library it points to is kept alive by the LoadedHandler
unsafe impl Send for LoadedHandler {}
unsafe impl Sync for LoadedHandler {}

/// Registry for loaded handlers
///
/// Manages loading, unloading, and hot-swapping of handler libraries.
/// Supports graceful draining for zero-downtime deployments.
pub struct HandlerRegistry {
    /// Map of endpoint ID to loaded handler (current version)
    handlers: RwLock<HashMap<String, Arc<LoadedHandler>>>,

    /// Handlers that are draining (previous versions waiting for requests to complete)
    draining_handlers: RwLock<Vec<Arc<LoadedHandler>>>,

    /// Directory where handler libraries are stored
    handlers_dir: PathBuf,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new(handlers_dir: PathBuf) -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            draining_handlers: RwLock::new(Vec::new()),
            handlers_dir,
        }
    }

    /// Load a handler from the handlers directory
    pub async fn load(&self, endpoint_id: &str) -> Result<()> {
        let lib_path = self.library_path(endpoint_id);

        if !lib_path.exists() {
            return Err(anyhow!("Handler library not found: {:?}", lib_path));
        }

        // Load the handler
        let handler = unsafe { LoadedHandler::load(&lib_path, endpoint_id)? };
        let handler = Arc::new(handler);

        // Store in registry
        let mut handlers = self.handlers.write().await;
        handlers.insert(endpoint_id.to_string(), handler);

        tracing::info!("Loaded handler: {} from {:?}", endpoint_id, lib_path);
        Ok(())
    }

    /// Load a handler from a specific path
    pub async fn load_from(&self, endpoint_id: &str, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("Handler library not found: {:?}", path));
        }

        // Load the handler
        let handler = unsafe { LoadedHandler::load(path, endpoint_id)? };
        let handler = Arc::new(handler);

        // Store in registry
        let mut handlers = self.handlers.write().await;
        handlers.insert(endpoint_id.to_string(), handler);

        tracing::info!("Loaded handler: {} from {:?}", endpoint_id, path);
        Ok(())
    }

    /// Unload a handler (immediate, does not wait for requests)
    pub async fn unload(&self, endpoint_id: &str) -> Result<()> {
        let mut handlers = self.handlers.write().await;

        if handlers.remove(endpoint_id).is_some() {
            tracing::info!("Unloaded handler: {}", endpoint_id);
        }

        Ok(())
    }

    /// Hot-swap a handler (atomic replace, old handler dropped immediately)
    pub async fn swap(&self, endpoint_id: &str, new_path: &Path) -> Result<()> {
        if !new_path.exists() {
            return Err(anyhow!("New handler library not found: {:?}", new_path));
        }

        // Load the new handler first
        let new_handler = unsafe { LoadedHandler::load(new_path, endpoint_id)? };
        let new_handler = Arc::new(new_handler);

        // Atomic swap
        let mut handlers = self.handlers.write().await;
        let old = handlers.insert(endpoint_id.to_string(), new_handler);

        tracing::info!("Hot-swapped handler: {} (old handler dropped)", endpoint_id);

        // Old handler is dropped here, which unloads the library
        drop(old);

        Ok(())
    }

    /// Graceful hot-swap: swap handler but drain old handler gracefully
    ///
    /// New requests go to the new handler, while the old handler finishes
    /// processing its in-flight requests. Once drained, the old handler is dropped.
    ///
    /// # Arguments
    /// * `endpoint_id` - The endpoint to swap
    /// * `new_path` - Path to the new handler library
    /// * `drain_timeout` - Maximum time to wait for old handler to drain
    pub async fn swap_graceful(
        &self,
        endpoint_id: &str,
        new_path: &Path,
        drain_timeout: Duration,
    ) -> Result<DrainResult> {
        if !new_path.exists() {
            return Err(anyhow!("New handler library not found: {:?}", new_path));
        }

        // Load the new handler first
        let new_handler = unsafe { LoadedHandler::load(new_path, endpoint_id)? };
        let new_handler = Arc::new(new_handler);

        // Get the old handler and start draining
        let old_handler = {
            let mut handlers = self.handlers.write().await;
            let old = handlers.insert(endpoint_id.to_string(), Arc::clone(&new_handler));
            old
        };

        let drain_result = if let Some(old_handler) = old_handler {
            let old_active = old_handler.active_request_count();

            if old_active > 0 {
                // Start draining the old handler
                old_handler.start_draining();

                // Add to draining list
                {
                    let mut draining = self.draining_handlers.write().await;
                    draining.push(Arc::clone(&old_handler));
                }

                // Spawn background task to clean up when drained
                let draining_handlers = Arc::clone(&old_handler);
                let drain_timeout_clone = drain_timeout;
                tokio::spawn(async move {
                    let start = Instant::now();

                    // Poll until drained or timeout
                    while !draining_handlers.is_drained() {
                        if start.elapsed() > drain_timeout_clone {
                            tracing::warn!(
                                handler = %draining_handlers.metadata.name,
                                remaining = draining_handlers.active_request_count(),
                                "Handler drain timeout, forcing drop"
                            );
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }

                    tracing::info!(
                        handler = %draining_handlers.metadata.name,
                        elapsed_ms = start.elapsed().as_millis(),
                        "Handler drained successfully"
                    );
                });

                DrainResult {
                    swapped: true,
                    old_requests_pending: old_active,
                    draining: true,
                }
            } else {
                // No active requests, drop immediately
                tracing::info!("Graceful swap: {} (no active requests)", endpoint_id);
                DrainResult {
                    swapped: true,
                    old_requests_pending: 0,
                    draining: false,
                }
            }
        } else {
            // No old handler, just loaded new one
            DrainResult {
                swapped: true,
                old_requests_pending: 0,
                draining: false,
            }
        };

        tracing::info!(
            "Graceful hot-swap: {} (draining: {}, pending: {})",
            endpoint_id,
            drain_result.draining,
            drain_result.old_requests_pending
        );

        Ok(drain_result)
    }

    /// Clean up fully drained handlers
    pub async fn cleanup_drained(&self) -> usize {
        let mut draining = self.draining_handlers.write().await;
        let before = draining.len();
        draining.retain(|h| !h.is_drained());
        let removed = before - draining.len();

        if removed > 0 {
            tracing::info!("Cleaned up {} drained handlers", removed);
        }

        removed
    }

    /// Get draining handler count
    pub async fn draining_count(&self) -> usize {
        let draining = self.draining_handlers.read().await;
        draining.len()
    }

    /// Get a handler by endpoint ID
    pub async fn get(&self, endpoint_id: &str) -> Option<Arc<LoadedHandler>> {
        let handlers = self.handlers.read().await;
        handlers.get(endpoint_id).cloned()
    }

    /// Check if a handler is loaded
    pub async fn is_loaded(&self, endpoint_id: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(endpoint_id)
    }

    /// List all loaded handlers
    pub async fn list(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    /// Get handler count
    pub async fn count(&self) -> usize {
        let handlers = self.handlers.read().await;
        handlers.len()
    }

    /// Execute a handler with request tracking for graceful draining
    pub async fn execute(
        &self,
        endpoint_id: &str,
        ctx: &SdkContext,
        req: Request,
    ) -> Result<Response> {
        let handler = self.get(endpoint_id).await
            .ok_or_else(|| anyhow!("Handler not loaded: {}", endpoint_id))?;

        // Acquire request guard for tracking
        let _guard = handler.acquire_request()
            .ok_or_else(|| anyhow!("Handler is draining, cannot accept new requests"))?;

        Ok(handler.execute(ctx, req))
    }

    /// Execute a handler with timeout and request tracking
    pub async fn execute_with_timeout(
        &self,
        endpoint_id: &str,
        ctx: &SdkContext,
        req: Request,
        timeout: Duration,
    ) -> Result<Response> {
        let handler = self.get(endpoint_id).await
            .ok_or_else(|| anyhow!("Handler not loaded: {}", endpoint_id))?;

        // Acquire request guard for tracking
        let _guard = handler.acquire_request()
            .ok_or_else(|| anyhow!("Handler is draining, cannot accept new requests"))?;

        // Get the entry function pointer (Copy/Send safe)
        let entry = handler.entry;

        // Clone context for spawn_blocking (SDK Context is Clone + Send)
        let ctx = ctx.clone();

        // Wrap sync execution in spawn_blocking for timeout support
        let future = tokio::task::spawn_blocking(move || {
            // Safety: entry is from a loaded library that remains alive
            unsafe { entry(&ctx, req) }
        });

        match tokio::time::timeout(timeout, future).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(anyhow!("Handler task panicked: {}", e)),
            Err(_) => Err(anyhow!("Handler execution timed out")),
        }
    }

    /// Get handler stats
    pub async fn stats(&self) -> HandlerStats {
        let handlers = self.handlers.read().await;
        let draining = self.draining_handlers.read().await;

        let mut total_active = 0u64;
        for handler in handlers.values() {
            total_active += handler.active_request_count();
        }

        let mut draining_active = 0u64;
        for handler in draining.iter() {
            draining_active += handler.active_request_count();
        }

        HandlerStats {
            loaded_count: handlers.len(),
            draining_count: draining.len(),
            active_requests: total_active,
            draining_requests: draining_active,
        }
    }

    /// Get the expected library path for an endpoint
    fn library_path(&self, endpoint_id: &str) -> PathBuf {
        let lib_name = format_library_name(endpoint_id);
        self.handlers_dir.join(endpoint_id).join(&lib_name)
    }
}

/// Result of a graceful drain operation
#[derive(Debug, Clone)]
pub struct DrainResult {
    /// Whether the swap was successful
    pub swapped: bool,
    /// Number of requests pending on old handler
    pub old_requests_pending: u64,
    /// Whether the old handler is currently draining
    pub draining: bool,
}

/// Statistics about loaded handlers
#[derive(Debug, Clone)]
pub struct HandlerStats {
    /// Number of loaded handlers
    pub loaded_count: usize,
    /// Number of handlers currently draining
    pub draining_count: usize,
    /// Total active requests across all handlers
    pub active_requests: u64,
    /// Active requests on draining handlers
    pub draining_requests: u64,
}

/// Format the library filename for the current platform
#[cfg(target_os = "windows")]
fn format_library_name(endpoint_id: &str) -> String {
    format!("handler_{}.dll", endpoint_id.replace('-', "_"))
}

#[cfg(target_os = "linux")]
fn format_library_name(endpoint_id: &str) -> String {
    format!("libhandler_{}.so", endpoint_id.replace('-', "_"))
}

#[cfg(target_os = "macos")]
fn format_library_name(endpoint_id: &str) -> String {
    format!("libhandler_{}.dylib", endpoint_id.replace('-', "_"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn format_library_name(endpoint_id: &str) -> String {
    format!("libhandler_{}.so", endpoint_id.replace('-', "_"))
}

/// Fallback handler that can be used when dynamic loading is not available
/// or for testing purposes
pub struct FallbackHandler {
    handlers: RwLock<HashMap<String, Arc<dyn Fn(&SdkContext, Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>>>,
}

impl FallbackHandler {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a handler function
    pub async fn register<F, Fut>(&self, endpoint_id: &str, handler: F)
    where
        F: Fn(&SdkContext, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let wrapper = Arc::new(move |ctx: &SdkContext, req: Request| -> Pin<Box<dyn Future<Output = Response> + Send>> {
            Box::pin(handler(ctx, req))
        });

        let mut handlers = self.handlers.write().await;
        handlers.insert(endpoint_id.to_string(), wrapper);
    }

    /// Execute a registered handler
    pub async fn execute(&self, endpoint_id: &str, ctx: &SdkContext, req: Request) -> Result<Response> {
        let handlers = self.handlers.read().await;
        let handler = handlers.get(endpoint_id)
            .ok_or_else(|| anyhow!("Handler not registered: {}", endpoint_id))?;

        Ok(handler(ctx, req).await)
    }
}

impl Default for FallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::services::Services;
    use crate::runtime::context::{ContextBuilder, RuntimeConfig};
    
    #[test]
    fn test_library_name_format() {
        let name = format_library_name("my-handler");
        
        #[cfg(target_os = "windows")]
        assert_eq!(name, "handler_my_handler.dll");
        
        #[cfg(target_os = "linux")]
        assert_eq!(name, "libhandler_my_handler.so");
        
        #[cfg(target_os = "macos")]
        assert_eq!(name, "libhandler_my_handler.dylib");
    }
    
    #[tokio::test]
    async fn test_fallback_handler() {
        let fallback = FallbackHandler::new();
        
        fallback.register("test", |_ctx, req| async move {
            Response::ok(serde_json::json!({
                "path": req.path,
                "method": req.method,
            }))
        }).await;
        
        let services = Services::new();
        let ctx = ContextBuilder::new(services).build();
        let req = Request::default();
        
        let response = fallback.execute("test", &ctx, req).await.unwrap();
        assert_eq!(response.status, 200);
    }
    
    #[tokio::test]
    async fn test_handler_registry_empty() {
        let registry = HandlerRegistry::new(PathBuf::from("/tmp/handlers"));
        
        assert!(!registry.is_loaded("nonexistent").await);
        assert_eq!(registry.count().await, 0);
        assert!(registry.list().await.is_empty());
    }
}