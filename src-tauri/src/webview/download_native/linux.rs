//! Linux implementation of browser-driven downloads.
//!
//! Uses WebKitGTK's native `download_uri()` so the download carries the full
//! browser session (including httpOnly cookies like JSESSIONID and BbRouter)
//! that the reqwest fallback can't access.

use serde_json::json;
use tauri::{Emitter, Manager};

use crate::util::fmt::{fmt_size, fmt_speed};
use crate::webview::download_native::shared::{
    after_browser_download, rehide_browser_if_not_browser_mode,
};

pub fn run(
    app: &tauri::AppHandle,
    task_id: &str,
    url: &str,
    filepath: &str,
) -> Result<(), String> {
    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    // Ensure browser-webview is visible — WebKitGTK may not process network
    // requests for hidden webviews.  Position off-screen so the user won't see
    // it flash.  `rehide_browser_if_not_browser_mode` puts it back when the
    // download finishes.
    let _ = browser.set_position(tauri::LogicalPosition::new(10000.0, 48.0));
    let _ = browser.show();

    let url_clone = url.to_string();
    let filepath_clone = filepath.to_string();
    let task_id_clone = task_id.to_string();
    let app_clone = app.clone();

    browser
        .with_webview(move |platform_wv| {
            use webkit2gtk::{DownloadExt, WebViewExt};

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
                            json!({
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
                        rehide_browser_if_not_browser_mode(&app_f);
                        after_browser_download(&app_f, &tid_f, &fp_f);
                    });

                    // ── failed: download error ──
                    let app_e = app_clone.clone();
                    let tid_e = task_id_clone.clone();
                    download.connect_failed(move |_dl, error| {
                        let msg = error.to_string();
                        eprintln!("[webkit-dl] failed: task={tid_e} error={msg}");
                        let _ = app_e.emit(
                            "download-error",
                            json!({
                                "taskId": tid_e,
                                "error": msg,
                            }),
                        );
                        rehide_browser_if_not_browser_mode(&app_e);
                    });
                }
                None => {
                    eprintln!("[webkit-dl] download_uri() returned None!");
                    let _ = app_clone.emit(
                        "download-error",
                        json!({
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
