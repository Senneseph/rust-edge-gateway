//! Database layer for admin authentication and API keys

use anyhow::{Context, Result};
use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;
use chrono::{Utc, DateTime};

/// Admin user credentials with password change requirement
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub requires_password_change: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API key for service authentication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub label: String,
    pub created_by: String, // admin user ID
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub permissions: Vec<String>, // e.g., ["read", "write", "admin"]
}

/// SQLite database wrapper for admin authentication and API keys
pub struct AdminDatabase {
    conn: Mutex<Connection>,
}

impl AdminDatabase {
    /// Create a new admin database connection
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("admin.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open admin database at {:?}", db_path))?;
        
        Ok(Self { conn: Mutex::new(conn) })
    }
    
    /// Run database migrations for admin tables
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(r#"
            -- Admin users table
            CREATE TABLE IF NOT EXISTS admin_users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                requires_password_change INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            -- API keys table
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                label TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                expires_at TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                permissions TEXT, -- JSON array of strings
                FOREIGN KEY (created_by) REFERENCES admin_users(id)
            );

            CREATE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key);
            CREATE INDEX IF NOT EXISTS idx_api_keys_enabled ON api_keys(enabled);
            CREATE INDEX IF NOT EXISTS idx_api_keys_created_by ON api_keys(created_by);
        "#)?;

        Ok(())
    }
    
    /// Create initial admin user from environment variable
    pub fn create_initial_admin(&self, username: &str, password: &str) -> Result<()> {
        use bcrypt::{hash, DEFAULT_COST};
        
        let password_hash = hash(password, DEFAULT_COST)?;
        
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO admin_users (id, username, password_hash, requires_password_change) VALUES (?, ?, ?, ?)",
            params![
                uuid::Uuid::new_v4().to_string(),
                username,
                password_hash,
                true,
            ],
        )?;
        
        Ok(())
    }
    
    /// Get admin user by username
    pub fn get_admin_by_username(&self, username: &str) -> Result<Option<AdminUser>> {
        let conn = self.conn.lock().unwrap();
        let user = conn.query_row(
            "SELECT id, username, password_hash, requires_password_change, created_at, updated_at FROM admin_users WHERE username = ?",
            [username],
            |row| {
                let created_at_str: String = row.get(4)?;
                let updated_at_str: String = row.get(5)?;

                Ok(AdminUser {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    requires_password_change: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            }
        ).optional()?;
        Ok(user)
    }
    
    /// Update admin user password and reset password change requirement
    pub fn update_admin_password(&self, username: &str, new_password: &str) -> Result<()> {
        use bcrypt::{hash, DEFAULT_COST};
        
        let password_hash = hash(new_password, DEFAULT_COST)?;
        
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE admin_users SET password_hash = ?, requires_password_change = 0, updated_at = CURRENT_TIMESTAMP WHERE username = ?",
            params![password_hash, username],
        )?;
        
        Ok(())
    }
    
    /// Create a new API key
    pub fn create_api_key(&self, label: &str, created_by: &str, permissions: Vec<String>) -> Result<ApiKey> {
        let key = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4().to_string();
        
        let permissions_json = serde_json::to_string(&permissions).unwrap_or_default();
        
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_keys (id, key, label, created_by, permissions) VALUES (?, ?, ?, ?, ?)",
            params![id, key, label, created_by, permissions_json],
        )?;
        
        Ok(ApiKey {
            id,
            key,
            label: label.to_string(),
            created_by: created_by.to_string(),
            created_at: Utc::now(),
            expires_at: None,
            enabled: true,
            permissions,
        })
    }
    
    /// Get API key by key value
    pub fn get_api_key_by_value(&self, key: &str) -> Result<Option<ApiKey>> {
        let conn = self.conn.lock().unwrap();
        let api_key = conn.query_row(
            "SELECT id, key, label, created_by, created_at, expires_at, enabled, permissions FROM api_keys WHERE key = ? AND enabled = 1",
            [key],
            |row| {
                let permissions_str: String = row.get(7)?;
                let permissions: Vec<String> = serde_json::from_str(&permissions_str).unwrap_or_default();
                let created_at_str: String = row.get(4)?;
                let expires_at_str: Option<String> = row.get(5).ok();

                Ok(ApiKey {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    label: row.get(2)?,
                    created_by: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: expires_at_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                    enabled: row.get(6)?,
                    permissions,
                })
            }
        ).optional()?;
        Ok(api_key)
    }
    
    /// List all API keys for an admin user
    pub fn list_api_keys(&self, created_by: &str) -> Result<Vec<ApiKey>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, key, label, created_by, created_at, expires_at, enabled, permissions FROM api_keys WHERE created_by = ? ORDER BY created_at DESC"
        )?;
        
        let api_keys = stmt.query_map([created_by], |row| {
            let permissions_str: String = row.get(7)?;
            let permissions: Vec<String> = serde_json::from_str(&permissions_str).unwrap_or_default();
            let created_at_str: String = row.get(4)?;
            let expires_at_str: Option<String> = row.get(5).ok();

            Ok(ApiKey {
                id: row.get(0)?,
                key: row.get(1)?,
                label: row.get(2)?,
                created_by: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: expires_at_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                }),
                enabled: row.get(6)?,
                permissions,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(api_keys)
    }
    
    /// Disable an API key
    pub fn disable_api_key(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET enabled = 0 WHERE key = ?",
            [key],
        )?;
        
        Ok(())
    }
    
    /// Update API key permissions
    pub fn update_api_key_permissions(&self, key: &str, permissions: Vec<String>) -> Result<()> {
        let permissions_json = serde_json::to_string(&permissions).unwrap_or_default();
        
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET permissions = ?, updated_at = CURRENT_TIMESTAMP WHERE key = ?",
            params![permissions_json, key],
        )?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_admin_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db = AdminDatabase::new(temp_dir.path()).unwrap();
        db.migrate().unwrap();
        
        // Create initial admin
        db.create_initial_admin("admin", "password123").unwrap();
        
        // Get admin user
        let user = db.get_admin_by_username("admin").unwrap().unwrap();
        assert_eq!(user.username, "admin");
        assert!(user.requires_password_change);
        
        // Update password
        db.update_admin_password("admin", "newpassword456").unwrap();
        
        // Verify password change requirement was reset
        let user = db.get_admin_by_username("admin").unwrap().unwrap();
        assert!(!user.requires_password_change);
        
        // Create API key
        let api_key = db.create_api_key("test-key", &user.id, vec!["read".to_string(), "write".to_string()]).unwrap();
        
        // Get API key by value
        let found_key = db.get_api_key_by_value(&api_key.key).unwrap().unwrap();
        assert_eq!(found_key.label, "test-key");
        assert_eq!(found_key.permissions, vec!["read".to_string(), "write".to_string()]);
    }
}
