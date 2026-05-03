//! Browser-driven downloads dispatch.
//!
//! On Linux we use WebKitGTK's native `download_uri()` so the request carries
//! the full browser session (incl. httpOnly cookies like JSESSIONID).  On
//! macOS / Windows we fall back to `reqwest` streaming with the JWT in the
//! URL — session cookies are not reachable from outside the WKWebView /
//! WebView2 cookie jar, but the JWT is sufficient for the
//! `course.pku.edu.cn` download endpoint.

pub mod shared;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(not(target_os = "linux"))]
pub mod fallback;

/// Public entry point used by the `browser_download` Tauri command.
pub async fn run(
    app: &tauri::AppHandle,
    task_id: String,
    url: String,
    filepath: String,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        linux::run(app, &task_id, &url, &filepath)
    }
    #[cfg(not(target_os = "linux"))]
    {
        fallback::run(app, task_id, url, filepath).await
    }
}
