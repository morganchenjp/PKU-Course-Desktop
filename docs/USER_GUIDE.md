# PKU Course Desktop — User Guide

A step-by-step guide for downloading PKU course videos with PKU Course Desktop.

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [First Launch & Login](#first-launch--login)
5. [Finding and Downloading Videos](#finding-and-downloading-videos)
6. [Download Management](#download-management)
7. [Settings](#settings)
8. [Audio Extraction](#audio-extraction)
9. [Troubleshooting](#troubleshooting)
10. [FAQ](#faq)

---

## Overview

PKU Course Desktop is a cross-platform desktop application that lets you download video lectures from Peking University's course platform (`course.pku.edu.cn`). It features:

- An embedded browser with automatic video detection
- One-click download with real-time progress (percentage, speed, ETA)
- Batch download queue with concurrency control
- Automatic m3u8-to-MP4 transcoding
- Optional audio extraction (MP3 / AAC / WAV)
- Light / dark theme support

Supported platforms: Windows 10+, macOS 11+, Ubuntu 24+.

---

## Prerequisites

Before using the app, you must install **FFmpeg** on your system. FFmpeg is required for:

- Converting m3u8 streams to MP4
- Extracting audio from video files

### macOS

```bash
brew install ffmpeg
```

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install ffmpeg
```

> **Tip for Linux users:** If you want to play HLS (m3u8) videos directly inside the embedded browser, also install GStreamer plugins:
> ```bash
> sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav
> ```

### Windows

1. Download FFmpeg from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) (recommended: `ffmpeg-git-essentials.7z`).
2. Extract it to `C:\ffmpeg`.
3. Add `C:\ffmpeg\bin` to your system `PATH` environment variable.

Verify the installation by opening a terminal and running:

```bash
ffmpeg -version
```

---

## Installation

1. Download the release package for your operating system from the [Releases](https://github.com/morganchenjp/PKU-Course-Desktop/releases) page.
2. Install it like any regular desktop application:
   - **Windows**: Run the `.msi` installer.
   - **macOS**: Open the `.dmg` and drag the app into `Applications`.
   - **Linux**: Make the AppImage executable (`chmod +x PKU-Course-Desktop.AppImage`) and run it.
3. Launch the app.

---

## First Launch & Login

When you open PKU Course Desktop for the first time:

1. The app automatically loads the PKU Course Portal (`course.pku.edu.cn`) in the built-in browser.
2. Click **"校园卡用户"** (Campus Card User) to log in with your IAAA credentials.
3. After successful login, you will land on the course portal dashboard.

> **Recommendation:** Maximize the app window for the best browsing experience.

---

## Finding and Downloading Videos

### Step 1 — Navigate to Your Course

Browse the course portal as you normally would:

1. Enter a course from your course list.
2. Click **"课堂实录"** (Classroom Recordings).
3. Click the date of the lecture you want to download.

### Step 2 — Detect the Video

When the video player page loads, the app automatically detects the video. You will see three buttons injected below the video player:

- **下载视频** (Download Video)
- **复制链接** (Copy Link)
- **刷新** (Refresh)

### Step 3 — Start Downloading

Click **"下载视频"** (Download Video). The app will:

1. Create a download task.
2. If the video is an m3u8 stream, it will first download the stream and then transcode it to MP4.
3. Show real-time progress in the download panel.

> **Note:** If you have **"自动开始下载"** (Auto Download) enabled in Settings, videos will be added to the queue and start downloading automatically as soon as they are detected.

---

## Download Management

Click the **"下载管理"** (Downloads) button in the top-right navigation bar to open the download panel.

### Task States

| State | Meaning |
|-------|---------|
| **等待中** (Pending) | Queued, waiting for a free download slot |
| **下载中** (Downloading) | Actively downloading with live progress |
| **已完成** (Completed) | Download finished successfully |
| **失败** (Error) | Download failed (see error message in task card) |

### Actions

- **Filter tasks**: Use the tabs (全部 / 下载中 / 等待中 / 已完成 / 失败) to filter by status.
- **Clear completed**: Click **"清空已完成"** to remove all completed tasks from the list.
- **Clear all**: Click **"清空全部"** to remove every task. This only clears the task list; downloaded files remain on disk.

### Progress Information

While a task is downloading, each task card shows:

- **Progress bar** with exact percentage
- **Download speed** (e.g., `2.5 MB/s`)
- **Estimated time remaining** (ETA)

---

## Settings

Click the **"设置"** (Settings) button in the top-right navigation bar to open the settings panel.

### Download Settings

| Setting | Description |
|---------|-------------|
| **下载路径** (Download Path) | The folder where videos are saved. Defaults to the system Downloads folder if left empty. |
| **文件命名规则** (Naming Pattern) | How downloaded files are named. Variables: `{courseName}`, `{subTitle}`, `{lecturerName}`, `{date}`, `{index}`. |
| **最大并发下载数** (Max Concurrent) | How many videos can download at the same time (1–2). |
| **自动开始下载** (Auto Download) | If enabled, detected videos are automatically added to the queue and start downloading. |

### Video Settings

| Setting | Description |
|---------|-------------|
| **默认视频质量** (Default Quality) | Preferred stream quality (highest / high / medium / low). |
| **同时提取音频文件** (Extract Audio) | If enabled, automatically extract audio after the video download completes. |
| **音频格式** (Audio Format) | Choose MP3, AAC, or WAV. Only visible when **Extract Audio** is enabled. |

### Saving Settings

Changes are only persisted after you click **"保存设置"** (Save Settings). To revert unsaved changes, click **"恢复默认"** (Reset to Defaults).

---

## Audio Extraction

Audio extraction is useful if you want to:

- Listen to lectures on devices that don't support video playback.
- Upload the audio to AI transcription tools (e.g., Tongyi Tingwu) to generate lecture notes.

### How to Enable

1. Open **Settings**.
2. Turn on **"同时提取音频文件"** (Extract Audio).
3. Select your preferred **Audio Format**:
   - **MP3** — Best compatibility, medium quality VBR (~185 kbps)
   - **AAC** — Better quality at similar file sizes (128 kbps)
   - **WAV** — Uncompressed, largest files
4. Click **"保存设置"** (Save Settings).

### Result

After a video finishes downloading, the app runs FFmpeg in the background to create an audio file with the same base name in the same folder. For example:

```
Download/
├── Calculus - Week 3 - Prof. Zhang.mp4
└── Calculus - Week 3 - Prof. Zhang.mp3   <-- extracted audio
```

> **Windows users:** FFmpeg runs silently in the background. No console window will appear.

---

## Troubleshooting

### "FFmpeg not found" error

FFmpeg is not installed or not on your system `PATH`. Follow the [Prerequisites](#prerequisites) section to install it, then restart the app.

### Download shows "失败" (Error) immediately

Common causes:

- **Session expired**: You may have been logged out. Navigate to the course page again to refresh the session.
- **Network issue**: Check your internet connection.
- **HTTP 500**: The server rejected the request. Try reloading the video page and clicking download again.

### Video player shows a black screen or won't load

- Make sure you are logged in (the IAAA session may have expired).
- Try refreshing the page using the **"刷新"** button below the video.
- On Linux, ensure GStreamer plugins are installed (see [Prerequisites](#prerequisites)).

### Download progress is stuck at 0%

- Check that you are not behind a proxy or firewall that blocks the download URL.
- Try reducing **Max Concurrent Downloads** to 1 in Settings.

### App window turns white after clicking "Download Video"

This was a bug in older versions. If you encounter it, update to the latest release.

---

## FAQ

**Q: Where are my downloaded files saved?**

By default, they go to your system's Downloads folder. You can change this in **Settings → 下载路径**.

**Q: Can I download multiple videos at the same time?**

Yes. Set **Max Concurrent Downloads** to 2 in Settings. Additional videos will be queued and start automatically when a slot frees up.

**Q: Does the app support resuming interrupted downloads?**

Not yet. If a download is interrupted, you will need to restart it from the beginning.

**Q: Is my login credentials safe?**

Yes. The app uses the system WebView engine (WebKitGTK on Linux, WebView2 on Windows, WKWebView on macOS). Your password is handled directly by the PKU IAAA login page; the app never reads or stores it.

**Q: Can I use this on campus VPN?**

Yes. As long as your system can reach `course.pku.edu.cn`, the app works normally.

**Q: The downloaded MP4 won't play in my media player.**

Try a modern player like VLC or MPV. If the file was converted from m3u8, it uses H.264/AAC codecs, which most players support.

---

*For developer documentation, see [CLAUDE.md](CLAUDE.md).*  
*For project overview and build instructions, see [README.md](README.md).*
