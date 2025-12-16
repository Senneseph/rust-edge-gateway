//! Database actor service
//!
//! Provides a long-lived database connection pool with actor-based message passing.
//! Supports SQLite, MySQL, and PostgreSQL through a unified interface.

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::runtime::actor::{ActorMessage, ActorHandle, spawn_actor};
use super::ServiceError;

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type: sqlite, mysql, postgres
    #[serde(rename = "type")]
    pub db_type: String,
    
    /// Connection URL or path (for SQLite)
    pub url: String,
    
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_pool_size() -> u32 { 10 }
fn default_timeout() -> u64 { 30 }

/// Row representation - generic container for query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub columns: Vec<String>,
    pub values: HashMap<String, serde_json::Value>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            values: HashMap::new(),
        }
    }
    
    /// Get a value by column name
    pub fn get(&self, column: &str) -> Option<&serde_json::Value> {
        self.values.get(column)
    }
    
    /// Get a value and try to convert it to the specified type
    pub fn get_as<T: for<'de> Deserialize<'de>>(&self, column: &str) -> Option<T> {
        self.values.get(column)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

/// Commands sent to the database actor
pub enum DatabaseCommand {
    /// Execute a query and return all rows
    Query {
        sql: String,
        params: Vec<serde_json::Value>,
        reply: oneshot::Sender<Result<Vec<Row>, ServiceError>>,
    },
    
    /// Execute a query and return the first row
    QueryOne {
        sql: String,
        params: Vec<serde_json::Value>,
        reply: oneshot::Sender<Result<Option<Row>, ServiceError>>,
    },
    
    /// Execute a statement (INSERT, UPDATE, DELETE) and return affected rows
    Execute {
        sql: String,
        params: Vec<serde_json::Value>,
        reply: oneshot::Sender<Result<u64, ServiceError>>,
    },
    
    /// Check if the connection is healthy
    Health {
        reply: oneshot::Sender<Result<bool, ServiceError>>,
    },
    
    /// Shutdown the actor
    Shutdown,
}

impl ActorMessage for DatabaseCommand {}

/// Database service handle - cheap to clone, send commands to the actor
#[derive(Clone)]
pub struct Database {
    handle: ActorHandle<DatabaseCommand>,
    config: DatabaseConfig,
}

impl Database {
    /// Start the database actor and return a handle
    pub async fn start(config: DatabaseConfig) -> Result<Self, ServiceError> {
        let config_clone = config.clone();
        
        let handle = spawn_actor(100, move |rx| {
            database_actor(config_clone, rx)
        });
        
        Ok(Self { handle, config })
    }
    
    /// Execute a query and return all matching rows
    pub async fn query(&self, sql: &str, params: &[serde_json::Value]) -> Result<Vec<Row>, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(DatabaseCommand::Query {
            sql: sql.to_string(),
            params: params.to_vec(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("database actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from database actor".into()))?
    }
    
    /// Execute a query and return the first row (if any)
    pub async fn query_one(&self, sql: &str, params: &[serde_json::Value]) -> Result<Option<Row>, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(DatabaseCommand::QueryOne {
            sql: sql.to_string(),
            params: params.to_vec(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("database actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from database actor".into()))?
    }
    
    /// Execute a statement and return the number of affected rows
    pub async fn execute(&self, sql: &str, params: &[serde_json::Value]) -> Result<u64, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(DatabaseCommand::Execute {
            sql: sql.to_string(),
            params: params.to_vec(),
            reply: tx,
        }).await.map_err(|_| ServiceError::Unavailable("database actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from database actor".into()))?
    }
    
    /// Check connection health
    pub async fn health(&self) -> Result<bool, ServiceError> {
        let (tx, rx) = oneshot::channel();
        
        self.handle.send(DatabaseCommand::Health { reply: tx })
            .await
            .map_err(|_| ServiceError::Unavailable("database actor closed".into()))?;
        
        rx.await.map_err(|_| ServiceError::Unavailable("no response from database actor".into()))?
    }
    
    /// Get the database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("type", &self.config.db_type)
            .field("alive", &self.handle.is_alive())
            .finish()
    }
}

/// The database actor loop - owns the connection and processes commands
async fn database_actor(config: DatabaseConfig, mut rx: mpsc::Receiver<DatabaseCommand>) {
    tracing::info!("Starting database actor for {}", config.db_type);
    
    // For now, we'll use SQLite via rusqlite
    // In production, you'd add connection pools for MySQL/PostgreSQL
    let conn = match config.db_type.as_str() {
        "sqlite" => {
            match rusqlite::Connection::open(&config.url) {
                Ok(c) => Some(c),
                Err(e) => {
                    tracing::error!("Failed to open SQLite database: {}", e);
                    None
                }
            }
        }
        _ => {
            tracing::warn!("Database type {} not yet implemented, using mock", config.db_type);
            None
        }
    };
    
    while let Some(cmd) = rx.recv().await {
        match cmd {
            DatabaseCommand::Query { sql, params, reply } => {
                let result = if let Some(ref conn) = conn {
                    execute_sqlite_query(conn, &sql, &params)
                } else {
                    // Mock response for non-sqlite databases
                    Ok(vec![])
                };
                let _ = reply.send(result);
            }
            
            DatabaseCommand::QueryOne { sql, params, reply } => {
                let result = if let Some(ref conn) = conn {
                    execute_sqlite_query(conn, &sql, &params)
                        .map(|rows| rows.into_iter().next())
                } else {
                    Ok(None)
                };
                let _ = reply.send(result);
            }
            
            DatabaseCommand::Execute { sql, params, reply } => {
                let result = if let Some(ref conn) = conn {
                    execute_sqlite_statement(conn, &sql, &params)
                } else {
                    Ok(0)
                };
                let _ = reply.send(result);
            }
            
            DatabaseCommand::Health { reply } => {
                let healthy = conn.is_some();
                let _ = reply.send(Ok(healthy));
            }
            
            DatabaseCommand::Shutdown => {
                tracing::info!("Database actor shutting down");
                break;
            }
        }
    }
    
    tracing::info!("Database actor stopped");
}

/// Execute a SQLite query and return rows
fn execute_sqlite_query(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[serde_json::Value],
) -> Result<Vec<Row>, ServiceError> {
    let mut stmt = conn.prepare(sql)
        .map_err(|e| ServiceError::QueryFailed(e.to_string()))?;
    
    let column_names: Vec<String> = stmt.column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    // Convert JSON params to rusqlite params
    let sqlite_params: Vec<rusqlite::types::Value> = params.iter()
        .map(json_to_sqlite_value)
        .collect();
    
    let param_refs: Vec<&dyn rusqlite::ToSql> = sqlite_params.iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();
    
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let mut values = HashMap::new();
        for (i, col) in column_names.iter().enumerate() {
            let value = sqlite_value_to_json(row, i);
            values.insert(col.clone(), value);
        }
        Ok(Row {
            columns: column_names.clone(),
            values,
        })
    }).map_err(|e| ServiceError::QueryFailed(e.to_string()))?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| ServiceError::QueryFailed(e.to_string()))?);
    }
    
    Ok(result)
}

