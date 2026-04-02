//! TUI UI rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
    Frame,
};

use super::app::{App, PathEditTarget, Tab, XmlStepState};
use crate::core::kgc;

pub fn draw(frame: &mut Frame, app: &App) {
    if app.show_help {
        draw_help_modal(frame, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header + tabs
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Status bar
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.active_tab {
                app.theme.active_tab_style()
            } else {
                app.theme.inactive_tab_style()
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(format!(" KGC Toolkit - {} ", kgc::GAME_NAME)))
        .highlight_style(app.theme.active_tab_style())
        .select(Tab::all().iter().position(|t| *t == app.active_tab).unwrap_or(0));

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_tab {
        Tab::Dashboard => draw_dashboard(frame, app, area),
        Tab::Download => draw_download(frame, app, area),
        Tab::Convert => draw_convert(frame, app, area),
        Tab::Compare => draw_compare(frame, app, area),
        Tab::Proxy => draw_proxy(frame, app, area),
        Tab::Unity => draw_unity(frame, app, area),
        Tab::Xml => draw_xml(frame, app, area),
        Tab::Config => draw_config(frame, app, area),
    }
}

fn draw_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    // Tools list with better formatting
    let tools_items: Vec<ListItem> = if app.tools_status.is_empty() {
        vec![
            ListItem::new(Span::styled("  No data yet", app.theme.muted_style())),
            ListItem::new(""),
            ListItem::new("  Press 'r' to check tools status"),
        ]
    } else {
        app.tools_status.iter().map(|(name, available)| {
            let style = if *available {
                app.theme.success_style()
            } else {
                app.theme.error_style()
            };
            let icon = if *available { "✓" } else { "✗" };
            ListItem::new(format!("  {} {}", icon, name)).style(style)
        }).collect()
    };

    let tools_list = List::new(tools_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(Span::styled(" 📦 Tools Status ", app.theme.title_style())));

    frame.render_widget(tools_list, chunks[0]);

    // Quick actions with improved layout
    let actions = vec![
        ListItem::new("  [?] Help       [r] Refresh Tools"),
        ListItem::new("  [Tab] Next Tab [Ctrl+Q] Quit"),
        ListItem::new(""),
        ListItem::new(Span::styled("  Hot Keys:", app.theme.title_style())),
        ListItem::new("  [D] Download  [C] Convert  [P] Proxy"),
    ];

    let actions_list = List::new(actions)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Quick Actions "));

    frame.render_widget(actions_list, chunks[1]);
}

fn draw_download(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    // Package info + output folder
    let info_text = vec![
        Line::from(vec![
            Span::raw("Package: "),
            Span::styled(kgc::PACKAGE_ID, app.theme.code_style()),
        ]),
        Line::from(vec![
            Span::raw("Game: "),
            Span::styled(kgc::GAME_NAME, app.theme.primary_style()),
        ]),
        Line::from(vec![
            Span::raw("Output: "),
            Span::styled(
                if app.editing_path_target == Some(PathEditTarget::DownloadOutput) {
                    format!("> {}", app.editing_path_value)
                } else {
                    app.output_folder_input.clone()
                },
                if app.editing_path_target == Some(PathEditTarget::DownloadOutput) {
                    app.theme.primary_style()
                } else {
                    app.theme.success_style()
                }
            ),
        ]),
    ];
    let info = Paragraph::new(info_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Download XAPK "));
    frame.render_widget(info, chunks[0]);

    // Versions list with selection indicator
    let versions: Vec<ListItem> = if app.available_versions.is_empty() {
        vec![ListItem::new(Span::styled(
            "  Loading versions... (press 'r' to retry)",
            app.theme.muted_style(),
        ))]
    } else {
        app.available_versions.iter().enumerate().map(|(i, v)| {
            let style = if i == app.selected_version_idx {
                app.theme.selected_style()
            } else {
                app.theme.normal_style()
            };
            let prefix = if i == app.selected_version_idx { "→" } else { " " };
            ListItem::new(format!("{}  {}", prefix, v)).style(style)
        }).collect()
    };

    let versions_list = List::new(versions)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Available Versions "));
    frame.render_widget(versions_list, chunks[1]);

    // Progress or Help
    let help_text = if let Some((progress, msg)) = &app.download_progress {
        format!("⟳ Downloading: {}% - {}", progress, msg)
    } else if app.editing_path_target == Some(PathEditTarget::DownloadOutput) {
        "Editing Download output path: type, [Backspace], [Enter]=Save, [Esc]=Cancel".to_string()
    } else {
        "[↑↓] Select  [f] Edit output path  [Enter] Download  [r] Refresh versions".to_string()
    };
    let help_widget = Paragraph::new(help_text)
        .style(app.theme.muted_style())
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border)));
    frame.render_widget(help_widget, chunks[2]);
}

