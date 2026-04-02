//! XML config bundle handling
//! Workflow: read source index URL -> detect xml bundle URL -> download -> extract xml.

use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::core::crypto;

const FIXED_XML_BASE_URL: &str = "https://kgc-cdn-1.awesomepiece.com/";

#[derive(Error, Debug)]
pub enum XmlConfigError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Extraction failed: {0}")]
    Extraction(String),
}

#[derive(Debug, Clone)]
pub struct XmlBundleDownloadResult {
    pub bundle_path: String,
    pub base_url: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
pub struct XmlFile {
    pub name: String,
    pub content: String,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct XmlPatchLiveOption {
    pub platform: String,
    pub time: String,
    pub download_url: String,
    pub items: Vec<String>,
    pub file_urls: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct XmlUrlDetection {
    selected_url: Option<String>,
    candidates: Vec<String>,
}

/// Get default XML directory
pub fn default_xml_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kgc-toolkit")
        .join("xml_config")
}

pub fn get_xml_directory() -> Result<String, XmlConfigError> {
    let dir = default_xml_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir.display().to_string())
}

/// Download XML bundle by reading source URL content and choosing a matching bundle URL.
pub async fn download_xml_bundle_from_source(output_dir: Option<&Path>) -> Result<XmlBundleDownloadResult, XmlConfigError> {
    let output_dir = output_dir.map(|p| p.to_path_buf()).unwrap_or_else(default_xml_dir);
    fs::create_dir_all(&output_dir)?;

    let base_url = FIXED_XML_BASE_URL.trim_end_matches('/').to_string();
    let client = reqwest::Client::new();

    // Read source content first, then decide the concrete bundle URL.
    let response = client.get(FIXED_XML_BASE_URL).send().await?;
    if !response.status().is_success() {
        return Err(XmlConfigError::NotFound(format!(
            "Failed to read XML source (status: {}) from {}",
            response.status(),
            FIXED_XML_BASE_URL
        )));
    }

    let index_content = response.text().await?;
    let options = build_patch_live_options(FIXED_XML_BASE_URL, &index_content);

    let detection = if let Some(first) = options.first() {
        XmlUrlDetection {
            selected_url: Some(first.download_url.clone()),
            candidates: options.iter().map(|o| o.download_url.clone()).collect(),
        }
    } else {
        detect_xml_download_url(FIXED_XML_BASE_URL, &index_content)
    };

    eprintln!(
        "[XML][DETECT] source={} candidates={}",
        FIXED_XML_BASE_URL,
        detection.candidates.len()
    );
    for (idx, url) in detection.candidates.iter().take(20).enumerate() {
        eprintln!("[XML][DETECT] candidate[{}]={}", idx, url);
    }

    let xml_url = detection.selected_url.ok_or_else(|| {
        XmlConfigError::NotFound(format!(
            "No suitable XML bundle URL found in source content ({} candidate(s))",
            detection.candidates.len()
        ))
    })?;
    eprintln!("[XML][DETECT] selected={}", xml_url);

    let response = client.get(&xml_url).send().await?;
    if !response.status().is_success() {
        return Err(XmlConfigError::NotFound(format!(
            "XML bundle download failed (status: {}) from {}",
            response.status(),
            xml_url
        )));
    }

    let bundle_path = output_dir.join(format!("xml_bundle_{}", unique_ts_millis()));
    let bytes = response.bytes().await?;
    fs::write(&bundle_path, bytes)?;

    Ok(XmlBundleDownloadResult {
        bundle_path: bundle_path.display().to_string(),
        base_url,
        download_url: xml_url,
    })
}

pub async fn list_patch_live_options_from_source() -> Result<Vec<XmlPatchLiveOption>, XmlConfigError> {
    let client = reqwest::Client::new();
    let response = client.get(FIXED_XML_BASE_URL).send().await?;
    if !response.status().is_success() {
        return Err(XmlConfigError::NotFound(format!(
            "Failed to read XML source (status: {}) from {}",
            response.status(),
            FIXED_XML_BASE_URL
        )));
    }

    let index_content = response.text().await?;
    Ok(build_patch_live_options(FIXED_XML_BASE_URL, &index_content))
}

pub async fn download_xml_bundle_from_url(
    output_dir: Option<&Path>,
    download_url: &str,
) -> Result<XmlBundleDownloadResult, XmlConfigError> {
    let output_dir = output_dir.map(|p| p.to_path_buf()).unwrap_or_else(default_xml_dir);
    fs::create_dir_all(&output_dir)?;

    let client = reqwest::Client::new();
    let response = client.get(download_url).send().await?;
    if !response.status().is_success() {
        return Err(XmlConfigError::NotFound(format!(
            "XML bundle download failed (status: {}) from {}",
            response.status(),
            download_url
        )));
    }

    let bundle_path = output_dir.join(format!("xml_bundle_{}", unique_ts_millis()));
    let bytes = response.bytes().await?;
    fs::write(&bundle_path, bytes)?;

    Ok(XmlBundleDownloadResult {
        bundle_path: bundle_path.display().to_string(),
        base_url: FIXED_XML_BASE_URL.trim_end_matches('/').to_string(),
        download_url: download_url.to_string(),
    })
}

fn build_patch_live_options(base_url: &str, content: &str) -> Vec<XmlPatchLiveOption> {
    let key_re = match Regex::new(r"<Key>([^<]+)</Key>") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    let patch_re = match Regex::new(r"^/?patch/LIVE/([^/]+)/([^/]+)/(.+)$") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    let base = match reqwest::Url::parse(base_url) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut grouped: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
    for cap in key_re.captures_iter(content) {
        let Some(m) = cap.get(1) else {
            continue;
        };
        let key = m.as_str().trim();
        let Some(parts) = patch_re.captures(key) else {
            continue;
        };

        let time = parts.get(1).map(|v| v.as_str()).unwrap_or("").to_string();
        let platform = parts.get(2).map(|v| v.as_str()).unwrap_or("").to_string();
        if time.is_empty() || platform.is_empty() {
            continue;
        }

        let full_url = base
            .join(key.trim_start_matches('/'))
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("{}{}", base_url.trim_end_matches('/'), key));
        let item = parts.get(3).map(|v| v.as_str()).unwrap_or("").to_string();
        grouped
            .entry((platform, time))
            .or_default()
            .push((item, full_url));
    }

