use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub download_path: String,
    pub naming_pattern: String,
    pub auto_download: bool,
    pub max_concurrent_downloads: u32,
    pub default_quality: String,
    pub extract_audio: bool,
    pub audio_format: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_path: String::new(),
            naming_pattern: "{courseName} - {subTitle} - {lecturerName}".to_string(),
            auto_download: false,
            max_concurrent_downloads: 3,
            default_quality: "highest".to_string(),
            extract_audio: false,
            audio_format: "mp3".to_string(),
        }
    }
}

fn get_settings_path() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    
    let app_config = config_dir.join("pku-course-desktop");
    std::fs::create_dir_all(&app_config)?;
    
    Ok(app_config.join("settings.json"))
}

pub fn load_settings() -> anyhow::Result<AppSettings> {
    let path = get_settings_path()?;
    
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    
    let content = std::fs::read_to_string(path)?;
    let settings: AppSettings = serde_json::from_str(&content)?;
    
    Ok(settings)
}

pub fn save_settings(settings: &AppSettings) -> anyhow::Result<()> {
    let path = get_settings_path()?;
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, content)?;
    Ok(())
}
