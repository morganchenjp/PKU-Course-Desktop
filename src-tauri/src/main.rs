// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod download;
mod ffmpeg;
mod ipc;
mod settings;
mod state;
mod util;
mod webview;

use commands::{
    browser_nav::{
        browser_go_back, browser_go_forward, browser_go_home, browser_reload, navigate_browser,
    },
    download_cmd::{browser_download, pause_download, start_download},
    files::{open_external_link, open_file_location},
    media::{convert_m3u8_to_mp4, extract_audio},
    settings_cmd::{get_default_download_path, load_settings, save_settings},
    view::{show_browser_view, show_main_view},
};
use state::AppState;

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
        .manage(AppState::new())
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
        .register_uri_scheme_protocol("pku-ipc", ipc::handle)
        .setup(webview::setup::setup)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
