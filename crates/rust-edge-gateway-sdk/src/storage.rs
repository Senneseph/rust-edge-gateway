//! Storage Backend Abstraction
//!
//! Provides a unified interface for storing and retrieving data across different
//! backend types: databases (SQLite, PostgreSQL, MySQL), object storage (MinIO),
//! and file transfer (FTP/SFTP).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::HandlerError;
use crate::services::DbPool;

/// Storage backend type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    /// SQL Database (SQLite, PostgreSQL, MySQL)
    Database,
    /// Object storage (MinIO/S3)
    ObjectStorage,
    /// File system via FTP/SFTP
    FileStorage,
}

/// A generic storage interface that works across different backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    /// The type of storage backend
    pub storage_type: StorageType,
    /// Pool/connection identifier
    pub pool_id: String,
    /// Base path for file-based storage
    pub base_path: Option<String>,
    /// Table name for database storage
    pub table_name: Option<String>,
}

impl Storage {
    /// Create a new database storage backend
    pub fn database(pool_id: &str, table_name: &str) -> Self {
        Self {
            storage_type: StorageType::Database,
            pool_id: pool_id.to_string(),
            base_path: None,
            table_name: Some(table_name.to_string()),
        }
    }

    /// Create a new object storage backend (MinIO/S3)
    pub fn object_storage(pool_id: &str, base_path: &str) -> Self {
        Self {
            storage_type: StorageType::ObjectStorage,
            pool_id: pool_id.to_string(),
            base_path: Some(base_path.to_string()),
            table_name: None,
        }
    }

    /// Create a new file storage backend (FTP/SFTP)
    pub fn file_storage(pool_id: &str, base_path: &str) -> Self {
        Self {
            storage_type: StorageType::FileStorage,
            pool_id: pool_id.to_string(),
            base_path: Some(base_path.to_string()),
            table_name: None,
        }
    }

    /// Get a record by ID
    pub fn get(&self, id: &str) -> Result<Option<JsonValue>, HandlerError> {
        match self.storage_type {
            StorageType::Database => self.db_get(id),
            StorageType::ObjectStorage => self.object_get(id),
            StorageType::FileStorage => self.file_get(id),
        }
    }

    /// List all records (optionally filtered)
    pub fn list(&self, filter: Option<&str>) -> Result<Vec<JsonValue>, HandlerError> {
        match self.storage_type {
            StorageType::Database => self.db_list(filter),
            StorageType::ObjectStorage => self.object_list(filter),
            StorageType::FileStorage => self.file_list(filter),
        }
    }

    /// Create a new record
    pub fn create(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        match self.storage_type {
            StorageType::Database => self.db_create(id, data),
            StorageType::ObjectStorage => self.object_create(id, data),
            StorageType::FileStorage => self.file_create(id, data),
        }
    }

    /// Update an existing record
    pub fn update(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        match self.storage_type {
            StorageType::Database => self.db_update(id, data),
            StorageType::ObjectStorage => self.object_update(id, data),
            StorageType::FileStorage => self.file_update(id, data),
        }
    }

    /// Delete a record
    pub fn delete(&self, id: &str) -> Result<bool, HandlerError> {
        match self.storage_type {
            StorageType::Database => self.db_delete(id),
            StorageType::ObjectStorage => self.object_delete(id),
            StorageType::FileStorage => self.file_delete(id),
        }
    }
}

// Database implementation
impl Storage {
    fn db_pool(&self) -> DbPool {
        DbPool { pool_id: self.pool_id.clone() }
    }

    fn table(&self) -> &str {
        self.table_name.as_deref().unwrap_or("data")
    }

    fn db_get(&self, id: &str) -> Result<Option<JsonValue>, HandlerError> {
        let sql = format!("SELECT * FROM {} WHERE id = ?", self.table());
        let result = self.db_pool().query(&sql, &[id])?;
        Ok(result.rows.into_iter().next().map(|row| serde_json::to_value(row).unwrap()))
    }

    fn db_list(&self, filter: Option<&str>) -> Result<Vec<JsonValue>, HandlerError> {
        let sql = match filter {
            Some(_) => format!("SELECT * FROM {} WHERE status = ?", self.table()),
            None => format!("SELECT * FROM {}", self.table()),
        };
        let params: Vec<&str> = filter.into_iter().collect();
        let result = self.db_pool().query(&sql, &params)?;
        Ok(result.rows.into_iter().map(|row| serde_json::to_value(row).unwrap()).collect())
    }

