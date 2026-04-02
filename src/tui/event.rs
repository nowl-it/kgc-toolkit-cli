//! TUI event handling

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

use super::app::{App, AppMessage, PathEditTarget, Tab, XmlDownloadStep, XmlStepState};
use crate::config::Settings;
use crate::deps;
use crate::core::downloader;
use crate::proxy::server;

/// Poll for terminal events with timeout
pub fn poll_event() -> anyhow::Result<Option<Event>> {
    if event::poll(Duration::from_millis(100))? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Handle terminal events. Returns false if app should quit.
pub fn handle_event(app: &mut App, event: Event) -> anyhow::Result<bool> {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Resize(_, _) => Ok(true),
        _ => Ok(true),
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    // Global shortcuts
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return Ok(false);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return Ok(false);
        }
        KeyCode::Tab => {
            app.active_tab = app.active_tab.next();
            return Ok(true);
        }
        KeyCode::BackTab => {
            app.active_tab = app.active_tab.prev();
            return Ok(true);
        }
        KeyCode::Char('?') => {
            app.toggle_help();
            return Ok(true);
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_theme();
            app.set_status("Theme toggled");
            return Ok(true);
        }
        _ => {}
    }

    // Close help modal
    if app.show_help {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
                app.toggle_help();
                return Ok(true);
            }
            _ => {}
        }
        return Ok(true);
    }

    if app.editing_path_target.is_some() {
        return handle_path_edit_key(app, key);
    }

    // Tab-specific handling
    match app.active_tab {
        Tab::Dashboard => handle_dashboard_key(app, key),
        Tab::Download => handle_download_key(app, key),
        Tab::Convert => handle_convert_key(app, key),
        Tab::Proxy => handle_proxy_key(app, key),
        Tab::Xml => handle_xml_key(app, key),
        Tab::Config => handle_config_key(app, key),
        _ => Ok(true),
    }
}

fn handle_dashboard_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Char('d') => {
            app.active_tab = Tab::Download;
            app.set_status("Switched to Download tab");
        }
        KeyCode::Char('c') => {
            app.active_tab = Tab::Convert;
            app.set_status("Switched to Convert tab");
        }
        KeyCode::Char('p') => {
            app.active_tab = Tab::Proxy;
            app.set_status("Switched to Proxy tab");
        }
        KeyCode::Char('r') => {
            app.set_status("Refreshing tools status...");
            spawn_refresh_tools(app.message_tx.clone());
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        _ => {}
    }
    Ok(true)
}

fn handle_download_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_version_idx > 0 {
                app.selected_version_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_version_idx < app.available_versions.len().saturating_sub(1) {
                app.selected_version_idx += 1;
            }
        }
        KeyCode::Enter => {
            let version = app.available_versions.get(app.selected_version_idx).cloned();
            if let Some(version) = version {
                if app.output_folder.is_none() {
                    app.set_status("Please set output folder first (press 'f')");
                } else {
                    app.set_status("Starting download...");
                    let folder = app.output_folder.clone();
                    spawn_download(app.message_tx.clone(), version, folder);
                }
            }
        }
        KeyCode::Char('r') => {
            app.set_status("Fetching versions...");
            spawn_fetch_versions(app.message_tx.clone());
        }
        KeyCode::Char('f') => {
            start_path_edit(app, PathEditTarget::DownloadOutput);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        _ => {}
    }
    Ok(true)
}

fn handle_proxy_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Char('s') => {
            if app.proxy_running {
                app.set_status("Stopping proxy...");
                spawn_stop_proxy(app.message_tx.clone());
            } else {
                app.set_status("Starting proxy...");
                spawn_start_proxy(app.message_tx.clone());
            }
        }
        KeyCode::Char('c') => {
            app.captured_requests.clear();
            app.set_status("Cleared captured requests");
        }
        KeyCode::Char('r') => {
            app.set_status("Refreshing traffic...");
            spawn_refresh_traffic(app.message_tx.clone());
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        _ => {}
    }
    Ok(true)
}

