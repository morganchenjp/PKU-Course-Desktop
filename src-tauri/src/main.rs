// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod download;
mod ffmpeg;
mod settings;

use download::{DownloadManager, DownloadTask};
use settings::AppSettings;
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, State};
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use url::Url;

const START_URL: &str = "https://course.pku.edu.cn";
const HEADER_HEIGHT: f64 = 48.0;

use std::collections::HashMap;

/// Info about a download triggered via the browser-webview's native HTTP stack.
struct PendingBrowserDownload {
    task_id: String,
    filepath: String,
}

// App state
pub struct AppState {
    download_manager: Mutex<DownloadManager>,
    settings: Mutex<AppSettings>,
    /// Tracks which view mode is active so the resize handler can reposition webviews.
    /// Values: "browser", "video", "main"
    current_view_mode: StdMutex<String>,
    /// URL currently loaded in the video-webview (empty = no video).
    /// Used for idempotent create: if the same URL is requested, just ensure layout.
    video_url: StdMutex<String>,
    /// Last captured video info from video-detector.js (via /video-info IPC).
    /// Embedded into the video-webview so floating download buttons can be shown
    /// without depending on cross-origin iframe script injection.
    video_info: StdMutex<Option<serde_json::Value>>,
    /// Pending browser-webview downloads, keyed by download URL (for on_download handler).
    pending_downloads: StdMutex<HashMap<String, PendingBrowserDownload>>,
}

#[tauri::command]
fn get_default_download_path() -> Result<String, String> {
    dirs::download_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not find downloads directory".to_string())
}

