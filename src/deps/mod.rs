//! Dependency management (tools)

mod tools;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use tools::*;

#[derive(Error, Debug)]
pub enum DepsError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
}

/// Check all tool dependencies
pub fn check_all() -> Result<HashMap<String, bool>, DepsError> {
    let mut status = HashMap::new();

    for tool in TOOLS {
        let path = get_tool_path(&tool.name);
        status.insert(tool.name.to_string(), path.exists());
    }

    Ok(status)
}

/// Install a specific tool
pub async fn install<F>(tool_name: &str, mut progress: F) -> Result<(), DepsError>
where
    F: FnMut(u32, &str),
{
    let tool = TOOLS.iter().find(|t| t.name == tool_name)
        .ok_or_else(|| DepsError::NotFound(tool_name.to_string()))?;

    progress(0, &format!("Downloading {}...", tool_name));

    // Download from GitHub releases
    download_tool(tool, |p, m| progress(p, m)).await?;

    progress(100, "Complete!");

    Ok(())
}

/// Install all tools
pub async fn install_all<F>(mut progress: F) -> Result<(), DepsError>
where
    F: FnMut(&str, u32, &str),
{
    for tool in TOOLS {
        progress(&tool.name, 0, "Starting...");
        download_tool(tool, |p, m| progress(&tool.name, p, m)).await?;
        progress(&tool.name, 100, "Complete!");
    }

    Ok(())
}

/// Get paths of all tools
pub fn get_paths() -> HashMap<String, PathBuf> {
    let mut paths = HashMap::new();

    for tool in TOOLS {
        paths.insert(tool.name.to_string(), get_tool_path(&tool.name));
    }

    paths
}

/// Get tool path
fn get_tool_path(name: &str) -> PathBuf {
    let base_dir = get_tools_dir();
    let binary_name = match name {
        "AssetRipper" => {
            if cfg!(target_os = "windows") {
                "AssetRipper.exe"
            } else {
                "AssetRipper.GUI.Free"
            }
        }
        "Il2CppDumper" => {
            if cfg!(target_os = "windows") {
                "Il2CppDumper.exe"
            } else {
                "il2cpp-dumper"
            }
        }
        _ => name,
    };

    base_dir.join(name).join(binary_name)
}

/// Get tools directory (relative to project root or binary location)
pub fn get_tools_dir() -> PathBuf {
    // First try: relative to current directory (for development)
    let relative_path = PathBuf::from("tools");
    if relative_path.exists() {
        return relative_path;
    }
    
    // Second try: relative to executable location (for installed binary)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let tools_path = exe_dir.join("tools");
            if tools_path.exists() {
                return tools_path;
            }
            
            // Try one level up (for target/release/kgc → tools/)
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
    
    // Fallback: use ./tools/
    PathBuf::from("tools")
}

/// Get AssetRipper path (for c2u)
pub fn get_asset_ripper_path(custom_dir: Option<&Path>) -> Option<PathBuf> {
    let path = if let Some(dir) = custom_dir {
        let binary = if cfg!(target_os = "windows") {
            "AssetRipper.exe"
        } else {
            "AssetRipper.GUI.Free"
        };
        dir.join("AssetRipper").join(binary)
    } else {
        get_tool_path("AssetRipper")
    };

    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Download a tool from GitHub
async fn download_tool<F>(tool: &ToolDef, mut progress: F) -> Result<(), DepsError>
where
    F: FnMut(u32, &str),
{
    use futures_util::StreamExt;
    use std::io::Write;

    progress(5, "Fetching release info...");

    let release_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        tool.repo
    );

    let client = reqwest::Client::builder()
        .user_agent("kgc-toolkit")
        .build()?;

    let response = client.get(&release_url).send().await?;

    if !response.status().is_success() {
        return Err(DepsError::DownloadFailed(format!(
            "Failed to fetch release: {}",
            response.status()
        )));
    }

    let release: serde_json::Value = response.json().await?;

    // Find matching asset
    let assets = release["assets"].as_array()
        .ok_or_else(|| DepsError::DownloadFailed("No assets found".into()))?;

    let asset = assets.iter().find(|a| {
        let name = a["name"].as_str().unwrap_or("");
        tool.asset_pattern.iter().any(|p| name.contains(p))
    }).ok_or_else(|| DepsError::DownloadFailed("No matching asset".into()))?;

    let download_url = asset["browser_download_url"].as_str()
        .ok_or_else(|| DepsError::DownloadFailed("No download URL".into()))?;

    progress(10, "Downloading...");

    // Download
    let response = client.get(download_url).send().await?;
    let total_size = response.content_length().unwrap_or(0);

    let tool_dir = get_tools_dir().join(&tool.name);
    std::fs::create_dir_all(&tool_dir)?;

    let temp_path = tool_dir.join("download.tmp");
    let mut file = std::fs::File::create(&temp_path)?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percent = ((downloaded as f64 / total_size as f64) * 80.0) as u32 + 10;
            progress(percent.min(90), "Downloading...");
        }
    }

    progress(90, "Extracting...");

    // Extract based on file type
    let asset_name = asset["name"].as_str().unwrap_or("download");

    if asset_name.ends_with(".zip") {
        extract_zip(&temp_path, &tool_dir)?;
    } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        extract_tar_gz(&temp_path, &tool_dir)?;
    } else {
        // Single binary
        let dest = get_tool_path(&tool.name);
        std::fs::rename(&temp_path, &dest)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms)?;
        }
    }

    let _ = std::fs::remove_file(&temp_path);

    progress(100, "Done!");

    Ok(())
}

fn extract_zip(archive_path: &Path, output_dir: &Path) -> Result<(), DepsError> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| DepsError::DownloadFailed(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| DepsError::DownloadFailed(e.to_string()))?;

        let outpath = match file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if file.unix_mode().is_some() {
                    let mut perms = std::fs::metadata(&outpath)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&outpath, perms)?;
                }
            }
        }
    }

    Ok(())
}

fn extract_tar_gz(archive_path: &Path, output_dir: &Path) -> Result<(), DepsError> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = std::fs::File::open(archive_path)?;
    let tar = GzDecoder::new(file);
    let mut archive = Archive::new(tar);

    archive.unpack(output_dir)?;

    Ok(())
}
