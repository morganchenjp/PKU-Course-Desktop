//! App `setup()` body: load settings, set window icon, pre-create the
//! browser-webview (hidden, with the four inject scripts), hide the main
//! webview, and register the window-resize handler.
//!
//! The four inject scripts (`nav-bar.js`, `video-detector.js`,
//! `hls.min.js`, `hls-player.js`) are NOT modified — they are just included
//! verbatim via `include_str!` like before.

use tauri::image::Image;
use tauri::{LogicalPosition, LogicalSize, Manager};
use url::Url;

use crate::settings;
use crate::state::AppState;
use crate::webview::layout::handle_window_resize;
use crate::webview::on_download::handle_download_event;

const START_URL: &str = "https://course.pku.edu.cn";

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Load settings on startup
    if let Ok(settings) = settings::load_settings() {
        let state = app.state::<AppState>();
        let mut app_settings = state.settings.lock().unwrap();
        *app_settings = settings;
    }

    // Pre-create the browser webview (hidden) at (0,0) full window size
    // with injected scripts for navigation bar and video detection.
    let main_window = app
        .get_window("main")
        .expect("main window not found during setup");

    // Set window icon (for Linux Dock / Windows taskbar)
    let icon_bytes = include_bytes!("../../icons/icon.png");
    if let Ok(icon) = Image::from_bytes(icon_bytes) {
        let _ = main_window.set_icon(icon);
    }

    let window_size = main_window.inner_size().unwrap_or_default();
    let scale = main_window.scale_factor().unwrap_or(1.0);
    let w = window_size.width as f64 / scale;
    let h = window_size.height as f64 / scale;

    let nav_bar_js = include_str!("../../inject-scripts/nav-bar.js");
    let video_detector_js = include_str!("../../inject-scripts/video-detector.js");
    let hls_min_js = include_str!("../../inject-scripts/hls.min.js");
    let hls_player_js = include_str!("../../inject-scripts/hls-player.js");

    let parsed_url: Url = START_URL.parse().expect("invalid START_URL");
    let builder = tauri::webview::WebviewBuilder::new(
        "browser-webview",
        tauri::WebviewUrl::External(parsed_url),
    )
    .initialization_script(nav_bar_js)
    .initialization_script(video_detector_js)
    .initialization_script(hls_min_js)
    .initialization_script(hls_player_js)
    .on_download(|webview, event| handle_download_event(&webview, event));

    match main_window.add_child(
        builder,
        LogicalPosition::new(0.0, 48.0),
        LogicalSize::new(w, h - 48.0),
    ) {
        Ok(browser) => {
            // Start hidden; BrowserView.svelte will call show_browser_view
            let _ = browser.hide();
            eprintln!(
                "[Rust] browser-webview pre-created (hidden, {}x{} at 0,48)",
                w,
                h - 48.0
            );
        }
        Err(e) => {
            eprintln!("[Rust] failed to pre-create browser-webview: {e}");
        }
    }

    // Explicitly hide the main webview at startup so only browser-webview is visible.
    // On macOS/WKWebView a hidden webview doesn't execute JS, so listeners registered
    // in Svelte's onMount won't fire from a hidden state.
    if let Some(main_wv) = app.get_webview("main") {
        let _ = main_wv.hide();
        eprintln!("[Rust] main webview explicitly hidden at startup");
    }

    // ─── Window resize handler ───
    // Reposition webviews when the window is resized, based on current view mode.
    let app_handle = app.handle().clone();
    let mw = main_window.clone();
    main_window.on_window_event(move |event| {
        if let tauri::WindowEvent::Resized(size) = event {
            handle_window_resize(&app_handle, &mw, *size);
        }
    });

    Ok(())
}

/// Re-export of the start URL so the `browser_go_home` command can use it.
pub const fn start_url() -> &'static str {
    START_URL
}
