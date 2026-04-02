//! Unity asset parsing utilities

pub mod prefab;

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum UnityError {
    #[error("Directory not found: {0}")]
    NotFound(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

/// Scan a Unity project for heroes
pub fn scan_heroes(project_path: &Path) -> Result<Vec<HeroInfo>, UnityError> {
    let hero_base = project_path.join("Assets/01_Fx/1_Hero");

    if !hero_base.exists() {
        return Ok(Vec::new());
    }

    let mut heroes = Vec::new();
    let hero_pattern = regex::Regex::new(r"Fx_(\d+)\s*\(([^)]+)\)").unwrap();

    for entry in WalkDir::new(&hero_base)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy();

        if let Some(caps) = hero_pattern.captures(&dir_name) {
            let id = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();

            heroes.push(HeroInfo {
                id,
                name,
                path: entry.path().display().to_string(),
            });
        }
    }

    heroes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(heroes)
}

/// Export hero assets to a directory
pub fn export_hero(hero_id: &str, project_path: &Path, output_dir: &Path) -> Result<(), UnityError> {
    let hero_base = project_path.join("Assets/01_Fx/1_Hero");

    if !hero_base.exists() {
        return Err(UnityError::NotFound(hero_base.display().to_string()));
    }

    // Find hero directory
    let mut hero_dir: Option<std::path::PathBuf> = None;
    let pattern = format!("Fx_{}", hero_id);

    for entry in std::fs::read_dir(&hero_base)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(&pattern) {
            hero_dir = Some(entry.path());
            break;
        }
    }

    let hero_dir = hero_dir.ok_or_else(|| {
        UnityError::NotFound(format!("Hero {} not found", hero_id))
    })?;

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Copy hero directory
    copy_dir_recursive(&hero_dir, &output_dir.join(hero_dir.file_name().unwrap()))?;

    // Also copy unit sprite if exists
    let unit_dir = project_path.join("Assets/00_Unit");
    if unit_dir.exists() {
        let unit_pattern = format!("Unit_{}", hero_id);
        for entry in std::fs::read_dir(&unit_dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with(&unit_pattern) {
                let dest = output_dir.join(entry.file_name());
                std::fs::copy(entry.path(), dest)?;
            }
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