fn draw_convert(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    let info_text = vec![
        Line::from(vec![
            Span::raw("Source: "),
            Span::styled(
                if app.editing_path_target == Some(PathEditTarget::C2uSource) {
                    format!("> {}", app.editing_path_value)
                } else {
                    app.c2u_source_path_input.clone()
                },
                if app.editing_path_target == Some(PathEditTarget::C2uSource) {
                    app.theme.primary_style()
                } else {
                    app.theme.code_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("Output: "),
            Span::styled(
                if app.editing_path_target == Some(PathEditTarget::C2uOutput) {
                    format!("> {}", app.editing_path_value)
                } else {
                    app.c2u_output_folder_input.clone()
                },
                if app.editing_path_target == Some(PathEditTarget::C2uOutput) {
                    app.theme.primary_style()
                } else {
                    app.theme.success_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("Selected: "),
            Span::styled(
                app.available_xapks
                    .get(app.selected_xapk_idx)
                    .cloned()
                    .unwrap_or_else(|| "(none)".to_string()),
                app.theme.primary_style(),
            ),
        ]),
    ];

    let info = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(app.theme.border))
                .title(" Convert (C2U) "),
        );
    frame.render_widget(info, chunks[0]);

    let file_items: Vec<ListItem> = if app.available_xapks.is_empty() {
        if app.c2u_show_logs {
            if app.c2u_logs.is_empty() {
                vec![ListItem::new(Span::styled(
                    "  No logs yet",
                    app.theme.muted_style(),
                ))]
            } else {
                app.c2u_logs
                    .iter()
                    .map(|line| ListItem::new(format!("  {}", line)).style(app.theme.normal_style()))
                    .collect()
            }
        } else {
            vec![ListItem::new(Span::styled(
                "  No XAPK files found (press 'r' to load from Source)",
                app.theme.muted_style(),
            ))]
        }
    } else {
        if app.c2u_show_logs {
            app.c2u_logs
                .iter()
                .map(|line| ListItem::new(format!("  {}", line)).style(app.theme.normal_style()))
                .collect()
        } else {
            app.available_xapks
                .iter()
                .enumerate()
                .map(|(i, file)| {
                    let style = if i == app.selected_xapk_idx {
                        app.theme.selected_style()
                    } else {
                        app.theme.normal_style()
                    };
                    let prefix = if i == app.selected_xapk_idx { "→" } else { " " };
                    ListItem::new(format!("{}  {}", prefix, file)).style(style)
                })
                .collect()
        }
    };

    let files = List::new(file_items).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(if app.c2u_show_logs { " C2U Logs " } else { " XAPK Files " }),
    );
    frame.render_widget(files, chunks[1]);

    let footer = if let Some((progress, msg)) = &app.c2u_progress {
        format!("⟳ Converting: {}% - {}", progress, msg)
    } else if app.editing_path_target.is_some() {
        "Editing path: type, [Backspace], [Enter]=Save, [Esc]=Cancel".to_string()
    } else {
        "[↑↓] Select file  [s] Edit source(file/folder)  [r] Load XAPK  [f] Edit output  [Enter] Convert".to_string()
    };

    let footer_widget = Paragraph::new(footer)
        .style(app.theme.muted_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(app.theme.border)),
        );
    frame.render_widget(footer_widget, chunks[2]);
}