fn handle_convert_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_xapk_idx > 0 {
                app.selected_xapk_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_xapk_idx < app.available_xapks.len().saturating_sub(1) {
                app.selected_xapk_idx += 1;
            }
        }
        KeyCode::Enter => {
            let selected_xapk = app.available_xapks.get(app.selected_xapk_idx).cloned();
            if let Some(xapk) = selected_xapk {
                let output_dir = app.c2u_output_folder.clone().unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                });
                app.set_status("Starting C2U conversion...");
                app.c2u_show_logs = true;
                spawn_c2u_convert(app.message_tx.clone(), std::path::PathBuf::from(xapk), output_dir);
            } else {
                app.set_status("No XAPK selected. Press 'r' to scan files.");
            }
        }
        KeyCode::Char('r') => {
            let source = app.c2u_source_path.clone();
            app.set_status(format!("Scanning XAPK from {}...", source.display()));
            spawn_scan_xapks(app.message_tx.clone(), source);
        }
        KeyCode::Char('f') => {
            start_path_edit(app, PathEditTarget::C2uOutput);
        }
        KeyCode::Char('s') => {
            start_path_edit(app, PathEditTarget::C2uSource);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        _ => {}
    }
    Ok(true)
}

fn handle_config_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.config_selected_idx > 0 {
                app.config_selected_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.config_selected_idx + 1 < App::config_item_count() {
                app.config_selected_idx += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            let target = match app.config_selected_idx {
                0 => PathEditTarget::ConfigToolsDirectory,
                1 => PathEditTarget::ConfigOutputDirectory,
                _ => PathEditTarget::ConfigProxyPort,
            };
            start_path_edit(app, target);
        }
        KeyCode::Char('s') => {
            match app.settings.save() {
                Ok(_) => app.set_status(format!(
                    "Settings saved: {}",
                    Settings::config_path().display()
                )),
                Err(e) => app.set_status(format!("Failed to save settings: {}", e)),
            }
        }
        KeyCode::Char('l') => {
            app.settings = Settings::load();

            app.output_folder = app.settings.output_directory.clone();
            app.c2u_output_folder = app.settings.output_directory.clone();

            app.output_folder_input = app
                .settings
                .output_directory
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            app.c2u_output_folder_input = app.output_folder_input.clone();

            app.set_status("Settings loaded from config file");
        }
        KeyCode::Char('d') => {
            app.settings = Settings::default();
            app.output_folder = app.settings.output_directory.clone();
            app.c2u_output_folder = app.settings.output_directory.clone();
            app.output_folder_input = String::new();
            app.c2u_output_folder_input = String::new();
            app.set_status("Settings reset to defaults (press 's' to save)");
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        _ => {}
    }

    Ok(true)
}

