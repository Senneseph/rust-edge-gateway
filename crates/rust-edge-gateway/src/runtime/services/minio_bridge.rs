//! Bridge between gateway MinioHandle and SDK MinioClient trait
//!
//! This module provides the implementation of the SDK's MinioClient trait
//! that wraps the gateway's MinioHandle, allowing handlers to use the
//! SDK trait interface while the gateway manages the actual connections.

use bytes::Bytes;

use rust_edge_gateway_sdk::services::{MinioClient, ObjectInfo, ServiceError, ServiceFuture};
use super::minio_actor::MinioHandle;

/// Wrapper that implements SDK's MinioClient trait using gateway's MinioHandle
pub struct MinioClientBridge {
    handle: MinioHandle,
}

impl MinioClientBridge {
    pub fn new(handle: MinioHandle) -> Self {
        Self { handle }
    }
}

impl MinioClient for MinioClientBridge {
    fn get_object<'a>(&'a self, bucket: &'a str, key: &'a str) -> ServiceFuture<'a, Vec<u8>> {
        let handle = self.handle.clone();
        let bucket = bucket.to_string();
        let key = key.to_string();
        
        Box::pin(async move {
            handle.get_object(&bucket, &key).await
                .map(|bytes| bytes.to_vec())
                .map_err(|e| ServiceError::OperationFailed(e.to_string()))
        })
    }
    
    fn put_object<'a>(&'a self, bucket: &'a str, key: &'a str, data: Vec<u8>, content_type: Option<&'a str>) -> ServiceFuture<'a, ()> {
        let handle = self.handle.clone();
        let bucket = bucket.to_string();
        let key = key.to_string();
        let content_type = content_type.map(String::from);
        
        Box::pin(async move {
            handle.put_object(&bucket, &key, Bytes::from(data), content_type.as_deref()).await
                .map_err(|e| ServiceError::OperationFailed(e.to_string()))
        })
    }
    
    fn delete_object<'a>(&'a self, bucket: &'a str, key: &'a str) -> ServiceFuture<'a, ()> {
        let handle = self.handle.clone();
        let bucket = bucket.to_string();
        let key = key.to_string();
        
        Box::pin(async move {
            handle.delete_object(&bucket, &key).await
                .map_err(|e| ServiceError::OperationFailed(e.to_string()))
        })
    }
    
    fn list_objects<'a>(&'a self, bucket: &'a str, prefix: &'a str) -> ServiceFuture<'a, Vec<ObjectInfo>> {
        let handle = self.handle.clone();
        let bucket = bucket.to_string();
        let prefix = prefix.to_string();
        
        Box::pin(async move {
            handle.list_objects(&bucket, &prefix).await
                .map(|objects| {
                    objects.into_iter().map(|o| ObjectInfo {
                        key: o.key,
                        size: o.size,
                        last_modified: o.last_modified,
                        etag: o.etag,
                        content_type: o.content_type,
                    }).collect()
                })
                .map_err(|e| ServiceError::OperationFailed(e.to_string()))
        })
    }
    
    fn default_bucket(&self) -> &str {
        &self.handle.default_bucket
    }
}

