use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Pre-refactor settings shape (snake_case keys on disk).
/// Used as a fallback parse target so existing config files keep working
/// across the upgrade.  Once parsed it converts straight into `AppSettings`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct LegacyAppSettings {
    download_path: String,
    naming_pattern: String,
    auto_download: bool,
    max_concurrent_downloads: u32,
    default_quality: String,
    extract_audio: bool,
    audio_format: String,
}

impl From<LegacyAppSettings> for AppSettings {
    fn from(l: LegacyAppSettings) -> Self {
        Self {
            download_path: l.download_path,
            naming_pattern: l.naming_pattern,
            auto_download: l.auto_download,
            max_concurrent_downloads: l.max_concurrent_downloads,
            default_quality: l.default_quality,
            extract_audio: l.extract_audio,
            audio_format: l.audio_format,
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

    let content = std::fs::read_to_string(&path)?;

    // Try the canonical (camelCase) format first.
    match serde_json::from_str::<AppSettings>(&content) {
        Ok(settings) => Ok(settings),
        Err(camel_err) => {
            // Fall back to the legacy snake_case format.  If that succeeds
            // we re-save in the new format so subsequent loads take the
            // fast path.
            match serde_json::from_str::<LegacyAppSettings>(&content) {
                Ok(legacy) => {
                    let migrated: AppSettings = legacy.into();
                    if let Err(e) = save_settings(&migrated) {
                        log::error!(
                            "[settings] legacy migration parse ok, but re-save failed: {e}"
                        );
                    } else {
                        log::info!("[settings] migrated legacy snake_case settings to camelCase");
                    }
                    Ok(migrated)
                }
                Err(legacy_err) => Err(anyhow::anyhow!(
                    "settings.json parse failed (camel: {camel_err}; legacy: {legacy_err})"
                )),
            }
        }
    }
}

pub fn save_settings(settings: &AppSettings) -> anyhow::Result<()> {
    let path = get_settings_path()?;
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, content)?;
    Ok(())
}
