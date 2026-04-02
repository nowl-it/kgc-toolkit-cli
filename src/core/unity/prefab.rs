//! Unity prefab hierarchy parsing
//! Ported from src-tauri/src/unity/prefab.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;
use unity_yaml_rust::yaml::Yaml;

#[derive(Error, Debug)]
pub enum PrefabError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyNode {
    pub id: i64,
    pub name: String,
    pub node_type: String,
    pub children: Option<Vec<HierarchyNode>>,
    pub sprite: Option<String>, // Base64 encoded sprite
    pub animator_controller: Option<String>,
    pub components: Vec<String>,
}

impl HierarchyNode {
    pub fn new(id: i64, name: String, node_type: String) -> Self {
        Self {
            id,
            name,
            node_type,
            children: None,
            sprite: None,
            animator_controller: None,
            components: Vec::new(),
        }
    }
}

/// Parse a prefab file and return the hierarchy
pub fn parse_hierarchy(prefab_path: &Path) -> Result<HierarchyNode, PrefabError> {
    if !prefab_path.exists() {
        return Err(PrefabError::NotFound(prefab_path.display().to_string()));
    }

    let content = fs::read_to_string(prefab_path)?;

    // Parse Unity YAML
    let docs = unity_yaml_rust::yaml::YamlLoader::load_from_str(&content)
        .map_err(|e| PrefabError::ParseError(e.to_string()))?;

    if docs.is_empty() {
        return Err(PrefabError::ParseError("Empty prefab file".into()));
    }

    // Build hierarchy from parsed elements
    let elements = parse_prefab_elements(&docs)?;
    let hierarchy = build_hierarchy(&elements)?;

    Ok(hierarchy)
}

#[derive(Debug)]
struct PrefabElement {
    id: i64,
    element_type: String,
    name: Option<String>,
    parent_id: Option<i64>,
    #[allow(dead_code)]
    children_ids: Vec<i64>,
    #[allow(dead_code)]
    sprite_guid: Option<String>,
    #[allow(dead_code)]
    controller_guid: Option<String>,
}

fn parse_prefab_elements(
    docs: &[Yaml],
) -> Result<Vec<PrefabElement>, PrefabError> {
    let mut elements = Vec::new();
    let mut game_objects: HashMap<i64, String> = HashMap::new();
    let mut transforms: HashMap<i64, (Option<i64>, Vec<i64>, i64)> = HashMap::new(); // id -> (parent, children, gameobject)

    for doc in docs {
        // This is a simplified parser - full implementation would parse all Unity YAML structures
        if let Yaml::Hash(ref hash) = doc {
            for (key, value) in hash.iter() {
                let key_str = match key {
                    Yaml::String(s) => s.as_str(),
                    _ => continue,
                };

                match key_str {
                    "GameObject" => {
                        if let Some(id) = extract_file_id(doc) {
                            let name = match &value["m_Name"] {
                                Yaml::String(s) => s.clone(),
                                _ => "Unnamed".to_string(),
                            };
                            game_objects.insert(id, name);
                        }
                    }
                    "Transform" | "RectTransform" => {
                        if let Some(id) = extract_file_id(doc) {
                            let parent_id = match &value["m_Father"]["fileID"] {
                                Yaml::Integer(i) => Some(*i),
                                _ => None,
                            };
                            let go_id = match &value["m_GameObject"]["fileID"] {
                                Yaml::Integer(i) => *i,
                                _ => 0,
                            };

                            let children: Vec<i64> = if let Yaml::Array(ref children_arr) = value["m_Children"] {
                                children_arr
                                    .iter()
                                    .filter_map(|c| match &c["fileID"] {
                                        Yaml::Integer(i) => Some(*i),
                                        _ => None,
                                    })
                                    .collect()
                            } else {
                                Vec::new()
                            };

                            transforms.insert(id, (parent_id, children, go_id));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Build element list
    for (transform_id, (parent_id, children, go_id)) in &transforms {
        let name = game_objects.get(go_id).cloned().unwrap_or_else(|| "Unknown".to_string());

        elements.push(PrefabElement {
            id: *transform_id,
            element_type: "GameObject".to_string(),
            name: Some(name),
            parent_id: *parent_id,
            children_ids: children.clone(),
            sprite_guid: None,
            controller_guid: None,
        });
    }

    Ok(elements)
}

fn extract_file_id(_yaml: &Yaml) -> Option<i64> {
    // Unity YAML has file IDs in document headers
    // This is a simplified extraction - returns None for now
    // Full implementation would parse the --- !u!X &Y header
    None
}

fn build_hierarchy(
    elements: &[PrefabElement],
) -> Result<HierarchyNode, PrefabError> {
    // Find root elements (no parent or parent is 0)
    let roots: Vec<&PrefabElement> = elements
        .iter()
        .filter(|e| e.parent_id.is_none() || e.parent_id == Some(0))
        .collect();

    if roots.is_empty() {
        // Return empty root
        return Ok(HierarchyNode::new(0, "Root".to_string(), "Root".to_string()));
    }

    // Build tree recursively
    fn build_node(
        element: &PrefabElement,
        all_elements: &[PrefabElement],
    ) -> HierarchyNode {
        let mut node = HierarchyNode::new(
            element.id,
            element.name.clone().unwrap_or_else(|| "Unknown".to_string()),
            element.element_type.clone(),
        );

        // Find children
        let children: Vec<HierarchyNode> = all_elements
            .iter()
            .filter(|e| e.parent_id == Some(element.id))
            .map(|e| build_node(e, all_elements))
            .collect();

        if !children.is_empty() {
            node.children = Some(children);
        }

        node
    }

    let root = roots.first().unwrap();
    Ok(build_node(root, elements))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_node_creation() {
        let node = HierarchyNode::new(1, "Test".to_string(), "GameObject".to_string());
        assert_eq!(node.id, 1);
        assert_eq!(node.name, "Test");
        assert!(node.children.is_none());
    }
}
