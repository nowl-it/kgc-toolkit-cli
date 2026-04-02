//! Path utilities

use std::path::PathBuf;

/// Get the application data directory
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kgc-toolkit")
}

/// Get the tools directory (relative to project/binary location)
pub fn tools_dir() -> PathBuf {
    // First try: relative to current directory (for development)
    let relative_path = PathBuf::from("tools");
    if relative_path.exists() {
        return relative_path;
    }
    
    // Second try: relative to executable location
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let tools_path = exe_dir.join("tools");
            if tools_path.exists() {
                return tools_path;
            }
            
            // Try parent directories (for target/release/kgc)
            if let Some(parent_dir) = exe_dir.parent() {
                if let Some(root_dir) = parent_dir.parent() {
                    let tools_path = root_dir.join("tools");
                    if tools_path.exists() {
                        return tools_path;
                    }
                }
            }
        }
    }
    
    // Fallback
    PathBuf::from("tools")
}

/// Get the cache directory
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kgc-toolkit")
}

/// Get the config directory
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kgc-toolkit")
}

/// Get the certificates directory
pub fn certs_dir() -> PathBuf {
    data_dir().join("certificates")
}

/// Get the logs directory
pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}
