use serde_json::json;
use tauri::{Emitter, Manager};

use crate::state::{AppState, ViewMode};

/// Tail run after a browser-webview download successfully writes a file to disk.
///
/// Emits `download-complete` immediately, then spawns a background thread that
/// runs audio extraction (if enabled in current settings) and emits
/// `audio-extract-complete` on success.  Used by both the Linux GTK callback
/// path and the macOS/Windows reqwest fallback — they previously had this
/// logic copy-pasted.
///
/// The audio extraction runs on a dedicated thread with its own Tokio runtime
/// because the GTK signal handler that calls this is not inside a Tokio
/// runtime context.
pub fn after_browser_download(app: &tauri::AppHandle, task_id: &str, filepath: &str) {
    let _ = app.emit("download-complete", json!({ "taskId": task_id }));

    let app_clone = app.clone();
    let task_id_owned = task_id.to_string();
    let filepath_owned = filepath.to_string();
    std::thread::spawn(move || {
        let (extract_audio, audio_format) = {
            let state = app_clone.state::<AppState>();
            let settings = state.settings.lock().expect("settings mutex poisoned");
            (settings.extract_audio, settings.audio_format.clone())
        };
        if !extract_audio {
            return;
        }
        // Browser-download paths preserve the original extension and append the
        // audio extension (e.g. lecture.mp4 -> lecture.mp4.mp3).  This matches
        // the pre-refactor behavior and is intentionally different from the
        // download.rs path which uses Path::with_extension.
        let audio_path = format!("{}.{}", filepath_owned, audio_format);
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("[after_browser_download] failed to create tokio runtime: {e}");
                return;
            }
        };
        match rt.block_on(crate::ffmpeg::extract_audio(
            &filepath_owned,
            &audio_path,
            &audio_format,
        )) {
            Ok(()) => emit_audio_extract_complete(&app_clone, &task_id_owned, &audio_path),
            Err(e) => log::error!(
                "[after_browser_download] audio extraction failed for {task_id_owned}: {e}"
            ),
        }
    });
}

/// Emit the `audio-extract-complete` event.  Centralizes the payload shape so
/// all three download paths produce identical event data.
pub fn emit_audio_extract_complete(app: &tauri::AppHandle, task_id: &str, audio_path: &str) {
    let _ = app.emit(
        "audio-extract-complete",
        json!({ "taskId": task_id, "audioPath": audio_path }),
    );
}

/// Hide the browser-webview if the active view mode is not `Browser`.
/// Called from download-completion paths so a temporary browser-webview that
/// was made visible for the download (Linux WebKitGTK requires this) gets
/// re-hidden afterwards if the user is on Downloads / Settings.
pub fn rehide_browser_if_not_browser_mode(app: &tauri::AppHandle) {
    let is_browser = app
        .state::<AppState>()
        .current_view_mode
        .lock()
        .map(|m| matches!(*m, ViewMode::Browser))
        .unwrap_or(false);
    if is_browser {
        return;
    }
    if let Some(b) = app.get_webview("browser-webview") {
        let _ = b.hide();
    }
}
