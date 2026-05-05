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
2. **Browser-native** (`browser_download` command) — Platform-specific:
   - **Linux**: WebKitGTK `download_uri()` (preserves session cookies natively, reports progress via `estimated_progress()`)
   - **macOS / Windows**: Extracts cookies (including `httpOnly`) from the browser-webview via `webview.cookies_for_url()`, then streams with `reqwest` using those cookies. This gives real byte-level progress, speed, and ETA — identical UX to the Linux path.

Progress/completion/error events are emitted via Tauri events: `download-progress`, `download-complete`, `download-error`.

### FFmpeg Integration

FFmpeg is invoked as an external process for:
- `convert_m3u8_to_mp4()` — Copies video+audio streams without re-encoding
- `extract_audio()` — Extracts audio as MP3/AAC/WAV

FFmpeg must be installed separately; the app checks `is_ffmpeg_available()` before use.

## Key Source Files

| File | Purpose |
|------|---------|
| `src-tauri/src/main.rs` | App entry, builder wiring, `setup()` only |
| `src-tauri/src/state.rs` | `AppState`, `ViewMode` enum, `PendingBrowserDownload` |
| `src-tauri/src/commands/*.rs` | Per-concern Tauri command modules (settings, download, view, browser nav, media, files) |
| `src-tauri/src/download.rs` | `DownloadManager`, HTTP streaming download helper (`download_with_progress`) |
| `src-tauri/src/ffmpeg.rs` | m3u8→MP4 conversion, audio extraction, `CREATE_NO_WINDOW` on Windows |
| `src-tauri/src/webview/` | Webview setup, layout (show/hide), `on_download` handler, platform-native download paths |
| `src-tauri/src/ipc/` | `pku-ipc://` custom URI scheme handler and route dispatch |
| `src-tauri/src/util/` | Shared helpers: `fmt_speed`/`fmt_size`/`fmt_duration`, `debug_log` |
| `src-tauri/inject-scripts/video-detector.js` | Video info interception, download button injection |
| `src-tauri/inject-scripts/nav-bar.js` | Navigation toolbar, URL tracking, link interception, postMessage relay |
| `src-tauri/inject-scripts/hls-player.js` | HLS.js integration, iframe auto-navigation (skipped on WebView2) |
| `src/App.svelte` | Root component, event listeners, view tabs |
| `src/lib/store.ts` | Svelte stores for view state, downloads, settings |
| `src/lib/download-queue.ts` | Queue logic with deduplication, concurrency slot management |
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

- **WebView2 "签名失败" on video page**: On Windows, `hls-player.js` unconditionally navigated the top frame to the player URL (`onlineroomse.pku.edu.cn/player`). This broke the BlackBoard auth context (wrapper-to-iframe postMessage, referrer chain, JWT propagation) and caused "签名失败". **Fix**: skip iframe auto-navigation on WebView2 (`isWebView2` check) and let the player stay in the cross-origin iframe, where `AddScriptToExecuteOnDocumentCreated` injects init scripts natively into all frames.

- **Windows download white screen**: Clicking "Download Video" in browser view caused a white screen because `fallback.rs` unconditionally called `browser.set_position(10000.0, 48.0)` before triggering the download. When the user was already in `Browser` mode, this moved the visible browser-webview off-screen, leaving the window blank (main webview is hidden in browser mode). **Fix**: only reposition off-screen when `current_view_mode` is NOT `Browser`.

- **Windows download HTTP 500**: The original macOS/Windows fallback used raw `reqwest` without session cookies. The PKU course server requires `JSESSIONID` and `BbRouter` cookies (including httpOnly) to serve the download file, so reqwest got HTTP 500. **Fix**: extract all cookies from the browser-webview via `webview.cookies_for_url()` (returns httpOnly/secure cookies) and pass them as a `Cookie` header to `reqwest`. This also enables real progress reporting because we can stream the response and read `Content-Length`.

- **Duplicate download tasks (3× per click) on Windows**: On WebView2, `video-detector.js` inside the cross-origin iframe sends the download request via two channels simultaneously: (1) direct XHR to `https://pku-ipc.localhost/add-download`, and (2) `postMessage` to top frame. Both `nav-bar.js` and `hls-player.js` in the top frame listen for the same `postMessage` and each relay it via another XHR, producing 3 identical backend requests. **Fix**: frontend deduplication in `download-queue.ts` using a module-level `Map<downloadUrl, timestamp>` with a 3-second TTL. (Inject scripts are off-limits per user constraint.)

- **No download progress on Windows/macOS**: Tauri v2's `DownloadEvent` only exposes `Requested` and `Finished` — there is no `Progress` variant in the version we use. The iframe-based browser download therefore could not report percentage. **Fix**: as noted above, switch the fallback path to `reqwest` with extracted cookies. The shared `download_with_progress()` helper streams chunks, tracks bytes received, and emits `download-progress` events every 500 ms with exact percentage, speed, and ETA.

- **FFmpeg black console window on Windows**: Spawning FFmpeg via `tokio::process::Command` on Windows creates a visible CMD window by default. **Fix**: on Windows, set `CREATE_NO_WINDOW` (`0x08000000`) creation flags on every FFmpeg command via `std::os::windows::process::CommandExt`.

- **Wayland dock icon**: `Window::set_icon()` only affects the title bar on Wayland. Dock icons require GTK app ID (`enableGTKAppId: true` in tauri.conf.json) plus a `.desktop` file installed to `~/.local/share/applications/`.

- **Linux WebKitGTK HLS**: WebKitGTK doesn't natively support HLS playback. The app works around this by injecting hls.js and overriding `canPlayType` to return `'probably'`, tricking cmcPlayer.js into making the API request.

- **Remote URL IPC**: `window.__TAURI__` is blocked on remote URLs due to CSP/capability restrictions. All IPC from inject scripts uses `pku-ipc://` custom URI scheme via XMLHttpRequest. Cross-origin iframes additionally require postMessage relay through the top frame.

- **WebKitGTK XHR response caching**: WebKitGTK caches XHR responses to custom URI schemes (like `pku-ipc://`) based on URL path only, ignoring query strings. This caused subsequent view-switch IPC calls to return cached "ok" responses without ever reaching the Rust handler. The first click would work, but all subsequent clicks on nav-bar buttons would silently fail. **Fix**: append a cache-busting query parameter (`&_=<timestamp>.<random>`) to every `pku-ipc://` URL in `ipcSend()` inside `nav-bar.js`. This makes each request URL unique, bypassing the cache.

## Development plan

### Phase-1,MVP
- Support to view singe video, can inject "Download" button on video page, then can download single video file
- can view progress when downloading video file.

### Phase-2
- can persistent settings locally
- auto detect new lecture videos
- extract audio fro video file.
- minimize to system tray.
