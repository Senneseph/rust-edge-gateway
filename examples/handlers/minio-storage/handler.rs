//! MinIO Storage Endpoint Handler Example
//!
//! This example demonstrates how an Endpoint Handler accesses the MinIO
//! Service Provider via the Context's dependency injection.
//!
//! The handler provides file upload, download, list, and delete operations
//! by communicating with the MinIO Service Provider through the message bus.
//!
//! # Architecture
//!
//! ```text
//! HTTP Request → Rust Edge Gateway → This Handler → ctx.minio() → MinIO Service Actor
//! ```
//!
//! The handler never manages connections directly - it uses the Context to
//! access the pre-configured MinIO Service Provider.

use rust_edge_gateway_sdk::prelude::*;

/// List objects in a bucket
///
/// GET /storage/list?prefix=uploads/
pub fn handle_list(ctx: &Context, req: Request) -> Response {
    let prefix = req.query_param("prefix").unwrap_or_default();
    let bucket = req.query_param("bucket");
    
    // Access MinIO Service Provider via Context
    let minio = match ctx.try_minio() {
        Some(m) => m,
        None => return Response::service_unavailable(json!({
            "error": "MinIO service not configured"
        })),
    };
    
    let bucket = bucket.as_deref().unwrap_or(minio.default_bucket());
    
    // Use tokio runtime to execute async operation
    let rt = tokio::runtime::Handle::current();
    match rt.block_on(minio.list_objects(bucket, &prefix)) {
        Ok(objects) => Response::ok(json!({
            "bucket": bucket,
            "prefix": prefix,
            "objects": objects.iter().map(|o| json!({
                "key": o.key,
                "size": o.size,
                "last_modified": o.last_modified
            })).collect::<Vec<_>>()
        })),
        Err(e) => Response::internal_error(json!({
            "error": format!("Failed to list objects: {}", e)
        })),
    }
}

/// Download an object
///
/// GET /storage/files/:key
pub fn handle_get(ctx: &Context, req: Request) -> Response {
    let key = match req.path_param("key") {
        Some(k) => k,
        None => return Response::bad_request(json!({"error": "Missing key parameter"})),
    };
    let bucket = req.query_param("bucket");
    
    let minio = match ctx.try_minio() {
        Some(m) => m,
        None => return Response::service_unavailable(json!({
            "error": "MinIO service not configured"
        })),
    };
    
    let bucket = bucket.as_deref().unwrap_or(minio.default_bucket());
    
    let rt = tokio::runtime::Handle::current();
    match rt.block_on(minio.get_object(bucket, &key)) {
        Ok(data) => {
            // Return binary data with appropriate content type
            let content_type = guess_content_type(&key);
            Response::ok_binary(data, content_type)
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("NoSuchKey") || error_msg.contains("not found") {
                Response::not_found(json!({"error": format!("Object not found: {}", key)}))
            } else {
                Response::internal_error(json!({"error": format!("Failed to get object: {}", e)}))
            }
        }
    }
}

/// Upload an object
///
/// POST /storage/files/:key
/// Body: raw file contents
pub fn handle_put(ctx: &Context, req: Request) -> Response {
    let key = match req.path_param("key") {
        Some(k) => k,
        None => return Response::bad_request(json!({"error": "Missing key parameter"})),
    };
    let bucket = req.query_param("bucket");
    let content_type = req.header("content-type");
    
    let minio = match ctx.try_minio() {
        Some(m) => m,
        None => return Response::service_unavailable(json!({
            "error": "MinIO service not configured"
        })),
    };
    
    let bucket = bucket.as_deref().unwrap_or(minio.default_bucket());
    let data = req.body_bytes();
    let size = data.len();
    
    let rt = tokio::runtime::Handle::current();
    match rt.block_on(minio.put_object(bucket, &key, data, content_type.as_deref())) {
        Ok(_) => Response::ok(json!({
            "key": key,
            "bucket": bucket,
            "size": size,
            "message": "Upload successful"
        })),
        Err(e) => Response::internal_error(json!({
            "error": format!("Failed to upload: {}", e)
        })),
    }
}

/// Delete an object
///
/// DELETE /storage/files/:key
pub fn handle_delete(ctx: &Context, req: Request) -> Response {
    let key = match req.path_param("key") {
        Some(k) => k,
        None => return Response::bad_request(json!({"error": "Missing key parameter"})),
    };
    let bucket = req.query_param("bucket");
    
    let minio = match ctx.try_minio() {
        Some(m) => m,
        None => return Response::service_unavailable(json!({
            "error": "MinIO service not configured"
        })),
    };
    
    let bucket = bucket.as_deref().unwrap_or(minio.default_bucket());
    
    let rt = tokio::runtime::Handle::current();
    match rt.block_on(minio.delete_object(bucket, &key)) {
        Ok(_) => Response::ok(json!({
            "key": key,
            "bucket": bucket,
            "deleted": true
        })),
        Err(e) => Response::internal_error(json!({
            "error": format!("Failed to delete: {}", e)
        })),
    }
}

/// Guess content type from file extension
fn guess_content_type(key: &str) -> &'static str {
    let ext = key.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

