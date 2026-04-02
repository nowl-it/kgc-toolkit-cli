//! Proxy types and data structures

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub enable_https: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8888,
            enable_https: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub local_ip: String,
    pub cert_download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub url: String,
    pub request_headers: String,
    pub request_body: Option<Vec<u8>>,
    pub response_status: u16,
    pub response_headers: String,
    pub response_body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterOptions {
    pub url_pattern: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<u16>,
}
