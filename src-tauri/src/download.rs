use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::RwLock;
use tokio::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub course_name: String,
    pub sub_title: String,
    pub lecturer_name: String,
    pub download_url: String,
    pub is_m3u8: bool,
    pub m3u8_url: Option<String>,
    pub resource_id: Option<String>,
    pub jwt: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: String,
    pub video_info: VideoInfo,
    pub filename: String,
    pub filepath: String,
    pub status: DownloadStatus,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub error: Option<String>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Error,
}

pub struct DownloadManager {
    client: Client,
    active_downloads: Arc<RwLock<HashMap<String, tokio::task::AbortHandle>>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_download(
        &self,
        task: DownloadTask,
        app: tauri::AppHandle,
        extract_audio: bool,
        audio_format: String,
    ) -> anyhow::Result<()> {
        let client = self.client.clone();
        let active_downloads = self.active_downloads.clone();
        let task_id = task.id.clone();
        let task_id_for_insert = task.id.clone();

        let handle = tokio::spawn(async move {
            let id = task.id.clone();
            match download_file(client, task, &app, extract_audio, &audio_format).await {
                Ok(()) => {
                    let _ = app.emit(
                        "download-complete",
                        serde_json::json!({ "taskId": id }),
                    );
                    eprintln!("[download] completed: {id}");
                }
                Err(e) => {
                    let msg = e.to_string();
                    let _ = app.emit(
                        "download-error",
                        serde_json::json!({ "taskId": id, "error": msg }),
                    );
                    eprintln!("[download] failed {id}: {msg}");
                }
            }

            // Remove from active downloads
            let mut downloads = active_downloads.write().await;
            downloads.remove(&task_id);
        });

        // Store abort handle
        let mut downloads = self.active_downloads.write().await;
        downloads.insert(task_id_for_insert, handle.abort_handle());

        Ok(())
    }

    pub async fn pause_download(&self, task_id: &str) -> anyhow::Result<()> {
        let downloads = self.active_downloads.read().await;
        if let Some(handle) = downloads.get(task_id) {
            handle.abort();
        }
        Ok(())
    }
}

async fn download_file(
    client: Client,
    task: DownloadTask,
    app: &tauri::AppHandle,
    extract_audio: bool,
    audio_format: &str,
) -> anyhow::Result<()> {
    let url = &task.video_info.download_url;
    let filepath = &task.filepath;
    let task_id = &task.id;

    eprintln!("[download] starting {task_id}: {url}");

    // Ensure directory exists
    if let Some(parent) = Path::new(filepath).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Build request
    let mut request = client.get(url);

    // Add JWT if available
    if let Some(jwt) = &task.video_info.jwt {
        request = request.header("Authorization", format!("Bearer {}", jwt));
    }

    // Send request
    let response = request.send().await?;
    let status = response.status();
    let final_url = response.url().to_string();
    if !status.is_success() {
        // Read response body for diagnostic info
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unable to read body>".to_string());
        let body_preview: String = body.chars().take(500).collect();
        eprintln!(
            "[download] HTTP error {status} for {task_id}\n  url: {url}\n  final_url: {final_url}\n  body: {body_preview}"
        );
        anyhow::bail!("HTTP {status} — {body_preview}");
    }
    let total_size = response.content_length().unwrap_or(0);

    eprintln!("[download] {task_id}: response {status}, size={total_size}, url={final_url}");

    // Create file
    let mut file = File::create(filepath)?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    let start_time = std::time::Instant::now();
    let mut last_update = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        // Emit progress every 500ms
        if last_update.elapsed().as_millis() > 500 {
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            let elapsed = start_time.elapsed().as_secs_f64();
            let speed_bps = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };

            let eta_secs = if speed_bps > 0.0 && total_size > 0 {
                (total_size - downloaded) as f64 / speed_bps
            } else {
                0.0
            };

            let _ = app.emit(
                "download-progress",
                serde_json::json!({
                    "taskId": task_id,
                    "progress": progress,
                    "speed": format_speed(speed_bps),
                    "eta": format_duration(eta_secs),
                }),
            );

            last_update = std::time::Instant::now();
        }
    }

    file.flush()?;

    // Handle m3u8 conversion if needed
    let final_video_path = if task.video_info.is_m3u8 {
        let mp4_path = filepath.replace(".m3u8", ".mp4");
        crate::ffmpeg::convert_m3u8_to_mp4(filepath, &mp4_path, task.video_info.jwt.as_deref())
            .await?;
        tokio::fs::remove_file(filepath).await.ok();
        mp4_path
    } else {
        filepath.to_string()
    };

    // Extract audio if enabled
    eprintln!("[download] extract_audio={}, audio_format={}", extract_audio, audio_format);
    if extract_audio {
        let audio_path = final_video_path.replace(
            final_video_path.rsplit_once('.').unwrap_or((&final_video_path, "")).1,
            &format!(".{}", audio_format),
        );
        eprintln!("[download] extracting audio to: {}", audio_path);
        match crate::ffmpeg::extract_audio(&final_video_path, &audio_path, audio_format).await {
            Ok(()) => {
                let _ = app.emit(
                    "audio-extract-complete",
                    serde_json::json!({ "taskId": task_id, "audioPath": audio_path }),
                );
                eprintln!("[download] audio extracted: {}", audio_path);
            }
            Err(e) => {
                eprintln!("[download] audio extraction failed for {task_id}: {e}");
                // Don't fail the download task — audio extraction is non-critical
            }
        }
    }

    Ok(())
}

fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec > 1024.0 * 1024.0 {
        format!("{:.2} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec > 1024.0 {
        format!("{:.2} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

fn format_duration(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{:.0}s", seconds)
    } else if seconds < 3600.0 {
        format!("{:.0}m {:.0}s", seconds / 60.0, seconds % 60.0)
    } else {
        format!("{:.0}h {:.0}m", seconds / 3600.0, (seconds % 3600.0) / 60.0)
    }
}