fn draw_compare(frame: &mut Frame, app: &App, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("📊 Compare Two Unity Projects", app.theme.title_style())),
        Line::from(""),
        Line::from("  Find added, removed, and modified files between"),
        Line::from("  two versions of the Unity project."),
        Line::from(""),
        Line::from(Span::styled("Usage in CLI:", app.theme.title_style())),
        Line::from(Span::styled("  kgc compare <old_path> <new_path>", app.theme.code_style())),
        Line::from(""),
        Line::from(Span::styled("Options:", app.theme.title_style())),
        Line::from("  --filter <pattern>  Filter results by pattern"),
        Line::from("  --output <file>     Save diff to JSON file"),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Compare "));
    frame.render_widget(paragraph, area);
}

fn draw_proxy(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    // Status section with better styling
    let status_style = if app.proxy_running {
        app.theme.success_style()
    } else {
        app.theme.error_style()
    };

    let status_icon = if app.proxy_running { "▶" } else { "⏸" };
    let status_text = vec![
        Line::from(vec![
            Span::styled(format!("{} Status: ", status_icon), status_style),
            Span::styled(
                if app.proxy_running { "RUNNING" } else { "STOPPED" },
                status_style,
            ),
        ]),
        Line::from(""),
        Line::from("  [s] Start/Stop proxy    [c] Clear traffic"),
        Line::from("  [e] Export to JSON       [↑↓] Navigate"),
    ];

    let status = Paragraph::new(status_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" MITM Proxy "));
    frame.render_widget(status, chunks[0]);

    // Captured requests
    let requests: Vec<ListItem> = if app.captured_requests.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No captured requests yet",
            app.theme.muted_style()
        ))]
    } else {
        app.captured_requests.iter().map(|r| {
            ListItem::new(format!("  {}", r)).style(app.theme.normal_style())
        }).collect()
    };

    let requests_list = List::new(requests)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Captured Traffic "));
    frame.render_widget(requests_list, chunks[1]);
}

fn draw_unity(frame: &mut Frame, app: &App, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("🎮 Unity Asset Tools", app.theme.title_style())),
        Line::from(""),
        Line::from("  • Parse prefab hierarchy"),
        Line::from("  • List heroes in project"),
        Line::from("  • Export hero assets"),
        Line::from(""),
        Line::from(Span::styled("Usage in CLI:", app.theme.title_style())),
        Line::from(Span::styled("  kgc unity parse-prefab <path>", app.theme.code_style())),
        Line::from(Span::styled("  kgc unity list-heroes <project>", app.theme.code_style())),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Unity Tools "));
    frame.render_widget(paragraph, area);
}

