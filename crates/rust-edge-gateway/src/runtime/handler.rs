//! Dynamic handler loading system
//!
//! Loads handler functions from dynamic libraries (.so/.dll) at runtime.
//! Supports hot-swapping handlers without gateway restart.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::pin::Pin;
use std::future::Future;

use libloading::{Library, Symbol};
use tokio::sync::RwLock;
use anyhow::{anyhow, Result};

use rust_edge_gateway_sdk::{Request, Response};
use super::context::Context;

/// Type alias for the handler entry point function
/// 
/// All handlers must export a function with this signature:
/// ```ignore
/// #[no_mangle]
/// pub extern "C" fn handler_entry(
///     ctx: &Context,
///     req: Request,
/// ) -> Pin<Box<dyn Future<Output = Response> + Send>>
/// ```
pub type HandlerFn = unsafe extern "C" fn(&Context, Request) -> Pin<Box<dyn Future<Output = Response> + Send>>;

/// A loaded handler with its library
pub struct LoadedHandler {
    /// The loaded library (must stay alive while handler is in use)
    _library: Library,
    
    /// The handler entry point
    entry: HandlerFn,
    
    /// Path the library was loaded from
    pub path: PathBuf,
    
    /// When the handler was loaded
    pub loaded_at: Instant,
    
    /// Handler metadata
    pub metadata: HandlerMetadata,
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
        })
    }
    
    /// Execute the handler
    /// 
    /// # Safety
    /// Calls into dynamically loaded code. The handler must be well-behaved.
    pub async fn execute(&self, ctx: &Context, req: Request) -> Response {
        // Call the handler entry point
        let future = unsafe { (self.entry)(ctx, req) };
        
        // Await the future
        future.await
    }
    
    /// Get the age of this loaded handler
    pub fn age(&self) -> std::time::Duration {
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
pub struct HandlerRegistry {
    /// Map of endpoint ID to loaded handler
    handlers: RwLock<HashMap<String, Arc<LoadedHandler>>>,
    
    /// Directory where handler libraries are stored
    handlers_dir: PathBuf,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new(handlers_dir: PathBuf) -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
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
    
    /// Unload a handler
    pub async fn unload(&self, endpoint_id: &str) -> Result<()> {
        let mut handlers = self.handlers.write().await;
        
        if handlers.remove(endpoint_id).is_some() {
            tracing::info!("Unloaded handler: {}", endpoint_id);
        }
        
        Ok(())
    }
    
    /// Hot-swap a handler (atomic replace)
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
    
    /// Execute a handler
    pub async fn execute(
        &self,
        endpoint_id: &str,
        ctx: &Context,
        req: Request,
    ) -> Result<Response> {
        let handler = self.get(endpoint_id).await
            .ok_or_else(|| anyhow!("Handler not loaded: {}", endpoint_id))?;
        
        Ok(handler.execute(ctx, req).await)
    }
    
    /// Execute a handler with timeout
    pub async fn execute_with_timeout(
        &self,
        endpoint_id: &str,
        ctx: &Context,
        req: Request,
        timeout: std::time::Duration,
    ) -> Result<Response> {
        let handler = self.get(endpoint_id).await
            .ok_or_else(|| anyhow!("Handler not loaded: {}", endpoint_id))?;
        
        match tokio::time::timeout(timeout, handler.execute(ctx, req)).await {
            Ok(response) => Ok(response),
            Err(_) => Err(anyhow!("Handler execution timed out")),
        }
    }
    
    /// Get the expected library path for an endpoint
    fn library_path(&self, endpoint_id: &str) -> PathBuf {
        let lib_name = format_library_name(endpoint_id);
        self.handlers_dir.join(endpoint_id).join(&lib_name)
    }
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
    handlers: RwLock<HashMap<String, Arc<dyn Fn(&Context, Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>>>,
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
        F: Fn(&Context, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let wrapper = Arc::new(move |ctx: &Context, req: Request| -> Pin<Box<dyn Future<Output = Response> + Send>> {
            Box::pin(handler(ctx, req))
        });
        
        let mut handlers = self.handlers.write().await;
        handlers.insert(endpoint_id.to_string(), wrapper);
    }
    
    /// Execute a registered handler
    pub async fn execute(&self, endpoint_id: &str, ctx: &Context, req: Request) -> Result<Response> {
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