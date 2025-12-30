use rust_edge_gateway_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TestQuery {
    query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryResult {
    success: bool,
    message: String,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

/// Test handler for SQLite connectivity
/// 
/// This handler demonstrates how to connect to the live-sqlite container
/// and execute queries. It supports the following endpoints:
///
/// GET /health - Check SQLite service health
/// POST /query - Execute a test query
/// 
/// Example requests:
/// ```
/// # Check health
/// curl http://localhost:8080/sqlite-test/health
///
/// # Execute a test query
/// curl -X POST http://localhost:8080/sqlite-test/query \
///   -H "Content-Type: application/json" \
///   -d '{"query": "SELECT sqlite_version();"}'
/// ```
pub async fn handle(req: Request) -> Response {
    // Route based on path
    match req.path.as_str() {
        "/health" => health_check(),
        "/query" => {
            if req.method == "POST" {
                query_handler(req).await
            } else {
                Response::method_not_allowed()
            }
        }
        "/test-connection" => test_connection().await,
        "/create-table" => create_test_table().await,
        "/insert-data" => insert_test_data().await,
        _ => Response::not_found()
    }
}

/// Health check endpoint - verifies the handler is running
fn health_check() -> Response {
    Response::ok(json!({
        "status": "ok",
        "handler": "a-icon-sqlite-test",
        "version": "0.1.0"
    }))
}

/// Test the connection to the SQLite service
async fn test_connection() -> Response {
    let sqlite_host = std::env::var("SQLITE_SERVICE_HOST")
        .unwrap_or_else(|_| "localhost".to_string());
    let sqlite_port = std::env::var("SQLITE_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8282);

    let base_url = format!("http://{}:{}", sqlite_host, sqlite_port);
    
    match reqwest::Client::new()
        .get(format!("{}/health", base_url))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                Response::ok(json!({
                    "success": true,
                    "message": "SQLite service is healthy",
                    "service": {
                        "host": sqlite_host,
                        "port": sqlite_port,
                        "base_url": base_url
                    }
                }))
            } else {
                Response::service_unavailable(json!({
                    "success": false,
                    "message": "SQLite service returned an error",
                    "status": response.status().as_u16()
                }))
            }
        }
        Err(e) => {
            Response::service_unavailable(json!({
                "success": false,
                "message": "Failed to connect to SQLite service",
                "error": e.to_string(),
                "service": {
                    "host": sqlite_host,
                    "port": sqlite_port
                }
            }))
        }
    }
}

/// Execute a query handler
async fn query_handler(req: Request) -> Response {
    let query_req: TestQuery = match req.json() {
        Ok(q) => q,
        Err(e) => {
            return Response::bad_request(json!({
                "success": false,
                "error": format!("Invalid request: {}", e)
            }));
        }
    };

    let query = match query_req.query {
        Some(q) => q,
        None => {
            return Response::bad_request(json!({
                "success": false,
                "error": "Missing 'query' field in request body"
            }));
        }
    };

    let result = execute_sqlite_query(&query).await;
    
    match result {
        Ok(data) => {
            Response::ok(json!({
                "success": true,
                "message": "Query executed successfully",
                "data": data
            }))
        }
        Err(error) => {
            Response::internal_error(json!({
                "success": false,
                "error": error
            }))
        }
    }
}

/// Create a test table
async fn create_test_table() -> Response {
    let sql = r#"
        CREATE TABLE IF NOT EXISTS test_table (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    "#;

    match execute_sqlite_query(sql).await {
        Ok(_) => {
            Response::ok(json!({
                "success": true,
                "message": "Test table created successfully"
            }))
        }
        Err(e) => {
            Response::internal_error(json!({
                "success": false,
                "error": format!("Failed to create table: {}", e)
            }))
        }
    }
}

/// Insert test data
async fn insert_test_data() -> Response {
    let sql = "INSERT INTO test_table (name) VALUES ('Test Entry')";

    match execute_sqlite_query(sql).await {
        Ok(response) => {
            Response::ok(json!({
                "success": true,
                "message": "Data inserted successfully",
                "response": response
            }))
        }
        Err(e) => {
            Response::internal_error(json!({
                "success": false,
                "error": format!("Failed to insert data: {}", e)
            }))
        }
    }
}

/// Execute a query against the SQLite service via HTTP
async fn execute_sqlite_query(sql: &str) -> Result<serde_json::Value, String> {
    let sqlite_host = std::env::var("SQLITE_SERVICE_HOST")
        .unwrap_or_else(|_| "localhost".to_string());
    let sqlite_port = std::env::var("SQLITE_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8282);

    let base_url = format!("http://{}:{}", sqlite_host, sqlite_port);
    let url = format!("{}/query", base_url);

    let client = reqwest::Client::new();
    let body = json!({
        "sql": sql,
        "params": []
    });

    eprintln!("Executing SQLite query: {}", sql);
    eprintln!("URL: {}", url);

    match client.post(&url).json(&body).send().await {
        Ok(response) => {
            match response.text().await {
                Ok(text) => {
                    eprintln!("Response: {}", text);
                    // Try to parse as JSON, otherwise return as string
                    serde_json::from_str(&text)
                        .or_else(|_| Ok(json!({"raw_response": text})))
                }
                Err(e) => Err(format!("Failed to read response body: {}", e))
            }
        }
        Err(e) => Err(format!("HTTP request failed: {}", e))
    }
}
