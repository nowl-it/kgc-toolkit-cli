//! Advanced UI components

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Instant;

/// Animated spinner for loading states
pub struct Spinner {
    frames: &'static [&'static str],
    current_frame: usize,
    last_update: Instant,
    frame_duration_ms: u64,
}

impl Spinner {
    pub fn new() -> Self {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        Self {
            frames: FRAMES,
            current_frame: 0,
            last_update: Instant::now(),
            frame_duration_ms: 80,
        }
    }

    pub fn current(&mut self) -> &'static str {
        let elapsed = self.last_update.elapsed().as_millis() as u64;
        if elapsed >= self.frame_duration_ms {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
        self.frames[self.current_frame]
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, style: Style) {
        let text = Span::styled(self.current(), style);
        let paragraph = Paragraph::new(text);
        frame.render_widget(paragraph, area);
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress gauge with visual indicator
pub struct ProgressGauge {
    percent: u32,
    label: String,
}

impl ProgressGauge {
    pub fn new(percent: u32, label: impl Into<String>) -> Self {
        Self {
            percent: percent.min(100),
            label: label.into(),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style) {
        if area.width < 20 {
            return;
        }

        let width = (area.width as u32 * self.percent / 100) as u16;
        let bar = "█".repeat(width as usize) + &"░".repeat((area.width - width) as usize);

        let text = format!("{} {}%", bar, self.percent);
        let paragraph = Paragraph::new(text)
            .style(style)
            .block(Block::default().borders(Borders::ALL).title(self.label.as_str()));

        frame.render_widget(paragraph, area);
    }
}

/// Status indicator with icon and color
pub struct StatusIndicator {
    status: StatusType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusType {
    Running,
    Success,
    Error,
    Warning,
    Idle,
}

impl StatusIndicator {
    pub fn new(status: StatusType) -> Self {
        Self { status }
    }

    pub fn icon(&self) -> &'static str {
        match self.status {
            StatusType::Running => "⟳",
            StatusType::Success => "✓",
            StatusType::Error => "✗",
            StatusType::Warning => "⚠",
            StatusType::Idle => "○",
        }
    }

    pub fn color(&self) -> Color {
        match self.status {
            StatusType::Running => Color::Cyan,
            StatusType::Success => Color::Green,
            StatusType::Error => Color::Red,
            StatusType::Warning => Color::Yellow,
            StatusType::Idle => Color::Gray,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let span = Span::styled(
            format!(" {} ", self.icon()),
            Style::default().fg(self.color()),
        );
        let paragraph = Paragraph::new(span);
        frame.render_widget(paragraph, area);
    }
}

/// Vertical bar chart for small displays
pub struct MiniChart {
    data: Vec<u32>,
    max_value: u32,
    width: u16,
}

impl MiniChart {
    pub fn new(data: Vec<u32>) -> Self {
        let max_value = *data.iter().max().unwrap_or(&1).max(&1);
        Self {
            data,
            max_value,
            width: 0,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, style: Style) {
        if area.height < 3 {
            return;
        }

        let bars: Vec<String> = self.data.iter().map(|&v| {
            let height = ((v as u16 * (area.height - 2)) / self.max_value as u16).min(area.height - 2);
            "█".repeat(height as usize)
        }).collect();

        let text = bars.join(" ");
        let paragraph = Paragraph::new(text).style(style);
        frame.render_widget(paragraph, area);
    }
}

/// Text with type indication (error, warning, info, success)
pub struct TypedMessage {
    message_type: MessageType,
    text: String,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    Error,
    Warning,
    Info,
    Success,
}

impl TypedMessage {
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Error,
            text: text.into(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Warning,
            text: text.into(),
        }
    }

    pub fn info(text: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Info,
            text: text.into(),
        }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Success,
            text: text.into(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self.message_type {
            MessageType::Error => "✗",
            MessageType::Warning => "⚠",
            MessageType::Info => "ℹ",
            MessageType::Success => "✓",
        }
    }

    pub fn color(&self) -> Color {
        match self.message_type {
            MessageType::Error => Color::Red,
            MessageType::Warning => Color::Yellow,
            MessageType::Info => Color::Blue,
            MessageType::Success => Color::Green,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let text = format!("{} {}", self.icon(), self.text);
        let span = Span::styled(text, Style::default().fg(self.color()));
        let paragraph = Paragraph::new(span);
        frame.render_widget(paragraph, area);
    }
}
