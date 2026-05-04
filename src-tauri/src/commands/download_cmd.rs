//! Download-related commands: queueing the reqwest-based `start_download`
//! pipeline, pausing an in-flight task, and the platform-dispatching
//! `browser_download` entry point that reaches into the browser-webview to
//! drive a native download (Linux WebKitGTK) or fall back to reqwest
//! (macOS / Windows).

use tauri::State;

use crate::download::DownloadTask;
use crate::state::AppState;
use crate::webview::download_native;

#[tauri::command]
pub async fn start_download(
    app: tauri::AppHandle,
    task: DownloadTask,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = state.settings.lock().unwrap().clone();
    let manager = state.download_manager.lock().await;
    manager
        .start_download(
            task,
            app.clone(),
            settings.extract_audio,
            settings.audio_format,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_download(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.download_manager.lock().await;
    manager
        .pause_download(&task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Download a video file using the platform's native webview download API.
///
/// On Linux (WebKitGTK): uses `download_uri()` which carries the full browser
/// session (including httpOnly cookies like JSESSIONID and BbRouter).
///
/// On macOS / Windows: falls back to reqwest streaming download with the JWT
/// token already embedded in the URL.
///
/// The command returns immediately on Linux.  Progress, completion and error
/// notifications are delivered via Tauri events:
///   - `download-progress`  { taskId, progress, speed, eta }
///   - `download-complete`  { taskId }
///   - `download-error`     { taskId, error }
#[tauri::command]
pub async fn browser_download(
    app: tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
    // For m3u8 videos the Blackboard API returns .mp4; ensure the filepath has the extension
    let is_m3u8 = url.contains("downloadVideo.action") || url.contains(".m3u8");
    let filepath = if is_m3u8 && !filepath.ends_with(".mp4") {
        format!("{}.mp4", filepath)
    } else {
        filepath
    };

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&filepath).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }

    log::info!("[browser_download] starting download for {task_id}");
    log::info!("[browser_download] url: {url}");
    log::info!("[browser_download] filepath: {filepath}");

    download_native::run(&app, task_id, url, filepath).await
}
