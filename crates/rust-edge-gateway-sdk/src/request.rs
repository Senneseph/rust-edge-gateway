//! HTTP Request representation for handlers

use crate::error::HandlerError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Represents an incoming HTTP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,

    /// Request path (e.g., "/items/123")
    pub path: String,

    /// Query parameters
    #[serde(default)]
    pub query: HashMap<String, String>,

    /// HTTP headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body (raw bytes as base64 for binary, or string for text)
    #[serde(default)]
    pub body: Option<String>,

    /// Path parameters extracted from route (e.g., {id} -> "123")
    #[serde(default)]
    pub params: HashMap<String, String>,

    /// Client IP address
    #[serde(default)]
    pub client_ip: Option<String>,

    /// Request ID for tracing
    #[serde(default)]
    pub request_id: String,
}

impl Request {
    /// Parse the body as JSON into a typed struct.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct CreateUser { name: String, email: String }
    ///
    /// let user: CreateUser = req.json()?;
    /// ```
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, HandlerError> {
        match &self.body {
            Some(body) => serde_json::from_str(body)
                .map_err(|e| HandlerError::BadRequest(format!("Invalid JSON: {}", e))),
            None => serde_json::from_str("null")
                .map_err(|e| HandlerError::BadRequest(format!("Invalid JSON: {}", e))),
        }
    }

    /// Get a query parameter as a string reference.
    ///
    /// # Example
    /// ```ignore
    /// // URL: /search?q=rust
    /// let query = req.query_param("q"); // Some(&"rust".to_string())
    /// ```
    pub fn query_param(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    /// Get a query parameter parsed as a specific type.
    /// Returns None if the parameter doesn't exist or can't be parsed.
    ///
    /// # Example
    /// ```ignore
    /// // URL: /items?page=2&limit=10
    /// let page: i64 = req.query_param_as("page").unwrap_or(1);
    /// let limit: usize = req.query_param_as("limit").unwrap_or(20);
    /// ```
    pub fn query_param_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.query.get(key).and_then(|v| v.parse().ok())
    }

    /// Get a required query parameter parsed as a specific type.
    /// Returns HandlerError::BadRequest if missing or invalid.
    ///
    /// # Example
    /// ```ignore
    /// let page: i64 = req.require_query_param("page")?;
    /// ```
    pub fn require_query_param<T: FromStr>(&self, key: &str) -> Result<T, HandlerError> {
        self.query.get(key)
            .ok_or_else(|| HandlerError::BadRequest(format!("Missing required query parameter: {}", key)))?
            .parse()
            .map_err(|_| HandlerError::BadRequest(format!("Invalid value for query parameter: {}", key)))
    }

    /// Get a path parameter as a string reference.
    ///
    /// # Example
    /// ```ignore
    /// // Route: /users/{id}, Request: /users/123
    /// let id = req.path_param("id"); // Some(&"123".to_string())
    /// ```
    pub fn path_param(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }

    /// Get a path parameter parsed as a specific type.
    ///
    /// # Example
    /// ```ignore
    /// // Route: /users/{id}, Request: /users/123
    /// let id: i64 = req.path_param_as("id").unwrap_or(0);
    /// ```
    pub fn path_param_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.params.get(key).and_then(|v| v.parse().ok())
    }

    /// Get a required path parameter parsed as a specific type.
    /// Returns HandlerError::BadRequest if missing or invalid.
    ///
    /// # Example
    /// ```ignore
    /// let user_id: i64 = req.require_path_param("id")?;
    /// ```
    pub fn require_path_param<T: FromStr>(&self, key: &str) -> Result<T, HandlerError> {
        self.params.get(key)
            .ok_or_else(|| HandlerError::BadRequest(format!("Missing required path parameter: {}", key)))?
            .parse()
            .map_err(|_| HandlerError::BadRequest(format!("Invalid value for path parameter: {}", key)))
    }

    /// Get a header value (case-insensitive lookup).
    ///
    /// # Example
    /// ```ignore
    /// let auth = req.header("Authorization"); // Works with any case
    /// let content_type = req.header("content-type");
    /// ```
    pub fn header(&self, key: &str) -> Option<&String> {
        let key_lower = key.to_lowercase();
        self.headers.iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v)
    }

    /// Get a required header value.
    /// Returns HandlerError::BadRequest if missing.
    ///
    /// # Example
    /// ```ignore
    /// let auth = req.require_header("Authorization")?;
    /// ```
    pub fn require_header(&self, key: &str) -> Result<&String, HandlerError> {
        self.header(key)
            .ok_or_else(|| HandlerError::BadRequest(format!("Missing required header: {}", key)))
    }

    /// Check if request method matches (case-insensitive).
    ///
    /// # Example
    /// ```ignore
    /// if req.is_method("POST") { /* handle POST */ }
    /// ```
    pub fn is_method(&self, method: &str) -> bool {
        self.method.eq_ignore_ascii_case(method)
    }

    /// Get the raw body as bytes (useful for binary data).
    /// The body is stored as a string, so this converts it to bytes.
    pub fn body_bytes(&self) -> Vec<u8> {
        self.body.as_ref()
            .map(|b| b.as_bytes().to_vec())
            .unwrap_or_default()
    }

    /// Get the Content-Type header value.
    pub fn content_type(&self) -> Option<&String> {
        self.header("Content-Type")
    }

    /// Check if the request has a JSON content type.
    pub fn is_json(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    /// Check if the request has a multipart/form-data content type.
    pub fn is_multipart(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("multipart/form-data"))
            .unwrap_or(false)
    }

    /// Parse multipart/form-data body.
    /// Returns a MultipartData struct containing parsed fields and files.
    ///
    /// # Example
    /// ```ignore
    /// let multipart = req.multipart()?;
    /// let title = multipart.field("title")?;
    /// let file = multipart.file("upload")?;
    /// ```
    pub fn multipart(&self) -> Result<MultipartData, HandlerError> {
        let content_type = self.content_type()
            .ok_or_else(|| HandlerError::BadRequest("Missing Content-Type header".into()))?;

        if !content_type.contains("multipart/form-data") {
            return Err(HandlerError::BadRequest("Content-Type is not multipart/form-data".into()));
        }

        // Extract boundary
        let boundary = content_type
            .split("boundary=")
            .nth(1)
            .map(|b| b.trim_matches('"').to_string())
            .ok_or_else(|| HandlerError::BadRequest("Missing multipart boundary".into()))?;

        let body = self.body.as_ref()
            .ok_or_else(|| HandlerError::BadRequest("Empty body for multipart request".into()))?;

        MultipartData::parse(body, &boundary)
    }
}

