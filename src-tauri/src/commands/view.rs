//! Thin command wrappers that delegate the actual work to `webview::layout`.
//! The layout module is the single source of truth for view-switching
//! semantics and timing — these commands exist only to expose the same
//! logic to the frontend invoke channel.

use crate::util::log::debug_log;
use crate::webview::layout;

/// Hide the main (Svelte) webview and show the browser webview.
#[tauri::command]
pub fn show_browser_view(app: tauri::AppHandle) -> Result<(), String> {
    layout::show_browser_view(&app)
}

/// Hide the browser webview and show the main (Svelte) webview.
/// Emits a "switch-to-main" event so the Svelte app can update its view.
#[tauri::command]
pub fn show_main_view(app: tauri::AppHandle, view: String) -> Result<(), String> {
    debug_log(&format!("show_main_view triggered: view={}", view));
    layout::do_show_main_view(&app, &view)
}
