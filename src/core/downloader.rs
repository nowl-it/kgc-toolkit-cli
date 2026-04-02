//! XAPK downloader using native APKPure HTTP flow

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::time::Duration;

use futures_util::StreamExt;
use regex::Regex;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use crate::core::kgc;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("No versions available")]
    NoVersions,
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    #[error("Output directory does not exist: {0}")]
    OutputNotFound(String),
}

/// List available versions from APKPure versions page.
pub async fn list_versions(package_id: &str) -> Result<Vec<String>, DownloadError> {
    if package_id != kgc::PACKAGE_ID {
        return Err(DownloadError::DownloadFailed(
            "Version listing is currently only supported for King God Castle package".to_string(),
        ));
    }

    let response = reqwest::Client::new()
        .get(kgc::APKPURE_VERSIONS_URL)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(DownloadError::DownloadFailed(format!(
            "Failed to fetch APKPure versions page: HTTP {}",
            response.status()
        )));
    }

    let html = response.text().await?;
    let mut versions = parse_apkpure_versions_from_html(&html);

    if versions.is_empty() {
        return Err(DownloadError::NoVersions);
    }

    sort_versions_desc(&mut versions);
    versions.dedup();

    Ok(versions)
}

/// Download XAPK from APKPure without apkeep.
pub async fn download_xapk<F>(
    package_id: &str,
    version: Option<&str>,
    output_dir: &Path,
    mut progress_callback: F,
) -> Result<PathBuf, DownloadError>
where
    F: FnMut(u32, &str),
{
    if package_id != kgc::PACKAGE_ID {
        return Err(DownloadError::DownloadFailed(
            "Native downloader is currently only supported for King God Castle package".to_string(),
        ));
    }

    if !output_dir.exists() {
        return Err(DownloadError::OutputNotFound(output_dir.display().to_string()));
    }

    progress_callback(5, "Checking available versions...");
    let versions = list_versions(package_id).await?;

    let target_version = if let Some(v) = version {
        if !versions.iter().any(|x| x == v) {
            return Err(DownloadError::VersionNotFound(v.to_string()));
        }
        v.to_string()
    } else {
        versions.first().cloned().ok_or(DownloadError::NoVersions)?
    };

    progress_callback(12, &format!("Resolving download link for {}...", target_version));

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(15))
        .connect_timeout(Duration::from_secs(6))
        .build()?;

    // Try multiple strategies to find actual download link
    let download_url = find_apkpure_download_link(&client, package_id, &target_version).await?;

    progress_callback(20, "Downloading...");
    let response = client
        .get(&download_url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(DownloadError::DownloadFailed(format!(
            "Failed to download: HTTP {}",
            response.status()
        )));
    }

    let filename = resolve_download_filename(&response, package_id, &target_version);
    let output_path = output_dir.join(filename);

    progress_callback(25, "Downloading...");
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut file = tokio::fs::File::create(&output_path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let pct = 25 + ((downloaded as f64 / total_size as f64) * 70.0) as u32;
            progress_callback(pct.min(95), "Downloading...");
        }
    }

    file.flush().await?;

    progress_callback(98, "Verifying download...");
    verify_xapk(&output_path)?;

    progress_callback(100, "Complete!");
    Ok(output_path)
}

fn looks_like_html(response: &reqwest::Response) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false)
}

async fn find_apkpure_download_link(
    client: &reqwest::Client,
    package_id: &str,
    version: &str,
) -> Result<String, DownloadError> {
    let mut sampled_candidates: Vec<String> = Vec::new();
    // Strategy 1: Try mobile version (simpler HTML)
    let mobile_url = format!(
        "https://m.apkpure.com/king-god-castle/{}/download/{}",
        package_id, version
    );
    if let Ok(links) = try_extract_download_from_page(client, &mobile_url).await {
        sampled_candidates.extend(links.iter().take(3).cloned());
        if let Some(link) = first_valid_download_link(client, links).await {
            return Ok(link);
        }
    }

    // Strategy 2: Try desktop version
    let desktop_url = format!(
        "https://apkpure.com/king-god-castle/{}/download/{}",
        package_id, version
    );
    if let Ok(links) = try_extract_download_from_page(client, &desktop_url).await {
        sampled_candidates.extend(links.iter().take(3).cloned());
        if let Some(link) = first_valid_download_link(client, links).await {
            return Ok(link);
        }
    }

    // Strategy 3: Try common CDN endpoints
    let cdn_patterns = vec![
        format!("https://d.apkpure.com/b/APK/{}_{}.apk", package_id, version),
        format!("https://d.apkpure.com/b/XAPK/{}_{}.xapk", package_id, version),
        format!("https://cdn.apkpure.com/{}_{}.apk", package_id, version),
    ];
    
    for cdn_url in cdn_patterns {
        if let Some(link) = probe_download_link(client, &cdn_url).await {
            return Ok(link);
        }
    }

    // Strategy 4: Try package ID without app name
    let no_name_url = format!(
        "https://m.apkpure.com/{}/download/{}",
        package_id, version
    );
    if let Ok(links) = try_extract_download_from_page(client, &no_name_url).await {
        sampled_candidates.extend(links.iter().take(3).cloned());
        if let Some(link) = first_valid_download_link(client, links).await {
            return Ok(link);
        }
    }

    Err(DownloadError::DownloadFailed(
        format!(
            "Could not resolve APKPure download link for {}-{}. candidates: {}",
            version,
            package_id,
            sampled_candidates.into_iter().take(6).collect::<Vec<_>>().join(" | ")
        )
    ))
}

