//! Database layer using SQLite

use anyhow::{Context, Result};
use rusqlite::{Connection, params, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::api::Endpoint;

/// Match a path pattern (e.g., "/pet/{petId}") against an actual path (e.g., "/pet/42")
/// Returns extracted path parameters if matched
fn match_path_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    if pattern_parts.len() != path_parts.len() {
        return None;
    }

    let mut params = HashMap::new();

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            // This is a path parameter
            let param_name = &pattern_part[1..pattern_part.len()-1];
            params.insert(param_name.to_string(), path_part.to_string());
        } else if pattern_part != path_part {
            // Static parts must match exactly
            return None;
        }
    }

    Some(params)
}

/// SQLite database wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create a new database connection
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("edge_hive.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;
        
        Ok(Self { conn: Mutex::new(conn) })
    }
    
    /// Run database migrations
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS endpoints (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                domain TEXT NOT NULL,
                path TEXT NOT NULL,
                method TEXT NOT NULL DEFAULT 'GET',
                code TEXT,
                compiled INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_endpoints_domain_path 
                ON endpoints(domain, path, method);
            
            CREATE TABLE IF NOT EXISTS endpoint_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                endpoint_id TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                total_duration_ms INTEGER NOT NULL DEFAULT 0,
                max_memory_bytes INTEGER NOT NULL DEFAULT 0,
                recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (endpoint_id) REFERENCES endpoints(id)
            );
            
            CREATE TABLE IF NOT EXISTS request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                endpoint_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                status INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                memory_bytes INTEGER,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (endpoint_id) REFERENCES endpoints(id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_request_logs_endpoint_created 
                ON request_logs(endpoint_id, created_at);
        "#)?;
        
        Ok(())
    }
    
    /// List all endpoints
    pub fn list_endpoints(&self) -> Result<Vec<Endpoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, domain, path, method, compiled, enabled, created_at, updated_at 
             FROM endpoints ORDER BY created_at DESC"
        )?;
        
        let endpoints = stmt.query_map([], |row| {
            Ok(Endpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                method: row.get(4)?,
                code: None,
                compiled: row.get(5)?,
                enabled: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        
        Ok(endpoints)
    }
    
    /// Get an endpoint by ID
    pub fn get_endpoint(&self, id: &str) -> Result<Option<Endpoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, domain, path, method, code, compiled, enabled, created_at, updated_at 
             FROM endpoints WHERE id = ?"
        )?;
        
        let endpoint = stmt.query_row([id], |row| {
            Ok(Endpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                method: row.get(4)?,
                code: row.get(5)?,
                compiled: row.get(6)?,
                enabled: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        }).optional()?;
        
        Ok(endpoint)
    }
    
    /// Create a new endpoint
    pub fn create_endpoint(&self, endpoint: &Endpoint) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO endpoints (id, name, domain, path, method, code, compiled, enabled)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                endpoint.id,
                endpoint.name,
                endpoint.domain,
                endpoint.path,
                endpoint.method,
                endpoint.code,
                endpoint.compiled,
                endpoint.enabled,
            ],
        )?;
        Ok(())
    }
    
    /// Update an endpoint
    pub fn update_endpoint(&self, endpoint: &Endpoint) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE endpoints SET name = ?, domain = ?, path = ?, method = ?,
             compiled = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
            params![
                endpoint.name,
                endpoint.domain,
                endpoint.path,
                endpoint.method,
                endpoint.compiled,
                endpoint.enabled,
                endpoint.id,
            ],
        )?;
        Ok(())
    }

    /// Update endpoint code
    pub fn update_endpoint_code(&self, id: &str, code: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE endpoints SET code = ?, compiled = 0, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
            params![code, id],
        )?;
        Ok(())
    }

    /// Mark endpoint as compiled
    pub fn mark_compiled(&self, id: &str, compiled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE endpoints SET compiled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![compiled, id],
        )?;
        Ok(())
    }

    /// Delete an endpoint
    pub fn delete_endpoint(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM endpoints WHERE id = ?", [id])?;
        Ok(())
    }

    /// Find endpoint by domain, path, and method
    /// Find endpoint by domain, path pattern, and method
    /// Returns (endpoint, extracted_path_params)
    pub fn find_endpoint(&self, domain: &str, path: &str, method: &str) -> Result<Option<(Endpoint, std::collections::HashMap<String, String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, domain, path, method, code, compiled, enabled, created_at, updated_at
             FROM endpoints WHERE domain = ? AND method = ? AND enabled = 1"
        )?;

        let endpoints: Vec<Endpoint> = stmt.query_map(params![domain, method], |row| {
            Ok(Endpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                method: row.get(4)?,
                code: row.get(5)?,
                compiled: row.get(6)?,
                enabled: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        // Try to match each endpoint's path pattern against the request path
        for endpoint in endpoints {
            if let Some(params) = match_path_pattern(&endpoint.path, path) {
                return Ok(Some((endpoint, params)));
            }
        }

        Ok(None)
    }

    /// Get endpoint count
    pub fn endpoint_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM endpoints", [], |row| row.get(0))?;
        Ok(count)
    }
}