    let mut options: Vec<XmlPatchLiveOption> = Vec::new();
    for ((platform, time), mut entries) in grouped {
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        let mut items: Vec<String> = entries.iter().map(|(item, _)| item.clone()).collect();
        items.sort();
        items.dedup();

        let picked = entries
            .iter()
            .find(|(item, url)| item.eq_ignore_ascii_case("xml") || url.to_ascii_lowercase().ends_with("/xml"))
            .map(|(_, url)| url.clone())
            .or_else(|| entries.first().map(|(_, url)| url.clone()));

        if let Some(download_url) = picked {
            let mut file_urls: Vec<(String, String)> = entries
                .into_iter()
                .filter(|(item, _)| !item.trim().is_empty() && !item.ends_with('/'))
                .collect();
            file_urls.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            file_urls.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

            options.push(XmlPatchLiveOption {
                platform,
                time,
                download_url,
                items,
                file_urls,
            });
        }
    }

    options.sort_by(|a, b| {
        a.platform
            .cmp(&b.platform)
            .then_with(|| a.time.cmp(&b.time))
    });
    options
}

pub async fn download_patch_live_files_to_tree(
    output_root: &Path,
    platform: &str,
    time: &str,
    file_urls: &[(String, String)],
) -> Result<usize, XmlConfigError> {
    let target_root = output_root.join(platform).join(time);
    fs::create_dir_all(&target_root)?;

    let client = reqwest::Client::new();
    let mut downloaded = 0usize;

    for (item, url) in file_urls {
        let rel = safe_relative_item_path(item);
        if rel.as_os_str().is_empty() {
            continue;
        }

        let out_path = target_root.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(XmlConfigError::NotFound(format!(
                "File download failed (status: {}) from {}",
                resp.status(),
                url
            )));
        }

        let bytes = resp.bytes().await?;
        let decoded = decode_patch_file_bytes(item, &bytes);
        fs::write(&out_path, decoded)?;

        // Most non-txt patch files here are UnityFS bundles, not plain encrypted text.
        // Auto-extract XMLs into a readable mirror tree.
        if is_unityfs_bundle(&bytes) {
            let decoded_dir = if let Some(parent) = rel.parent() {
                target_root.join("_decoded").join(parent)
            } else {
                target_root.join("_decoded")
            };
            let _ = extract_xml_files(&out_path, &decoded_dir);
        }

        downloaded += 1;
    }

    Ok(downloaded)
}

fn safe_relative_item_path(item: &str) -> PathBuf {
    use std::path::Component;

    let mut out = PathBuf::new();
    for comp in Path::new(item.trim_start_matches('/')).components() {
        if let Component::Normal(seg) = comp {
            out.push(seg);
        }
    }
    out
}