fn handle_xml_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    #[derive(Clone)]
    enum XmlTreeNode {
        Platform(String),
        Time { platform: String, time: String, count: usize },
    }

    fn build_xml_tree_nodes(app: &App) -> Vec<XmlTreeNode> {
        let mut platforms: Vec<String> = app
            .xml_remote_options
            .iter()
            .map(|o| o.platform.clone())
            .collect();
        platforms.sort();
        platforms.dedup();

        let mut nodes = Vec::new();
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
        nodes
    }

    fn selected_time_key(platform: &str, time: &str) -> String {
        format!("{}::{}", platform, time)
    }

    let nodes = build_xml_tree_nodes(app);

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.xml_tree_cursor > 0 {
                app.xml_tree_cursor -= 1;
                if app.xml_tree_cursor < app.xml_tree_scroll {
                    app.xml_tree_scroll = app.xml_tree_cursor;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.xml_tree_cursor + 1 < nodes.len() {
                app.xml_tree_cursor += 1;
                let visible_hint = 20usize;
                if app.xml_tree_cursor >= app.xml_tree_scroll + visible_hint {
                    app.xml_tree_scroll = app.xml_tree_cursor.saturating_sub(visible_hint - 1);
                }
            }
        }
        KeyCode::Enter => {
            if let Some(node) = nodes.get(app.xml_tree_cursor).cloned() {
                match node {
                    XmlTreeNode::Platform(platform) => {
                        if app.xml_expanded_platforms.iter().any(|p| p == &platform) {
                            app.xml_expanded_platforms.retain(|p| p != &platform);
                        } else {
                            app.xml_expanded_platforms.push(platform);
                        }
                    }
                    XmlTreeNode::Time { platform, time, .. } => {
                        let key = selected_time_key(&platform, &time);
                        if app.xml_selected_times.iter().any(|k| k == &key) {
                            app.xml_selected_times.retain(|k| k != &key);
                        } else {
                            app.xml_selected_times.push(key);
                        }
                    }
                }
            }
        }
        KeyCode::Char('f') => {
            start_path_edit(app, PathEditTarget::XmlOutput);
        }
        KeyCode::Char('d') => {
            app.xml_download_step = XmlDownloadStep {
                step: XmlStepState::Scanning,
                message: Some("Preparing selected platform/time files...".to_string()),
                progress: Some(10),
            };

            let mut selections: Vec<(String, String, Vec<(String, String)>)> = app
                .xml_selected_times
                .iter()
                .filter_map(|key| {
                    let (platform, time) = key.split_once("::")?;
                    let files = app
                        .xml_remote_options
                        .iter()
                        .find(|o| o.platform == platform && o.time == time)
                        .map(|o| o.file_urls.clone())?;
                    Some((platform.to_string(), time.to_string(), files))
                })
                .collect();

            if selections.is_empty() {
                // No explicit selection: download all currently loaded platform/time sets.
                selections = app
                    .xml_remote_options
                    .iter()
                    .map(|o| (o.platform.clone(), o.time.clone(), o.file_urls.clone()))
                    .collect();
            }

            if let Some((platform, time, _)) = selections.first() {
                app.xml_detected_base_url = Some(format!("{} / {}", platform, time));
                let output = app.xml_output_folder.clone();
                spawn_xml_download_flow(app.message_tx.clone(), output, selections);
            } else {
                app.xml_download_step = XmlDownloadStep {
                    step: XmlStepState::Error,
                    message: Some("No time selected. Press Enter on time nodes to select.".to_string()),
                    progress: None,
                };
            }
        }
        KeyCode::Char('R') => {
            app.xml_download_step = XmlDownloadStep {
                step: XmlStepState::Scanning,
                message: Some("Loading remote patch options...".to_string()),
                progress: Some(5),
            };
            spawn_load_xml_remote_options(app.message_tx.clone());
        }
        KeyCode::Char('r') => {
            app.xml_download_step = XmlDownloadStep {
                step: XmlStepState::Idle,
                message: Some("Scanning local XML files...".to_string()),
                progress: None,
            };
            let output = app.xml_output_folder.clone();
            spawn_scan_xml_files(app.message_tx.clone(), output);
        }
        KeyCode::Char('x') => {
            let selected = app.xml_bundles.get(app.selected_xml_bundle_idx).cloned();
            if let Some(bundle) = selected {
                app.xml_download_step = XmlDownloadStep {
                    step: XmlStepState::Extracting,
                    message: Some("Extracting XML files...".to_string()),
                    progress: Some(75),
                };
                spawn_xml_extract(
                    app.message_tx.clone(),
                    std::path::PathBuf::from(bundle),
                    app.xml_output_folder.clone(),
                );
            } else {
                app.set_status("No bundle selected. Press 'r' to scan or 'd' to fetch.");
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        _ => {}
    }

    Ok(true)
}

fn start_path_edit(app: &mut App, target: PathEditTarget) {
    app.editing_path_target = Some(target);
    app.editing_path_value = match target {
        PathEditTarget::DownloadOutput => app.output_folder_input.clone(),
        PathEditTarget::C2uOutput => app.c2u_output_folder_input.clone(),
        PathEditTarget::C2uSource => app.c2u_source_path_input.clone(),
        PathEditTarget::ConfigToolsDirectory => app
            .settings
            .tools_directory
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        PathEditTarget::ConfigOutputDirectory => app
            .settings
            .output_directory
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        PathEditTarget::ConfigProxyPort => app.settings.proxy_port.to_string(),
        PathEditTarget::XmlOutput => app.xml_output_folder_input.clone(),
    };

    let label = match target {
        PathEditTarget::DownloadOutput => "Download output path",
        PathEditTarget::C2uOutput => "C2U output path",
        PathEditTarget::C2uSource => "C2U source path (file/folder)",
        PathEditTarget::ConfigToolsDirectory => "Config tools directory",
        PathEditTarget::ConfigOutputDirectory => "Config output directory",
        PathEditTarget::ConfigProxyPort => "Config proxy port",
        PathEditTarget::XmlOutput => "XML output directory",
    };

    app.set_status(format!("Editing {}: Enter=save, Esc=cancel", label));
}

fn handle_path_edit_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.editing_path_target = None;
            app.editing_path_value.clear();
            app.set_status("Path edit canceled");
        }
        KeyCode::Backspace => {
            app.editing_path_value.pop();
        }
        KeyCode::Char(c) => {
            app.editing_path_value.push(c);
        }
        KeyCode::Enter => {
            let Some(target) = app.editing_path_target else {
                return Ok(true);
            };

            let raw = app.editing_path_value.trim();
            if raw.is_empty() {
                app.set_status("Path cannot be empty");
                return Ok(true);
            }

            let path = std::path::PathBuf::from(raw);
            match target {
                PathEditTarget::DownloadOutput => {
                    app.output_folder = Some(path.clone());
                    app.output_folder_input = raw.to_string();
                    app.set_status(format!("Download output set: {}", path.display()));
                }
                PathEditTarget::C2uOutput => {
                    app.c2u_output_folder = Some(path.clone());
                    app.c2u_output_folder_input = raw.to_string();
                    app.set_status(format!("C2U output set: {}", path.display()));
                }
                PathEditTarget::C2uSource => {
                    app.c2u_source_path = path.clone();
                    app.c2u_source_path_input = raw.to_string();
                    app.set_status(format!("C2U source set: {}", path.display()));
                }
                PathEditTarget::ConfigToolsDirectory => {
                    app.settings.tools_directory = if raw.is_empty() {
                        None
                    } else {
                        Some(path.clone())
                    };
                    app.set_status(format!("Config tools directory set: {}", path.display()));
                }
                PathEditTarget::ConfigOutputDirectory => {
                    app.settings.output_directory = if raw.is_empty() {
                        None
                    } else {
                        Some(path.clone())
                    };

                    app.output_folder = app.settings.output_directory.clone();
                    app.c2u_output_folder = app.settings.output_directory.clone();
                    app.output_folder_input = raw.to_string();
                    app.c2u_output_folder_input = raw.to_string();

                    app.set_status(format!("Config output directory set: {}", path.display()));
                }
                PathEditTarget::ConfigProxyPort => {
                    match raw.parse::<u16>() {
                        Ok(port) if port > 0 => {
                            app.settings.proxy_port = port;
                            app.set_status(format!("Config proxy port set: {}", port));
                        }
                        _ => {
                            app.set_status("Invalid proxy port (expected 1..65535)");
                            return Ok(true);
                        }
                    }
                }
                PathEditTarget::XmlOutput => {
                    app.xml_output_folder = path.clone();
                    app.xml_output_folder_input = raw.to_string();
                    app.set_status(format!("XML output set: {}", path.display()));
                }
            }

            app.editing_path_target = None;
            app.editing_path_value.clear();
        }
        _ => {}
    }

    Ok(true)
}

