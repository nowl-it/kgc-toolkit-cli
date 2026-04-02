//! King God Castle specific constants and helpers

/// Package ID for King God Castle
pub const PACKAGE_ID: &str = "com.awesomepiece.castle";

/// Game name
pub const GAME_NAME: &str = "King God Castle";

/// APKPure versions page for KGC.
pub const APKPURE_VERSIONS_URL: &str = "https://apkpure.com/king-god-castle/com.awesomepiece.castle/versions";

/// APKPure download URL prefix for KGC versions.
pub const APKPURE_DOWNLOAD_URL_PREFIX: &str = "https://apkpure.com/king-god-castle/com.awesomepiece.castle/download/";

/// AES-128 key for decrypting API responses
pub const AES_KEY: &str = "cnf1tl65djs2wp3g";

/// CDN URL regex pattern for KGC
pub const CDN_REGEX: &str = r"https://kgc-cdn-1\.awesomepiece\.com/patch/LIVE/[^/]+/[^/]+/[^/]+";

/// Unity version used by KGC
pub const UNITY_VERSION: &str = "2022.3.62f3";

/// Game engine type
pub const GAME_ENGINE: &str = "unity";

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Build API headers for KGC requests
pub fn build_api_headers() -> HashMap<String, String> {
    let mut headers = HashMap::new();

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time_hash = format!("{:x}", timestamp);

    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "*/*".to_string());
    headers.insert("Accept-Language".to_string(), "vi-VN,vi;q=0.9".to_string());
    headers.insert("Accept-Encoding".to_string(), "gzip, deflate, br".to_string());
    headers.insert("x-unity-version".to_string(), UNITY_VERSION.to_string());
    headers.insert("version".to_string(), "165.0.00".to_string());
    headers.insert("time".to_string(), time_hash);
    headers.insert("encryptedwithhex".to_string(), "true".to_string());

    headers
}

/// Default API body for KGC requests
pub fn default_api_body() -> serde_json::Value {
    serde_json::json!({
        "platform": "IOS",
        "version": "165.0.00"
    })
}

/// Check if URL matches KGC CDN pattern
pub fn is_cdn_url(url: &str) -> bool {
    regex::Regex::new(CDN_REGEX)
        .map(|re| re.is_match(url))
        .unwrap_or(false)
}