fn draw_xml(frame: &mut Frame, app: &App, area: Rect) {
    #[derive(Clone)]
    enum XmlTreeNode {
        Platform(String),
        Time {
            platform: String,
            time: String,
            count: usize,
        },
    }

    let selected_count = app.xml_selected_times.len();
    let selected_preview_limit = 3usize;
    let mut selected_preview: Vec<String> = app
        .xml_selected_times
        .iter()
        .take(selected_preview_limit)
        .map(|k| {
            if let Some((platform, time)) = k.split_once("::") {
                format!("{}/{}", platform, time)
            } else {
                k.clone()
            }
        })
        .collect();
    if selected_preview.is_empty() {
        selected_preview.push("(none)".to_string());
    }
    let selected_summary = if selected_count > selected_preview_limit {
        format!(
            "{} (+{} more)",
            selected_preview.join(", "),
            selected_count - selected_preview_limit
        )
    } else {
        selected_preview.join(", ")
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Output: "),
            Span::styled(
                if app.editing_path_target == Some(PathEditTarget::XmlOutput) {
                    format!("> {}", app.editing_path_value)
                } else {
                    app.xml_output_folder_input.clone()
                },
                if app.editing_path_target == Some(PathEditTarget::XmlOutput) {
                    app.theme.primary_style()
                } else {
                    app.theme.success_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("Bundle: "),
            Span::styled(
                app.xml_bundles
                    .get(app.selected_xml_bundle_idx)
                    .cloned()
                    .unwrap_or_else(|| "(none)".to_string()),
                app.theme.code_style(),
            ),
        ]),
        Line::from(vec![
            Span::raw("Selected times: "),
            Span::styled(selected_count.to_string(), app.theme.primary_style()),
            Span::raw("  |  "),
            Span::styled(selected_summary, app.theme.code_style()),
        ]),
        Line::from(vec![
            Span::raw("Step:   "),
            Span::styled(
                match app.xml_download_step.step {
                    XmlStepState::Idle => "idle",
                    XmlStepState::Scanning => "scanning",
                    XmlStepState::Downloading => "downloading",
                    XmlStepState::Extracting => "extracting",
                    XmlStepState::Complete => "complete",
                    XmlStepState::Error => "error",
                },
                match app.xml_download_step.step {
                    XmlStepState::Complete => app.theme.success_style(),
                    XmlStepState::Error => app.theme.error_style(),
                    _ => app.theme.primary_style(),
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("Base:   "),
            Span::styled(
                app.xml_detected_base_url
                    .clone()
                    .unwrap_or_else(|| "(not detected yet)".to_string()),
                app.theme.code_style(),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" XML Downloader "),
    );
    frame.render_widget(info, chunks[0]);

    let lists = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    let remote_items: Vec<ListItem> = if app.xml_remote_options.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No remote options loaded (press 'R')",
            app.theme.muted_style(),
        ))]
    } else {
        let mut platforms: Vec<String> = app
            .xml_remote_options
            .iter()
            .map(|o| o.platform.clone())
            .collect();
        platforms.sort();
        platforms.dedup();

        let mut nodes: Vec<XmlTreeNode> = Vec::new();
        for platform in platforms {
            nodes.push(XmlTreeNode::Platform(platform.clone()));
            if app.xml_expanded_platforms.iter().any(|p| p == &platform) {
                let mut times: Vec<(String, usize)> = app
                    .xml_remote_options
                    .iter()
                    .filter(|o| o.platform == platform)
                    .map(|o| (o.time.clone(), o.file_urls.len()))
                    .collect();
                times.sort_by(|a, b| a.0.cmp(&b.0));
                times.dedup_by(|a, b| a.0 == b.0);

                for (time, count) in times {
                    nodes.push(XmlTreeNode::Time {
                        platform: platform.clone(),
                        time,
                        count,
                    });
                }
            }
        }

        let view_height = usize::from(lists[0].height.saturating_sub(2)).max(1);
        let start = app.xml_tree_scroll.min(nodes.len().saturating_sub(1));
        let end = (start + view_height).min(nodes.len());

        let mut lines: Vec<ListItem> = Vec::new();
        for (idx, node) in nodes.iter().enumerate().take(end).skip(start) {
            let is_cursor = idx == app.xml_tree_cursor;
            match node {
                XmlTreeNode::Platform(platform) => {
                    let expanded = app.xml_expanded_platforms.iter().any(|p| p == platform);
                    let marker = if expanded { "v" } else { ">" };
                    let line = format!("{} {}", marker, platform);
                    let style = if is_cursor {
                        app.theme.selected_style()
                    } else {
                        app.theme.normal_style()
                    };
                    lines.push(ListItem::new(format!("  {}", line)).style(style));
                }
                XmlTreeNode::Time {
                    platform,
                    time,
                    count,
                } => {
                    let key = format!("{}::{}", platform, time);
                    let checked = if app.xml_selected_times.iter().any(|k| k == &key) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let line = format!("    {} {} ({} files)", checked, time, count);
                    let style = if is_cursor {
                        app.theme.selected_style()
                    } else {
                        app.theme.normal_style()
                    };
                    lines.push(ListItem::new(line).style(style));
                }
            }
        }

        if lines.is_empty() {
            lines.push(ListItem::new(Span::styled(
                "  No tree nodes available",
                app.theme.muted_style(),
            )));
        }

        lines
    };

    let remote = List::new(remote_items).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Remote Tree (Enter: expand/select) "),
    );
    frame.render_widget(remote, lists[0]);

    let xml_items: Vec<ListItem> = if app.xml_files.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No XML files",
            app.theme.muted_style(),
        ))]
    } else {
        app.xml_files
            .iter()
            .map(|path| ListItem::new(format!("  {}", path)).style(app.theme.normal_style()))
            .collect()
    };

    let xmls = List::new(xml_items).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" XML Files "),
    );
    frame.render_widget(xmls, lists[1]);

    let footer = if app.editing_path_target == Some(PathEditTarget::XmlOutput) {
        "Editing XML output: type, [Backspace], [Enter]=Save, [Esc]=Cancel".to_string()
    } else if let Some(msg) = &app.xml_download_step.message {
        if let Some(progress) = app.xml_download_step.progress {
            format!("⟳ [{}%] {}", progress, msg)
        } else {
            format!("⟳ {}", msg)
        }
    } else {
        "[R] Load options  [↑↓] Move  [Enter] Expand/select time  [d] Download selected times  [r] Scan local  [x] Extract local  [f] Output config".to_string()
    };

    let footer_widget = Paragraph::new(footer)
        .style(app.theme.muted_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(app.theme.border)),
        );
    frame.render_widget(footer_widget, chunks[2]);
}

