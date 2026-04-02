//! Progress component with animated indicator

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub struct ProgressBar {
    current: u64,
    total: u64,
    label: String,
    style: Style,
}

impl ProgressBar {
    pub fn new(total: u64, label: impl Into<String>) -> Self {
        Self {
            current: 0,
            total,
            label: label.into(),
            style: Style::default().fg(Color::Cyan),
        }
    }

    pub fn set_current(&mut self, current: u64) {
        self.current = (current).min(self.total);
    }

    pub fn increment(&mut self) {
        self.set_current(self.current + 1);
    }

    pub fn percentage(&self) -> u16 {
        if self.total == 0 {
            0
        } else {
            ((self.current * 100) / self.total) as u16
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(Block::default()
                .borders(Borders::ALL)
                .title(self.label.as_str()))
            .gauge_style(self.style)
            .percent(self.percentage())
            .label(format!("{}%", self.percentage()));

        frame.render_widget(gauge, area);
    }

    pub fn render_simple(&self, frame: &mut Frame, area: Rect) {
        let percent = self.percentage();
        let bar_width = area.width.saturating_sub(10) as usize;
        let filled = (bar_width * percent as usize) / 100;

        let bar = format!(
            "[{}{}] {}%",
            "█".repeat(filled),
            "░".repeat(bar_width - filled),
            percent
        );

        let paragraph = Paragraph::new(bar)
            .style(self.style)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(self.label.as_str()));

        frame.render_widget(paragraph, area);
    }
}

pub struct MultiProgress {
    tasks: Vec<(String, u16)>,
}

impl MultiProgress {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add_task(&mut self, name: impl Into<String>, percent: u16) {
        self.tasks.push((name.into(), percent.min(100)));
    }

    pub fn update_task(&mut self, index: usize, percent: u16) {
        if let Some(task) = self.tasks.get_mut(index) {
            task.1 = percent.min(100);
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style) {
        if self.tasks.is_empty() {
            return;
        }

        let item_height = area.height as usize / self.tasks.len().max(1);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(item_height as u16); self.tasks.len()])
            .split(area);

        for (i, (name, percent)) in self.tasks.iter().enumerate() {
            let bar_width = chunks[i].width.saturating_sub(15) as usize;
            let filled = (bar_width * *percent as usize) / 100;
            let bar = format!(
                "{:<12} [{}{}] {}%",
                name,
                "█".repeat(filled),
                "░".repeat(bar_width - filled),
                percent
            );

            let text = Span::styled(bar, style);
            let p = Paragraph::new(text);
            frame.render_widget(p, chunks[i]);
        }
    }
}

impl Default for MultiProgress {
    fn default() -> Self {
        Self::new()
    }
}
