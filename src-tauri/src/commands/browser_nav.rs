//! Browser-webview navigation commands: navigate, back / forward / reload,
//! and "go home" (to the configured `START_URL`).
//!
//! All commands look up the browser-webview by its `"browser-webview"` label
//! (created in `webview::setup`) and either call `navigate(...)` directly or
//! `eval(...)` a tiny snippet of JS for back/forward/reload.

use tauri::Manager;
use url::Url;

use crate::webview::setup::start_url;

#[tauri::command]
pub fn navigate_browser(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    let parsed_url: Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    webview.navigate(parsed_url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_go_back(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    webview
        .eval("window.history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_go_forward(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    webview
        .eval("window.history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_reload(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    webview
        .eval("window.location.reload()")
        .map_err(|e| e.to_string())
}

/// Navigate the browser webview to the start URL.
#[tauri::command]
pub fn browser_go_home(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;
    let parsed: Url = start_url().parse().unwrap();
    webview.navigate(parsed).map_err(|e| e.to_string())
}
