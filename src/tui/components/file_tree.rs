//! File tree component for hierarchical file display

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub level: usize,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

impl TreeNode {
    pub fn new(name: impl Into<String>, path: PathBuf, is_dir: bool, level: usize) -> Self {
        Self {
            name: name.into(),
            path,
            is_dir,
            level,
            children: Vec::new(),
            expanded: level == 0,
        }
    }

    pub fn display_name(&self) -> String {
        let indent = "  ".repeat(self.level);
        let icon = if self.is_dir {
            if self.expanded { "📂" } else { "📁" }
        } else {
            "📄"
        };
        format!("{}{} {}", indent, icon, self.name)
    }

    pub fn flatten(&self) -> Vec<TreeNode> {
        let mut result = vec![self.clone()];
        if self.expanded && self.is_dir {
            for child in &self.children {
                result.extend(child.flatten());
            }
        }
        result
    }
}

pub struct FileTree {
    root: TreeNode,
    selected: usize,
}

impl FileTree {
    pub fn new(root_name: impl Into<String>, root_path: PathBuf) -> Self {
        Self {
            root: TreeNode::new(root_name, root_path, true, 0),
            selected: 0,
        }
    }

    pub fn add_child(&mut self, name: impl Into<String>, path: PathBuf, is_dir: bool) {
        self.root.children.push(TreeNode::new(
            name,
            path,
            is_dir,
            self.root.level + 1,
        ));
    }

    pub fn get_visible_nodes(&self) -> Vec<TreeNode> {
        self.root.flatten()
    }

    pub fn next(&mut self) {
        let nodes = self.get_visible_nodes();
        if self.selected < nodes.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn toggle_selected(&mut self) {
        let nodes = self.get_visible_nodes();
        if let Some(node) = nodes.get(self.selected) {
            if node.is_dir {
                // Find the node in root and toggle it
                let target_path = node.path.clone();
                toggle_node_in_tree(&mut self.root, &target_path);
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style) {
        let nodes = self.get_visible_nodes();
        let items: Vec<ListItem> = nodes.iter().enumerate().map(|(i, node)| {
            let item_style = if i == self.selected {
                style.add_modifier(Modifier::BOLD).bg(Color::DarkGray)
            } else {
                style
            };
            ListItem::new(node.display_name()).style(item_style)
        }).collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" File Tree "))
            .highlight_symbol("> ");

        frame.render_widget(list, area);
    }

    pub fn get_selected_path(&self) -> Option<PathBuf> {
        let nodes = self.get_visible_nodes();
        nodes.get(self.selected).map(|n| n.path.clone())
    }
}

/// Helper function to toggle node in tree by path
fn toggle_node_in_tree(node: &mut TreeNode, target_path: &PathBuf) {
    if node.path == *target_path {
        node.expanded = !node.expanded;
        return;
    }
    for child in &mut node.children {
        toggle_node_in_tree(child, target_path);
    }
}
