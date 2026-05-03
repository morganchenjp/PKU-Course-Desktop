//! Standalone FFmpeg commands exposed to the frontend: m3u8→MP4 conversion
//! and audio-extract (independent of the download pipeline).

use crate::ffmpeg;

#[tauri::command]
pub async fn convert_m3u8_to_mp4(
    m3u8_url: String,
    output_path: String,
    jwt: Option<String>,
) -> Result<(), String> {
    ffmpeg::convert_m3u8_to_mp4(&m3u8_url, &output_path, jwt.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_audio(
    video_path: String,
    output_path: String,
    format: String,
) -> Result<(), String> {
    ffmpeg::extract_audio(&video_path, &output_path, &format)
        .await
        .map_err(|e| e.to_string())
}
