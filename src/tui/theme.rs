//! Theme system for TUI

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

pub struct Theme {
    pub mode: ThemeMode,
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub bg: Color,
    pub fg: Color,
    pub border: Color,
    pub highlight: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            primary: Color::Cyan,
            secondary: Color::Magenta,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            info: Color::Blue,
            bg: Color::Black,
            fg: Color::White,
            border: Color::Gray,
            highlight: Color::Cyan,
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            primary: Color::Blue,
            secondary: Color::Magenta,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            info: Color::Cyan,
            bg: Color::White,
            fg: Color::Black,
            border: Color::DarkGray,
            highlight: Color::Blue,
        }
    }

    /// Title style for sections
    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Active tab style
    pub fn active_tab_style(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(Modifier::BOLD)
    }

    /// Inactive tab style
    pub fn inactive_tab_style(&self) -> Style {
        Style::default().fg(Color::Gray)
    }

    /// Success/OK style
    pub fn success_style(&self) -> Style {
        Style::default()
            .fg(self.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Error style
    pub fn error_style(&self) -> Style {
        Style::default()
            .fg(self.error)
            .add_modifier(Modifier::BOLD)
    }

    /// Warning style
    pub fn warning_style(&self) -> Style {
        Style::default()
            .fg(self.warning)
            .add_modifier(Modifier::BOLD)
    }

    /// Selected item style
    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(Modifier::BOLD)
    }

    /// Normal item style
    pub fn normal_style(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Muted/secondary style
    pub fn muted_style(&self) -> Style {
        Style::default().fg(Color::Gray)
    }

    /// Code/monospace style
    pub fn code_style(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::DIM)
    }

    /// Primary text style
    pub fn primary_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Info/secondary text style
    pub fn info_style(&self) -> Style {
        Style::default().fg(self.info)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
