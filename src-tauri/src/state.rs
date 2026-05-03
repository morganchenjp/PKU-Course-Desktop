use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;

use crate::download::DownloadManager;
use crate::settings::AppSettings;

/// Active high-level view mode.  Sub-views (downloads vs settings) are
/// purely a frontend concern and are not tracked here — they're passed
/// as a one-shot `view` argument to `do_show_main_view`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Browser,
    Main,
}

/// Info about a download triggered via the browser-webview's native HTTP stack.
pub struct PendingBrowserDownload {
    pub task_id: String,
    pub filepath: String,
}

/// Application-wide state shared across Tauri commands and the IPC protocol handler.
pub struct AppState {
    pub download_manager: Mutex<DownloadManager>,
    pub settings: StdMutex<AppSettings>,
    /// Tracks which view mode is active so the resize handler and
    /// post-download cleanup can reposition / re-hide webviews accordingly.
    pub current_view_mode: StdMutex<ViewMode>,
    /// Pending browser-webview downloads, keyed by download URL
    /// (used by the on_download handler to resolve a Tauri-generated download
    /// back to the originating task).
    pub pending_downloads: StdMutex<HashMap<String, PendingBrowserDownload>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            download_manager: Mutex::new(DownloadManager::new()),
            settings: StdMutex::new(AppSettings::default()),
            current_view_mode: StdMutex::new(ViewMode::Browser),
            pending_downloads: StdMutex::new(HashMap::new()),
        }
    }
}
