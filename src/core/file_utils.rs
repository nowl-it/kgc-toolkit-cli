//! File system utilities

use std::fs;
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("Path does not exist: {0}")]
    NotFound(String),
    #[error("Not a directory: {0}")]
    NotDirectory(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Check if a path exists
pub fn check_exists(path: &Path) -> bool {
    path.exists()
}

/// Check if path is a directory
pub fn check_is_directory(path: &Path) -> bool {
    path.is_dir()
}

/// Read directory contents (non-recursive)
pub fn read_directory(path: &Path) -> Result<Vec<String>, FileError> {
    if !path.exists() {
        return Err(FileError::NotFound(path.display().to_string()));
    }

    if !path.is_dir() {
        return Err(FileError::NotDirectory(path.display().to_string()));
    }

    let entries = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    Ok(entries)
}

/// Read file as bytes
pub fn read_file_as_bytes(path: &Path) -> Result<Vec<u8>, FileError> {
    if !path.exists() {
        return Err(FileError::NotFound(path.display().to_string()));
    }

    Ok(fs::read(path)?)
}

/// Read file as text
pub fn read_text_file(path: &Path) -> Result<String, FileError> {
    if !path.exists() {
        return Err(FileError::NotFound(path.display().to_string()));
    }

    Ok(fs::read_to_string(path)?)
}

/// Write text to file
pub fn write_text_file(path: &Path, content: &str) -> Result<(), FileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(fs::write(path, content)?)
}

/// Write bytes to file
pub fn write_bytes_file(path: &Path, content: &[u8]) -> Result<(), FileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(fs::write(path, content)?)
}

/// Recursively scan directory for files
pub fn scan_files(dir: &Path, filter: Option<&str>) -> Result<Vec<String>, FileError> {
    if !dir.exists() {
        return Err(FileError::NotFound(dir.display().to_string()));
    }

    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let relative = path
            .strip_prefix(dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if let Some(pattern) = filter {
            if !relative.contains(pattern) {
                continue;
            }
        }

        files.push(relative);
    }

    Ok(files)
}

/// Open path in system file manager
pub fn open_in_system(path: &Path) -> Result<(), FileError> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()?;
    }

    Ok(())
}