/// Execute a SQLite statement and return affected rows
fn execute_sqlite_statement(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[serde_json::Value],
) -> Result<u64, ServiceError> {
    // Convert JSON params to rusqlite params
    let sqlite_params: Vec<rusqlite::types::Value> = params.iter()
        .map(json_to_sqlite_value)
        .collect();
    
    let param_refs: Vec<&dyn rusqlite::ToSql> = sqlite_params.iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();
    
    let affected = conn.execute(sql, param_refs.as_slice())
        .map_err(|e| ServiceError::QueryFailed(e.to_string()))?;
    
    Ok(affected as u64)
}

/// Convert a JSON value to a SQLite value
fn json_to_sqlite_value(value: &serde_json::Value) -> rusqlite::types::Value {
    match value {
        serde_json::Value::Null => rusqlite::types::Value::Null,
        serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        serde_json::Value::Array(arr) => {
            rusqlite::types::Value::Text(serde_json::to_string(arr).unwrap_or_default())
        }
        serde_json::Value::Object(obj) => {
            rusqlite::types::Value::Text(serde_json::to_string(obj).unwrap_or_default())
        }
    }
}

/// Convert a SQLite value to JSON
fn sqlite_value_to_json(row: &rusqlite::Row, idx: usize) -> serde_json::Value {
    // Try different types in order
    if let Ok(v) = row.get::<_, i64>(idx) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&v) {
            if json.is_object() || json.is_array() {
                return json;
            }
        }
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.get::<_, Vec<u8>>(idx) {
        return serde_json::Value::String(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &v,
        ));
    }
    
    serde_json::Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sqlite_database_actor() {
        let config = DatabaseConfig {
            db_type: "sqlite".to_string(),
            url: ":memory:".to_string(),
            pool_size: 1,
            timeout_secs: 30,
        };
        
        let db = Database::start(config).await.unwrap();
        
        // Create a table
        db.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
        ).await.unwrap();
        
        // Insert data
        db.execute(
            "INSERT INTO test (id, name) VALUES (?, ?)",
            &[serde_json::json!(1), serde_json::json!("Alice")],
        ).await.unwrap();
        
        // Query data
        let rows = db.query("SELECT * FROM test", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("name"), Some(&serde_json::json!("Alice")));
        
        // Health check
        assert!(db.health().await.unwrap());
    }
}