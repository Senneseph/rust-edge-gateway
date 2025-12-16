//! SQLite HTTP Client for accessing remote SQLite databases
//!
//! Handlers can use this module to query the SQLite database via the HTTP API.
//! The SQLite service is accessed via environment variables:
//! - `SQLITE_SERVICE_HOST`: Host of the SQLite service (default: "localhost")
//! - `SQLITE_SERVICE_PORT`: Port of the SQLite service (default: 8282)

use serde_json::{json, Value};
use std::env;
use crate::error::HandlerError;

/// SQLite HTTP client configuration
pub struct SqliteClient {
    host: String,
    port: u16,
}

impl SqliteClient {
    /// Create a new SQLite client from environment variables
    pub fn from_env() -> Self {
        let host = env::var("SQLITE_SERVICE_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = env::var("SQLITE_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8282);
        
        Self { host, port }
    }

    /// Create a new SQLite client with explicit host and port
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Get the base URL for the SQLite service
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Execute a query and return the result as JSON
    /// 
    /// # Example
    /// ```ignore
    /// let client = SqliteClient::from_env();
    /// let result = client.query("SELECT * FROM users WHERE id = ?1", &[&1])?;
    /// ```
    pub fn query(&self, sql: &str, params: &[&dyn std::fmt::Display]) -> Result<Vec<Value>, HandlerError> {
        let url = format!("{}/query", self.base_url());
        
        let param_strings: Vec<String> = params.iter().map(|p| p.to_string()).collect();
        let body = json!({
            "sql": sql,
            "params": param_strings,
        });

        // Since we don't have async in the sync context, we'll use a simple approach
        // For now, return an error indicating that the feature needs to be called properly
        eprintln!("SQLite query: {} with params: {:?}", sql, param_strings);
        eprintln!("Would call: {}", url);
        
        // TODO: Implement actual HTTP client
        // For MVP, handlers should be built with async support
        Err(HandlerError::IpcError("Synchronous SQLite queries require async support. Use handler_loop_async_result! macro.".into()))
    }

    /// Execute an insert/update/delete statement
    /// 
    /// # Example
    /// ```ignore
    /// let client = SqliteClient::from_env();
    /// let rows_affected = client.execute("INSERT INTO users (name, email) VALUES (?1, ?2)", &[&"John", &"john@example.com"])?;
    /// ```
    pub fn execute(&self, sql: &str, params: &[&dyn std::fmt::Display]) -> Result<u64, HandlerError> {
        let url = format!("{}/execute", self.base_url());
        
        let param_strings: Vec<String> = params.iter().map(|p| p.to_string()).collect();
        let body = json!({
            "sql": sql,
            "params": param_strings,
        });

        eprintln!("SQLite execute: {} with params: {:?}", sql, param_strings);
        eprintln!("Would call: {}", url);
        
        // TODO: Implement actual HTTP client
        Err(HandlerError::IpcError("Synchronous SQLite queries require async support. Use handler_loop_async_result! macro.".into()))
    }

    /// Check if the SQLite service is healthy
    pub fn health_check(&self) -> Result<(), HandlerError> {
        let url = format!("{}/health", self.base_url());
        eprintln!("Checking SQLite service health at: {}", url);
        
        // For MVP, just verify configuration
        if self.host.is_empty() {
            return Err(HandlerError::IpcError("SQLite service host not configured".into()));
        }
        
        Ok(())
    }
}

#[cfg(feature = "async")]
pub mod r#async {
    use super::*;

    /// Async SQLite HTTP client
    pub struct AsyncSqliteClient {
        client: reqwest::Client,
        base_url: String,
    }

    impl AsyncSqliteClient {
        /// Create a new async SQLite client from environment variables
        pub fn from_env() -> Self {
            let host = env::var("SQLITE_SERVICE_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port: u16 = env::var("SQLITE_SERVICE_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8282);
            
            let base_url = format!("http://{}:{}", host, port);
            
            Self {
                client: reqwest::Client::new(),
                base_url,
            }
        }

        /// Create a new async SQLite client with explicit host and port
        pub fn new(host: impl Into<String>, port: u16) -> Self {
            let base_url = format!("http://{}:{}", host.into(), port);
            Self {
                client: reqwest::Client::new(),
                base_url,
            }
        }

        /// Execute a query and return the result as JSON
        pub async fn query(&self, sql: &str, params: &[&dyn std::fmt::Display]) -> Result<Vec<Value>, HandlerError> {
            let url = format!("{}/query", self.base_url);
            
            let param_strings: Vec<String> = params.iter().map(|p| p.to_string()).collect();
            let body = json!({
                "sql": sql,
                "params": param_strings,
            });

            let response = self.client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| HandlerError::IpcError(format!("Failed to query SQLite: {}", e)))?;

            if !response.status().is_success() {
                return Err(HandlerError::IpcError(format!("SQLite query failed with status {}", response.status())));
            }

            let result: Vec<Value> = response.json().await
                .map_err(|e| HandlerError::IpcError(format!("Failed to parse SQLite response: {}", e)))?;

            Ok(result)
        }

        /// Execute an insert/update/delete statement
        pub async fn execute(&self, sql: &str, params: &[&dyn std::fmt::Display]) -> Result<u64, HandlerError> {
            let url = format!("{}/execute", self.base_url);
            
            let param_strings: Vec<String> = params.iter().map(|p| p.to_string()).collect();
            let body = json!({
                "sql": sql,
                "params": param_strings,
            });

            let response = self.client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| HandlerError::IpcError(format!("Failed to execute SQLite statement: {}", e)))?;

            if !response.status().is_success() {
                return Err(HandlerError::IpcError(format!("SQLite execute failed with status {}", response.status())));
            }

            let result_json: Value = response.json().await
                .map_err(|e| HandlerError::IpcError(format!("Failed to parse SQLite response: {}", e)))?;

            // Extract rows_affected from response
            let rows_affected = result_json["rows_affected"]
                .as_u64()
                .unwrap_or(0);

            Ok(rows_affected)
        }

        /// Check if the SQLite service is healthy
        pub async fn health_check(&self) -> Result<(), HandlerError> {
            let url = format!("{}/health", self.base_url);
            
            let response = self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| HandlerError::IpcError(format!("SQLite health check failed: {}", e)))?;

            if response.status().is_success() {
                Ok(())
            } else {
                Err(HandlerError::IpcError("SQLite service unhealthy".into()))
            }
        }
    }
}
