//! C2U - Convert XAPK to Unity project
//! Ported from src-tauri/src/c2u.rs

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use regex::Regex;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Error, Debug)]
pub enum C2uError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),
    #[error("AssetRipper not found. Run 'kgc deps install AssetRipper'")]
    AssetRipperNotFound,
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

/// Convert XAPK to Unity project
pub async fn convert<F>(
    xapk_path: &Path,
    output_dir: &Path,
    tools_dir: Option<&Path>,
    mut progress_callback: F,
) -> Result<(), C2uError>
where
    F: FnMut(&str, u32, &str),
{
    // Validate input
    if !xapk_path.exists() {
        return Err(C2uError::NotFound(xapk_path.display().to_string()));
    }

    if !output_dir.exists() {
        return Err(C2uError::NotFound(output_dir.display().to_string()));
    }

    // Get AssetRipper path
    let asset_ripper = crate::deps::get_asset_ripper_path(tools_dir)
        .ok_or(C2uError::AssetRipperNotFound)?;

    // Extract version from filename
    let file_name = xapk_path
        .file_stem()
        .ok_or_else(|| C2uError::InvalidFormat("Cannot get filename".into()))?
        .to_string_lossy();

    let version = infer_version_from_filename(&file_name)
        .ok_or_else(|| {
            C2uError::InvalidFormat(
                "Cannot infer version from filename. Supported examples: <app_id>@<version>.xapk or <name>_<version>_APKPure.xapk"
                    .into(),
            )
        })?;

    // Create output directory for this version
    let version_output = output_dir.join(version);
    if version_output.exists() {
        fs::remove_dir_all(&version_output)?;
    }
    fs::create_dir_all(&version_output)?;

    // Create temp directory
    let temp_dir = version_output.join("temp");
    fs::create_dir_all(&temp_dir)?;

    // Step 1: Extract XAPK
    progress_callback("extract", 10, "Extracting XAPK...");
    extract_zip(xapk_path, &temp_dir)?;

    // Step 2: Find and classify APKs
    progress_callback("analyze", 20, "Analyzing APK files...");

    let mut base_assets_path: Option<PathBuf> = None;
    let mut config_apks: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(&temp_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = path.file_name().unwrap().to_string_lossy();

        if file_name.ends_with(".apk") {
            if file_name == "base_assets.apk" {
                base_assets_path = Some(path);
            } else if file_name.starts_with("config.") {
                config_apks.push(path);
            }
        }
    }

    // Step 3: Extract APKs
    let assets_dir = temp_dir.join("extracted_assets");
    fs::create_dir_all(&assets_dir)?;

    if let Some(base_path) = base_assets_path {
        progress_callback("extract", 30, "Extracting base_assets.apk...");
        extract_apk_assets(&base_path, &assets_dir)?;
    }

    for (i, config_apk) in config_apks.iter().enumerate() {
        let progress = 30 + (i as u32 * 10 / config_apks.len().max(1) as u32);
        progress_callback(
            "extract",
            progress,
            &format!("Extracting {}...", config_apk.file_name().unwrap().to_string_lossy()),
        );
        extract_apk_assets(config_apk, &assets_dir)?;
    }

    // Step 4: Run AssetRipper
    progress_callback("convert", 50, "Running AssetRipper...");

    let unity_output = version_output.join("UnityProject");

    run_asset_ripper(&asset_ripper, &assets_dir, &unity_output, |msg| {
        progress_callback("convert", 60, msg);
    })?;

    // Step 5: Rename Plugins to Plugins~
    let plugins_dir = unity_output.join("Assets").join("Plugins");
    if plugins_dir.exists() {
        let plugins_hidden = unity_output.join("Assets").join("Plugins~");
        fs::rename(&plugins_dir, &plugins_hidden)?;
    }

    // Step 6: Cleanup
    progress_callback("cleanup", 90, "Cleaning up temporary files...");
    let _ = fs::remove_dir_all(&temp_dir);

    progress_callback("done", 100, "Conversion complete!");

    Ok(())
}

/// Extract a ZIP file
fn extract_zip(zip_path: &Path, output_dir: &Path) -> Result<(), C2uError> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => output_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

