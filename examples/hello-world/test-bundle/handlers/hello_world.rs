//! Hello World handler
use rust_edge_gateway_sdk::prelude::*;

/// Handle incoming requests
fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path,
        "method": req.method,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

handler_loop!(handle);