    fn db_create(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        let json_str = serde_json::to_string(data).map_err(|e| HandlerError::InternalError(e.to_string()))?;
        let sql = format!(
            "INSERT INTO {} (id, data, created_at) VALUES (?, ?, datetime('now'))",
            self.table()
        );
        self.db_pool().execute(&sql, &[id, &json_str])?;
        Ok(())
    }

    fn db_update(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        let json_str = serde_json::to_string(data).map_err(|e| HandlerError::InternalError(e.to_string()))?;
        let sql = format!("UPDATE {} SET data = ? WHERE id = ?", self.table());
        self.db_pool().execute(&sql, &[&json_str, id])?;
        Ok(())
    }

    fn db_delete(&self, id: &str) -> Result<bool, HandlerError> {
        let sql = format!("DELETE FROM {} WHERE id = ?", self.table());
        let affected = self.db_pool().execute(&sql, &[id])?;
        Ok(affected > 0)
    }
}

// Object storage (MinIO/S3) implementation
impl Storage {
    fn object_path(&self, id: &str) -> String {
        match &self.base_path {
            Some(base) => format!("{}/{}.json", base, id),
            None => format!("{}.json", id),
        }
    }

    fn object_get(&self, id: &str) -> Result<Option<JsonValue>, HandlerError> {
        let request = serde_json::json!({
            "service": "minio",
            "pool_id": self.pool_id,
            "action": "get",
            "path": self.object_path(id),
        });
        match crate::ipc::call_service::<Option<String>>(request) {
            Ok(Some(content)) => {
                let value: JsonValue = serde_json::from_str(&content)
                    .map_err(|e| HandlerError::InternalError(e.to_string()))?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(HandlerError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn object_list(&self, filter: Option<&str>) -> Result<Vec<JsonValue>, HandlerError> {
        let request = serde_json::json!({
            "service": "minio",
            "pool_id": self.pool_id,
            "action": "list",
            "path": self.base_path.as_deref().unwrap_or(""),
            "filter": filter,
        });
        crate::ipc::call_service(request)
    }

    fn object_create(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| HandlerError::InternalError(e.to_string()))?;
        let request = serde_json::json!({
            "service": "minio",
            "pool_id": self.pool_id,
            "action": "put",
            "path": self.object_path(id),
            "content": content,
            "content_type": "application/json",
        });
        crate::ipc::call_service::<()>(request)
    }

    fn object_update(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        // For object storage, update is same as create (overwrite)
        self.object_create(id, data)
    }

    fn object_delete(&self, id: &str) -> Result<bool, HandlerError> {
        let request = serde_json::json!({
            "service": "minio",
            "pool_id": self.pool_id,
            "action": "delete",
            "path": self.object_path(id),
        });
        crate::ipc::call_service::<bool>(request)
    }
}

// File storage (FTP/SFTP) implementation
impl Storage {
    fn file_path(&self, id: &str) -> String {
        match &self.base_path {
            Some(base) => format!("{}/{}.json", base, id),
            None => format!("{}.json", id),
        }
    }

    fn file_get(&self, id: &str) -> Result<Option<JsonValue>, HandlerError> {
        let request = serde_json::json!({
            "service": "ftp",
            "pool_id": self.pool_id,
            "action": "get",
            "path": self.file_path(id),
        });
        match crate::ipc::call_service::<Option<String>>(request) {
            Ok(Some(content)) => {
                let value: JsonValue = serde_json::from_str(&content)
                    .map_err(|e| HandlerError::InternalError(e.to_string()))?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(HandlerError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn file_list(&self, filter: Option<&str>) -> Result<Vec<JsonValue>, HandlerError> {
        let request = serde_json::json!({
            "service": "ftp",
            "pool_id": self.pool_id,
            "action": "list",
            "path": self.base_path.as_deref().unwrap_or(""),
            "filter": filter,
        });
        crate::ipc::call_service(request)
    }

    fn file_create(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| HandlerError::InternalError(e.to_string()))?;
        let request = serde_json::json!({
            "service": "ftp",
            "pool_id": self.pool_id,
            "action": "put",
            "path": self.file_path(id),
            "content": content,
        });
        crate::ipc::call_service::<()>(request)
    }

    fn file_update(&self, id: &str, data: &JsonValue) -> Result<(), HandlerError> {
        // For file storage, update is same as create (overwrite)
        self.file_create(id, data)
    }

    fn file_delete(&self, id: &str) -> Result<bool, HandlerError> {
        let request = serde_json::json!({
            "service": "ftp",
            "pool_id": self.pool_id,
            "action": "delete",
            "path": self.file_path(id),
        });
        crate::ipc::call_service::<bool>(request)
    }
}

