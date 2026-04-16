// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod download;
mod ffmpeg;
mod settings;

use download::{DownloadManager, DownloadTask};
use settings::AppSettings;
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, State};
use tauri::image::Image;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use url::Url;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::io::Write;

const START_URL: &str = "https://course.pku.edu.cn";

// Donation QR code PNG baked in at compile time — no runtime file paths needed
const DONATION_QR_PNG: &[u8] = include_bytes!("../../public/morgan-wechat-qrcode.png");

/// Write a debug message to a log file.
/// - Linux: $HOME/.local/share/pku-course-desktop/pku-course-desktop.log
/// - macOS: PKU Course Desktop.app/Contents/MacOS/pku-course-desktop.log 
/// Windows: same directory as the app executable
fn debug_log(msg: &str) {
    let ts = chrono::Local::now().format("%H:%M:%S%.3f");

    let log_path = if cfg!(target_os = "linux") {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pku-course-desktop");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("pku-course-desktop.log")
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pku-course-desktop.log")
    };

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "{} [DEBUG] {}", ts, msg);
    }
    eprintln!("{} [DEBUG] {}", ts, msg);
}

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
    /// Values: "browser", "main"
    current_view_mode: StdMutex<String>,
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
    let settings = state.settings.lock().await.clone();
    let manager = state.download_manager.lock().await;
    manager
        .start_download(task, app.clone(), settings.extract_audio, settings.audio_format)
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
#[tauri::command]
fn show_browser_view(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    debug_log("show_browser_view triggered");

    // Set mode BEFORE operations so that if anything fails, state is correct for retry
    let mut mode = state.current_view_mode.lock().map_err(|e| e.to_string())?;
    *mode = "browser".to_string();

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
    browser.show().map_err(|e| format!("show browser: {e}"))?;
    let _ = browser.set_focus(); // Ensure browser webview receives input focus

    debug_log("show_browser_view: hide(main) + show(browser) complete");

    eprintln!("[Rust] show_browser_view: {}x{} at (0, {})", w, h - header_height, header_height);
    Ok(())
}

/// Shared logic for switching from browser to main view.
/// Used by both the `show_main_view` command and the `pku-ipc` protocol handler.
fn do_show_main_view(app: &tauri::AppHandle, view: &str) -> Result<(), String> {
    debug_log(&format!("do_show_main_view called: view={}", view));

    // Set mode BEFORE operations so that if anything fails, state is correct for retry
    let app_state = app.state::<AppState>();
    let mut mode = app_state.current_view_mode.lock().map_err(|e| e.to_string())?;
    *mode = "main".to_string();

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
    main_webview
        .show()
        .map_err(|e| format!("show main: {e}"))?;
    let _ = main_webview.set_focus(); // Ensure main webview receives input focus

    debug_log("do_show_main_view: hide(browser) + show(main) complete");

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
    debug_log(&format!("show_main_view triggered: view={}", view));
    do_show_main_view(&app, &view)
}

/// Download a video file using the platform's native webview download API.
///
/// On Linux (WebKitGTK): uses `download_uri()` which carries the full browser
/// session (including httpOnly cookies like JSESSIONID and BbRouter).
///
/// On macOS/Windows: falls back to reqwest streaming download with the JWT
/// token already embedded in the URL.
///
/// The command returns immediately on Linux.  Progress, completion and error
/// notifications are delivered via Tauri events:
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

    eprintln!("[browser_download] starting download for {task_id}");
    eprintln!("[browser_download] url: {url}");
    eprintln!("[browser_download] filepath: {filepath}");

    #[cfg(target_os = "linux")]
    {
        browser_download_linux(&app, &task_id, &url, &filepath)?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        browser_download_fallback(&app, task_id, url, filepath).await?;
    }

    Ok(())
}

