//! PostgreSQL Service Connector

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ServiceConnector;
use crate::api::ServiceType;

/// PostgreSQL connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database host
    pub host: String,
    /// Database port
    #[serde(default = "default_postgres_port")]
    pub port: u16,
    /// Database name
    pub database: String,
    /// Username
    pub username: String,
    /// Password (not serialized for safety)
    #[serde(skip_serializing)]
    pub password: String,
    /// Use SSL/TLS
    #[serde(default)]
    pub use_ssl: bool,
    /// SSL mode (disable, allow, prefer, require, verify-ca, verify-full)
    #[serde(default = "default_ssl_mode")]
    pub ssl_mode: String,
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

fn default_postgres_port() -> u16 { 5432 }
fn default_ssl_mode() -> String { "prefer".to_string() }
fn default_pool_size() -> u32 { 10 }

/// PostgreSQL service connector
pub struct PostgresConnector {
    config: PostgresConfig,
}

impl PostgresConnector {
    pub fn new(config: PostgresConfig) -> Self {
        Self { config }
    }
    
    /// Get the connection string (without password)
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}@{}:{}/{}?sslmode={}",
            self.config.username,
            self.config.host,
            self.config.port,
            self.config.database,
            self.config.ssl_mode
        )
    }
    
    /// Get configuration for establishing connection
    pub fn get_config(&self) -> &PostgresConfig {
        &self.config
    }
}

impl ServiceConnector for PostgresConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Postgres
    }
    
    fn test_connection(&self) -> Result<()> {
        // Would establish a test connection
        // Placeholder - actual implementation would use tokio-postgres or sqlx
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "postgres",
            "host": self.config.host,
            "port": self.config.port,
            "database": self.config.database,
            "username": self.config.username,
            "ssl_mode": self.config.ssl_mode,
            "pool_size": self.config.pool_size,
        })
    }
}