fn is_unityfs_bundle(data: &[u8]) -> bool {
    data.starts_with(b"UnityFS")
}

fn decode_patch_file_bytes(item: &str, raw: &[u8]) -> Vec<u8> {
    if item.to_ascii_lowercase().ends_with(".txt") {
        return raw.to_vec();
    }

    if looks_like_text_payload(raw) {
        if let Ok(s) = std::str::from_utf8(raw) {
            if let Ok(decoded) = crypto::decrypt_base64(s) {
                return decoded.into_bytes();
            }
            if is_likely_hex(s) {
                if let Ok(decoded) = crypto::decrypt_hex(s) {
                    return decoded.into_bytes();
                }
            }
        }
    }

    if let Ok(decoded) = crypto::decrypt_raw(raw) {
        if looks_like_useful_decoded(&decoded) {
            return decoded;
        }
    }

    raw.to_vec()
}

fn looks_like_text_payload(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let printable = data
        .iter()
        .filter(|b| b.is_ascii_graphic() || **b == b'\n' || **b == b'\r' || **b == b'\t' || **b == b' ')
        .count();
    printable * 100 / data.len() > 90
}

fn is_likely_hex(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.len() % 2 == 0 && t.bytes().all(|b| b.is_ascii_hexdigit())
}

fn looks_like_useful_decoded(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let t = s.trim_start_matches('\u{feff}').trim_start();
        return t.starts_with("<?xml")
            || t.starts_with('<')
            || t.starts_with('{')
            || t.starts_with('[')
            || t.contains("<root")
            || t.contains("<Table")
            || t.contains("<datas");
    }

    false
}

fn detect_xml_download_url(base_url: &str, content: &str) -> XmlUrlDetection {
    let mut candidates: Vec<String> = Vec::new();

    if let Ok(href_re) = Regex::new(r#"href=[\"']([^\"']+)[\"']"#) {
        for cap in href_re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                candidates.push(m.as_str().to_string());
            }
        }
    }

    // Primary source: parse XML listing keys directly and filter by
    // patch/LIVE/<time>/<platform>/* pattern.
    if let Ok(key_re) = Regex::new(r"<Key>([^<]+)</Key>") {
        let patch_key_re = Regex::new(r"^/?patch/LIVE/[^/]+/[^/]+/.+").ok();
        if let Some(patch_key_re) = patch_key_re {
            for cap in key_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let key = m.as_str().trim();
                    if patch_key_re.is_match(key) {
                        candidates.push(key.to_string());
                    }
                }
            }
        }
    }

    if let Ok(absolute_re) = Regex::new(r#"https?://[^\s\"'<>]+"#) {
        for cap in absolute_re.captures_iter(content) {
            if let Some(m) = cap.get(0) {
                candidates.push(m.as_str().to_string());
            }
        }
    }

    let base = match reqwest::Url::parse(base_url) {
        Ok(v) => v,
        Err(_) => {
            return XmlUrlDetection {
                selected_url: None,
                candidates: Vec::new(),
            };
        }
    };
    let mut normalized: Vec<String> = Vec::new();
    for raw in candidates {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }

        let url = if raw.starts_with("http://") || raw.starts_with("https://") {
            raw.to_string()
        } else if let Ok(joined) = base.join(raw) {
            joined.to_string()
        } else {
            continue;
        };

        let lower = url.to_ascii_lowercase();
        if lower.ends_with(".txt") {
            continue;
        }
        normalized.push(url);
    }

    normalized.sort();
    normalized.dedup();

    let mut best: Option<(i32, String)> = None;
    for url in &normalized {
        let lower = url.to_ascii_lowercase();
        let mut score = 0;
        if lower.contains("xml") {
            score += 5;
        }
        if lower.contains("bundle") {
            score += 3;
        }
        if lower.ends_with(".unity3d") || lower.ends_with(".bundle") || lower.ends_with(".ab") {
            score += 4;
        }
        if lower.ends_with('/') {
            score -= 2;
        }

        if score > 0 {
            match &best {
                Some((best_score, _)) if score <= *best_score => {}
                _ => best = Some((score, url.clone())),
            }
        }
    }

    let selected_url = if let Some((_, url)) = best {
        Some(url)
    } else {
        // Fallback for old direct endpoint.
        base.join("xml").ok().map(|u| u.to_string())
    };

    XmlUrlDetection {
        selected_url,
        candidates: normalized,
    }
}

