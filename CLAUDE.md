# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PKU Course Desktop is a Tauri-based desktop application for downloading video lectures from Peking University's course platform (course.pku.edu.cn). The app features an embedded browser with video detection, batch download management, and m3u8-to-MP4 transcoding. And this app can support 3 major OSs including Ubuntu, Windows 10 and MacOS.

## Build Commands

```bash
bun install          # Install frontend dependencies
cargo tauri dev      # Start development (runs vite + tauri)
cargo tauri build    # Production build

# Individual commands
bun run dev          # Frontend dev server only
bun run build        # Frontend build only
cargo tauri build     # Full production build
```

## Architecture

### Dual-Webview Design

The app uses **two webviews** managed by Rust:
- **main webview**: Svelte app for downloads/settings UI
- **browser-webview**: Hidden WebKit webview loading course.pku.edu.cn

View switching is done by hiding/showing webviews and repositioning them. The browser webview is pre-created at startup (hidden) and positioned off-screen when not in use.

### Custom IPC Protocol

Inject scripts (running on remote URLs) cannot use `window.__TAURI__` due to CSP/capability restrictions. Instead they communicate via the `pku-ipc://` custom URI scheme registered in Rust (`register_uri_scheme_protocol`).

Key IPC endpoints:
- `pku-ipc://localhost/video-info` — video detected in player page
- `pku-ipc://localhost/add-download` — user clicked download button
- `pku-ipc://localhost/show-main-view?view=downloads|settings` — switch views
- `pku-ipc://localhost/open-external` — open URL in system browser
- `pku-ipc://localhost/download-diag` — download diagnostic log

### Inject Scripts

Injection scripts can refer to another Github project,PKU-Art: https://github.com/zhuozhiyongde/PKU-Art
Located in `src-tauri/inject-scripts/` and bundled as resources:
- `nav-bar.js` — Injected toolbar with navigation controls and view-switch buttons. Runs in top frame only.
- `video-detector.js` — Intercepts XHR to detect video info, overrides `HTMLVideoElement.canPlayType` to claim HLS support (tricks the player's capability check)
- `hls.min.js` + `hls-player.js` — HLS playback support for the embedded browser

### Download Pipeline

Two parallel download paths:
1. **Rust reqwest** (`start_download` command) — Used by the downloads panel for direct URL downloads
2. **Browser-native** (`browser_download` command) — Uses WebKitGTK's `download_uri()` on Linux (preserves session cookies like JSESSIONID), falls back to reqwest on macOS/Windows

Progress/completion/error events are emitted via Tauri events: `download-progress`, `download-complete`, `download-error`.

### FFmpeg Integration

FFmpeg is invoked as an external process for:
- `convert_m3u8_to_mp4()` — Copies video+audio streams without re-encoding
- `extract_audio()` — Extracts audio as MP3/AAC/WAV

FFmpeg must be installed separately; the app checks `is_ffmpeg_available()` before use.

## Key Source Files

| File | Purpose |
|------|---------|
| `src-tauri/src/main.rs` | App entry, Tauri commands, IPC handler, webview setup |
| `src-tauri/src/download.rs` | DownloadManager, HTTP streaming downloads |
| `src-tauri/src/ffmpeg.rs` | m3u8→MP4 conversion, audio extraction |
| `src-tauri/inject-scripts/video-detector.js` | Video info interception, button injection |
| `src-tauri/inject-scripts/nav-bar.js` | Navigation toolbar, URL tracking, link interception |
| `src/App.svelte` | Root component, view routing, event listeners |
| `src/lib/store.ts` | Svelte stores for view state, downloads, settings |
| `src/lib/download-utils.ts` | `createDownloadTask()` helper |

## Code Style

- **Frontend**: Svelte 5 + TypeScript (strict), 2-space indent
- **Backend**: Rust with `cargo fmt` + `cargo clippy`
- **Commits**: Semantic format (`feat:`, `fix:`, `docs:`, etc.) per CONTRIBUTING.md

## Known Limitations

- No resume support — interrupted downloads must restart from scratch
- M3U8 transcoding happens after download completes, not while downloading
- No download history persistence — records lost on app close
- FFmpeg must be installed separately by the user

## Debugging Notes

- **iframe auto-nav deletion bug**: v0.2.0 removed video-webview but accidentally deleted iframe auto-navigation code in `hls-player.js`. This broke video-detector.js button injection because it only runs on `onlineroomse.pku.edu.cn/player`, which is loaded inside an iframe. The iframe auto-nav (`navigateToPlayer()`) is what triggers the top-frame navigation to the player URL. Always check all call sites before deleting code during refactors.

- **Wayland dock icon**: `Window::set_icon()` only affects the title bar on Wayland. Dock icons require GTK app ID (`enableGTKAppId: true` in tauri.conf.json) plus a `.desktop` file installed to `~/.local/share/applications/`.

- **Linux WebKitGTK HLS**: WebKitGTK doesn't natively support HLS playback. The app works around this by injecting hls.js and overriding `canPlayType` to return `'probably'`, tricking cmcPlayer.js into making the API request.

- **Remote URL IPC**: `window.__TAURI__` is blocked on remote URLs due to CSP/capability restrictions. All IPC from inject scripts uses `pku-ipc://` custom URI scheme via XMLHttpRequest. Cross-origin iframes additionally require postMessage relay through the top frame.

## Development plan

### Phase-1,MVP
- Support to view singe video, can inject "Download" button on video page, then can download single video file
- can view progress when downloading video file.

### Phase-2
- can persistent settings locally
- auto detect new lecture videos
- extract audio fro video file. 
- minimize to system tray.

