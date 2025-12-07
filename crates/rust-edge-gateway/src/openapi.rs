//! OpenAPI 3.x import functionality
//!
//! Parses OpenAPI specs and creates endpoints from them.

use anyhow::{Context, Result};
use openapiv3::{OpenAPI, PathItem, Operation, ReferenceOr};
use uuid::Uuid;

use crate::api::Endpoint;

/// Parsed endpoint from OpenAPI spec
#[derive(Debug, Clone)]
pub struct ParsedEndpoint {
    pub name: String,
    pub path: String,
    pub method: String,
    pub description: Option<String>,
}

/// Import result containing collection info and endpoints
#[derive(Debug)]
pub struct OpenApiImportResult {
    pub title: String,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub version: String,
    pub base_path: String,
    pub endpoints: Vec<ParsedEndpoint>,
}

/// Parse OpenAPI spec from YAML or JSON string
pub fn parse_openapi(content: &str) -> Result<OpenApiImportResult> {
    // Try YAML first, then JSON
    let spec: OpenAPI = serde_yaml::from_str(content)
        .or_else(|_| serde_json::from_str(content))
        .context("Failed to parse OpenAPI spec as YAML or JSON")?;
    
    let title = spec.info.title.clone();
    let description = spec.info.description.clone();
    let version = spec.info.version.clone();
    
    // Extract base path from servers if available
    let base_path = spec.servers
        .first()
        .and_then(|s| {
            url::Url::parse(&s.url).ok().map(|u| u.path().to_string())
        })
        .unwrap_or_default();
    
    let mut endpoints = Vec::new();
    
    for (path, path_item) in &spec.paths.paths {
        if let ReferenceOr::Item(item) = path_item {
            endpoints.extend(extract_operations(path, item));
        }
    }
    
    Ok(OpenApiImportResult {
        title,
        description,
        version,
        base_path,
        endpoints,
    })
}

fn extract_operations(path: &str, item: &PathItem) -> Vec<ParsedEndpoint> {
    let mut endpoints = Vec::new();
    
    let methods = [
        ("GET", &item.get),
        ("POST", &item.post),
        ("PUT", &item.put),
        ("DELETE", &item.delete),
        ("PATCH", &item.patch),
        ("HEAD", &item.head),
        ("OPTIONS", &item.options),
    ];
    
    for (method, op) in methods {
        if let Some(operation) = op {
            endpoints.push(create_parsed_endpoint(path, method, operation));
        }
    }
    
    endpoints
}

fn create_parsed_endpoint(path: &str, method: &str, op: &Operation) -> ParsedEndpoint {
    let name = op.operation_id.clone()
        .unwrap_or_else(|| format!("{} {}", method, path));
    
    ParsedEndpoint {
        name,
        path: convert_openapi_path(path),
        method: method.to_string(),
        description: op.summary.clone().or_else(|| op.description.clone()),
    }
}

/// Convert OpenAPI path params {id} to our format {id}
/// OpenAPI uses {param} which is the same as ours, so just return as-is
fn convert_openapi_path(path: &str) -> String {
    path.to_string()
}

/// Create Endpoint structs from parsed OpenAPI result
pub fn create_endpoints_from_import(
    import: &OpenApiImportResult,
    domain: &str,
    collection_id: Option<&str>,
) -> Vec<Endpoint> {
    import.endpoints.iter().map(|parsed| {
        Endpoint {
            id: Uuid::new_v4().to_string(),
            collection_id: collection_id.map(|s| s.to_string()),
            name: parsed.name.clone(),
            domain: domain.to_string(),
            path: parsed.path.clone(),
            method: parsed.method.clone(),
            description: parsed.description.clone(),
            code: Some(generate_default_handler(&parsed.name)),
            compiled: false,
            enabled: true,
            created_at: None,
            updated_at: None,
        }
    }).collect()
}

/// Generate a default handler stub for an endpoint
fn generate_default_handler(name: &str) -> String {
    format!(r#"// Handler for: {}
use rust_edge_gateway_sdk::{{Request, Response}};

pub fn handle(req: Request) -> Response {{
    Response::json(serde_json::json!({{
        "message": "Not implemented: {}",
        "method": req.method,
        "path": req.path
    }}))
}}
"#, name, name)
}