async fn try_extract_download_from_page(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<String>, DownloadError> {
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    if !looks_like_html(&response) {
        // Direct binary file
        return Ok(vec![response.url().to_string()]);
    }

    let page_url = response.url().to_string();
    let html = response.text().await?;
    Ok(extract_download_links_from_html(&html, &page_url))
}

async fn first_valid_download_link(client: &reqwest::Client, links: Vec<String>) -> Option<String> {
    for link in links.into_iter().take(8) {
        if let Some(usable) = probe_download_link(client, &link).await {
            return Some(usable);
        }
    }
    None
}

async fn probe_download_link(client: &reqwest::Client, url: &str) -> Option<String> {
    let request = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .header("Referer", "https://apkpure.com/")
        .header("Range", "bytes=0-0");

    let response = match timeout(Duration::from_secs(6), request.send()).await {
        Ok(Ok(r)) if r.status().is_success() => r,
        _ => return None,
    };

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let content_disposition = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let final_url = response.url().to_string();
    let final_path = response.url().path().to_ascii_lowercase();

    if content_type.contains("text/html")
        || content_type.contains("application/json")
        || content_type.contains("text/css")
        || content_type.contains("javascript")
        || content_type.starts_with("text/")
    {
        return None;
    }

    let path_looks_like_package = final_path.contains(".apk")
        || final_path.contains(".xapk")
        || final_path.contains(".apkm")
        || final_path.contains("/b/apk/")
        || final_path.contains("/b/xapk/");

    let disposition_looks_like_package = content_disposition.contains(".apk")
        || content_disposition.contains(".xapk")
        || content_disposition.contains(".apkm");

    let type_looks_like_package = content_type.contains("application/vnd.android.package-archive")
        || content_type.contains("application/xapk-package-archive")
        || content_type.contains("application/octet-stream")
        || content_type.contains("binary/octet-stream")
        || content_type.contains("application/zip")
        || content_type.contains("application/x-zip-compressed");

    if path_looks_like_package || disposition_looks_like_package || type_looks_like_package {
        return Some(final_url);
    }

    None
}

fn extract_download_links_from_html(html: &str, page_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(extract_direct_cdn_links(html));
    out.extend(extract_from_data_attribute(html, page_url));
    out.extend(extract_from_onclick(html, page_url));
    out.extend(extract_href_pattern(html, page_url));
    out.extend(extract_from_link_text(html));

    // Rank likely direct links first.
    out.sort_by_key(|u| {
        if u.contains("d.apkpure.com") || u.contains(".xapk") || u.contains(".apk") {
            0
        } else if u.contains("/download") {
            1
        } else {
            2
        }
    });

    let mut seen = HashSet::new();
    out.retain(|u| seen.insert(u.clone()));
    out
}

fn extract_direct_cdn_links(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(re) = Regex::new(r#"https://d\.apkpure\.com/b/[^"'<>\s]+"#) {
        for m in re.find_iter(html) {
            let url = decode_html_entities(m.as_str());
            out.push(url);
        }
    }
    out
}

fn absolutize_candidate(raw: &str, page_url: &str) -> Option<String> {
    let decoded = decode_html_entities(raw);
    let cleaned = decoded.trim().trim_matches('\'').trim_matches('"');
    if cleaned.is_empty() {
        return None;
    }

    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        return Some(cleaned.to_string());
    }

    if cleaned.starts_with("//") {
        return Some(format!("https:{}", cleaned));
    }

    if cleaned.starts_with('/') {
        let base = if page_url.contains("m.apkpure.com") {
            "https://m.apkpure.com"
        } else {
            "https://apkpure.com"
        };
        return Some(format!("{}{}", base, cleaned));
    }

    None
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn is_obvious_static_asset(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.ends_with(".css")
        || lower.ends_with(".js")
        || lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".svg")
        || lower.ends_with(".webp")
        || lower.ends_with(".woff")
        || lower.ends_with(".woff2")
}

fn extract_from_data_attribute(html: &str, page_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Look for data-url, data-link, data-href attributes
    for pattern in &[
        "data-url=[\"']([^\"']+)[\"']",
        "data-link=[\"']([^\"']+)[\"']",
        "data-href=[\"']([^\"']+)[\"']",
    ] {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    let url = m.as_str();
                    if let Some(full_url) = absolutize_candidate(url, page_url) {
                        if !is_obvious_static_asset(&full_url)
                            && (full_url.contains(".apk")
                            || full_url.contains(".xapk")
                            || full_url.contains(".apkm")
                            || full_url.contains("/download")
                            || full_url.contains("/b/APK/")
                            || full_url.contains("/b/XAPK/"))
                        {
                            out.push(full_url);
                        }
                    }
                }
            }
        }
    }
    out
}

