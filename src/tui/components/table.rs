//! Table widget for structured data display

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Row, Table},
    Frame,
};

/// Data table for displaying structured information
pub struct DataTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    selected_row: usize,
    widths: Vec<u16>,
}

impl DataTable {
    pub fn new(headers: Vec<&str>) -> Self {
        let widths = headers.iter().map(|h| (h.len() as u16).max(10)).collect();
        Self {
            headers: headers.iter().map(|h| h.to_string()).collect(),
            rows: Vec::new(),
            selected_row: 0,
            widths,
        }
    }

    pub fn add_row(&mut self, row: Vec<&str>) {
        self.rows.push(row.iter().map(|s| s.to_string()).collect());
    }

    pub fn next_row(&mut self) {
        if self.selected_row < self.rows.len().saturating_sub(1) {
            self.selected_row += 1;
        }
    }

    pub fn prev_row(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn get_selected(&self) -> Option<&Vec<String>> {
        self.rows.get(self.selected_row)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, title: &str, style: Style) {
        let header_row = Row::new(self.headers.clone())
            .style(style.add_modifier(Modifier::BOLD));

        let rows = self.rows.iter().enumerate().map(|(i, row)| {
            let row_style = if i == self.selected_row {
                style.add_modifier(Modifier::BOLD).bg(Color::DarkGray)
            } else {
                style
            };
            Row::new(row.clone()).style(row_style)
        });

        let table = Table::new(rows, self.widths.iter().map(|w| ratatui::layout::Constraint::Length(*w)))
            .header(header_row)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title))
            .highlight_symbol("> ");

        frame.render_widget(table, area);
    }
}

/// Stat card for displaying key metrics
pub struct StatCard {
    label: String,
    value: String,
    unit: String,
}

impl StatCard {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            unit: String::new(),
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style) {
        let content = format!("{}\n{} {}", self.label, self.value, self.unit);
        let widget = ratatui::widgets::Paragraph::new(content)
            .style(style)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(widget, area);
    }
}

/// List item with icon and description
pub struct IconListItem {
    pub icon: String,
    pub title: String,
    pub description: String,
}

impl IconListItem {
    pub fn new(icon: impl Into<String>, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            title: title.into(),
            description: description.into(),
        }
    }

    pub fn render_span(&self, selected: bool, style: Style) -> Vec<Span> {
        let bg = if selected {
            Color::DarkGray
        } else {
            Color::Reset
        };

        vec![
            Span::styled(format!("{} ", self.icon), style.bg(bg)),
            Span::styled(format!("{:<20}", self.title), style.add_modifier(Modifier::BOLD).bg(bg)),
            Span::styled(format!("- {}", self.description), style.add_modifier(Modifier::DIM).bg(bg)),
        ]
    }
}
