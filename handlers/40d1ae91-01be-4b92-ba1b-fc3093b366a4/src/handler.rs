use edge_hive_sdk::prelude::*;

pub fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path
    }))
}