//! Settings-related commands: load, save, default download path.

use tauri::State;

use crate::settings::{self, AppSettings};
use crate::state::AppState;

#[tauri::command]
pub fn get_default_download_path() -> Result<String, String> {
    dirs::download_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not find downloads directory".to_string())
}

#[tauri::command]
pub fn load_settings() -> Result<AppSettings, String> {
    settings::load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    eprintln!(
        "[save_settings] received: extract_audio={}, audio_format={}",
        settings.extract_audio, settings.audio_format
    );
    crate::settings::save_settings(&settings).map_err(|e| e.to_string())?;
    // Also update in-memory state so the change takes effect immediately
    let mut current = state.settings.lock().unwrap();
    *current = settings.clone();
    eprintln!(
        "[save_settings] in-memory updated: extract_audio={}, audio_format={}",
        current.extract_audio, current.audio_format
    );
    Ok(())
}