fn extract_from_link_text(html: &str) -> Option<String> {
    // Look for download links in common patterns
    let patterns = [
        ">([^<]*d\\.apkpure\\.com[^<]+)<",
        ">([^<]*cdn[^<]*\\.apk[^<]*)<",
    ];
    
    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    if let Some(url_match) = Regex::new("https?://[^\\s'\"<>]+")
                        .ok()
                        .and_then(|r| r.find(m.as_str()))
                    {
                        return Some(url_match.as_str().to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_href_pattern(html: &str, page_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    let patterns = [
        "href=\"([^\"]+)\"",
        "href='([^']+)'",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    let raw = m.as_str();
                    if let Some(url) = absolutize_candidate(raw, page_url) {
                        if !is_obvious_static_asset(&url)
                            && (url.contains(".apk")
                            || url.contains(".xapk")
                            || url.contains(".apkm")
                            || url.contains("/download")
                            || url.contains("/b/APK/")
                            || url.contains("/b/XAPK/"))
                        {
                            out.push(url);
                        }
                    }
                }
            }
        }
    }

    out
}

fn extract_from_onclick(html: &str, page_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Try onclick attribute with double quotes
    if let Ok(re) = Regex::new("onclick=\\\"([^\\\"]*)\\\"") {
        for cap in re.captures_iter(html) {
            if let Some(m) = cap.get(1) {
                let content = m.as_str();
                if let Some(url_match) = Regex::new("https?://[^\\s'\\\"<>]+")
                    .ok()
                    .and_then(|r| r.find(content))
                {
                    out.push(url_match.as_str().to_string());
                }
                if let Some(path_match) = Regex::new("/[^\\s'\\\">]+/download/[^\\s'\\\">]+")
                    .ok()
                    .and_then(|r| r.find(content))
                {
                    if let Some(url) = absolutize_candidate(path_match.as_str(), page_url) {
                        out.push(url);
                    }
                }
            }
        }
    }

    // Try onclick attribute with single quotes
    if let Ok(re) = Regex::new(r#"onclick='([^']*)'"#) {
        for cap in re.captures_iter(html) {
            if let Some(m) = cap.get(1) {
                let content = m.as_str();
                if let Some(url_match) = Regex::new("https?://[^\\s'\\\"<>]+")
                    .ok()
                    .and_then(|r| r.find(content))
                {
                    out.push(url_match.as_str().to_string());
                }
            }
        }
    }
    out
}

fn resolve_download_filename(response: &reqwest::Response, package_id: &str, version: &str) -> String {
    // Keep a stable filename format so Download and C2U interoperate consistently.
    let extension = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_filename_from_disposition)
        .and_then(|name| name.rsplit('.').next().map(|e| e.to_ascii_lowercase()))
        .or_else(|| {
            response
                .url()
                .path_segments()
                .and_then(|mut s| s.next_back())
                .and_then(|seg| seg.rsplit('.').next().map(|e| e.to_ascii_lowercase()))
        })
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "xapk".to_string());

    format!("{}@{}.{}", package_id, version, extension)
}

fn parse_filename_from_disposition(disposition: &str) -> Option<String> {
    // Basic filename extraction for values like: attachment; filename="file.xapk"
    for part in disposition.split(';').map(|p| p.trim()) {
        if let Some(value) = part.strip_prefix("filename=") {
            let trimmed = value.trim_matches('"').trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn parse_apkpure_versions_from_html(html: &str) -> Vec<String> {
    let mut versions = Vec::new();

    if let Ok(download_re) = Regex::new(r"/download/(\d+\.\d+\.\d+)") {
        for cap in download_re.captures_iter(html) {
            if let Some(m) = cap.get(1) {
                versions.push(m.as_str().to_string());
            }
        }
    }

    if let Ok(title_re) = Regex::new(r"King God Castle\s+(\d+\.\d+\.\d+)") {
        for cap in title_re.captures_iter(html) {
            if let Some(m) = cap.get(1) {
                versions.push(m.as_str().to_string());
            }
        }
    }

    versions
}

fn sort_versions_desc(versions: &mut Vec<String>) {
    versions.sort_by(|a, b| {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
        b_parts.cmp(&a_parts)
    });
}

/// Verify downloaded file is a valid XAPK/ZIP.
fn verify_xapk(path: &Path) -> Result<(), DownloadError> {
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 4];

    use std::io::Read;
    file.read_exact(&mut magic)?;

    // ZIP magic bytes: PK\x03\x04
    if magic != [0x50, 0x4B, 0x03, 0x04] {
        return Err(DownloadError::DownloadFailed(
            "Downloaded file is not a valid XAPK/ZIP archive".to_string(),
        ));
    }

    Ok(())
}
