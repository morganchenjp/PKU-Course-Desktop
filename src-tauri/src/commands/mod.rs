//! Tauri commands invoked by the frontend, grouped by concern.
//!
//! The split mirrors the file boundaries chosen during the refactor:
//!   - `settings_cmd`: load / save settings, default download path
//!   - `files`:        open a file's folder, open URLs in the system browser
//!   - `browser_nav`:  navigate / back / forward / reload / home for the
//!     embedded browser-webview
//!   - `download_cmd`: queue / pause downloads, dispatch browser downloads
//!   - `media`:        m3u8→mp4 and audio extraction stand-alone commands
//!   - `view`:         show_browser_view / show_main_view (thin wrappers)

pub mod browser_nav;
pub mod download_cmd;
pub mod files;
pub mod media;
pub mod settings_cmd;
pub mod view;
