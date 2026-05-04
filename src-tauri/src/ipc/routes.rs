//! Per-route dispatch for the `pku-ipc://` URI scheme.
//!
//! Inject scripts running on remote URLs (course.pku.edu.cn) cannot use
//! `window.__TAURI__` due to Tauri's CSP / capability restrictions.  Instead
//! they POST to this custom URI scheme via XMLHttpRequest (or via the
//! cross-origin iframe bridge in `bridge.rs`).
//!
//! Each route below is matched by a substring on the request URI and either:
//!   1. Returns a response body inline (`/bridge.html`, `/donation-qr`).
//!   2. Translates the request into a backend action (e.g. emits a Tauri
//!      event the Svelte app listens for, or calls into `webview::layout`).

use serde_json::Value;
use tauri::Emitter;

use crate::ipc::bridge::{DONATION_QR_PNG, IPC_BRIDGE_HTML};
use crate::util::log::debug_log;
use crate::webview::layout::do_show_main_view;

/// Top-level handler installed via
/// `Builder::register_uri_scheme_protocol("pku-ipc", ...)`.
pub fn handle(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let app = ctx.app_handle();
    debug_log(&format!(
        "pku-ipc protocol hit: method={} uri={}",
        request.method(),
        request.uri()
    ));

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

    // Serve the IPC bridge HTML page for the iframe-based cross-platform IPC
    if uri.contains("/bridge.html") {
        return tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .body(IPC_BRIDGE_HTML.as_bytes().to_vec())
            .unwrap();
    }

    if uri.contains("/download-diag") {
        let body_str = String::from_utf8_lossy(request.body());
        if let Ok(value) = serde_json::from_str::<Value>(&body_str) {
            let msg = value
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or(&body_str);
            log::debug!("[download-diag] {msg}");
        } else {
            log::debug!("[download-diag] {body_str}");
        }
    } else if uri.contains("/show-main-view") {
        debug_log(&format!("IPC /show-main-view received, uri={}", uri));
        let view = if uri.contains("view=settings") {
            "settings"
        } else {
            "downloads"
        };
        if let Err(e) = do_show_main_view(app, view) {
            log::error!("[pku-ipc] show_main_view error: {e}");
        }
    } else if uri.contains("/video-info") {
        let body_str = String::from_utf8_lossy(request.body());
        if let Ok(value) = serde_json::from_str::<Value>(&body_str) {
            let payload = serde_json::json!({
                "type": "video-info",
                "data": value
            });
            let _ = app.emit("webview-message", payload);
            log::info!("[pku-ipc] video-info emitted");
        }
    } else if uri.contains("/add-download") {
        let body_str = String::from_utf8_lossy(request.body());
        if let Ok(value) = serde_json::from_str::<Value>(&body_str) {
            let _ = app.emit("add-download-from-browser", value);
            log::info!("[pku-ipc] add-download emitted");
        }
    } else if uri.contains("/open-external") {
        let body_str = String::from_utf8_lossy(request.body());
        if let Ok(value) = serde_json::from_str::<Value>(&body_str) {
            if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                match open::that(url) {
                    Ok(_) => log::info!("[pku-ipc] open-external: {url}"),
                    Err(e) => log::error!("[pku-ipc] open-external error: {e}"),
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
}
