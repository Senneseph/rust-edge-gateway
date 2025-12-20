//! Handler macros and utilities for the new v2 runtime
//!
//! This module provides macros for defining handlers that work with the
//! dynamic library loading system. Handlers are compiled as shared libraries
//! and loaded at runtime by the gateway.
//!
//! # Example
//!
//! ```ignore
//! use rust_edge_gateway_sdk::prelude::*;
//!
//! // Simple handler
//! handler!(async fn my_handler(ctx: &Context, req: Request) -> Response {
//!     Response::ok(json!({"message": "Hello!"}))
//! });
//!
//! // Handler with error handling
//! handler_result!(async fn my_handler(ctx: &Context, req: Request) -> Result<Response, HandlerError> {
//!     let id: i64 = req.require_path_param("id")?;
//!     Ok(Response::ok(json!({"id": id})))
//! });
//! ```

use std::future::Future;
use std::pin::Pin;
 
use crate::{Request, Response};

/// Type alias for boxed future returned by handlers
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Type alias for the handler function signature
pub type HandlerFn<Ctx> = fn(&Ctx, Request) -> BoxFuture<'static, Response>;

/// Trait for types that can be used as handler context
/// 
/// This is implemented by the gateway's Context type
pub trait HandlerContext: Send + Sync + 'static {}

// Note: From<HandlerError> for Response is already implemented in error.rs

/// Macro for defining a simple async handler
///
/// # Example
///
/// ```ignore
/// handler!(async fn get_user(ctx: &Context, req: Request) -> Response {
///     let id: i64 = req.path_param_as("id").unwrap_or(0);
///     Response::ok(json!({"id": id}))
/// });
/// ```
#[macro_export]
macro_rules! handler {
    (async fn $name:ident($ctx:ident: &$ctx_ty:ty, $req:ident: Request) -> Response $body:block) => {
        async fn $name($ctx: &$ctx_ty, $req: $crate::Request) -> $crate::Response $body
        
        #[no_mangle]
        pub extern "C" fn handler_entry(
            ctx: &$ctx_ty,
            req: $crate::Request,
        ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = $crate::Response> + Send + 'static>> {
            // Clone what we need to make the future 'static
            let req = req;
            // We need to use unsafe to transmute the lifetime since the context
            // is guaranteed to live for the duration of the handler call
            let ctx_ptr = ctx as *const $ctx_ty;
            Box::pin(async move {
                let ctx = unsafe { &*ctx_ptr };
                $name(ctx, req).await
            })
        }
    };
}

/// Macro for defining a handler that returns Result
///
/// # Example
///
/// ```ignore
/// handler_result!(async fn create_user(ctx: &Context, req: Request) -> Result<Response, HandlerError> {
///     let data: CreateUserRequest = req.json()?;
///     let id = ctx.services.db.execute("INSERT INTO users (name) VALUES (?)", &[data.name.into()]).await?;
///     Ok(Response::created(json!({"id": id})))
/// });
/// ```
#[macro_export]
macro_rules! handler_result {
    (async fn $name:ident($ctx:ident: &$ctx_ty:ty, $req:ident: Request) -> Result<Response, HandlerError> $body:block) => {
        async fn $name($ctx: &$ctx_ty, $req: $crate::Request) -> Result<$crate::Response, $crate::HandlerError> $body
        
        #[no_mangle]
        pub extern "C" fn handler_entry(
            ctx: &$ctx_ty,
            req: $crate::Request,
        ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = $crate::Response> + Send + 'static>> {
            let req = req;
            let ctx_ptr = ctx as *const $ctx_ty;
            Box::pin(async move {
                let ctx = unsafe { &*ctx_ptr };
                match $name(ctx, req).await {
                    Ok(response) => response,
                    Err(err) => err.into_response(),
                }
            })
        }
    };
}

// V1 handler_loop macros are defined in ipc.rs for backward compatibility

#[cfg(test)]
mod tests {
    use crate::Response;
    use crate::HandlerError;
    
    #[test]
    fn test_handler_error_conversion() {
        let err = HandlerError::BadRequest("test error".to_string());
        let response: Response = err.into();
        assert_eq!(response.status, 400);
    }
}