fn spawn_xml_download_flow(
    tx: mpsc::UnboundedSender<AppMessage>,
    output_dir: std::path::PathBuf,
    selected_sets: Vec<(String, String, Vec<(String, String)>)>,
) {
    tokio::spawn(async move {
        let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
            step: XmlStepState::Scanning,
            message: Some("Preparing selected source files...".to_string()),
            progress: Some(15),
        }));

        let total = selected_sets.len().max(1) as u32;
        for (idx, (platform, time, file_urls)) in selected_sets.into_iter().enumerate() {
            let base_progress = (idx as u32 * 100) / total;
            let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
                step: XmlStepState::Downloading,
                message: Some(format!(
                    "[{}/{}] Downloading {} / {} ({} files)",
                    idx + 1,
                    total,
                    platform,
                    time,
                    file_urls.len()
                )),
                progress: Some(base_progress.saturating_add(20).min(90)),
            }));

            let count = match crate::core::xml_config::download_patch_live_files_to_tree(
                &output_dir,
                &platform,
                &time,
                &file_urls,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
                        step: XmlStepState::Error,
                        message: Some(format!("XML download failed: {}", e)),
                        progress: None,
                    }));
                    let _ = tx.send(AppMessage::Error(format!("XML fetch failed: {}", e)));
                    return;
                }
            };

            let _ = tx.send(AppMessage::XmlBaseUrlDetected(format!("{} / {}", platform, time)));
            let _ = tx.send(AppMessage::XmlExtracted(count));
        }

        let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
            step: XmlStepState::Complete,
            message: Some("Complete: all selected time(s) processed".to_string()),
            progress: Some(100),
        }));
        spawn_scan_xml_files(tx, output_dir);
    });
}

