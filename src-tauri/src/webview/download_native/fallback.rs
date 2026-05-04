//! macOS / Windows fallback for browser-driven downloads.
//!
//! WebKitGTK is Linux-only.  On the other two platforms we fall back to
//! `reqwest` streaming with the JWT already embedded in the URL.  Session
//! cookies (JSESSIONID, BbRouter) are inaccessible from this path — they
//! live in the WKWebView / WebView2 cookie jar — but the JWT in the URL
//! is sufficient for `course.pku.edu.cn`'s download endpoint.

use futures::StreamExt;
use serde_json::json;
use std::io::Write;
use tauri::Emitter;

use crate::util::fmt::{fmt_size, fmt_speed};
use crate::webview::download_native::shared::after_browser_download;

pub async fn run(
    app: &tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
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
            log::error!("[browser_download] {msg}");
            let _ = app.emit(
                "download-error",
                json!({ "taskId": task_id, "error": msg }),
            );
            msg
        })?;

    let status = resp.status();
    log::info!("[browser_download] response: status={status}");

    if !status.is_success() {
        let msg = format!("HTTP {status}");
        let _ = app.emit(
            "download-error",
            json!({ "taskId": task_id, "error": msg }),
        );
        return Err(msg);
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(&filepath).map_err(|e| format!("create file: {e}"))?;
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
                json!({
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
    log::info!("[browser_download] completed: {task_id} ({downloaded} bytes)");

    after_browser_download(app, &task_id, &filepath);

    Ok(())
}