#[tauri::command]
async fn start_download(
    app: tauri::AppHandle,
    task: DownloadTask,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.download_manager.lock().await;
    manager
        .start_download(task, app.clone())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pause_download(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.download_manager.lock().await;
    manager.pause_download(&task_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn open_file_location(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        
        std::process::Command::new("xdg-open")
            .arg(&parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
fn open_external_link(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_settings() -> Result<AppSettings, String> {
    settings::load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    settings::save_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn convert_m3u8_to_mp4(
    m3u8_url: String,
    output_path: String,
    jwt: Option<String>,
) -> Result<(), String> {
    ffmpeg::convert_m3u8_to_mp4(&m3u8_url, &output_path, jwt.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn extract_audio(
    video_path: String,
    output_path: String,
    format: String,
) -> Result<(), String> {
    ffmpeg::extract_audio(&video_path, &output_path, &format)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn navigate_browser(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    let parsed_url: Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    webview.navigate(parsed_url).map_err(|e| e.to_string())
}

#[tauri::command]
fn browser_go_back(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    webview
        .eval("window.history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn browser_go_forward(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    webview
        .eval("window.history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn browser_reload(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or_else(|| "Browser webview not found".to_string())?;
    webview
        .eval("window.location.reload()")
        .map_err(|e| e.to_string())
}

/// Hide the main (Svelte) webview and show the browser webview.
/// Destroys any existing video-webview.
#[tauri::command]
fn show_browser_view(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let main_webview = app.get_webview("main").ok_or("Main webview not found")?;
    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    let window_size = main_window.inner_size().map_err(|e| e.to_string())?;
    let scale = main_window.scale_factor().map_err(|e| e.to_string())?;
    let w = window_size.width as f64 / scale;
    let h = window_size.height as f64 / scale;

    // Ensure browser webview fills the whole window at the correct position
    browser
        .set_position(LogicalPosition::new(0.0, 0.0))
        .map_err(|e| format!("set_pos browser: {e}"))?;
    browser
        .set_size(LogicalSize::new(w, h))
        .map_err(|e| format!("set_size browser: {e}"))?;

    // Destroy video-webview if it exists (free resources)
    if let Some(video_wv) = app.get_webview("video-webview") {
        let _ = video_wv.close();
    }
    if let Ok(mut vurl) = state.video_url.lock() {
        vurl.clear();
    }

    main_webview.hide().map_err(|e| format!("hide main: {e}"))?;
    browser.show().map_err(|e| format!("show browser: {e}"))?;

    if let Ok(mut mode) = state.current_view_mode.lock() {
        *mode = "browser".to_string();
    }

    eprintln!("[Rust] show_browser_view: {}x{}", w, h);
    Ok(())
}

/// Shared logic for switching from browser to main view.
/// Used by both the `show_main_view` command and the `pku-ipc` protocol handler.
/// Destroys any existing video-webview.
fn do_show_main_view(app: &tauri::AppHandle, view: &str) -> Result<(), String> {
    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let main_webview = app.get_webview("main").ok_or("Main webview not found")?;

    if let Some(browser) = app.get_webview("browser-webview") {
        let _ = browser.hide();
    }
    // Destroy video-webview (free resources)
    if let Some(video_wv) = app.get_webview("video-webview") {
        let _ = video_wv.close();
    }
    let app_state = app.state::<AppState>();
    if let Ok(mut vurl) = app_state.video_url.lock() {
        vurl.clear();
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
    main_webview
        .show()
        .map_err(|e| format!("show main: {e}"))?;

    let app_state = app.state::<AppState>();
    if let Ok(mut mode) = app_state.current_view_mode.lock() {
        *mode = "main".to_string();
    }

    // Tell the Svelte app which view to show
    app.emit("switch-to-main", serde_json::json!({ "view": view }))
        .map_err(|e| format!("emit switch-to-main: {e}"))?;

    eprintln!("[Rust] show_main_view: view={}", view);
    Ok(())
}

/// Hide the browser webview and show the main (Svelte) webview.
/// Emits a "switch-to-main" event so the Svelte app can update its view.
#[tauri::command]
fn show_main_view(app: tauri::AppHandle, view: String) -> Result<(), String> {
    do_show_main_view(&app, &view)
}

// ─── Video WebView lifecycle ───────────────────────────────────────────────
// The video-webview is created on demand and destroyed when leaving the video
// tab.  This avoids WebKitGTK z-order bugs with overlapping child webviews.
//
// Layout in video mode (NO overlap):
//   main webview  → (0, 0)            size (w, HEADER_HEIGHT)   ← header only
//   video-webview → (0, HEADER_HEIGHT) size (w, h-HEADER_HEIGHT)

/// Create (or re-use) the video-webview and switch to the split layout.
/// Idempotent: if a video-webview with the same URL already exists the layout
/// is just re-applied.
fn do_create_video_view(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let main_webview = app.get_webview("main").ok_or("Main webview not found")?;

    let window_size = main_window.inner_size().map_err(|e| e.to_string())?;
    let scale = main_window.scale_factor().map_err(|e| e.to_string())?;
    let w = window_size.width as f64 / scale;
    let h = window_size.height as f64 / scale;

    let app_state = app.state::<AppState>();

    // Check if video-webview already exists
    let existing = app.get_webview("video-webview");
    if let Some(ref vw) = existing {
        let same_url = app_state
            .video_url
            .lock()
            .map(|u| u.as_str() == url)
            .unwrap_or(false);
        if same_url {
            // Same URL — just make sure the layout is correct (e.g. after resize)
            if let Some(browser) = app.get_webview("browser-webview") {
                let _ = browser.hide();
            }
            main_webview
                .set_position(LogicalPosition::new(0.0, 0.0))
                .map_err(|e| format!("set_pos main: {e}"))?;
            main_webview
                .set_size(LogicalSize::new(w, HEADER_HEIGHT))
                .map_err(|e| format!("set_size main: {e}"))?;
            main_webview
                .show()
                .map_err(|e| format!("show main: {e}"))?;
            vw.set_position(LogicalPosition::new(0.0, HEADER_HEIGHT))
                .map_err(|e| format!("set_pos video: {e}"))?;
            vw.set_size(LogicalSize::new(w, h - HEADER_HEIGHT))
                .map_err(|e| format!("set_size video: {e}"))?;
            if let Ok(mut mode) = app_state.current_view_mode.lock() {
                *mode = "video".to_string();
            }
            eprintln!("[Rust] create_video_view: same URL, layout ensured");
            return Ok(());
        }
        // Different URL — destroy old webview first
        let _ = vw.close();
        eprintln!("[Rust] create_video_view: closed old video-webview");
    }

    // Hide browser-webview
    if let Some(browser) = app.get_webview("browser-webview") {
        let _ = browser.hide();
    }

    // Shrink main webview to header-only strip (no overlap with video-webview)
    main_webview
        .set_position(LogicalPosition::new(0.0, 0.0))
        .map_err(|e| format!("set_pos main: {e}"))?;
    main_webview
        .set_size(LogicalSize::new(w, HEADER_HEIGHT))
        .map_err(|e| format!("set_size main: {e}"))?;
    main_webview
        .show()
        .map_err(|e| format!("show main: {e}"))?;

    // Build and create the video-webview
    let hls_min_js = include_str!("../inject-scripts/hls.min.js");
    let hls_player_js = include_str!("../inject-scripts/hls-player.js");
    let vid_detector_js = include_str!("../inject-scripts/video-detector.js");
    let overscroll_css = r#"(function(){
        var s=document.createElement('style');
        s.textContent='html,body{overscroll-behavior:none!important}';
        (document.head||document.documentElement).appendChild(s);
    })();"#;

    // Embed stored video info so the top frame can show download buttons
    // without depending on cross-origin iframe script injection.
    let video_info_js = {
        if let Ok(vi) = app_state.video_info.lock() {
            if let Some(ref info) = *vi {
                if let Ok(json_str) = serde_json::to_string(info) {
                    format!("window.__PKU_VIDEO_INFO__ = {};", json_str)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    };

    let parsed_url: Url = url
        .parse()
        .unwrap_or_else(|_| START_URL.parse().unwrap());
    let mut builder = tauri::webview::WebviewBuilder::new(
        "video-webview",
        tauri::WebviewUrl::External(parsed_url),
    )
    .initialization_script(hls_min_js)
    .initialization_script(hls_player_js)
    .initialization_script(vid_detector_js)
    .initialization_script(overscroll_css)
    .on_download(|webview, event| {
        handle_download_event(&webview, event)
    });

    if !video_info_js.is_empty() {
        builder = builder.initialization_script(&video_info_js);
        eprintln!("[Rust] create_video_view: embedding video_info into webview");
    }

    main_window
        .add_child(
            builder,
            LogicalPosition::new(0.0, HEADER_HEIGHT),
            LogicalSize::new(w, h - HEADER_HEIGHT),
        )
        .map_err(|e| format!("add_child video: {e}"))?;

    // Update state
    if let Ok(mut vurl) = app_state.video_url.lock() {
        *vurl = url.to_string();
    }
    if let Ok(mut mode) = app_state.current_view_mode.lock() {
        *mode = "video".to_string();
    }

    eprintln!(
        "[Rust] create_video_view: {}x{} (video at y={}) url={}",
        w,
        h - HEADER_HEIGHT,
        HEADER_HEIGHT,
        url
    );
    Ok(())
}

/// Destroy the video-webview and restore the main webview to full window size.
/// Idempotent: safe to call when no video-webview exists.
fn do_destroy_video_view(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(video_wv) = app.get_webview("video-webview") {
        let _ = video_wv.close();
        eprintln!("[Rust] destroy_video_view: closed video-webview");
    }

    let app_state = app.state::<AppState>();
    if let Ok(mut vurl) = app_state.video_url.lock() {
        vurl.clear();
    }

    // Restore main webview to full window size
    let main_window = app.get_window("main").ok_or("Main window not found")?;
    let main_webview = app.get_webview("main").ok_or("Main webview not found")?;

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
    main_webview
        .show()
        .map_err(|e| format!("show main: {e}"))?;

    Ok(())
}

/// Tauri command: create (or re-use) the video-webview.
#[tauri::command]
fn create_video_view(app: tauri::AppHandle, url: String) -> Result<(), String> {
    do_create_video_view(&app, &url)
}

/// Tauri command: destroy the video-webview and restore main to full size.
#[tauri::command]
fn destroy_video_view(app: tauri::AppHandle) -> Result<(), String> {
    do_destroy_video_view(&app)?;
    let app_state = app.state::<AppState>();
    if let Ok(mut mode) = app_state.current_view_mode.lock() {
        *mode = "main".to_string();
    }
    Ok(())
}

/// Download a video file using WebKitGTK's native `download_uri()` API.
///
/// This uses the browser-webview's underlying `webkit2gtk::WebView` to initiate
/// the download, which means it carries the full session (including httpOnly
/// cookies like JSESSIONID and BbRouter) that the server requires.
///
/// The command returns immediately after setting up the download.  Progress,
/// completion and error notifications are delivered via Tauri events:
///   - `download-progress`  { taskId, progress, speed, eta }
///   - `download-complete`  { taskId }
///   - `download-error`     { taskId, error }
#[tauri::command]
async fn browser_download(
    app: tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&filepath).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }

    eprintln!("[browser_download] starting webkit download_uri for {task_id}");
    eprintln!("[browser_download] url: {url}");
    eprintln!("[browser_download] filepath: {filepath}");

    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    // Ensure browser-webview is visible (WebKitGTK may not process network
    // requests for hidden webviews).  Position off-screen so the user won't
    // see it flash.
    let _ = browser.set_position(tauri::LogicalPosition::new(10000.0, 0.0));
    let _ = browser.show();

    let url_clone = url.clone();
    let filepath_clone = filepath.clone();
    let task_id_clone = task_id.clone();
    let app_clone = app.clone();

    browser
        .with_webview(move |platform_wv| {
            use webkit2gtk::{WebViewExt, DownloadExt};

            let wk_webview: webkit2gtk::WebView = platform_wv.inner();

            match wk_webview.download_uri(&url_clone) {
                Some(download) => {
                    eprintln!("[webkit-dl] download_uri() returned Download object");

                    // ── decide-destination: set the correct file path ──
                    let fp = filepath_clone.clone();
                    download.connect_decide_destination(move |dl, suggested| {
                        let dest_uri = format!("file://{}", fp);
                        eprintln!(
                            "[webkit-dl] decide-destination: suggested={suggested}, dest={dest_uri}"
                        );
                        dl.set_allow_overwrite(true);
                        dl.set_destination(&dest_uri);
                        true
                    });

                    // ── received-data: report progress (throttled to ~2 Hz) ──
                    let app_p = app_clone.clone();
                    let tid_p = task_id_clone.clone();
                    let start = std::time::Instant::now();
                    let last_emit = std::cell::Cell::new(std::time::Instant::now());
                    download.connect_received_data(move |dl, _chunk_len| {
                        if last_emit.get().elapsed().as_millis() < 500 {
                            return;
                        }
                        last_emit.set(std::time::Instant::now());

                        let progress = dl.estimated_progress() * 100.0;
                        let received = dl.received_data_length();
                        let elapsed = start.elapsed().as_secs_f64();
                        let speed = if elapsed > 0.0 {
                            received as f64 / elapsed
                        } else {
                            0.0
                        };
                        let eta_str = if speed > 0.0 && progress > 0.0 {
                            let total_est = received as f64 / (progress / 100.0);
                            let remaining = (total_est - received as f64) / speed;
                            if remaining >= 3600.0 {
                                format!(
                                    "{:.0}h {:.0}m",
                                    remaining / 3600.0,
                                    (remaining % 3600.0) / 60.0
                                )
                            } else if remaining >= 60.0 {
                                format!("{:.0}m {:.0}s", remaining / 60.0, remaining % 60.0)
                            } else {
                                format!("{:.0}s", remaining)
                            }
                        } else {
                            fmt_size(received)
                        };

                        let _ = app_p.emit(
                            "download-progress",
                            serde_json::json!({
                                "taskId": tid_p,
                                "progress": progress,
                                "speed": fmt_speed(speed),
                                "eta": eta_str,
                            }),
                        );
                    });

                    // ── finished: download completed successfully ──
                    let app_f = app_clone.clone();
                    let tid_f = task_id_clone.clone();
                    let fp_f = filepath_clone.clone();
                    download.connect_finished(move |dl| {
                        let received = dl.received_data_length();
                        eprintln!(
                            "[webkit-dl] finished: task={tid_f} bytes={received} dest={fp_f}"
                        );
                        let _ = app_f.emit(
                            "download-complete",
                            serde_json::json!({ "taskId": tid_f }),
                        );
                        // Re-hide browser-webview if not in browser mode
                        let state = app_f.state::<AppState>();
                        let mode = state
                            .current_view_mode
                            .lock()
                            .map(|m| m.clone())
                            .unwrap_or_default();
                        if mode != "browser" {
                            if let Some(b) = app_f.get_webview("browser-webview") {
                                let _ = b.hide();
                                eprintln!("[webkit-dl] browser-webview re-hidden");
                            }
                        }
                    });

                    // ── failed: download error ──
                    let app_e = app_clone.clone();
                    let tid_e = task_id_clone.clone();
                    download.connect_failed(move |_dl, error| {
                        let msg = error.to_string();
                        eprintln!("[webkit-dl] failed: task={tid_e} error={msg}");
                        let _ = app_e.emit(
                            "download-error",
                            serde_json::json!({
                                "taskId": tid_e,
                                "error": msg,
                            }),
                        );
                        // Re-hide browser-webview if not in browser mode
                        let state = app_e.state::<AppState>();
                        let mode = state
                            .current_view_mode
                            .lock()
                            .map(|m| m.clone())
                            .unwrap_or_default();
                        if mode != "browser" {
                            if let Some(b) = app_e.get_webview("browser-webview") {
                                let _ = b.hide();
                                eprintln!("[webkit-dl] browser-webview re-hidden (after error)");
                            }
                        }
                    });
                }
                None => {
                    eprintln!("[webkit-dl] download_uri() returned None!");
                    let _ = app_clone.emit(
                        "download-error",
                        serde_json::json!({
                            "taskId": task_id_clone,
                            "error": "WebKitGTK download_uri returned None",
                        }),
                    );
                }
            }
        })
        .map_err(|e| format!("with_webview: {e}"))?;

    Ok(())
}

fn fmt_speed(bps: f64) -> String {
    if bps > 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bps / (1024.0 * 1024.0))
    } else if bps > 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}

fn fmt_size(bytes: u64) -> String {
    if bytes > 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes > 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Shared on_download handler logic for any webview.
/// Returns true to allow the download, false to reject.
fn handle_download_event(
    webview: &tauri::Webview,
    event: tauri::webview::DownloadEvent,
) -> bool {
    use tauri::webview::DownloadEvent;
    let wv_label = webview.label().to_string();
    match event {
        DownloadEvent::Requested { url, destination } => {
            let url_str = url.to_string();
            eprintln!("[on_download:{wv_label}] Requested: {}", url_str);
            let state = webview.state::<AppState>();
            if let Ok(pending) = state.pending_downloads.lock() {
                // Try exact URL match first
                if let Some(info) = pending.get(&url_str) {
                    *destination = std::path::PathBuf::from(&info.filepath);
                    eprintln!(
                        "[on_download:{wv_label}] matched (exact): {} -> {}",
                        url_str, info.filepath
                    );
                    return true;
                }
                // Fallback: URL may differ due to redirect.
                if let Some((_orig_url, info)) = pending.iter().next() {
                    *destination = std::path::PathBuf::from(&info.filepath);
                    eprintln!(
                        "[on_download:{wv_label}] matched (fallback): {} -> {} (orig: {})",
                        url_str, info.filepath, _orig_url
                    );
                    return true;
                }
            }
            eprintln!("[on_download:{wv_label}] untracked download: {}", url_str);
            true
        }
        DownloadEvent::Finished { url, path, success } => {
            let url_str = url.to_string();
            eprintln!(
                "[on_download:{wv_label}] Finished: url={} path={:?} success={}",
                url_str, path, success
            );
            let state = webview.state::<AppState>();
            let pending_info = state
                .pending_downloads
                .lock()
                .ok()
                .and_then(|mut p| {
                    if let Some(info) = p.remove(&url_str) {
                        return Some(info);
                    }
                    let key = p.keys().next().cloned();
                    key.and_then(|k| p.remove(&k))
                });
            if let Some(info) = pending_info {
                let app = webview.app_handle();
                if success {
                    let _ = app.emit(
                        "download-complete",
                        serde_json::json!({ "taskId": info.task_id }),
                    );
                    eprintln!(
                        "[on_download:{wv_label}] completed: task={} path={:?}",
                        info.task_id, path
                    );
                } else {
                    let _ = app.emit(
                        "download-error",
                        serde_json::json!({
                            "taskId": info.task_id,
                            "error": format!("Browser download failed ({})", wv_label)
                        }),
                    );
                    eprintln!("[on_download:{wv_label}] failed: task={}", info.task_id);
                }
            } else {
                eprintln!(
                    "[on_download:{wv_label}] Finished but no pending entry found"
                );
            }
            // Re-hide browser-webview if not in browser mode
            let mode = webview
                .state::<AppState>()
                .current_view_mode
                .lock()
                .map(|m| m.clone())
                .unwrap_or_default();
            if mode != "browser" {
                if let Some(b) = webview.app_handle().get_webview("browser-webview") {
                    let _ = b.hide();
                    eprintln!("[on_download:{wv_label}] browser-webview re-hidden");
                }
            }
            // Destroy temporary download webview (label starts with "dl-")
            if wv_label.starts_with("dl-") {
                let app = webview.app_handle().clone();
                let label = wv_label.clone();
                // Defer close to avoid borrow issues inside callback
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Some(wv) = app.get_webview(&label) {
                        let _ = wv.close();
                        eprintln!("[on_download] temp webview '{label}' destroyed");
                    }
                });
            }
            true
        }
        _ => true,
    }
}

/// Navigate the browser webview to the start URL.
#[tauri::command]
fn browser_go_home(app: tauri::AppHandle) -> Result<(), String> {
    let webview = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;
    let parsed: Url = START_URL.parse().unwrap();
    webview.navigate(parsed).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        // notification plugin removed: it causes "is_permission_granted not allowed"
        // errors on the browser-webview at remote URLs.
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(AppState {
            download_manager: Mutex::new(DownloadManager::new()),
            settings: Mutex::new(AppSettings::default()),
            current_view_mode: StdMutex::new("browser".to_string()),
            video_url: StdMutex::new(String::new()),
            video_info: StdMutex::new(None),
            pending_downloads: StdMutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_default_download_path,
            start_download,
            pause_download,
            open_file_location,
            open_external_link,
            load_settings,
            save_settings,
            convert_m3u8_to_mp4,
            extract_audio,
            navigate_browser,
            browser_go_back,
            browser_go_forward,
            browser_reload,
            browser_go_home,
            browser_download,
            show_browser_view,
            show_main_view,
            create_video_view,
            destroy_video_view,
        ])
        // ─── Custom IPC protocol for inject scripts ───
        // Inject scripts in the browser-webview (remote URLs) cannot use
        // window.__TAURI__ due to capability restrictions.  Instead they send
        // requests to this custom "pku-ipc" URI scheme via XMLHttpRequest.
        .register_uri_scheme_protocol("pku-ipc", |ctx, request| {
            let app = ctx.app_handle();

            // Handle CORS preflight
            if *request.method() == "OPTIONS" {
                return tauri::http::Response::builder()
                    .status(200)
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                    .header("Access-Control-Allow-Headers", "Content-Type")
                    .body(Vec::new())
                    .unwrap();
            }

            let uri = request.uri().to_string();

            if uri.contains("/download-diag") {
                let body_str = String::from_utf8_lossy(request.body());
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let msg = value.get("msg").and_then(|v| v.as_str()).unwrap_or(&body_str);
                    eprintln!("[download-diag] {msg}");
                } else {
                    eprintln!("[download-diag] {body_str}");
                }
            } else if uri.contains("/show-main-view") {
                let view = if uri.contains("view=settings") {
                    "settings"
                } else {
                    "downloads"
                };
                if let Err(e) = do_show_main_view(app, view) {
                    eprintln!("[pku-ipc] show_main_view error: {e}");
                }
            } else if uri.contains("/video-info") {
                let body_str = String::from_utf8_lossy(request.body());
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    // Store for embedding into video-webview later
                    let app_state = app.state::<AppState>();
                    if let Ok(mut vi) = app_state.video_info.lock() {
                        *vi = Some(value.clone());
                    }
                    let payload = serde_json::json!({
                        "type": "video-info",
                        "data": value
                    });
                    let _ = app.emit("webview-message", payload);
                    eprintln!("[pku-ipc] video-info stored and emitted");
                }
            } else if uri.contains("/add-download") {
                let body_str = String::from_utf8_lossy(request.body());
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let _ = app.emit("add-download-from-browser", value);
                    eprintln!("[pku-ipc] add-download emitted");
                }
            } else if uri.contains("/open-external") {
                let body_str = String::from_utf8_lossy(request.body());
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                        match open::that(url) {
                            Ok(_) => eprintln!("[pku-ipc] open-external: {url}"),
                            Err(e) => eprintln!("[pku-ipc] open-external error: {e}"),
                        }
                    }
                }
            } else if uri.contains("/open-video") {
                let body_str = String::from_utf8_lossy(request.body());
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                        eprintln!("[pku-ipc] open-video: {url}");

                        // Create (or re-use) the video-webview with the split layout
                        if let Err(e) = do_create_video_view(app, url) {
                            eprintln!("[pku-ipc] create_video_view error: {e}");
                        }

                        // Tell Svelte to switch to the video tab
                        let _ = app.emit("switch-to-video", serde_json::json!({ "url": url }));
                    }
                }
            }

            tauri::http::Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .body(b"ok".to_vec())
                .unwrap()
        })
        .setup(|app| {
            // Load settings on startup
            if let Ok(settings) = settings::load_settings() {
                let state = app.state::<AppState>();
                let mut app_settings = state.settings.blocking_lock();
                *app_settings = settings;
            }

            // Pre-create the browser webview (hidden) at (0,0) full window size
            // with injected scripts for navigation bar and video detection.
            let main_window = app
                .get_window("main")
                .expect("main window not found during setup");

            let window_size = main_window.inner_size().unwrap_or_default();
            let scale = main_window.scale_factor().unwrap_or(1.0);
            let w = window_size.width as f64 / scale;
            let h = window_size.height as f64 / scale;

            let nav_bar_js = include_str!("../inject-scripts/nav-bar.js");
            let video_detector_js = include_str!("../inject-scripts/video-detector.js");

            let parsed_url: Url = START_URL.parse().expect("invalid START_URL");
            let builder = tauri::webview::WebviewBuilder::new(
                "browser-webview",
                tauri::WebviewUrl::External(parsed_url),
            )
            .initialization_script(nav_bar_js)
            .initialization_script(video_detector_js)
            .on_download(|webview, event| {
                handle_download_event(&webview, event)
            });

            match main_window.add_child(
                builder,
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(w, h),
            ) {
                Ok(browser) => {
                    // Start hidden; BrowserView.svelte will call show_browser_view
                    let _ = browser.hide();
                    eprintln!("[Rust] browser-webview pre-created (hidden, {}x{})", w, h);
                }
                Err(e) => {
                    eprintln!("[Rust] failed to pre-create browser-webview: {e}");
                }
            }

            // ─── Window resize handler ───
            // Reposition webviews when the window is resized, based on current view mode.
            let app_handle = app.handle().clone();
            let mw = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(size) = event {
                    let scale = mw.scale_factor().unwrap_or(1.0);
                    let w = size.width as f64 / scale;
                    let h = size.height as f64 / scale;

                    let state = app_handle.state::<AppState>();
                    let mode = state
                        .current_view_mode
                        .lock()
                        .map(|m| m.clone())
                        .unwrap_or_else(|e| e.into_inner().clone());

                    match mode.as_str() {
                        "browser" => {
                            if let Some(browser) = app_handle.get_webview("browser-webview") {
                                let _ = browser.set_position(LogicalPosition::new(0.0, 0.0));
                                let _ = browser.set_size(LogicalSize::new(w, h));
                            }
                        }
                        "video" => {
                            // Split layout: main = header only, video = rest
                            if let Some(main_wv) = app_handle.get_webview("main") {
                                let _ = main_wv.set_position(LogicalPosition::new(0.0, 0.0));
                                let _ = main_wv.set_size(LogicalSize::new(w, HEADER_HEIGHT));
                            }
                            if let Some(video_wv) = app_handle.get_webview("video-webview") {
                                let _ = video_wv.set_position(LogicalPosition::new(0.0, HEADER_HEIGHT));
                                let _ = video_wv.set_size(LogicalSize::new(w, h - HEADER_HEIGHT));
                            }
                        }
                        _ => {
                            // "main" mode (downloads/settings) - main webview full size
                            if let Some(main_wv) = app_handle.get_webview("main") {
                                let _ = main_wv.set_position(LogicalPosition::new(0.0, 0.0));
                                let _ = main_wv.set_size(LogicalSize::new(w, h));
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
