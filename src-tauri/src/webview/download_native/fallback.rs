//! macOS / Windows fallback for browser-driven downloads.
//!
//! Instead of using the webview's native download mechanism (which provides no
//! progress events in this Tauri version), we extract the browser's cookies
//! (including httpOnly / secure cookies) via Tauri's `cookies_for_url` API and
//! stream the file with `reqwest`.  This gives us real byte-level progress,
//! speed and ETA — identical UX to the Linux WebKitGTK path.

use tauri::{Emitter, Manager};
use tokio::time::Duration;

use crate::state::{AppState, ViewMode};
use crate::webview::download_native::shared::after_browser_download;

pub async fn run(
    app: &tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    // ── 1. Extract cookies from the browser webview ──────────────────────
    // `cookies_for_url` returns *all* cookies including httpOnly / secure
    // cookies that JavaScript cannot read.  On Windows this must be called
    // from an async context (Tokio worker thread) to avoid deadlocking the
    // WebView2 UI thread.
    let parsed_url = url
        .parse::<url::Url>()
        .map_err(|e| format!("invalid download url: {e}"))?;
    let cookies = browser
        .cookies_for_url(parsed_url)
        .map_err(|e| format!("failed to read webview cookies: {e}"))?;
    let cookie_header = if cookies.is_empty() {
        None
    } else {
        Some(
            cookies
                .iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect::<Vec<_>>()
                .join("; "),
        )
    };

    log::info!(
        "[fallback] cookie extraction: {} cookies for {url}",
        cookies.len()
    );

    // ── 2. Create a one-off HTTP client and stream the file ──────────────
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;

    let result = crate::download::download_with_progress(
        &client,
        &url,
        &filepath,
        &task_id,
        None, // JWT is already in the URL query string for browser downloads
        cookie_header.as_deref(),
        app,
    )
    .await;

    // ── 3. Completion / error handling ───────────────────────────────────
    match result {
        Ok(()) => {
            let _ = app.emit(
                "download-complete",
                serde_json::json!({ "taskId": task_id }),
            );
            log::info!("[fallback] download completed: {task_id}");
            after_browser_download(app, &task_id, &filepath);
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = app.emit(
                "download-error",
                serde_json::json!({ "taskId": task_id, "error": msg }),
            );
            log::error!("[fallback] download failed {task_id}: {msg}");
        }
    }

    // Re-hide browser-webview if the user is no longer in browser mode
    let is_browser = app
        .state::<AppState>()
        .current_view_mode
        .lock()
        .map(|m| matches!(*m, ViewMode::Browser))
        .unwrap_or(false);
    if !is_browser {
        if let Some(b) = app.get_webview("browser-webview") {
            let _ = b.hide();
        }
    }

    Ok(())
}
