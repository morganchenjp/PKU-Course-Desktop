//! View-switching and window-resize layout logic.
//!
//! This file is OFF-LIMITS for behavior changes — only the relocation from
//! `main.rs` and the `ViewMode` enum migration are permitted.  In particular:
//! - The 150 ms deferred `switch-to-main` emit is preserved exactly.
//! - The off-screen positioning belt-and-suspenders is preserved.
//! - The `set_focus()` call after `show()` is preserved (macOS responder
//!   chain quirk).

use serde_json::json;
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager};

use crate::state::{AppState, ViewMode};
use crate::util::log::debug_log;

/// Hide the main (Svelte) webview and show the browser webview.
pub fn show_browser_view(app: &tauri::AppHandle) -> Result<(), String> {
    debug_log("show_browser_view triggered");

    // Set mode BEFORE operations so that if anything fails, state is correct for retry
    let state = app.state::<AppState>();
    let mut mode = state.current_view_mode.lock().map_err(|e| e.to_string())?;
    *mode = ViewMode::Browser;
    drop(mode);

    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let main_webview = app.get_webview("main").ok_or("Main webview not found")?;
    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    let window_size = main_window.inner_size().map_err(|e| e.to_string())?;
    let scale = main_window.scale_factor().map_err(|e| e.to_string())?;
    let w = window_size.width as f64 / scale;
    let h = window_size.height as f64 / scale;

    // Get the header height (48px in CSS, but we need to account for scale)
    let header_height = 48.0;

    // Position browser webview below the header
    browser
        .set_position(LogicalPosition::new(0.0, header_height))
        .map_err(|e| format!("set_pos browser: {e}"))?;
    browser
        .set_size(LogicalSize::new(w, h - header_height))
        .map_err(|e| format!("set_size browser: {e}"))?;

    main_webview.hide().map_err(|e| format!("hide main: {e}"))?;
    // Belt-and-suspenders: move off-screen as well since macOS child-webview
    // hide() alone may not reliably lower the view from the responder chain.
    let _ = main_webview.set_position(LogicalPosition::new(10000.0, 0.0));
    browser.show().map_err(|e| format!("show browser: {e}"))?;
    let _ = browser.set_focus(); // Ensure browser webview receives input focus

    debug_log("show_browser_view: hide(main) + show(browser) complete");

    eprintln!(
        "[Rust] show_browser_view: {}x{} at (0, {})",
        w,
        h - header_height,
        header_height
    );
    Ok(())
}

/// Shared logic for switching from browser to main view.
/// Used by both the `show_main_view` command and the `pku-ipc` protocol handler.
pub fn do_show_main_view(app: &tauri::AppHandle, view: &str) -> Result<(), String> {
    debug_log(&format!("do_show_main_view called: view={}", view));

    // Set mode BEFORE operations so that if anything fails, state is correct for retry
    let app_state = app.state::<AppState>();
    let mut mode = app_state.current_view_mode.lock().map_err(|e| e.to_string())?;
    *mode = ViewMode::Main;
    drop(mode);

    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let main_webview = app.get_webview("main").ok_or("Main webview not found")?;

    // Hide browser and move off-screen as belt-and-suspenders.
    // hide() alone works on macOS/Windows; on Linux/WebKitGTK we also move off-screen
    // since hide() may not reliably lower Z-order in all configurations.
    if let Some(browser) = app.get_webview("browser-webview") {
        let _ = browser.hide();
        let _ = browser.set_position(LogicalPosition::new(10000.0, 48.0));
    }

    let window_size = main_window.inner_size().map_err(|e| e.to_string())?;
    let scale = main_window.scale_factor().map_err(|e| e.to_string())?;
    let w = window_size.width as f64 / scale;
    let h = window_size.height as f64 / scale;

    main_webview
        .set_position(LogicalPosition::new(0.0, 0.0))
        .map_err(|e| format!("set_pos main: {e}"))?;
    main_webview
        .set_size(LogicalSize::new(w, h))
        .map_err(|e| format!("set_size main: {e}"))?;
    main_webview.show().map_err(|e| format!("show main: {e}"))?;
    let _ = main_webview.set_focus(); // Ensure main webview receives input focus

    debug_log("do_show_main_view: hide(browser) + show(main) complete");

    // Tell the Svelte app which view to show.
    // On macOS/WKWebView a hidden webview has its JS execution paused.
    // After show() it takes a short time for the JS event loop to resume.
    // Emitting immediately can cause the event to be lost, so we defer.
    let app_clone = app.clone();
    let view_clone = view.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        let state = app_clone.state::<AppState>();
        let is_main = state
            .current_view_mode
            .lock()
            .map(|m| matches!(*m, ViewMode::Main))
            .unwrap_or(false);
        if is_main {
            let _ = app_clone.emit("switch-to-main", json!({ "view": view_clone }));
            eprintln!(
                "[Rust] switch-to-main emitted (deferred): view={}",
                view_clone
            );
        }
    });

    eprintln!("[Rust] show_main_view: view={}", view);
    Ok(())
}

/// Reposition webviews on a window-resize event based on the active view mode.
/// Called from the closure passed to `Window::on_window_event` in `setup`.
pub fn handle_window_resize(
    app: &tauri::AppHandle,
    main_window: &tauri::Window,
    new_size: tauri::PhysicalSize<u32>,
) {
    let scale = main_window.scale_factor().unwrap_or(1.0);
    let w = new_size.width as f64 / scale;
    let h = new_size.height as f64 / scale;

    let state = app.state::<AppState>();
    let mode = state
        .current_view_mode
        .lock()
        .map(|m| *m)
        .unwrap_or(ViewMode::Browser);

    match mode {
        ViewMode::Browser => {
            // Browser mode: webview below header (48px)
            let header_height = 48.0;
            if let Some(browser) = app.get_webview("browser-webview") {
                let _ = browser.set_position(LogicalPosition::new(0.0, header_height));
                let _ = browser.set_size(LogicalSize::new(w, h - header_height));
            }
        }
        ViewMode::Main => {
            // Main mode (downloads/settings) - main webview full size
            if let Some(main_wv) = app.get_webview("main") {
                let _ = main_wv.set_position(LogicalPosition::new(0.0, 0.0));
                let _ = main_wv.set_size(LogicalSize::new(w, h));
            }
        }
    }
}
