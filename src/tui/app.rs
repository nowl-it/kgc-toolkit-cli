//! TUI application state

use tokio::sync::mpsc;
use crate::config::Settings;
use crate::core::kgc;
use crate::utils::paths;
use super::theme::{Theme, ThemeMode};
use super::keybindings::KeybindingsHelp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathEditTarget {
    DownloadOutput,
    C2uOutput,
    C2uSource,
    ConfigToolsDirectory,
    ConfigOutputDirectory,
    ConfigProxyPort,
    XmlOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Download,
    Convert,
    Compare,
    Proxy,
    Unity,
    Xml,
    Config,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    ToolsStatusUpdated(Vec<(String, bool)>),
    VersionsLoaded(Vec<String>),
    DownloadProgress(u32, String),
    DownloadComplete(String),
    C2uFilesLoaded(Vec<String>),
    C2uProgress(u32, String),
    C2uLog(String),
    C2uLogsReset,
    C2uComplete(String),
    XmlBaseUrlDetected(String),
    XmlBundleFetched(String),
    XmlRemoteOptionsLoaded(Vec<(String, String, String, Vec<String>, Vec<(String, String)>)>),
    XmlRemoteOptionsNoticeExpired,
    XmlExtracted(usize),
    XmlFilesLoaded(Vec<String>),
    XmlStepUpdate(XmlDownloadStep),
    ProxyStatusChanged(bool),
    TrafficCaptured(Vec<String>),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct XmlRemoteOption {
    pub platform: String,
    pub time: String,
    pub download_url: String,
    pub items: Vec<String>,
    pub file_urls: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct XmlDownloadStep {
    pub step: XmlStepState,
    pub message: Option<String>,
    pub progress: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlStepState {
    Idle,
    Scanning,
    Downloading,
    Extracting,
    Complete,
    Error,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Download => "Download",
            Tab::Convert => "Convert (C2U)",
            Tab::Compare => "Compare",
            Tab::Proxy => "Proxy",
            Tab::Unity => "Unity",
            Tab::Xml => "XML",
            Tab::Config => "Config",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[
            Tab::Dashboard,
            Tab::Download,
            Tab::Convert,
            Tab::Compare,
            Tab::Proxy,
            Tab::Unity,
            Tab::Xml,
            Tab::Config,
        ]
    }

    pub fn next(&self) -> Tab {
        let tabs = Self::all();
        let idx = tabs.iter().position(|t| t == self).unwrap_or(0);
        tabs[(idx + 1) % tabs.len()]
    }

    pub fn prev(&self) -> Tab {
        let tabs = Self::all();
        let idx = tabs.iter().position(|t| t == self).unwrap_or(0);
        tabs[(idx + tabs.len() - 1) % tabs.len()]
    }
}

pub struct App {
    pub active_tab: Tab,
    pub should_quit: bool,
    pub previous_tab: Tab,

    // Theme & UI
    pub theme: Theme,
    pub show_help: bool,
    pub keybindings_help: KeybindingsHelp,

    // Dashboard state
    pub tools_status: Vec<(String, bool)>,

    // Download state
    pub available_versions: Vec<String>,
    pub selected_version_idx: usize,
    pub download_progress: Option<(u32, String)>,
    pub output_folder: Option<std::path::PathBuf>,
    pub output_folder_input: String,

    // Convert (C2U) state
    pub available_xapks: Vec<String>,
    pub selected_xapk_idx: usize,
    pub c2u_progress: Option<(u32, String)>,
    pub c2u_output_folder: Option<std::path::PathBuf>,
    pub c2u_output_folder_input: String,
    pub c2u_source_path: std::path::PathBuf,
    pub c2u_source_path_input: String,
    pub c2u_logs: Vec<String>,
    pub c2u_show_logs: bool,

    // XML tab state
    pub xml_output_folder: std::path::PathBuf,
    pub xml_output_folder_input: String,
    pub xml_bundles: Vec<String>,
    pub selected_xml_bundle_idx: usize,
    pub xml_files: Vec<String>,
    pub xml_remote_options: Vec<XmlRemoteOption>,
    pub selected_xml_platform_idx: usize,
    pub selected_xml_time_idx: usize,
    pub xml_tree_cursor: usize,
    pub xml_tree_scroll: usize,
    pub xml_expanded_platforms: Vec<String>,
    pub xml_selected_times: Vec<String>,
    pub xml_download_step: XmlDownloadStep,
    pub xml_detected_base_url: Option<String>,

    // Config tab state
    pub config_selected_idx: usize,
    pub settings: Settings,

    // Shared path editor state
    pub editing_path_target: Option<PathEditTarget>,
    pub editing_path_value: String,

    // Proxy state
    pub proxy_running: bool,
    pub captured_requests: Vec<String>,

    // Status message
    pub status_message: Option<String>,
    
    // Async task communication
    pub message_tx: mpsc::UnboundedSender<AppMessage>,
    pub message_rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut settings = Settings::load();
        let default_output = settings
            .output_directory
            .clone()
            .unwrap_or_else(|| current_dir.clone());
        let default_output_text = default_output.display().to_string();

        if settings.output_directory.is_none() {
            settings.output_directory = Some(default_output.clone());
        }
        
        Self {
            active_tab: Tab::Dashboard,
            should_quit: false,
            previous_tab: Tab::Dashboard,
            theme: Theme::dark(),
            show_help: false,
            keybindings_help: KeybindingsHelp::new(),
            tools_status: Vec::new(),
            available_versions: Vec::new(),
            selected_version_idx: 0,
            download_progress: None,
            output_folder: Some(default_output.clone()),
            output_folder_input: default_output_text.clone(),
            available_xapks: Vec::new(),
            selected_xapk_idx: 0,
            c2u_progress: None,
            c2u_output_folder: Some(default_output.clone()),
            c2u_output_folder_input: default_output_text,
            c2u_source_path: current_dir,
            c2u_source_path_input: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .display()
                .to_string(),
            c2u_logs: Vec::new(),
            c2u_show_logs: false,
            xml_output_folder: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            xml_output_folder_input: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .display()
                .to_string(),
            xml_bundles: Vec::new(),
            selected_xml_bundle_idx: 0,
            xml_files: Vec::new(),
            xml_remote_options: Vec::new(),
            selected_xml_platform_idx: 0,
            selected_xml_time_idx: 0,
            xml_tree_cursor: 0,
            xml_tree_scroll: 0,
            xml_expanded_platforms: Vec::new(),
            xml_selected_times: Vec::new(),
            xml_download_step: XmlDownloadStep {
                step: XmlStepState::Idle,
                message: None,
                progress: None,
            },
            xml_detected_base_url: None,
            config_selected_idx: 0,
            settings,
            editing_path_target: None,
            editing_path_value: String::new(),
            proxy_running: false,
            captured_requests: Vec::new(),
            status_message: Some(format!("KGC Toolkit - {} • Press '?' for help", kgc::GAME_NAME)),
            message_tx: tx,
            message_rx: rx,
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme.mode {
            ThemeMode::Dark => Theme::light(),
            ThemeMode::Light => Theme::dark(),
        };
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    fn export_error_to_file(
        err: &str,
        c2u_logs: Option<&[String]>,
    ) -> Result<std::path::PathBuf, std::io::Error> {
        let logs_dir = paths::logs_dir();
        std::fs::create_dir_all(&logs_dir)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let file_name = format!("error_{}_{}.log", now.as_secs(), now.subsec_nanos());
        let log_path = logs_dir.join(file_name);

        let mut content = format!(
            "KGC Toolkit Error\nTimestamp(unix): {}.{}\n\nError Message:\n{}\n",
            now.as_secs(),
            now.subsec_nanos(),
            err
        );

        if let Some(lines) = c2u_logs {
            content.push_str("\n--- C2U Runtime Logs ---\n");
            if lines.is_empty() {
                content.push_str("(no C2U logs captured)\n");
            } else {
                for line in lines {
                    content.push_str(line);
                    content.push('\n');
                }
            }
        }

        std::fs::write(&log_path, content)?;
        Ok(log_path)
    }

    pub fn config_item_count() -> usize {
        3
    }
    
    /// Process async messages from background tasks
    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.message_rx.try_recv() {
            match msg {
                AppMessage::ToolsStatusUpdated(status) => {
                    self.tools_status = status;
                    self.set_status("Tools status refreshed");
                }
                AppMessage::VersionsLoaded(versions) => {
                    self.available_versions = versions;
                    self.selected_version_idx = 0;
                    self.set_status(format!("Loaded {} versions", self.available_versions.len()));
                }
                AppMessage::DownloadProgress(percent, msg) => {
                    self.download_progress = Some((percent, msg));
                }
                AppMessage::DownloadComplete(path) => {
                    self.download_progress = None;
                    self.set_status(format!("Download complete: {}", path));
                }
                AppMessage::C2uFilesLoaded(files) => {
                    self.available_xapks = files;
                    self.selected_xapk_idx = 0;
                    self.c2u_show_logs = false;
                    self.set_status(format!("Found {} XAPK file(s)", self.available_xapks.len()));
                }
                AppMessage::C2uProgress(percent, msg) => {
                    self.c2u_progress = Some((percent, msg));
                }
                AppMessage::C2uLog(line) => {
                    self.c2u_logs.push(line);
                    if self.c2u_logs.len() > 300 {
                        let drain_count = self.c2u_logs.len().saturating_sub(300);
                        self.c2u_logs.drain(0..drain_count);
                    }
                }
                AppMessage::C2uLogsReset => {
                    self.c2u_logs.clear();
                }
                AppMessage::C2uComplete(path) => {
                    self.c2u_progress = None;
                    self.c2u_show_logs = false;
                    self.set_status(format!("C2U complete: {}", path));
                }
                AppMessage::XmlBundleFetched(path) => {
                    self.set_status(format!("XML bundle fetched: {}", path));
                }
                AppMessage::XmlRemoteOptionsLoaded(options) => {
                    self.xml_remote_options = options
                        .into_iter()
                        .map(|(platform, time, download_url, items, file_urls)| XmlRemoteOption {
                            platform,
                            time,
                            download_url,
                            items,
                            file_urls,
                        })
                        .collect();
                    self.selected_xml_platform_idx = 0;
                    self.selected_xml_time_idx = 0;
                    self.xml_tree_cursor = 0;
                    self.xml_tree_scroll = 0;
                    self.xml_expanded_platforms.clear();
                    self.xml_selected_times.clear();
                    self.set_status(format!(
                        "Loaded {} remote XML option(s)",
                        self.xml_remote_options.len()
                    ));
                }
                AppMessage::XmlBaseUrlDetected(base_url) => {
                    self.xml_detected_base_url = Some(base_url);
                }
                AppMessage::XmlExtracted(count) => {
                    self.set_status(format!("Extracted {} XML file(s)", count));
                }
                AppMessage::XmlFilesLoaded(files) => {
                    let mut bundles = Vec::new();
                    let mut xml_files = Vec::new();

                    for item in files {
                        if item.to_lowercase().ends_with(".unity3d") {
                            bundles.push(item);
                        } else if item.to_lowercase().ends_with(".xml") {
                            xml_files.push(item);
                        }
                    }

                    self.xml_bundles = bundles;
                    self.xml_files = xml_files;
                    self.selected_xml_bundle_idx = 0;
                    self.set_status(format!(
                        "XML scan complete: {} bundle(s), {} xml file(s)",
                        self.xml_bundles.len(),
                        self.xml_files.len()
                    ));
                }
                AppMessage::XmlStepUpdate(step) => {
                    self.xml_download_step = step;
                }
                AppMessage::XmlRemoteOptionsNoticeExpired => {
                    if self.xml_download_step.message.as_deref() == Some("Remote options loaded") {
                        self.xml_download_step = XmlDownloadStep {
                            step: XmlStepState::Idle,
                            message: None,
                            progress: None,
                        };
                    }
                }
                AppMessage::ProxyStatusChanged(running) => {
                    self.proxy_running = running;
                    self.set_status(if running { "Proxy started" } else { "Proxy stopped" });
                }
                AppMessage::TrafficCaptured(requests) => {
                    self.captured_requests = requests;
                }
                AppMessage::Error(err) => {
                    if err.starts_with("C2U failed:") {
                        self.c2u_progress = None;
                        self.c2u_show_logs = false;
                    }

                    let c2u_log_snapshot = if err.starts_with("C2U failed:") {
                        Some(self.c2u_logs.as_slice())
                    } else {
                        None
                    };

                    match Self::export_error_to_file(&err, c2u_log_snapshot) {
                        Ok(path) => {
                            self.set_status(format!(
                                "Error: {} | Exported: {}",
                                err,
                                path.display()
                            ));
                        }
                        Err(export_err) => {
                            self.set_status(format!(
                                "Error: {} | Export failed: {}",
                                err,
                                export_err
                            ));
                        }
                    }
                }
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
