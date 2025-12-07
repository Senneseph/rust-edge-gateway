//! SQLite Service Connector

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;

use super::ServiceConnector;
use crate::api::ServiceType;

/// SQLite connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Path to the SQLite database file
    pub path: String,
    /// Whether to create the database if it doesn't exist
    #[serde(default = "default_true")]
    pub create_if_missing: bool,
}

fn default_true() -> bool { true }

/// SQLite service connector
pub struct SqliteConnector {
    config: SqliteConfig,
    conn: Mutex<Connection>,
}

impl SqliteConnector {
    pub fn new(config: SqliteConfig) -> Result<Self> {
        let conn = if config.create_if_missing {
            Connection::open(&config.path)
        } else {
            Connection::open_with_flags(
                &config.path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
            )
        }.context("Failed to open SQLite database")?;
        
        Ok(Self {
            config,
            conn: Mutex::new(conn),
        })
    }
    
    /// Execute a query and return results as JSON
    pub fn query(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        
        let column_count = stmt.column_count();
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        
        let rows = stmt.query_map(params, |row| {
            let mut obj = serde_json::Map::new();
            for i in 0..column_count {
                let value: Value = match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(i) => json!(i),
                    rusqlite::types::ValueRef::Real(f) => json!(f),
                    rusqlite::types::ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
                    rusqlite::types::ValueRef::Blob(b) => json!(format!("0x{}", b.iter().map(|byte| format!("{:02x}", byte)).collect::<String>())),
                };
                obj.insert(column_names[i].clone(), value);
            }
            Ok(Value::Object(obj))
        })?;
        
        let results: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(results)
    }
    
    /// Execute a statement (INSERT, UPDATE, DELETE)
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(sql, params)?;
        Ok(affected)
    }
}

impl ServiceConnector for SqliteConnector {
    fn service_type(&self) -> ServiceType {
        ServiceType::Sqlite
    }
    
    fn test_connection(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("SELECT 1")?;
        Ok(())
    }
    
    fn connection_info(&self) -> Value {
        json!({
            "type": "sqlite",
            "path": self.config.path,
        })
    }
}

