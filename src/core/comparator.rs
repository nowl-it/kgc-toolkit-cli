//! Compare two Unity project directories
//! Ported from src-tauri/src/comparator.rs

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum CompareError {
    #[error("Directory not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub added_files: Vec<FileInfo>,
    pub removed_files: Vec<FileInfo>,
    pub modified_files: Vec<FileInfo>,
    pub unchanged_files: Vec<FileInfo>,
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub total_added: usize,
    pub total_removed: usize,
    pub total_modified: usize,
    pub total_unchanged: usize,
    pub new_heroes: Vec<String>,
    pub removed_heroes: Vec<String>,
    pub new_assets: HashMap<String, usize>,
}

/// Compare two Unity project directories
pub fn compare_projects(
    old_path: &Path,
    new_path: &Path,
    filter_pattern: Option<&str>,
) -> Result<ComparisonResult, CompareError> {
    if !old_path.exists() {
        return Err(CompareError::NotFound(old_path.display().to_string()));
    }

    if !new_path.exists() {
        return Err(CompareError::NotFound(new_path.display().to_string()));
    }

    // Scan both directories
    let old_files = scan_directory(old_path, filter_pattern)?;
    let new_files = scan_directory(new_path, filter_pattern)?;

    // Create maps for comparison
    let old_map: HashMap<String, FileInfo> = old_files
        .into_iter()
        .map(|f| (f.path.clone(), f))
        .collect();

    let new_map: HashMap<String, FileInfo> = new_files
        .into_iter()
        .map(|f| (f.path.clone(), f))
        .collect();

    let old_keys: HashSet<String> = old_map.keys().cloned().collect();
    let new_keys: HashSet<String> = new_map.keys().cloned().collect();

    // Find added files
    let added_keys: HashSet<_> = new_keys.difference(&old_keys).collect();
    let mut added_files: Vec<FileInfo> = added_keys
        .iter()
        .filter_map(|k| new_map.get(*k).cloned())
        .collect();
    added_files.sort_by(|a, b| a.path.cmp(&b.path));

    // Find removed files
    let removed_keys: HashSet<_> = old_keys.difference(&new_keys).collect();
    let mut removed_files: Vec<FileInfo> = removed_keys
        .iter()
        .filter_map(|k| old_map.get(*k).cloned())
        .collect();
    removed_files.sort_by(|a, b| a.path.cmp(&b.path));

    // Find modified and unchanged files
    let common_keys: HashSet<_> = old_keys.intersection(&new_keys).cloned().collect();
    let mut modified_files = Vec::new();
    let mut unchanged_files = Vec::new();

    for key in common_keys {
        let old_file = old_map.get(&key).unwrap();
        let new_file = new_map.get(&key).unwrap();

        if old_file.size != new_file.size || old_file.modified != new_file.modified {
            modified_files.push(new_file.clone());
        } else {
            unchanged_files.push(new_file.clone());
        }
    }

    modified_files.sort_by(|a, b| a.path.cmp(&b.path));
    unchanged_files.sort_by(|a, b| a.path.cmp(&b.path));

    // Analyze changes
    let mut summary = analyze_changes(&added_files, &removed_files, &modified_files);
    summary.total_unchanged = unchanged_files.len();

    Ok(ComparisonResult {
        added_files,
        removed_files,
        modified_files,
        unchanged_files,
        summary,
    })
}

/// Scan directory and collect file information
fn scan_directory(dir: &Path, filter_pattern: Option<&str>) -> Result<Vec<FileInfo>, CompareError> {
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
        let relative_path = path
            .strip_prefix(dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Apply filter if specified
        if let Some(pattern) = filter_pattern {
            if !relative_path.contains(pattern) {
                continue;
            }
        }

        let metadata = fs::metadata(path)?;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        files.push(FileInfo {
            path: relative_path,
            size: metadata.len(),
            modified,
        });
    }

    Ok(files)
}

/// Analyze changes to generate summary statistics
fn analyze_changes(
    added: &[FileInfo],
    removed: &[FileInfo],
    modified: &[FileInfo],
) -> ComparisonSummary {
    let mut new_heroes = Vec::new();
    let mut removed_heroes = Vec::new();
    let mut new_assets: HashMap<String, usize> = HashMap::new();

    // Detect new heroes (in Assets/01_Fx/1_Hero/Fx_XXX directories)
    for file in added {
        if file.path.contains("Assets/01_Fx/1_Hero/Fx_") {
            if let Some(hero_name) = extract_hero_name(&file.path) {
                if !new_heroes.contains(&hero_name) {
                    new_heroes.push(hero_name);
                }
            }
        }

        // Count new assets by category
        if let Some(category) = extract_asset_category(&file.path) {
            *new_assets.entry(category).or_insert(0) += 1;
        }
    }

    // Detect removed heroes
    for file in removed {
        if file.path.contains("Assets/01_Fx/1_Hero/Fx_") {
            if let Some(hero_name) = extract_hero_name(&file.path) {
                if !removed_heroes.contains(&hero_name) {
                    removed_heroes.push(hero_name);
                }
            }
        }
    }

    new_heroes.sort();
    removed_heroes.sort();

    ComparisonSummary {
        total_added: added.len(),
        total_removed: removed.len(),
        total_modified: modified.len(),
        total_unchanged: 0,
        new_heroes,
        removed_heroes,
        new_assets,
    }
}

/// Extract hero name from path like "Assets/01_Fx/1_Hero/Fx_001 (Knight)/..."
fn extract_hero_name(path: &str) -> Option<String> {
    if let Some(start) = path.find("Fx_") {
        let after_fx = &path[start..];
        if let Some(end) = after_fx.find('/') {
            return Some(after_fx[..end].to_string());
        }
    }
    None
}

/// Extract asset category from path
fn extract_asset_category(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 && parts[0] == "Assets" {
        let category = parts[1].to_string();

        if category.starts_with("01_Fx") {
            if path.contains("/1_Hero/") {
                return Some("Heroes".to_string());
            } else if path.contains("/2_Monster/") {
                return Some("Monsters".to_string());
            }
            return Some("Effects".to_string());
        } else if category.starts_with("00_Unit") {
            return Some("Units".to_string());
        } else if category.starts_with("02_UI") {
            return Some("UI".to_string());
        } else if category.starts_with("03_Skill") {
            return Some("Skills".to_string());
        } else if category.contains("Texture") {
            return Some("Textures".to_string());
        } else if category.contains("Prefab") {
            return Some("Prefabs".to_string());
        }

        return Some(category);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hero_name() {
        assert_eq!(
            extract_hero_name("Assets/01_Fx/1_Hero/Fx_001 (Knight)/skill.asset"),
            Some("Fx_001 (Knight)".to_string())
        );
        assert_eq!(
            extract_hero_name("Assets/01_Fx/1_Hero/Fx_10280 (Dragon)/prefab.prefab"),
            Some("Fx_10280 (Dragon)".to_string())
        );
        assert_eq!(extract_hero_name("Assets/Other/file.txt"), None);
    }

    #[test]
    fn test_extract_asset_category() {
        assert_eq!(
            extract_asset_category("Assets/01_Fx/1_Hero/Fx_001/skill.asset"),
            Some("Heroes".to_string())
        );
        assert_eq!(
            extract_asset_category("Assets/00_Unit/Unit_001.png"),
            Some("Units".to_string())
        );
    }
}