pub(crate) fn spawn_load_xml_remote_options(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        match crate::core::xml_config::list_patch_live_options_from_source().await {
            Ok(options) => {
                let payload: Vec<(String, String, String, Vec<String>, Vec<(String, String)>)> = options
                    .into_iter()
                    .map(|o| (o.platform, o.time, o.download_url, o.items, o.file_urls))
                    .collect();
                let _ = tx.send(AppMessage::XmlRemoteOptionsLoaded(payload));
                let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
                    step: XmlStepState::Idle,
                    message: Some("Remote options loaded".to_string()),
                    progress: None,
                }));

                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                let _ = tx.send(AppMessage::XmlRemoteOptionsNoticeExpired);
            }
            Err(e) => {
                let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
                    step: XmlStepState::Error,
                    message: Some(format!("Failed to load remote options: {}", e)),
                    progress: None,
                }));
                let _ = tx.send(AppMessage::Error(format!("Failed to load remote XML options: {}", e)));
            }
        }
    });
}

fn spawn_xml_extract(
    tx: mpsc::UnboundedSender<AppMessage>,
    bundle_path: std::path::PathBuf,
    output_dir: std::path::PathBuf,
) {
    tokio::spawn(async move {
        match crate::core::xml_config::extract_xml_files(&bundle_path, &output_dir) {
            Ok(extracted) => {
                let count = extracted.len();
                let _ = tx.send(AppMessage::XmlExtracted(count));
                let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
                    step: XmlStepState::Complete,
                    message: Some(format!("Complete: extracted {} XML file(s)", count)),
                    progress: Some(100),
                }));
                spawn_scan_xml_files(tx, output_dir);
            }
            Err(e) => {
                let _ = tx.send(AppMessage::XmlStepUpdate(XmlDownloadStep {
                    step: XmlStepState::Error,
                    message: Some(format!("XML extract failed: {}", e)),
                    progress: None,
                }));
                let _ = tx.send(AppMessage::Error(format!("XML extract failed: {}", e)));
            }
        }
    });
}

pub(crate) fn spawn_scan_xml_files(
    tx: mpsc::UnboundedSender<AppMessage>,
    source_dir: std::path::PathBuf,
) {
    tokio::spawn(async move {
        fn walk_collect(dir: &std::path::Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    walk_collect(&path, out)?;
                    continue;
                }

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();

                let is_unityfs = if ext.is_empty() {
                    match std::fs::read(&path) {
                        Ok(bytes) => bytes.starts_with(b"UnityFS"),
                        Err(_) => false,
                    }
                } else {
                    false
                };

                if ext == "xml" || ext == "unity3d" || is_unityfs {
                    out.push(path.display().to_string());
                }
            }
            Ok(())
        }

        let mut files: Vec<String> = Vec::new();
        match walk_collect(&source_dir, &mut files) {
            Ok(()) => {
                files.sort();
                let _ = tx.send(AppMessage::XmlFilesLoaded(files));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!(
                    "Failed to scan XML folder {}: {}",
                    source_dir.display(),
                    e
                )));
            }
        }
    });
}

// Async task spawners

pub(crate) fn spawn_refresh_tools(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        match deps::check_all() {
            Ok(status) => {
                let status_list: Vec<(String, bool)> = status.iter()
                    .map(|(name, installed)| (name.to_string(), *installed))
                    .collect();
                let _ = tx.send(AppMessage::ToolsStatusUpdated(status_list));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!("Failed to check tools: {}", e)));
            }
        }
    });
}

