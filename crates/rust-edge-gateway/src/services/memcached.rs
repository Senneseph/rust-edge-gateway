//! Memcached Service Connector

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ServiceConnector;
use crate::api::ServiceType;

/// Memcached connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemcachedConfig {
    /// Memcached servers (host:port format)
    pub servers: Vec<String>,
    /// Connection pool size per server
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    /// Connection timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_pool_size() -> u32 { 5 }
fn default_timeout() -> u64 { 1000 }

/// Memcached service connector
pub struct MemcachedConnector {
    config: MemcachedConfig,
}

impl MemcachedConnector {
    pub fn new(config: MemcachedConfig) -> Self {
        Self { config }
    }
    
    /// Get list of servers
    pub fn servers(&self) -> &[String] {
        &self.config.servers
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &MemcachedConfig {
        &self.config
    }
}

impl ServiceConnector for MemcachedConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Memcached
    }
    
    fn test_connection(&self) -> Result<()> {
        // Would test connection to each server
        // Placeholder - actual implementation would use memcache crate
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "memcached",
            "servers": self.config.servers,
            "pool_size": self.config.pool_size,
            "timeout_ms": self.config.timeout_ms,
        })
    }
}