/// Backward-compatible fetch wrapper.
pub async fn fetch_bundle(output_dir: Option<&Path>) -> Result<PathBuf, XmlConfigError> {
    let result = download_xml_bundle_from_source(output_dir).await?;
    Ok(PathBuf::from(result.bundle_path))
}

/// Extract XML files from bundle. Prefer packaged extractor, fallback to built-in parser.
pub fn extract_xml_files(bundle_path: &Path, output_dir: &Path) -> Result<Vec<String>, XmlConfigError> {
    if !bundle_path.exists() {
        return Err(XmlConfigError::NotFound(bundle_path.display().to_string()));
    }

    fs::create_dir_all(output_dir)?;

    // Some selected CDN targets are already raw XML files (not Unity bundles).
    // In that case, write them directly and skip bundle extractors.
    if let Ok(extracted) = extract_direct_xml_file(bundle_path, output_dir) {
        if !extracted.is_empty() {
            return Ok(extracted);
        }
    }

    if let Ok(extracted) = run_packaged_extractor(bundle_path, output_dir) {
        if !extracted.is_empty() {
            return Ok(extracted);
        }
    }

    if let Ok(extracted) = run_python_extractor(bundle_path, output_dir) {
        if !extracted.is_empty() {
            return Ok(extracted);
        }
    }

    if let Ok(extracted) = run_asset_ripper_extractor(bundle_path, output_dir) {
        if !extracted.is_empty() {
            return Ok(extracted);
        }
    }

    Err(XmlConfigError::Extraction(
        "No XML extracted: source is not raw XML and no extractor succeeded".to_string(),
    ))
}

fn run_asset_ripper_extractor(bundle_path: &Path, output_dir: &Path) -> Result<Vec<String>, XmlConfigError> {
    let asset_ripper = crate::deps::get_asset_ripper_path(None)
        .ok_or_else(|| XmlConfigError::NotFound("AssetRipper not found".to_string()))?;

    let temp_root = std::env::temp_dir().join(format!("kgc_xml_ar_{}", unique_ts_millis()));
    let temp_input = temp_root.join("input");
    let temp_output = temp_root.join("output");
    fs::create_dir_all(&temp_input)?;
    fs::create_dir_all(&temp_output)?;

    let input_name = bundle_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("bundle");
    let staged_bundle = temp_input.join(input_name);
    fs::copy(bundle_path, &staged_bundle)?;

    let status = Command::new(&asset_ripper)
        .arg("--cli")
        .arg("--input")
        .arg(&temp_input)
        .arg("--output")
        .arg(&temp_output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        let _ = fs::remove_dir_all(&temp_root);
        return Err(XmlConfigError::Extraction(format!(
            "AssetRipper failed with exit code {:?}",
            status.code()
        )));
    }

    let mut xml_paths = Vec::new();
    collect_files_with_ext(&temp_output, "xml", &mut xml_paths)?;

    let mut copied = Vec::new();
    for src in xml_paths {
        let rel = src
            .strip_prefix(&temp_output)
            .unwrap_or(src.as_path());
        let dst = output_dir.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&src, &dst)?;
        copied.push(dst.display().to_string());
    }

    let _ = fs::remove_dir_all(&temp_root);
    Ok(copied)
}

fn collect_files_with_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) -> Result<(), XmlConfigError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_ext(&path, ext, out)?;
            continue;
        }

        let matches = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false);
        if matches {
            out.push(path);
        }
    }
    Ok(())
}

fn unique_ts_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn extract_direct_xml_file(bundle_path: &Path, output_dir: &Path) -> Result<Vec<String>, XmlConfigError> {
    let content = match fs::read_to_string(bundle_path) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };

    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    if !(trimmed.starts_with("<?xml") || trimmed.starts_with('<')) {
        return Ok(Vec::new());
    }

    // Basic sanity check to avoid writing arbitrary HTML/text as XML.
    if !trimmed.contains("</") {
        return Ok(Vec::new());
    }

    let stem = bundle_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("xml");
    let file_name = format!("{}.xml", stem);
    let out_path = output_dir.join(&file_name);
    fs::write(&out_path, content)?;

    Ok(vec![file_name])
}

