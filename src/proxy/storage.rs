//! Traffic storage using SQLite
//! Ported from src-tauri/src/proxy/storage.rs

use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

use super::types::CapturedRequest;
use crate::core::crypto;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Lock error")]
    LockError,
    #[error("Storage not initialized")]
    NotInitialized,
}

static STORAGE: OnceLock<TrafficStorage> = OnceLock::new();

fn default_data_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kgc-toolkit")
}

/// TrafficStorage wrapper for SQLite database
pub struct TrafficStorage {
    conn: Mutex<Connection>,
}

impl TrafficStorage {
    pub fn new(data_dir: &Path) -> Result<Self, StorageError> {
        let db_path = data_dir.join("traffic.db");
        let conn = Connection::open(&db_path)?;

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS traffic (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                request_headers TEXT,
                request_body BLOB,
                response_status INTEGER,
                response_headers TEXT,
                response_body BLOB
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_url ON traffic(url)",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Store a captured request
    pub fn store_request(&self, request: &CapturedRequest) -> Result<i64, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::LockError)?;

        conn.execute(
            "INSERT INTO traffic (timestamp, method, url, request_headers, request_body, response_status, response_headers, response_body)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.timestamp,
                request.method,
                request.url,
                request.request_headers,
                request.request_body,
                request.response_status,
                request.response_headers,
                request.response_body,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get captured requests with optional filter
    pub fn get_requests_impl(&self, filter: Option<&str>, limit: usize) -> Result<Vec<CapturedRequest>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::LockError)?;

        let query = if let Some(pattern) = filter {
            format!(
                "SELECT id, timestamp, method, url, request_headers, request_body, response_status, response_headers, response_body
                 FROM traffic WHERE url LIKE '%{}%' ORDER BY id DESC LIMIT {}",
                pattern.replace('\'', "''"),
                limit
            )
        } else {
            format!(
                "SELECT id, timestamp, method, url, request_headers, request_body, response_status, response_headers, response_body
                 FROM traffic ORDER BY id DESC LIMIT {}",
                limit
            )
        };

        let mut stmt = conn.prepare(&query)?;

        let requests = stmt.query_map([], |row| {
            Ok(CapturedRequest {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                request_headers: row.get(4)?,
                request_body: row.get(5)?,
                response_status: row.get(6)?,
                response_headers: row.get(7)?,
                response_body: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(requests)
    }

    /// Get a specific request by ID
    pub fn get_request_impl(&self, id: i64) -> Result<CapturedRequest, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::LockError)?;

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, method, url, request_headers, request_body, response_status, response_headers, response_body
             FROM traffic WHERE id = ?1"
        )?;

        let request = stmt.query_row([id], |row| {
            Ok(CapturedRequest {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                url: row.get(3)?,
                request_headers: row.get(4)?,
                request_body: row.get(5)?,
                response_status: row.get(6)?,
                response_headers: row.get(7)?,
                response_body: row.get(8)?,
            })
        })?;

        Ok(request)
    }

    /// Decrypt a response body using KGC AES key
    pub fn decrypt_response_impl(&self, request_id: i64) -> Result<String, StorageError> {
        let request = self.get_request_impl(request_id)?;

        let body = request.response_body.ok_or_else(|| {
            StorageError::NotFound("No response body".into())
        })?;

        // Convert bytes to hex string (as stored by proxy)
        let hex_body: String = body.iter().map(|b| format!("{:02x}", b)).collect();

        // Try hex decryption first
        match crypto::decrypt_hex(&hex_body) {
            Ok(decrypted) => Ok(decrypted),
            Err(_) => {
                // Try base64 decryption
                let b64_body = String::from_utf8_lossy(&body);
                match crypto::decrypt_base64(&b64_body) {
                    Ok(decrypted) => Ok(decrypted),
                    Err(_) => {
                        // Return as plain text
                        Ok(String::from_utf8_lossy(&body).to_string())
                    }
                }
            }
        }
    }

    /// Clear all stored traffic
    pub fn clear_all_impl(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::LockError)?;
        conn.execute("DELETE FROM traffic", [])?;
        Ok(())
    }
}

// Global functions that use the static storage

/// Initialize storage with database path
pub fn initialize(data_dir: &Path) -> Result<(), StorageError> {
    let storage = TrafficStorage::new(data_dir)?;
    let _ = STORAGE.set(storage);
    Ok(())
}

fn get_storage() -> Result<&'static TrafficStorage, StorageError> {
    STORAGE.get().ok_or(StorageError::NotInitialized)
}

fn ensure_initialized() -> Result<(), StorageError> {
    if STORAGE.get().is_some() {
        return Ok(());
    }

    let data_dir = default_data_dir();
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| StorageError::NotFound(format!("Failed to create data dir {}: {}", data_dir.display(), e)))?;
    initialize(&data_dir)?;
    Ok(())
}

/// Get captured requests with optional filter
pub fn get_requests(filter: Option<&str>, limit: usize) -> Result<Vec<CapturedRequest>, StorageError> {
    let _ = ensure_initialized();
    get_storage()?.get_requests_impl(filter, limit)
}

/// Store a captured request
pub fn store_request(request: &CapturedRequest) -> Result<i64, StorageError> {
    let _ = ensure_initialized();
    get_storage()?.store_request(request)
}

/// Get a specific request by ID
pub fn get_request(id: i64) -> Result<CapturedRequest, StorageError> {
    let _ = ensure_initialized();
    get_storage()?.get_request_impl(id)
}

/// Decrypt a response body using KGC AES key
pub fn decrypt_response(request_id: i64) -> Result<String, StorageError> {
    let _ = ensure_initialized();
    get_storage()?.decrypt_response_impl(request_id)
}

/// Clear all stored traffic
pub fn clear_all() -> Result<(), StorageError> {
    let _ = ensure_initialized();
    get_storage()?.clear_all_impl()
}

/// Get all captured request URLs.
pub fn get_all_request_urls() -> Result<Vec<String>, StorageError> {
    let requests = get_requests(None, usize::MAX)?;
    Ok(requests.into_iter().map(|r| r.url).collect())
}

/// Find latest captured request that contains path fragment.
pub fn find_request_by_path(path_fragment: &str) -> Result<Option<CapturedRequest>, StorageError> {
    let requests = get_requests(Some(path_fragment), 1)?;
    Ok(requests.into_iter().next())
}
