//! Cache actor service
//!
//! Provides a long-lived cache service with actor-based message passing.
//! Supports Redis-like key-value operations.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::runtime::actor::{ActorMessage, ActorHandle, spawn_actor};
use super::ServiceError;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache type: redis, memcached, memory
    #[serde(rename = "type")]
    pub cache_type: String,
    
    /// Connection URL (for Redis/Memcached)
    #[serde(default)]
    pub url: Option<String>,
    
    /// Default TTL in seconds
    #[serde(default = "default_ttl")]
    pub default_ttl_secs: u64,
    
    /// Maximum entries (for in-memory cache)
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
}

fn default_ttl() -> u64 { 3600 }
fn default_max_entries() -> usize { 10000 }

/// Commands sent to the cache actor
pub enum CacheCommand {
    /// Get a value by key
    Get {
        key: String,
        reply: oneshot::Sender<Result<Option<String>, ServiceError>>,
    },
    
    /// Set a value with optional TTL
    Set {
        key: String,
        value: String,
        ttl_secs: Option<u64>,
        reply: oneshot::Sender<Result<(), ServiceError>>,
    },
    
    /// Delete a key
    Delete {
        key: String,
        reply: oneshot::Sender<Result<bool, ServiceError>>,
    },
    
    /// Check if key exists
    Exists {
        key: String,
        reply: oneshot::Sender<Result<bool, ServiceError>>,
    },
    
    /// Get multiple keys
    MGet {
        keys: Vec<String>,
        reply: oneshot::Sender<Result<Vec<Option<String>>, ServiceError>>,
    },
    
    /// Increment a numeric value
    Incr {
        key: String,
        amount: i64,
        reply: oneshot::Sender<Result<i64, ServiceError>>,
    },
    
    /// Health check
    Health {
        reply: oneshot::Sender<Result<bool, ServiceError>>,
    },
    
    /// Shutdown
    Shutdown,
}

impl ActorMessage for CacheCommand {}

/// Cache entry with expiration
struct CacheEntry {
    value: String,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Instant::now() > e).unwrap_or(false)
    }
}

/// Cache service handle - cheap to clone
#[derive(Clone)]
pub struct Cache {
    handle: ActorHandle<CacheCommand>,
    config: CacheConfig,
}

impl Cache {
    /// Start the cache actor and return a handle
    pub async fn start(config: CacheConfig) -> Result<Self, ServiceError> {
        let config_clone = config.clone();
        
        let handle = spawn_actor(100, move |rx| {
            cache_actor(config_clone, rx)
        });
        
        Ok(Self { handle, config })
    }
    
