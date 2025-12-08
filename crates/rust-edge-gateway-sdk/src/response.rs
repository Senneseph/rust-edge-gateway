//! HTTP Response representation for handlers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an outgoing HTTP response.
///
/// # Quick Reference
///
/// | Method | Status | Use Case |
/// |--------|--------|----------|
/// | `ok(body)` | 200 | Successful GET/PUT response |
/// | `created(body)` | 201 | Successful POST (resource created) |
/// | `no_content()` | 204 | Successful DELETE |
/// | `bad_request(msg)` | 400 | Invalid input |
/// | `unauthorized(msg)` | 401 | Missing/invalid auth |
/// | `forbidden(msg)` | 403 | Not authorized |
/// | `not_found()` | 404 | Resource not found |
/// | `internal_error(msg)` | 500 | Server error |
///
/// # Binary Responses
///
/// For serving files/images:
/// ```ignore
/// Response::binary(200, image_bytes, "image/png")
///     .with_header("Content-Disposition", "inline; filename=\"photo.png\"")
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// HTTP status code
    pub status: u16,

    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Response body
    #[serde(default)]
    pub body: Option<String>,
}

impl Response {
    /// Create a new response with the given status code (no body).
    ///
    /// # Example
    /// ```ignore
    /// Response::new(204) // 204 No Content
    /// Response::new(301).with_header("Location", "/new-path")
    /// ```
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Create a 200 OK response with JSON body.
    ///
    /// # Example
    /// ```ignore
    /// Response::ok(json!({"message": "Success"}))
    /// Response::ok(my_struct) // If my_struct implements Serialize
    /// ```
    pub fn ok<T: Serialize>(body: T) -> Self {
        Self::json(200, body)
    }

    /// Create a JSON response with a custom status code.
    ///
    /// # Example
    /// ```ignore
    /// Response::json(201, json!({"id": "new-id"}))
    /// Response::json(400, json!({"error": "Invalid input"}))
    /// ```
    pub fn json<T: Serialize>(status: u16, body: T) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Self {
            status,
            headers,
            body: serde_json::to_string(&body).ok(),
        }
    }

    /// Create a plain text response.
    ///
    /// # Example
    /// ```ignore
    /// Response::text(200, "Hello, World!")
    /// Response::text(500, "Internal Server Error")
    /// ```
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain; charset=utf-8".to_string());

        Self {
            status,
            headers,
            body: Some(body.into()),
        }
    }

    /// Create a binary response (for files, images, etc.).
    ///
    /// The data is base64-encoded for transport over JSON IPC.
    /// The gateway will decode it before sending to the client.
    ///
    /// # Example
    /// ```ignore
    /// // Serve an image
    /// Response::binary(200, image_bytes, "image/png")
    ///
    /// // Serve a PDF with download prompt
    /// Response::binary(200, pdf_bytes, "application/pdf")
    ///     .with_header("Content-Disposition", "attachment; filename=\"report.pdf\"")
    /// ```
    pub fn binary(status: u16, data: impl AsRef<[u8]>, content_type: impl Into<String>) -> Self {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(data.as_ref());

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.into());
        headers.insert("X-Binary-Response".to_string(), "base64".to_string());

        Self {
            status,
            headers,
            body: Some(encoded),
        }
    }

    /// Create an HTML response.
    ///
    /// # Example
    /// ```ignore
    /// Response::html(200, "<html><body><h1>Hello!</h1></body></html>")
    /// ```
    pub fn html(status: u16, body: impl Into<String>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/html; charset=utf-8".to_string());

        Self {
            status,
            headers,
            body: Some(body.into()),
        }
    }

    /// Create a 404 Not Found response.
    pub fn not_found() -> Self {
        Self::json(404, serde_json::json!({"error": "Not Found"}))
    }

    /// Create a 404 Not Found response with a custom message.
    pub fn not_found_msg(message: impl Into<String>) -> Self {
        Self::json(404, serde_json::json!({"error": message.into()}))
    }

    /// Create a 400 Bad Request response.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::json(400, serde_json::json!({"error": message.into()}))
    }

    /// Create a 401 Unauthorized response.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::json(401, serde_json::json!({"error": message.into()}))
    }

    /// Create a 403 Forbidden response.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::json(403, serde_json::json!({"error": message.into()}))
    }

    /// Create a 409 Conflict response.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::json(409, serde_json::json!({"error": message.into()}))
    }

    /// Create a 500 Internal Server Error response.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::json(500, serde_json::json!({"error": message.into()}))
    }

    /// Create a 503 Service Unavailable response.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::json(503, serde_json::json!({"error": message.into()}))
    }

    /// Create a 201 Created response with JSON body.
    pub fn created<T: Serialize>(body: T) -> Self {
        Self::json(201, body)
    }

    /// Create a 202 Accepted response with JSON body.
    pub fn accepted<T: Serialize>(body: T) -> Self {
        Self::json(202, body)
    }

    /// Create a 204 No Content response.
    pub fn no_content() -> Self {
        Self::new(204)
    }

    /// Create a redirect response.
    ///
    /// # Example
    /// ```ignore
    /// Response::redirect(302, "https://example.com/new-location")
    /// Response::redirect(301, "/permanent-new-path") // Permanent redirect
    /// ```
    pub fn redirect(status: u16, location: impl Into<String>) -> Self {
        Self::new(status).with_header("Location", location)
    }

    /// Add a header to the response (builder pattern).
    ///
    /// # Example
    /// ```ignore
    /// Response::ok(json!({"data": "value"}))
    ///     .with_header("Cache-Control", "max-age=3600")
    ///     .with_header("X-Custom-Header", "value")
    /// ```
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the body (builder pattern).
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Add CORS headers for cross-origin requests.
    ///
    /// # Example
    /// ```ignore
    /// Response::ok(data).with_cors("*")
    /// Response::ok(data).with_cors("https://myapp.com")
    /// ```
    pub fn with_cors(self, origin: impl Into<String>) -> Self {
        self.with_header("Access-Control-Allow-Origin", origin)
            .with_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
            .with_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
    }

    /// Add caching headers.
    ///
    /// # Example
    /// ```ignore
    /// Response::ok(data).with_cache(3600) // Cache for 1 hour
    /// Response::ok(data).with_cache(0)    // No cache
    /// ```
    pub fn with_cache(self, max_age_seconds: u32) -> Self {
        if max_age_seconds == 0 {
            self.with_header("Cache-Control", "no-store, no-cache, must-revalidate")
        } else {
            self.with_header("Cache-Control", format!("max-age={}", max_age_seconds))
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new(200)
    }
}

