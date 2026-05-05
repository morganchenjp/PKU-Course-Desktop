//! macOS / Windows fallback for browser-driven downloads.
//!
//! Uses the browser-webview itself to trigger the download: we evaluate a
//! small script that creates a hidden iframe and navigates it to the download
//! URL. This carries the full browser session (including httpOnly cookies
//! like JSESSIONID) because the request originates from the webview.
//! Tauri's `on_download` handler intercepts the download and routes it to
//! the desired file path.

use tauri::{LogicalPosition, Manager};

use crate::state::{AppState, PendingBrowserDownload};

pub async fn run(
    app: &tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
    let browser = app
        .get_webview("browser-webview")
        .ok_or("Browser webview not found")?;

    // Register the pending download so on_download can route the
    // DownloadEvent::Requested callback back to this task.
    {
        let state = app.state::<AppState>();
        if let Ok(mut pending) = state.pending_downloads.lock() {
            pending.insert(
                url.clone(),
                PendingBrowserDownload {
                    task_id,
                    filepath,
                },
            );
        }
    }

    // Ensure browser-webview is visible — WebView2/WKWebView may skip
    // network requests for completely hidden webviews. Position off-screen
    // so the user won't see it flash. `rehide_browser_if_not_browser_mode`
    // puts it back when the download finishes.
    let _ = browser.set_position(LogicalPosition::new(10000.0, 48.0));
    let _ = browser.show();

    // Trigger the download by navigating a hidden iframe to the URL.
    // The iframe loads on the same origin as the wrapper page
    // (course.pku.edu.cn), so all session cookies are sent.
    let script = format!(
        r#"(function() {{
            var iframe = document.createElement('iframe');
            iframe.style.cssText = 'position:fixed;width:0;height:0;border:0;visibility:hidden;';
            iframe.src = '{}';
            if (document.body) {{
                document.body.appendChild(iframe);
            }} else {{
                document.documentElement.appendChild(iframe);
            }}
            setTimeout(function() {{
                if (iframe.parentNode) iframe.parentNode.removeChild(iframe);
            }}, 120000);
        }})();"#,
        url.replace('\\', "\\\\").replace('\'', "\\'")
    );

    browser
        .evaluate_script(&script)
        .map_err(|e| format!("evaluate_script failed: {e}"))?;

    // Return immediately — the actual download progresses via Tauri's
    // on_download handler (DownloadEvent::Requested / Progress / Finished).
    Ok(())
}
