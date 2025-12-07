//! Redis Service Connector

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ServiceConnector;
use crate::api::ServiceType;

/// Redis connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis host
    pub host: String,
    /// Redis port
    #[serde(default = "default_redis_port")]
    pub port: u16,
    /// Password (optional, not serialized)
    #[serde(skip_serializing, default)]
    pub password: Option<String>,
    /// Database number (0-15)
    #[serde(default)]
    pub database: u8,
    /// Use TLS
    #[serde(default)]
    pub use_tls: bool,
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    /// Username (for Redis 6+ ACL)
    pub username: Option<String>,
}

fn default_redis_port() -> u16 { 6379 }
fn default_pool_size() -> u32 { 10 }

/// Redis service connector
pub struct RedisConnector {
    config: RedisConfig,
}

impl RedisConnector {
    pub fn new(config: RedisConfig) -> Self {
        Self { config }
    }
    
    /// Get the connection URL (without password)
    pub fn connection_url(&self) -> String {
        let protocol = if self.config.use_tls { "rediss" } else { "redis" };
        format!(
            "{}://{}:{}/{}",
            protocol,
            self.config.host,
            self.config.port,
            self.config.database
        )
    }
    
    /// Get configuration for establishing connection
    pub fn get_config(&self) -> &RedisConfig {
        &self.config
    }
}

impl ServiceConnector for RedisConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Redis
    }
    
    fn test_connection(&self) -> Result<()> {
        // Would send PING command
        // Placeholder - actual implementation would use redis crate
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "redis",
            "host": self.config.host,
            "port": self.config.port,
            "database": self.config.database,
            "use_tls": self.config.use_tls,
            "pool_size": self.config.pool_size,
        })
    }
}

