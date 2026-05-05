//! `on_download` handler attached to the browser-webview.
//!
//! Tauri delivers `Requested` and `Finished` callbacks for downloads
//! triggered by the webview itself (e.g. user clicking a link with
//! `Content-Disposition: attachment`).  We use a `pending_downloads` map
//! keyed by download URL to resolve the callback back to the originating
//! task, then emit `download-complete` / `download-error` accordingly.

use serde_json::json;
use tauri::{Emitter, Manager};

use crate::state::AppState;
use crate::webview::download_native::shared::{
    after_browser_download, rehide_browser_if_not_browser_mode,
};

/// Shared on_download handler logic for any webview.
/// Returns true to allow the download, false to reject.
pub fn handle_download_event(
    webview: &tauri::Webview,
    event: tauri::webview::DownloadEvent,
) -> bool {
    use tauri::webview::DownloadEvent;
    let wv_label = webview.label().to_string();
    match event {
        DownloadEvent::Requested { url, destination } => {
            let url_str = url.to_string();
            log::info!("[on_download:{wv_label}] Requested: {}", url_str);
            let state = webview.state::<AppState>();
            if let Ok(pending) = state.pending_downloads.lock() {
                // Try exact URL match first
                if let Some(info) = pending.get(&url_str) {
                    *destination = std::path::PathBuf::from(&info.filepath);
                    log::info!(
                        "[on_download:{wv_label}] matched (exact): {} -> {}",
                        url_str,
                        info.filepath
                    );
                    return true;
                }
                // Fallback: URL may differ due to redirect.
                if let Some((_orig_url, info)) = pending.iter().next() {
                    *destination = std::path::PathBuf::from(&info.filepath);
                    log::info!(
                        "[on_download:{wv_label}] matched (fallback): {} -> {} (orig: {})",
                        url_str,
                        info.filepath,
                        _orig_url
                    );
                    return true;
                }
            }
            log::warn!("[on_download:{wv_label}] untracked download: {}", url_str);
            true
        }
        DownloadEvent::Finished { url, path, success } => {
            let url_str = url.to_string();
            log::info!(
                "[on_download:{wv_label}] Finished: url={} path={:?} success={}",
                url_str,
                path,
                success
            );
            let state = webview.state::<AppState>();
            let pending_info = state.pending_downloads.lock().ok().and_then(|mut p| {
                if let Some(info) = p.remove(&url_str) {
                    return Some(info);
                }
                let key = p.keys().next().cloned();
                key.and_then(|k| p.remove(&k))
            });
            if let Some(info) = pending_info {
                let app = webview.app_handle();
                if success {
                    if let Some(ref p) = path {
                        after_browser_download(app, &info.task_id, p.to_str().unwrap_or(""));
                    } else {
                        let _ = app.emit("download-complete", json!({ "taskId": info.task_id }));
                    }
                    log::info!(
                        "[on_download:{wv_label}] completed: task={} path={:?}",
                        info.task_id,
                        path
                    );
                } else {
                    let _ = app.emit(
                        "download-error",
                        json!({
                            "taskId": info.task_id,
                            "error": format!("Browser download failed ({})", wv_label)
                        }),
                    );
                    log::error!("[on_download:{wv_label}] failed: task={}", info.task_id);
                }
            } else {
                log::warn!("[on_download:{wv_label}] Finished but no pending entry found");
            }
            // Re-hide browser-webview if the user is no longer in browser mode
            rehide_browser_if_not_browser_mode(webview.app_handle());
            // Destroy temporary download webview (label starts with "dl-")
            if wv_label.starts_with("dl-") {
                let app = webview.app_handle().clone();
                let label = wv_label.clone();
                // Defer close to avoid borrow issues inside callback
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Some(wv) = app.get_webview(&label) {
                        let _ = wv.close();
                        log::info!("[on_download] temp webview '{label}' destroyed");
                    }
                });
            }
            true
        }
        _ => true,
    }
}
