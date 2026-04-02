//! Traffic list component for displaying captured requests

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

#[derive(Clone, Debug)]
pub struct TrafficItem {
    pub id: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub timestamp: String,
    pub size: usize,
}

impl TrafficItem {
    pub fn new(id: impl Into<String>, method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            method: method.into(),
            url: url.into(),
            status: 200,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            size: 0,
        }
    }

    pub fn format_display(&self) -> String {
        let status_str = if self.status >= 400 {
            format!("✗ {}", self.status)
        } else {
            format!("✓ {}", self.status)
        };
        format!("  {} {} {} [{}] {}", self.method, self.url, status_str, self.timestamp, self.size)
    }
}

pub struct TrafficList {
    items: Vec<TrafficItem>,
    selected: usize,
}

impl TrafficList {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
        }
    }

    pub fn add_item(&mut self, item: TrafficItem) {
        self.items.push(item);
        if self.items.len() == 1 {
            self.selected = 0;
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected = 0;
    }

    pub fn next(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn get_selected(&self) -> Option<&TrafficItem> {
        self.items.get(self.selected)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, title: &str, style: Style) {
        let items: Vec<ListItem> = self.items.iter().enumerate().map(|(i, req)| {
            let item_style = if i == self.selected {
                style.bg(ratatui::style::Color::DarkGray)
            } else {
                style
            };
            ListItem::new(req.format_display()).style(item_style)
        }).collect();

        if items.is_empty() {
            let item = ListItem::new("  No captured traffic");
            let list = List::new(vec![item])
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(title));
            frame.render_widget(list, area);
        } else {
            let list = List::new(items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(title))
                .highlight_symbol("> ");
            frame.render_widget(list, area);
        }
    }
}

impl Default for TrafficList {
    fn default() -> Self {
        Self::new()
    }
}