pub(crate) fn spawn_fetch_versions(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        use crate::core::kgc::PACKAGE_ID;
        match downloader::list_versions(PACKAGE_ID).await {
            Ok(versions) => {
                let _ = tx.send(AppMessage::VersionsLoaded(versions));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!("Failed to fetch versions: {}", e)));
            }
        }
    });
}

fn spawn_download(tx: mpsc::UnboundedSender<AppMessage>, version: String, output_folder: Option<std::path::PathBuf>) {
    tokio::spawn(async move {
        use crate::core::kgc::PACKAGE_ID;
        let output_dir = output_folder.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        
        match downloader::download_xapk(
            PACKAGE_ID,
            Some(&version),
            &output_dir,
            |percent, msg| {
                let _ = tx.send(AppMessage::DownloadProgress(percent, msg.to_string()));
            }
        ).await {
            Ok(path) => {
                let _ = tx.send(AppMessage::DownloadComplete(path.display().to_string()));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!("Download failed: {}", e)));
            }
        }
    });
}

pub(crate) fn spawn_scan_xapks(tx: mpsc::UnboundedSender<AppMessage>, source_dir: std::path::PathBuf) {
    tokio::spawn(async move {
        let mut files: Vec<String> = Vec::new();
        if source_dir.is_file() {
            let is_xapk = source_dir
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("xapk"))
                .unwrap_or(false);

            if is_xapk {
                files.push(source_dir.display().to_string());
                let _ = tx.send(AppMessage::C2uFilesLoaded(files));
            } else {
                let _ = tx.send(AppMessage::Error(format!(
                    "Selected file is not .xapk: {}",
                    source_dir.display()
                )));
            }
            return;
        }

        match std::fs::read_dir(&source_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_xapk = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("xapk"))
                        .unwrap_or(false);
                    if path.is_file() && is_xapk {
                        files.push(path.display().to_string());
                    }
                }
                files.sort();
                let _ = tx.send(AppMessage::C2uFilesLoaded(files));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!(
                    "Failed to scan XAPK files in {}: {}",
                    source_dir.display(),
                    e
                )));
            }
        }
    });
}

fn spawn_c2u_convert(
    tx: mpsc::UnboundedSender<AppMessage>,
    xapk_path: std::path::PathBuf,
    output_dir: std::path::PathBuf,
) {
    tokio::spawn(async move {
        let _ = tx.send(AppMessage::C2uLogsReset);
        let _ = tx.send(AppMessage::C2uLog(format!(
            "[START] Convert {} -> {}",
            xapk_path.display(),
            output_dir.display()
        )));

        match crate::core::c2u::convert(&xapk_path, &output_dir, None, |stage, percent, msg| {
            let text = format!("{}: {}", stage, msg);
            let _ = tx.send(AppMessage::C2uProgress(percent, text));
            let _ = tx.send(AppMessage::C2uLog(format!("[{}][{}%] {}", stage, percent, msg)));
        })
        .await
        {
            Ok(_) => {
                let _ = tx.send(AppMessage::C2uLog("[DONE] Conversion complete".to_string()));
                let _ = tx.send(AppMessage::C2uComplete(output_dir.display().to_string()));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::C2uLog(format!("[ERROR] {}", e)));
                let _ = tx.send(AppMessage::Error(format!("C2U failed: {}", e)));
            }
        }
    });
}

fn spawn_start_proxy(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        match server::start(8888, "0.0.0.0").await {
            Ok(_) => {
                let _ = tx.send(AppMessage::ProxyStatusChanged(true));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!("Failed to start proxy: {}", e)));
            }
        }
    });
}

fn spawn_stop_proxy(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        match server::stop().await {
            Ok(_) => {
                let _ = tx.send(AppMessage::ProxyStatusChanged(false));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!("Failed to stop proxy: {}", e)));
            }
        }
    });
}

fn spawn_refresh_traffic(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        match crate::proxy::storage::get_requests(None, 50) {
            Ok(requests) => {
                let traffic: Vec<String> = requests.iter()
                    .map(|r| format!("{} {} - {}", r.method, r.url, r.timestamp))
                    .collect();
                let _ = tx.send(AppMessage::TrafficCaptured(traffic));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::Error(format!("Failed to fetch traffic: {}", e)));
            }
        }
    });
}
