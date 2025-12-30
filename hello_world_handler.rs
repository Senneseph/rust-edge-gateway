use rust_edge_gateway_sdk::prelude::*;

/// Hello World handler
/// Returns a simple JSON response with "Hello, World!" message
///
/// This is a sync handler that matches the gateway's expected signature:
/// `pub fn handle(ctx: &Context, req: Request) -> Response`
///
/// The `ctx` parameter provides access to service providers (database, cache, storage)
/// when configured. For this simple example, we don't use it.
pub fn handle(_ctx: &Context, req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path,
        "method": req.method
    }))
}