    /// Get a value by key
    pub async fn get(&self, key: &str) -> Result<Option<String>, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::Get {
            key: key.to_string(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
    
    /// Get a value and deserialize from JSON
    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, ServiceError> {
        let value = self.get(key).await?;
        match value {
            Some(s) => serde_json::from_str(&s)
                .map(Some)
                .map_err(|e| ServiceError::QueryFailed(format!("JSON parse error: {}", e))),
            None => Ok(None),
        }
    }
    
    /// Set a value with optional TTL
    pub async fn set(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::Set {
            key: key.to_string(),
            value: value.to_string(),
            ttl_secs,
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
    
    /// Set a value serialized as JSON
    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T, ttl_secs: Option<u64>) -> Result<(), ServiceError> {
        let json = serde_json::to_string(value)
            .map_err(|e| ServiceError::QueryFailed(format!("JSON serialize error: {}", e)))?;
        self.set(key, &json, ttl_secs).await
    }
    
    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<bool, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::Delete {
            key: key.to_string(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
    
    /// Check if key exists
    pub async fn exists(&self, key: &str) -> Result<bool, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::Exists {
            key: key.to_string(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
    
    /// Get multiple keys at once
    pub async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<String>>, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::MGet {
            keys: keys.iter().map(|s| s.to_string()).collect(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
    
    /// Increment a numeric value
    pub async fn incr(&self, key: &str, amount: i64) -> Result<i64, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::Incr {
            key: key.to_string(),
            amount,
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
    
    /// Health check
    pub async fn health(&self) -> Result<bool, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(CacheCommand::Health { reply: tx })
            .await
            .map_err(|_| ServiceError::Unavailable("cache actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from cache actor".into()))?
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("type", &self.config.cache_type)
            .field("alive", &self.handle.is_alive())
            .finish()
    }
}

/// The cache actor loop
async fn cache_actor(config: CacheConfig, mut rx: mpsc::Receiver<CacheCommand>) {
    tracing::info!("Starting cache actor ({})", config.cache_type);
    
    // In-memory cache storage (for memory type or as fallback)
    let mut store: HashMap<String, CacheEntry> = HashMap::new();
    let default_ttl = config.default_ttl_secs;
    let max_entries = config.max_entries;
    
    while let Some(cmd) = rx.recv().await {
        // Clean expired entries periodically (simple approach)
        if store.len() > max_entries {
            store.retain(|_, v| !v.is_expired());
        }
        
        match cmd {
            CacheCommand::Get { key, reply } => {
                let result = match store.get(&key) {
                    Some(entry) if !entry.is_expired() => Ok(Some(entry.value.clone())),
                    Some(_) => {
                        store.remove(&key);
                        Ok(None)
                    }
                    None => Ok(None),
                };
                let _ = reply.send(result);
            }
            
            CacheCommand::Set { key, value, ttl_secs, reply } => {
                let ttl = ttl_secs.unwrap_or(default_ttl);
                let expires_at = if ttl > 0 {
                    Some(Instant::now() + Duration::from_secs(ttl))
                } else {
                    None
                };
                
                store.insert(key, CacheEntry { value, expires_at });
                let _ = reply.send(Ok(()));
            }
            
            CacheCommand::Delete { key, reply } => {
                let existed = store.remove(&key).is_some();
                let _ = reply.send(Ok(existed));
            }
            
            CacheCommand::Exists { key, reply } => {
                let exists = store.get(&key)
                    .map(|e| !e.is_expired())
                    .unwrap_or(false);
                let _ = reply.send(Ok(exists));
            }
            
            CacheCommand::MGet { keys, reply } => {
                let values: Vec<Option<String>> = keys.iter()
                    .map(|key| {
                        store.get(key)
                            .filter(|e| !e.is_expired())
                            .map(|e| e.value.clone())
                    })
                    .collect();
                let _ = reply.send(Ok(values));
            }
            
            CacheCommand::Incr { key, amount, reply } => {
                let result = match store.get_mut(&key) {
                    Some(entry) if !entry.is_expired() => {
                        match entry.value.parse::<i64>() {
                            Ok(n) => {
                                let new_val = n + amount;
                                entry.value = new_val.to_string();
                                Ok(new_val)
                            }
                            Err(_) => Err(ServiceError::QueryFailed("value is not an integer".into())),
                        }
                    }
                    _ => {
                        // Key doesn't exist, create with initial value
                        store.insert(key, CacheEntry {
                            value: amount.to_string(),
                            expires_at: None,
                        });
                        Ok(amount)
                    }
                };
                let _ = reply.send(result);
            }
            
            CacheCommand::Health { reply } => {
                let _ = reply.send(Ok(true));
            }
            
            CacheCommand::Shutdown => {
                tracing::info!("Cache actor shutting down");
                break;
            }
        }
    }
    
    tracing::info!("Cache actor stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig {
            cache_type: "memory".to_string(),
            url: None,
            default_ttl_secs: 3600,
            max_entries: 1000,
        };
        
        let cache = Cache::start(config).await.unwrap();
        
        // Set and get
        cache.set("key1", "value1", None).await.unwrap();
        let value = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
        
        // Non-existent key
        let missing = cache.get("missing").await.unwrap();
        assert!(missing.is_none());
        
        // Delete
        let deleted = cache.delete("key1").await.unwrap();
        assert!(deleted);
        
        // Exists
        cache.set("key2", "value2", None).await.unwrap();
        assert!(cache.exists("key2").await.unwrap());
        assert!(!cache.exists("missing").await.unwrap());
        
        // Increment
        let val = cache.incr("counter", 1).await.unwrap();
        assert_eq!(val, 1);
        let val = cache.incr("counter", 5).await.unwrap();
        assert_eq!(val, 6);
    }
    
    #[tokio::test]
    async fn test_cache_json() {
        let config = CacheConfig {
            cache_type: "memory".to_string(),
            url: None,
            default_ttl_secs: 3600,
            max_entries: 1000,
        };
        
        let cache = Cache::start(config).await.unwrap();
        
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct User {
            id: i32,
            name: String,
        }
        
        let user = User { id: 1, name: "Alice".to_string() };
        cache.set_json("user:1", &user, None).await.unwrap();
        
        let loaded: User = cache.get_json("user:1").await.unwrap().unwrap();
        assert_eq!(loaded, user);
    }
}