/// Extract XML files from bundle
pub fn extract_xml(bundle_path: &Path, output_dir: &Path) -> Result<usize, XmlConfigError> {
    if !bundle_path.exists() {
        return Err(XmlConfigError::NotFound(bundle_path.display().to_string()));
    }

    fs::create_dir_all(output_dir)?;

    // Simple extraction attempt - Unity bundles are complex
    // This is a simplified version that tries to extract text content
    // Full implementation would use UnityPy (Python) or a Rust Unity bundle parser
    
    let bundle_data = fs::read(bundle_path)?;
    let mut extracted_count = 0;
    
    // Unity bundles often contain XML as plain text embedded in the file
    // Try to extract text chunks that look like XML
    let text = String::from_utf8_lossy(&bundle_data);
    let mut current_xml = String::new();
    let mut in_xml = false;
    let mut xml_files = Vec::new();
    
    for line in text.lines() {
        if line.trim().starts_with("<?xml") || line.trim().starts_with("<") {
            in_xml = true;
            current_xml.clear();
        }
        
        if in_xml {
            current_xml.push_str(line);
            current_xml.push('\n');
            
            // Check if this looks like end of XML
            if line.contains("</") && current_xml.len() > 100 {
                // Try to extract a name from the XML
                let name = extract_xml_name(&current_xml);
                if !name.is_empty() {
                    xml_files.push((name, current_xml.clone()));
                    extracted_count += 1;
                }
                in_xml = false;
                current_xml.clear();
            }
        }
    }
    
    // Write extracted XML files
    for (name, content) in xml_files {
        let output_path = output_dir.join(format!("{}.xml", name));
        if let Err(e) = fs::write(&output_path, content) {
            eprintln!("Failed to write {}: {}", output_path.display(), e);
        }
    }
    
    Ok(extracted_count)
}

fn run_packaged_extractor(bundle_path: &Path, output_dir: &Path) -> Result<Vec<String>, XmlConfigError> {
    let extractor_candidates = [
        PathBuf::from("tools/xml-extractor/xml-extractor"),
        PathBuf::from("tools/xml-extractor/xml-extractor.exe"),
        PathBuf::from("tools/xml-extractor"),
    ];

    let extractor = extractor_candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| XmlConfigError::NotFound("No packaged XML extractor found".to_string()))?;

    let mut child = Command::new(extractor)
        .arg(bundle_path)
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut extracted = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix("[EXTRACTED]") {
                extracted.push(rest.trim().to_string());
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(XmlConfigError::Extraction(format!(
            "Packaged extractor failed with exit code {:?}",
            status.code()
        )));
    }

    Ok(extracted)
}

fn run_python_extractor(bundle_path: &Path, output_dir: &Path) -> Result<Vec<String>, XmlConfigError> {
    let script_candidates = [
        PathBuf::from("tools/xml-extractor/extract_xml.py"),
        PathBuf::from("scripts/extract_xml.py"),
    ];

    let script = script_candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| XmlConfigError::NotFound("No python extractor script found".to_string()))?;

    let mut child = Command::new("python3")
        .arg(script)
        .arg(bundle_path)
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut extracted = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix("[EXTRACTED]") {
                extracted.push(rest.trim().to_string());
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(XmlConfigError::Extraction(format!(
            "Python extractor failed with exit code {:?}",
            status.code()
        )));
    }

    Ok(extracted)
}

/// Try to extract a meaningful name from XML content
fn extract_xml_name(xml: &str) -> String {
    // Try to find root element name
    if let Some(start) = xml.find('<') {
        if let Some(end) = xml[start+1..].find(|c: char| c.is_whitespace() || c == '>') {
            let tag = &xml[start+1..start+1+end];
            if !tag.is_empty() && !tag.starts_with('?') && !tag.starts_with('/') {
                return tag.to_string();
            }
        }
    }
    
    // Fallback to hash
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    xml.hash(&mut hasher);
    format!("config_{:x}", hasher.finish())
}

/// List XML files in directory
pub fn list_files(dir: &Path) -> Result<Vec<String>, XmlConfigError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "xml" {
                    if let Some(name) = path.file_name() {
                        files.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

pub fn list_xml_files(dir: &Path) -> Result<Vec<XmlFile>, XmlConfigError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_xml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("xml"))
            .unwrap_or(false);

        if path.is_file() && is_xml {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            let content = fs::read_to_string(&path)?;
            let size = content.len();
            files.push(XmlFile { name, content, size });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Read an XML file
pub fn read_file(file_name: &str) -> Result<String, XmlConfigError> {
    let xml_dir = default_xml_dir();

    // Check if it's an absolute path or relative to xml_dir
    let path = if Path::new(file_name).is_absolute() {
        PathBuf::from(file_name)
    } else {
        xml_dir.join(file_name)
    };

    if !path.exists() {
        return Err(XmlConfigError::NotFound(path.display().to_string()));
    }

    Ok(fs::read_to_string(path)?)
}
