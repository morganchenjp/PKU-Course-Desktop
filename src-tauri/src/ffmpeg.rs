use std::process::Stdio;
use tokio::process::Command;

/// Convert m3u8 stream to mp4 using ffmpeg
pub async fn convert_m3u8_to_mp4(
    m3u8_url: &str,
    output_path: &str,
    jwt: Option<&str>,
) -> anyhow::Result<()> {
    // Check if ffmpeg is available
    if !is_ffmpeg_available().await {
        return Err(anyhow::anyhow!(
            "FFmpeg not found. Please install FFmpeg to convert m3u8 videos."
        ));
    }
    
    let mut cmd = Command::new("ffmpeg");
    
    // Add input
    cmd.arg("-i").arg(m3u8_url);
    
    // Add headers if JWT is provided
    if let Some(token) = jwt {
        cmd.arg("-headers").arg(format!("Authorization: Bearer {}", token));
    }
    
    // Add conversion options
    cmd.arg("-c:v").arg("copy")  // Copy video stream without re-encoding
        .arg("-c:a").arg("copy")  // Copy audio stream without re-encoding
        .arg("-bsf:a").arg("aac_adtstoasc")  // Fix AAC audio stream
        .arg("-movflags").arg("+faststart")  // Enable fast start for web playback
        .arg("-y")  // Overwrite output file
        .arg(output_path);
    
    // Execute command
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("FFmpeg conversion failed: {}", stderr));
    }
    
    Ok(())
}

/// Extract audio from video file
pub async fn extract_audio(
    video_path: &str,
    output_path: &str,
    format: &str,
) -> anyhow::Result<()> {
    // Check if ffmpeg is available
    if !is_ffmpeg_available().await {
        return Err(anyhow::anyhow!(
            "FFmpeg not found. Please install FFmpeg to extract audio."
        ));
    }
    
    // Determine codec based on format
    let (codec, ext) = match format.to_lowercase().as_str() {
        "mp3" => ("libmp3lame", "mp3"),
        "aac" => ("aac", "aac"),
        "wav" => ("pcm_s16le", "wav"),
        _ => return Err(anyhow::anyhow!("Unsupported audio format: {}", format)),
    };
    
    // Ensure output path has correct extension
    let output_path = if !output_path.ends_with(&format!(".{}", ext)) {
        format!("{}.{}", output_path, ext)
    } else {
        output_path.to_string()
    };
    
    let mut cmd = Command::new("ffmpeg");
    
    cmd.arg("-i").arg(video_path)
        .arg("-vn")  // No video
        .arg("-c:a").arg(codec);
    
    // Add format-specific options (medium quality)
    match format.to_lowercase().as_str() {
        "mp3" => {
            cmd.arg("-q:a").arg("5");  // Medium quality VBR (~185kbps)
        }
        "aac" => {
            cmd.arg("-b:a").arg("128k");  // 128kbps bitrate (medium)
        }
        _ => {}
    }
    
    cmd.arg("-y").arg(&output_path);
    
    // Execute command
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("FFmpeg audio extraction failed: {}", stderr));
    }
    
    Ok(())
}

/// Check if ffmpeg is available in system PATH
async fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
