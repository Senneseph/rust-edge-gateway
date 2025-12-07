//! Database layer using SQLite

use anyhow::{Context, Result};
use rusqlite::{Connection, params, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::api::{Collection, Domain, Endpoint, Service, ServiceType};

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
        let db_path = data_dir.join("rust_edge_gateway.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;
        
        Ok(Self { conn: Mutex::new(conn) })
    }
    
    /// Run database migrations
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(r#"
            -- Domains: top-level organization (e.g., "api.example.com")
            CREATE TABLE IF NOT EXISTS domains (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL UNIQUE,
                description TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            -- Collections: group endpoints within a domain (e.g., "Pet Store", "Users")
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                domain_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                base_path TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (domain_id) REFERENCES domains(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_collections_domain
                ON collections(domain_id);

            -- Services: backend service connections (databases, caches, storage)
            CREATE TABLE IF NOT EXISTS services (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                service_type TEXT NOT NULL,
                config TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_services_type
                ON services(service_type);

            -- Endpoints: API endpoints within collections
            CREATE TABLE IF NOT EXISTS endpoints (
                id TEXT PRIMARY KEY,
                collection_id TEXT,
                name TEXT NOT NULL,
                domain TEXT NOT NULL,
                path TEXT NOT NULL,
                method TEXT NOT NULL DEFAULT 'GET',
                description TEXT,
                code TEXT,
                compiled INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_endpoints_domain_path
                ON endpoints(domain, path, method);

            CREATE INDEX IF NOT EXISTS idx_endpoints_collection
                ON endpoints(collection_id);

            -- Endpoint-Service bindings: which services an endpoint can use
            CREATE TABLE IF NOT EXISTS endpoint_services (
                endpoint_id TEXT NOT NULL,
                service_id TEXT NOT NULL,
                alias TEXT NOT NULL,
                PRIMARY KEY (endpoint_id, service_id),
                FOREIGN KEY (endpoint_id) REFERENCES endpoints(id) ON DELETE CASCADE,
                FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE
            );

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
            "SELECT id, collection_id, name, domain, path, method, description, compiled, enabled, created_at, updated_at
             FROM endpoints ORDER BY created_at DESC"
        )?;

        let endpoints = stmt.query_map([], |row| {
            Ok(Endpoint {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                domain: row.get(3)?,
                path: row.get(4)?,
                method: row.get(5)?,
                description: row.get(6)?,
                code: None,
                compiled: row.get(7)?,
                enabled: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(endpoints)
    }

    /// Get an endpoint by ID
    pub fn get_endpoint(&self, id: &str) -> Result<Option<Endpoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, name, domain, path, method, description, code, compiled, enabled, created_at, updated_at
             FROM endpoints WHERE id = ?"
        )?;

        let endpoint = stmt.query_row([id], |row| {
            Ok(Endpoint {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                domain: row.get(3)?,
                path: row.get(4)?,
                method: row.get(5)?,
                description: row.get(6)?,
                code: row.get(7)?,
                compiled: row.get(8)?,
                enabled: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        }).optional()?;

        Ok(endpoint)
    }

    /// Create a new endpoint
    pub fn create_endpoint(&self, endpoint: &Endpoint) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO endpoints (id, collection_id, name, domain, path, method, description, code, compiled, enabled)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                endpoint.id,
                endpoint.collection_id,
                endpoint.name,
                endpoint.domain,
                endpoint.path,
                endpoint.method,
                endpoint.description,
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
            "UPDATE endpoints SET collection_id = ?, name = ?, domain = ?, path = ?, method = ?,
             description = ?, compiled = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
            params![
                endpoint.collection_id,
                endpoint.name,
                endpoint.domain,
                endpoint.path,
                endpoint.method,
                endpoint.description,
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

    /// Find endpoint by domain, path pattern, and method
    /// Returns (endpoint, extracted_path_params)
    pub fn find_endpoint(&self, domain: &str, path: &str, method: &str) -> Result<Option<(Endpoint, std::collections::HashMap<String, String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, collection_id, name, domain, path, method, description, code, compiled, enabled, created_at, updated_at
             FROM endpoints WHERE domain = ? AND method = ? AND enabled = 1"
        )?;

        let endpoints: Vec<Endpoint> = stmt.query_map(params![domain, method], |row| {
            Ok(Endpoint {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                domain: row.get(3)?,
                path: row.get(4)?,
                method: row.get(5)?,
                description: row.get(6)?,
                code: row.get(7)?,
                compiled: row.get(8)?,
                enabled: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
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

    // ========================================================================
    // Domain CRUD
    // ========================================================================

    /// List all domains
    pub fn list_domains(&self) -> Result<Vec<Domain>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, host, description, enabled, created_at, updated_at
             FROM domains ORDER BY name"
        )?;

        let domains = stmt.query_map([], |row| {
            Ok(Domain {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(domains)
    }

    /// Get a domain by ID
    pub fn get_domain(&self, id: &str) -> Result<Option<Domain>> {
        let conn = self.conn.lock().unwrap();
        let domain = conn.query_row(
            "SELECT id, name, host, description, enabled, created_at, updated_at
             FROM domains WHERE id = ?",
            [id],
            |row| Ok(Domain {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                description: row.get(3)?,
                enabled: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        ).optional()?;
        Ok(domain)
    }

    /// Create a new domain
    pub fn create_domain(&self, domain: &Domain) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO domains (id, name, host, description, enabled) VALUES (?, ?, ?, ?, ?)",
            params![domain.id, domain.name, domain.host, domain.description, domain.enabled],
        )?;
        Ok(())
    }

    /// Update a domain
    pub fn update_domain(&self, domain: &Domain) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE domains SET name = ?, host = ?, description = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![domain.name, domain.host, domain.description, domain.enabled, domain.id],
        )?;
        Ok(())
    }

    /// Delete a domain
    pub fn delete_domain(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM domains WHERE id = ?", [id])?;
        Ok(())
    }

    // ========================================================================
    // Collection CRUD
    // ========================================================================

    /// List collections, optionally filtered by domain
    pub fn list_collections(&self, domain_id: Option<&str>) -> Result<Vec<Collection>> {
        let conn = self.conn.lock().unwrap();

        if let Some(did) = domain_id {
            let mut stmt = conn.prepare(
                "SELECT id, domain_id, name, description, base_path, enabled, created_at, updated_at
                 FROM collections WHERE domain_id = ? ORDER BY name"
            )?;
            let collections = stmt.query_map([did], |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    domain_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    base_path: row.get(4)?,
                    enabled: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            Ok(collections)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, domain_id, name, description, base_path, enabled, created_at, updated_at
                 FROM collections ORDER BY name"
            )?;
            let collections = stmt.query_map([], |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    domain_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    base_path: row.get(4)?,
                    enabled: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            Ok(collections)
        }
    }

    /// Get a collection by ID
    pub fn get_collection(&self, id: &str) -> Result<Option<Collection>> {
        let conn = self.conn.lock().unwrap();
        let collection = conn.query_row(
            "SELECT id, domain_id, name, description, base_path, enabled, created_at, updated_at
             FROM collections WHERE id = ?",
            [id],
            |row| Ok(Collection {
                id: row.get(0)?,
                domain_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                base_path: row.get(4)?,
                enabled: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        ).optional()?;
        Ok(collection)
    }

    /// Create a new collection
    pub fn create_collection(&self, collection: &Collection) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO collections (id, domain_id, name, description, base_path, enabled) VALUES (?, ?, ?, ?, ?, ?)",
            params![collection.id, collection.domain_id, collection.name, collection.description, collection.base_path, collection.enabled],
        )?;
        Ok(())
    }

    /// Update a collection
    pub fn update_collection(&self, collection: &Collection) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collections SET name = ?, description = ?, base_path = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![collection.name, collection.description, collection.base_path, collection.enabled, collection.id],
        )?;
        Ok(())
    }

    /// Delete a collection
    pub fn delete_collection(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM collections WHERE id = ?", [id])?;
        Ok(())
    }

    // ========================================================================
    // Service CRUD
    // ========================================================================

    /// List all services
    pub fn list_services(&self) -> Result<Vec<Service>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, service_type, config, enabled, created_at, updated_at
             FROM services ORDER BY name"
        )?;

        let services = stmt.query_map([], |row| {
            let type_str: String = row.get(2)?;
            let config_str: String = row.get(3)?;
            Ok(Service {
                id: row.get(0)?,
                name: row.get(1)?,
                service_type: type_str.parse().unwrap_or(ServiceType::Sqlite),
                config: serde_json::from_str(&config_str).unwrap_or(serde_json::json!({})),
                enabled: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(services)
    }

    /// Get a service by ID
    pub fn get_service(&self, id: &str) -> Result<Option<Service>> {
        let conn = self.conn.lock().unwrap();
        let service = conn.query_row(
            "SELECT id, name, service_type, config, enabled, created_at, updated_at
             FROM services WHERE id = ?",
            [id],
            |row| {
                let type_str: String = row.get(2)?;
                let config_str: String = row.get(3)?;
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    service_type: type_str.parse().unwrap_or(ServiceType::Sqlite),
                    config: serde_json::from_str(&config_str).unwrap_or(serde_json::json!({})),
                    enabled: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            }
        ).optional()?;
        Ok(service)
    }

    /// Create a new service
    pub fn create_service(&self, service: &Service) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let config_str = serde_json::to_string(&service.config)?;
        conn.execute(
            "INSERT INTO services (id, name, service_type, config, enabled) VALUES (?, ?, ?, ?, ?)",
            params![service.id, service.name, service.service_type.to_string(), config_str, service.enabled],
        )?;
        Ok(())
    }

    /// Update a service
    pub fn update_service(&self, service: &Service) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let config_str = serde_json::to_string(&service.config)?;
        conn.execute(
            "UPDATE services SET name = ?, config = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![service.name, config_str, service.enabled, service.id],
        )?;
        Ok(())
    }

    /// Delete a service
    pub fn delete_service(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM services WHERE id = ?", [id])?;
        Ok(())
    }
}

