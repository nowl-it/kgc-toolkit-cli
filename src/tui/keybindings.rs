//! Keyboard shortcuts and help system

use std::collections::HashMap;
use ratatui::text::{Line, Span};
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Keybinding {
    pub key: String,
    pub description: String,
    pub context: &'static str, // "Global", "Download", "Proxy", etc.
}

pub struct KeybindingsHelp {
    bindings: HashMap<String, Vec<Keybinding>>,
}

impl KeybindingsHelp {
    pub fn new() -> Self {
        let mut bindings = HashMap::new();

        // Global bindings
        bindings.insert(
            "Global".to_string(),
            vec![
                Keybinding {
                    key: "Tab / Shift+Tab".to_string(),
                    description: "Switch between tabs".to_string(),
                    context: "Global",
                },
                Keybinding {
                    key: "Ctrl+Q / Ctrl+C".to_string(),
                    description: "Quit application".to_string(),
                    context: "Global",
                },
                Keybinding {
                    key: "?".to_string(),
                    description: "Show help".to_string(),
                    context: "Global",
                },
                Keybinding {
                    key: "Ctrl+L".to_string(),
                    description: "Toggle light/dark theme".to_string(),
                    context: "Global",
                },
            ],
        );

        // Download tab
        bindings.insert(
            "Download".to_string(),
            vec![
                Keybinding {
                    key: "↑ / k".to_string(),
                    description: "Move up".to_string(),
                    context: "Download",
                },
                Keybinding {
                    key: "↓ / j".to_string(),
                    description: "Move down".to_string(),
                    context: "Download",
                },
                Keybinding {
                    key: "Enter".to_string(),
                    description: "Download selected version".to_string(),
                    context: "Download",
                },
                Keybinding {
                    key: "r".to_string(),
                    description: "Refresh version list".to_string(),
                    context: "Download",
                },
                Keybinding {
                    key: "f".to_string(),
                    description: "Edit custom output path".to_string(),
                    context: "Download",
                },
            ],
        );

        // Convert tab
        bindings.insert(
            "Convert".to_string(),
            vec![
                Keybinding {
                    key: "↑ / k".to_string(),
                    description: "Move up in XAPK list".to_string(),
                    context: "Convert",
                },
                Keybinding {
                    key: "↓ / j".to_string(),
                    description: "Move down in XAPK list".to_string(),
                    context: "Convert",
                },
                Keybinding {
                    key: "r".to_string(),
                    description: "Load XAPK from source path".to_string(),
                    context: "Convert",
                },
                Keybinding {
                    key: "f".to_string(),
                    description: "Edit C2U output path".to_string(),
                    context: "Convert",
                },
                Keybinding {
                    key: "s".to_string(),
                    description: "Edit source path (file/folder)".to_string(),
                    context: "Convert",
                },
                Keybinding {
                    key: "Enter".to_string(),
                    description: "Convert selected XAPK".to_string(),
                    context: "Convert",
                },
            ],
        );

        // Proxy tab
        bindings.insert(
            "Proxy".to_string(),
            vec![
                Keybinding {
                    key: "s".to_string(),
                    description: "Start/Stop proxy".to_string(),
                    context: "Proxy",
                },
                Keybinding {
                    key: "c".to_string(),
                    description: "Clear captured traffic".to_string(),
                    context: "Proxy",
                },
                Keybinding {
                    key: "e".to_string(),
                    description: "Export traffic to JSON".to_string(),
                    context: "Proxy",
                },
                Keybinding {
                    key: "↑ / k".to_string(),
                    description: "Select previous request".to_string(),
                    context: "Proxy",
                },
                Keybinding {
                    key: "↓ / j".to_string(),
                    description: "Select next request".to_string(),
                    context: "Proxy",
                },
                Keybinding {
                    key: "Enter".to_string(),
                    description: "Decrypt & view request".to_string(),
                    context: "Proxy",
                },
            ],
        );

        // Dashboard tab
        bindings.insert(
            "Dashboard".to_string(),
            vec![
                Keybinding {
                    key: "r".to_string(),
                    description: "Check tools status".to_string(),
                    context: "Dashboard",
                },
                Keybinding {
                    key: "d".to_string(),
                    description: "Switch to Download tab".to_string(),
                    context: "Dashboard",
                },
                Keybinding {
                    key: "c".to_string(),
                    description: "Switch to Convert tab".to_string(),
                    context: "Dashboard",
                },
                Keybinding {
                    key: "p".to_string(),
                    description: "Switch to Proxy tab".to_string(),
                    context: "Dashboard",
                },
            ],
        );

        // Config tab
        bindings.insert(
            "Config".to_string(),
            vec![
                Keybinding {
                    key: "↑ / k".to_string(),
                    description: "Move up in settings".to_string(),
                    context: "Config",
                },
                Keybinding {
                    key: "↓ / j".to_string(),
                    description: "Move down in settings".to_string(),
                    context: "Config",
                },
                Keybinding {
                    key: "Enter / e".to_string(),
                    description: "Edit selected setting".to_string(),
                    context: "Config",
                },
                Keybinding {
                    key: "s".to_string(),
                    description: "Save settings to config file".to_string(),
                    context: "Config",
                },
                Keybinding {
                    key: "l".to_string(),
                    description: "Load settings from config file".to_string(),
                    context: "Config",
                },
                Keybinding {
                    key: "d".to_string(),
                    description: "Reset settings to default".to_string(),
                    context: "Config",
                },
            ],
        );

        // XML tab
        bindings.insert(
            "XML".to_string(),
            vec![
                Keybinding {
                    key: "f".to_string(),
                    description: "Edit XML output path".to_string(),
                    context: "XML",
                },
                Keybinding {
                    key: "R".to_string(),
                    description: "Load remote options (platform/time)".to_string(),
                    context: "XML",
                },
                Keybinding {
                    key: "r".to_string(),
                    description: "Scan output folder".to_string(),
                    context: "XML",
                },
                Keybinding {
                    key: "h / l".to_string(),
                    description: "Switch platform".to_string(),
                    context: "XML",
                },
                Keybinding {
                    key: "↓ / j".to_string(),
                    description: "Select time".to_string(),
                    context: "XML",
                },
                Keybinding {
                    key: "d".to_string(),
                    description: "Download selected platform/time XML".to_string(),
                    context: "XML",
                },
                Keybinding {
                    key: "x".to_string(),
                    description: "Extract selected bundle to XML".to_string(),
                    context: "XML",
                },
            ],
        );

        Self { bindings }
    }

    pub fn get_help_lines(&self, context: &str) -> Vec<Line> {
        let mut lines = vec![];

        lines.push(Line::from(vec![
            Span::styled("KGC Toolkit - Keyboard Shortcuts", 
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        ]));
        lines.push(Line::from(""));

        // Global bindings
        if let Some(global_binds) = self.bindings.get("Global") {
            lines.push(Line::from(Span::styled(
                "Global Shortcuts",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            for bind in global_binds {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<15}", bind.key), 
                        Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" {}", bind.description)),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Context-specific bindings
        if context != "Global" {
            if let Some(ctx_binds) = self.bindings.get(context) {
                lines.push(Line::from(Span::styled(
                    format!("{} Tab Shortcuts", context),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
                for bind in ctx_binds {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {:<15}", bind.key), 
                            Style::default().fg(Color::Yellow)),
                        Span::raw(format!(" {}", bind.description)),
                    ]));
                }
            }
        }

        lines
    }
}

impl Default for KeybindingsHelp {
    fn default() -> Self {
        Self::new()
    }
}