/// Extract assets from APK
fn extract_apk_assets(apk_path: &Path, output_dir: &Path) -> Result<(), C2uError> {
    let file = File::open(apk_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        // Only extract assets
        if !name.starts_with("assets/") {
            continue;
        }

        let outpath = output_dir.join(&name);

        if name.ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

/// Run AssetRipper with file descriptor limit handling
fn run_asset_ripper<F>(
    asset_ripper: &Path,
    input_dir: &Path,
    output_dir: &Path,
    mut log_callback: F,
) -> Result<(), C2uError>
where
    F: FnMut(&str),
{
    enum StreamLine {
        Stdout(String),
        Stderr(String),
    }

    // Use Command directly to pass arguments properly
    let mut child = Command::new(asset_ripper)
        .arg("--cli")
        .arg("--input")
        .arg(input_dir)
        .arg("--output")
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // Log the command being executed
    log_callback(&format!(
        "[DEBUG] Running: {} --cli --input {} --output {}",
        asset_ripper.display(),
        input_dir.display(),
        output_dir.display()
    ));

    // Capture both stdout and stderr
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut error_messages = Vec::new();
    let mut has_errors = false;

    let (tx, rx) = std::sync::mpsc::channel::<StreamLine>();

    let stdout_handle = stdout.map(|stdout_pipe| {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout_pipe);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx.send(StreamLine::Stdout(line));
            }
        })
    });

    let stderr_handle = stderr.map(|stderr_pipe| {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr_pipe);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx.send(StreamLine::Stderr(line));
            }
        })
    });

    drop(tx);

    for item in rx {
        match item {
            StreamLine::Stdout(line) => {
                if should_display_log(&line) {
                    log_callback(&line);
                }
            }
            StreamLine::Stderr(line) => {
                has_errors = true;
                error_messages.push(line.clone());
                log_callback(&format!("[STDERR] {}", line));
            }
        }
    }

    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }

    // Wait for child process to complete and check exit status
    let status = child.wait();

    // AssetRipper writes .log files in its own folder; always cleanup after run.
    if let Err(e) = cleanup_asset_ripper_logs(asset_ripper) {
        log_callback(&format!(
            "[WARN] Failed to cleanup AssetRipper logs in {}: {}",
            asset_ripper
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .display(),
            e
        ));
    }

    let status = status?;
    
    // If there were stderr messages or non-zero exit, report error
    if has_errors || !status.success() {
        let error_text = if error_messages.is_empty() {
            format!("AssetRipper failed with exit code: {:?}", status.code())
        } else {
            format!(
                "AssetRipper failed:\n{}",
                error_messages.join("\n")
            )
        };
        return Err(C2uError::ExtractionFailed(error_text));
    }

    Ok(())
}

fn cleanup_asset_ripper_logs(asset_ripper: &Path) -> Result<(), std::io::Error> {
    let Some(dir) = asset_ripper.parent() else {
        return Ok(());
    };

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let is_log = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("log"))
            .unwrap_or(false);

        if path.is_file() && is_log {
            let _ = fs::remove_file(path);
        }
    }

    // AssetRipper can leave a temp folder beside the executable.
    let temp_dir = dir.join("temp");
    if temp_dir.exists() && temp_dir.is_dir() {
        let _ = fs::remove_dir_all(temp_dir);
    }

    Ok(())
}

fn infer_version_from_filename(file_name: &str) -> Option<String> {
    // Legacy/custom format: com.app.id@167.0.01.xapk
    if let Some(v) = file_name.split('@').nth(1) {
        if is_semver_like(v) {
            return Some(v.to_string());
        }
    }

    // APKPure/original names: King God Castle_167.0.01_APKPure.xapk
    let patterns = [
        r"(?i)_((?:\d+\.){2,}\d+)_apkpure$",
        r"(?i)v((?:\d+\.){2,}\d+)$",
        r"(?i)((?:\d+\.){2,}\d+)$",
        r"(?i)((?:\d+\.){2,}\d+)",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(cap) = re.captures(file_name) {
                if let Some(m) = cap.get(1) {
                    let v = m.as_str();
                    if is_semver_like(v) {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }

    None
}

fn is_semver_like(v: &str) -> bool {
    if let Ok(re) = Regex::new(r"^\d+(?:\.\d+){2,}$") {
        return re.is_match(v);
    }
    false
}

/// Filter AssetRipper logs to show only important information
fn should_display_log(line: &str) -> bool {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_lowercase();

    // Skip debug/trace prefixes
    if lower.starts_with("[debug]")
        || lower.starts_with("[trace]")
        || lower.starts_with("  ->")
        || lower.starts_with("    ")
    {
        return false;
    }

    // Include progress, errors, and summaries
    lower.contains('%')
        || lower.contains("processing")
        || lower.contains("exporting")
        || lower.contains("completed")
        || lower.contains("finished")
        || lower.contains("error")
        || lower.contains("warning")
        || lower.contains("failed")
}