/// Linux implementation: use WebKitGTK's native `download_uri()`.
#[cfg(target_os = "linux")]
fn browser_download_linux(
    app: &tauri::AppHandle,
    task_id: &str,
    url: &str,
    filepath: &str,
) -> Result<(), String> {
    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    // Ensure browser-webview is visible (WebKitGTK may not process network
    // requests for hidden webviews).  Position off-screen so the user won't
    // see it flash.
    let _ = browser.set_position(tauri::LogicalPosition::new(10000.0, 48.0));
    let _ = browser.show();

    let url_clone = url.to_string();
    let filepath_clone = filepath.to_string();
    let task_id_clone = task_id.to_string();
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
                    let app_state_f = app_f.state::<AppState>();
                    let (extract_audio, audio_format) = {
                        let settings = app_state_f.settings.blocking_lock();
                        (settings.extract_audio, settings.audio_format.clone())
                    };
                    drop(app_state_f);
                    let app_f2 = app_f.clone();
                    download.connect_finished(move |dl| {
                        let received = dl.received_data_length();
                        eprintln!(
                            "[webkit-dl] finished: task={tid_f} bytes={received} dest={fp_f}"
                        );
                        // Re-hide browser-webview if not in browser mode
                        let state = app_f.state::<AppState>();
                        let mode = state
                            .current_view_mode
                            .lock()
                            .map(|m| m.clone())
                            .unwrap_or_default();
                        drop(state);
                        if mode != "browser" {
                            if let Some(b) = app_f.get_webview("browser-webview") {
                                let _ = b.hide();
                                eprintln!("[webkit-dl] browser-webview re-hidden");
                            }
                        }
                        // Emit download-complete first, then extract audio asynchronously
                        let _ = app_f.emit(
                            "download-complete",
                            serde_json::json!({ "taskId": tid_f }),
                        );
                        if extract_audio {
                            let fp = fp_f.clone();
                            let tid = tid_f.clone();
                            let af = audio_format.clone();
                            let app_audio = app_f2.clone();
                            tokio::spawn(async move {
                                let audio_path = format!("{}.{}", fp, af);
                                eprintln!("[webkit-dl] extracting audio to: {}", audio_path);
                                match crate::ffmpeg::extract_audio(&fp, &audio_path, &af).await {
                                    Ok(()) => {
                                        let _ = app_audio.emit(
                                            "audio-extract-complete",
                                            serde_json::json!({ "taskId": tid, "audioPath": audio_path }),
                                        );
                                        eprintln!("[webkit-dl] audio extracted: {}", audio_path);
                                    }
                                    Err(e) => {
                                        eprintln!("[webkit-dl] audio extraction failed for {tid}: {e}");
                                    }
                                }
                            });
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

/// Fallback for macOS/Windows: use reqwest with the JWT token in the URL.
#[cfg(not(target_os = "linux"))]
async fn browser_download_fallback(
    app: &tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
    use futures::StreamExt;
    use std::io::Write;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("client: {e}"))?;

    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (compatible; PKUCourseDesktop/0.1)",
        )
        .header("Referer", "https://course.pku.edu.cn/")
        .header("Accept", "*/*")
        .send()
        .await
        .map_err(|e| {
            let msg = format!("request failed: {e}");
            eprintln!("[browser_download] {msg}");
            let _ = app.emit(
                "download-error",
                serde_json::json!({ "taskId": task_id, "error": msg }),
            );
            msg
        })?;

    let status = resp.status();
    eprintln!("[browser_download] response: status={status}");

    if !status.is_success() {
        let msg = format!("HTTP {status}");
        let _ = app.emit(
            "download-error",
            serde_json::json!({ "taskId": task_id, "error": msg }),
        );
        return Err(msg);
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut file =
        std::fs::File::create(&filepath).map_err(|e| format!("create file: {e}"))?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let start = std::time::Instant::now();
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream: {e}"))?;
        file.write_all(&chunk).map_err(|e| format!("write: {e}"))?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed().as_millis() > 500 {
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                -1.0
            };
            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };
            let eta_str = if total_size > 0 && speed > 0.0 {
                let remaining = (total_size - downloaded) as f64 / speed;
                if remaining >= 60.0 {
                    format!("{:.0}m {:.0}s", remaining / 60.0, remaining % 60.0)
                } else {
                    format!("{:.0}s", remaining)
                }
            } else {
                fmt_size(downloaded)
            };
            let _ = app.emit(
                "download-progress",
                serde_json::json!({
                    "taskId": task_id,
                    "progress": progress,
                    "speed": fmt_speed(speed),
                    "eta": eta_str,
                }),
            );
            last_emit = std::time::Instant::now();
        }
    }

    file.flush().map_err(|e| format!("flush: {e}"))?;
    eprintln!("[browser_download] completed: {task_id} ({downloaded} bytes)");

    // Extract audio if enabled (settings accessed via app state)
    let (extract_audio, audio_format) = {
        let settings = app.state::<crate::AppState>().settings.blocking_lock();
        (settings.extract_audio, settings.audio_format.clone())
    };

    if extract_audio {
        let audio_path = format!("{}.{}", filepath, audio_format);
        eprintln!("[browser_download] extracting audio to: {}", audio_path);
        match crate::ffmpeg::extract_audio(&filepath, &audio_path, &audio_format).await {
            Ok(()) => {
                let _ = app.emit(
                    "audio-extract-complete",
                    serde_json::json!({ "taskId": task_id, "audioPath": audio_path }),
                );
                eprintln!("[browser_download] audio extracted: {}", audio_path);
            }
            Err(e) => {
                eprintln!("[browser_download] audio extraction failed for {task_id}: {e}");
            }
        }
    }

    let _ = app.emit(
        "download-complete",
        serde_json::json!({ "taskId": task_id }),
    );

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
        ])
        // ─── Custom IPC protocol for inject scripts ───
        // Inject scripts in the browser-webview (remote URLs) cannot use
        // window.__TAURI__ due to capability restrictions.  Instead they send
        // requests to this custom "pku-ipc" URI scheme via XMLHttpRequest.
        .register_uri_scheme_protocol("pku-ipc", |ctx, request| {
            let app = ctx.app_handle();
            debug_log(&format!("pku-ipc protocol hit: method={} uri={}", request.method(), request.uri()));

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
                debug_log(&format!("IPC /show-main-view received, uri={}", uri));
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
                    let payload = serde_json::json!({
                        "type": "video-info",
                        "data": value
                    });
                    let _ = app.emit("webview-message", payload);
                    eprintln!("[pku-ipc] video-info emitted");
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
            } else if uri.contains("/donation-qr") {
                // Serve the donation QR code PNG (baked in at compile time)
                return tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", "image/png")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(DONATION_QR_PNG.to_vec())
                    .unwrap();
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

            // Set window icon (for Linux Dock / Windows taskbar)
            let icon_bytes = include_bytes!("../icons/icon.png");
            if let Ok(icon) = Image::from_bytes(icon_bytes) {
                let _ = main_window.set_icon(icon);
            }

            let window_size = main_window.inner_size().unwrap_or_default();
            let scale = main_window.scale_factor().unwrap_or(1.0);
            let w = window_size.width as f64 / scale;
            let h = window_size.height as f64 / scale;

            let nav_bar_js = include_str!("../inject-scripts/nav-bar.js");
            let video_detector_js = include_str!("../inject-scripts/video-detector.js");
            let hls_min_js = include_str!("../inject-scripts/hls.min.js");
            let hls_player_js = include_str!("../inject-scripts/hls-player.js");

            let parsed_url: Url = START_URL.parse().expect("invalid START_URL");
            let builder = tauri::webview::WebviewBuilder::new(
                "browser-webview",
                tauri::WebviewUrl::External(parsed_url),
            )
            .initialization_script(nav_bar_js)
            .initialization_script(video_detector_js)
            .initialization_script(hls_min_js)
            .initialization_script(hls_player_js)
            .on_download(|webview, event| {
                handle_download_event(&webview, event)
            });

            match main_window.add_child(
                builder,
                LogicalPosition::new(0.0, 48.0),
                LogicalSize::new(w, h - 48.0),
            ) {
                Ok(browser) => {
                    // Start hidden; BrowserView.svelte will call show_browser_view
                    let _ = browser.hide();
                    eprintln!("[Rust] browser-webview pre-created (hidden, {}x{} at 0,48)", w, h - 48.0);
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
                            // Browser mode: webview below header (48px)
                            let header_height = 48.0;
                            if let Some(browser) = app_handle.get_webview("browser-webview") {
                                let _ = browser.set_position(LogicalPosition::new(0.0, header_height));
                                let _ = browser.set_size(LogicalSize::new(w, h - header_height));
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