/// Parsed multipart/form-data content
#[derive(Debug, Clone, Default)]
pub struct MultipartData {
    /// Text fields (name -> value)
    pub fields: HashMap<String, String>,
    /// File uploads (name -> MultipartFile)
    pub files: HashMap<String, MultipartFile>,
}

/// A file uploaded via multipart/form-data
#[derive(Debug, Clone)]
pub struct MultipartFile {
    /// Original filename
    pub filename: String,
    /// Content-Type of the file
    pub content_type: String,
    /// File content as bytes
    pub data: Vec<u8>,
}

impl MultipartData {
    /// Parse multipart body with the given boundary
    pub fn parse(body: &str, boundary: &str) -> Result<Self, HandlerError> {
        let mut result = MultipartData::default();
        let delimiter = format!("--{}", boundary);
        let _end_delimiter = format!("--{}--", boundary);

        // Split by boundary
        let parts: Vec<&str> = body.split(&delimiter).collect();

        for part in parts {
            let part = part.trim();
            if part.is_empty() || part == "--" || part.starts_with("--") {
                continue;
            }

            // Find the header/body separator (empty line)
            let header_end = part.find("\r\n\r\n")
                .or_else(|| part.find("\n\n"));

            if let Some(pos) = header_end {
                let headers_str = &part[..pos];
                let body_start = if part[pos..].starts_with("\r\n\r\n") { pos + 4 } else { pos + 2 };
                let body_content = part[body_start..].trim_end_matches("\r\n").trim_end_matches("--");

                // Parse Content-Disposition header
                let mut name = None;
                let mut filename = None;
                let mut content_type = "text/plain".to_string();

                for line in headers_str.lines() {
                    let line = line.trim();
                    if line.to_lowercase().starts_with("content-disposition:") {
                        // Parse name and filename from Content-Disposition
                        if let Some(n) = extract_header_param(line, "name") {
                            name = Some(n);
                        }
                        if let Some(f) = extract_header_param(line, "filename") {
                            filename = Some(f);
                        }
                    } else if line.to_lowercase().starts_with("content-type:") {
                        content_type = line.split(':').nth(1)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "text/plain".to_string());
                    }
                }

                if let Some(field_name) = name {
                    if let Some(file_name) = filename {
                        // This is a file
                        result.files.insert(field_name, MultipartFile {
                            filename: file_name,
                            content_type,
                            data: body_content.as_bytes().to_vec(),
                        });
                    } else {
                        // This is a text field
                        result.fields.insert(field_name, body_content.to_string());
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get a text field by name
    pub fn field(&self, name: &str) -> Option<&String> {
        self.fields.get(name)
    }

    /// Get a required text field by name
    pub fn require_field(&self, name: &str) -> Result<&String, HandlerError> {
        self.fields.get(name)
            .ok_or_else(|| HandlerError::BadRequest(format!("Missing required field: {}", name)))
    }

    /// Get a file by name
    pub fn file(&self, name: &str) -> Option<&MultipartFile> {
        self.files.get(name)
    }

    /// Get a required file by name
    pub fn require_file(&self, name: &str) -> Result<&MultipartFile, HandlerError> {
        self.files.get(name)
            .ok_or_else(|| HandlerError::BadRequest(format!("Missing required file: {}", name)))
    }
}

/// Helper to extract a parameter value from a header like Content-Disposition
fn extract_header_param(header: &str, param: &str) -> Option<String> {
    let search = format!("{}=", param);
    header.find(&search).map(|pos| {
        let start = pos + search.len();
        let rest = &header[start..];
        if rest.starts_with('"') {
            // Quoted value
            rest[1..].split('"').next()
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            // Unquoted value
            rest.split(|c| c == ';' || c == ' ')
                .next()
                .map(|s| s.to_string())
                .unwrap_or_default()
        }
    })
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            path: "/".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            params: HashMap::new(),
            client_ip: None,
            request_id: String::new(),
        }
    }
}