fn draw_config(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    let config_path = crate::config::Settings::config_path();
    let info_text = vec![
        Line::from(vec![
            Span::raw("Config file: "),
            Span::styled(config_path.display().to_string(), app.theme.code_style()),
        ]),
        Line::from(vec![
            Span::raw("Press [s] to save, [l] to reload from disk"),
        ]),
        Line::from(vec![Span::raw("Press [d] to reset defaults")]),
    ];

    let info = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Config "),
    );
    frame.render_widget(info, chunks[0]);

    let tools_value = if app.editing_path_target == Some(PathEditTarget::ConfigToolsDirectory) {
        format!("> {}", app.editing_path_value)
    } else {
        app.settings
            .tools_directory
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".to_string())
    };

    let output_value = if app.editing_path_target == Some(PathEditTarget::ConfigOutputDirectory) {
        format!("> {}", app.editing_path_value)
    } else {
        app.settings
            .output_directory
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".to_string())
    };

    let proxy_value = if app.editing_path_target == Some(PathEditTarget::ConfigProxyPort) {
        format!("> {}", app.editing_path_value)
    } else {
        app.settings.proxy_port.to_string()
    };

    let items_data = [
        ("Tools Directory", tools_value),
        ("Output Directory", output_value),
        ("Proxy Port", proxy_value),
    ];

    let items: Vec<ListItem> = items_data
        .iter()
        .enumerate()
        .map(|(idx, (name, value))| {
            let selected = idx == app.config_selected_idx;
            let style = if selected {
                app.theme.selected_style()
            } else {
                app.theme.normal_style()
            };
            let marker = if selected { "->" } else { "  " };
            ListItem::new(format!("{} {:<18} {}", marker, name, value)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border))
            .title(" Settings "),
    );
    frame.render_widget(list, chunks[1]);

    let footer = if matches!(
        app.editing_path_target,
        Some(PathEditTarget::ConfigToolsDirectory)
            | Some(PathEditTarget::ConfigOutputDirectory)
            | Some(PathEditTarget::ConfigProxyPort)
    ) {
        "Editing config value: type, [Backspace], [Enter]=Save value, [Esc]=Cancel".to_string()
    } else {
        "[↑↓] Select  [Enter/e] Edit  [s] Save file  [l] Load file  [d] Reset default".to_string()
    };

    let footer_widget = Paragraph::new(footer)
        .style(app.theme.muted_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(app.theme.border)),
        );
    frame.render_widget(footer_widget, chunks[2]);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = app.status_message.as_deref().unwrap_or("Ready");

    let paragraph = Paragraph::new(format!("  {}", status))
        .style(app.theme.info_style())
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.border)));

    frame.render_widget(paragraph, area);
}

fn draw_help_modal(frame: &mut Frame, app: &App) {
    // Semi-transparent overlay
    let popup_area = centered_rect(80, 80, frame.area());

    // Help content
    let context = match app.active_tab {
        Tab::Convert => "Convert",
        _ => app.active_tab.title(),
    };
    let help_lines = app.keybindings_help.get_help_lines(context);

    let paragraph = Paragraph::new(help_lines)
        .style(app.theme.normal_style())
        .block(Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(app.theme.primary))
            .title(" Help - Press '?' or 'q' to close ")
            .title_alignment(ratatui::layout::Alignment::Center)
        )
        .scroll((0, 0));

    // Create a white background for the modal
    let bg = Block::default()
        .style(Style::default().bg(app.theme.bg));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(bg, popup_area);
    frame.render_widget(paragraph, popup_area);
}

/// Helper function to create centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
