//! MongoDB Service Connector

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ServiceConnector;
use crate::api::ServiceType;

/// MongoDB connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongodbConfig {
    /// MongoDB connection URI (mongodb:// or mongodb+srv://)
    /// Password will be masked in connection_info
    pub uri: String,
    /// Default database name
    pub database: String,
    /// Connection pool min size
    #[serde(default = "default_min_pool")]
    pub min_pool_size: u32,
    /// Connection pool max size
    #[serde(default = "default_max_pool")]
    pub max_pool_size: u32,
    /// Connection timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub connect_timeout_ms: u64,
}

fn default_min_pool() -> u32 { 1 }
fn default_max_pool() -> u32 { 10 }
fn default_timeout() -> u64 { 10000 }

/// MongoDB service connector
pub struct MongodbConnector {
    config: MongodbConfig,
}

impl MongodbConnector {
    pub fn new(config: MongodbConfig) -> Self {
        Self { config }
    }
    
    /// Get the database name
    pub fn database(&self) -> &str {
        &self.config.database
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &MongodbConfig {
        &self.config
    }
    
    /// Get sanitized URI (password masked)
    fn sanitized_uri(&self) -> String {
        // Simple password masking - replace password in URI
        if let Some(at_pos) = self.config.uri.find('@') {
            if let Some(colon_pos) = self.config.uri[..at_pos].rfind(':') {
                let prefix = &self.config.uri[..colon_pos + 1];
                let suffix = &self.config.uri[at_pos..];
                return format!("{}****{}", prefix, suffix);
            }
        }
        self.config.uri.clone()
    }
}

impl ServiceConnector for MongodbConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Mongodb
    }
    
    fn test_connection(&self) -> Result<()> {
        // Would test connection
        // Placeholder - actual implementation would use mongodb crate
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "mongodb",
            "uri": self.sanitized_uri(),
            "database": self.config.database,
            "min_pool_size": self.config.min_pool_size,
            "max_pool_size": self.config.max_pool_size,
            "connect_timeout_ms": self.config.connect_timeout_ms,
        })
    }
}

