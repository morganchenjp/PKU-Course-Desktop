# PKU Course Desktop — Refactor Plan

## Context

`src-tauri/src/main.rs` has grown to **1166 lines / 45.9 KB** and now mixes 8
distinct concerns: app entry, 17 Tauri commands, the `pku-ipc://` URI-scheme
handler, webview show/hide layout, two cross-platform `browser_download`
implementations (~350 lines), embedded HTML/PNG constants, format helpers, and
ad-hoc debug logging. Format helpers (`fmt_speed`/`fmt_size` vs
`format_speed`/`format_duration`) are duplicated between `main.rs` and
`download.rs` with subtly different output, and the audio-extraction
"tail" (extract → emit `audio-extract-complete`) is copy-pasted **three** times
(Linux GTK callback, macOS/Windows fallback, `download.rs::download_file`).
On the frontend, `App.svelte` carries the download-queue logic
(`startNextPendingDownload` plus the slot check inside the
`add-download-from-browser` listener) which belongs in `lib/`. `AppSettings`
serializes as snake_case so `SettingsPanel.svelte` does manual
camelCase→snake_case mapping before every `save_settings` call —
inconsistent with `VideoInfo` and `DownloadTask` which already use
`#[serde(rename_all = "camelCase")]`. The `current_view_mode` is a
`StdMutex<String>` with stringly-typed `"browser"`/`"main"` values that gets
compared via `*m == "browser"` in five places.

Goals:
1. Split `main.rs` into a layered module tree by concern.
2. Eliminate the three duplicated audio-extraction tails and two duplicated
   format helpers.
3. Replace the stringly-typed view-mode state with a `ViewMode` enum.
4. Make `AppSettings` camelCase end-to-end so the frontend can stop
   hand-mapping field names.
5. Move the download-queue/concurrency logic out of `App.svelte` into
   `lib/download-queue.ts`.

## Constraints (user-specified)

- **Inject scripts are off-limits.** No edits to
  `src-tauri/inject-scripts/nav-bar.js`, `video-detector.js`, or
  `hls-player.js`. (DESIGN.md §7 documents the painful 3-platform debug
  history that produced their current state.)
- **View-switching semantics/timing are off-limits.** `show_browser_view`,
  `do_show_main_view`, the deferred 150 ms `switch-to-main` emit, the
  belt-and-suspenders off-screen positioning, the Resize handler — all
  preserved byte-for-byte. Files may move; behavior may not.

## Target Layout

```
src-tauri/src/
  main.rs                  ← Builder wiring + setup() body only (~120 lines)
  state.rs                 ← AppState, PendingBrowserDownload, ViewMode enum
  commands/
    mod.rs                 ← pub use everything
    settings_cmd.rs        ← load_settings, save_settings, get_default_download_path
    files.rs               ← open_file_location, open_external_link
    browser_nav.rs         ← navigate_browser, browser_go_back/_forward/_reload/_home
    download_cmd.rs        ← start_download, pause_download, browser_download dispatch
    media.rs               ← convert_m3u8_to_mp4, extract_audio
    view.rs                ← show_browser_view, show_main_view (thin wrappers)
  webview/
    mod.rs
    setup.rs               ← pre-create browser-webview + initialization_script chain
    layout.rs              ← do_show_main_view, the Resize handler
                             (logic unchanged, just relocated)
    on_download.rs         ← handle_download_event
    download_native/
      mod.rs               ← public `run(app, task_id, url, filepath)` dispatch
      linux.rs             ← browser_download_linux (WebKitGTK)
      fallback.rs          ← browser_download_fallback (reqwest)
      shared.rs            ← extract_audio_after_download(),
                             rehide_browser_if_not_browser_mode()
  ipc/
    mod.rs                 ← register_uri_scheme_protocol setup
    routes.rs              ← per-route dispatch (video-info, add-download, ...)
    bridge.rs              ← IPC_BRIDGE_HTML, DONATION_QR_PNG constants
  util/
    mod.rs
    fmt.rs                 ← unified fmt_speed, fmt_size, fmt_duration
    log.rs                 ← debug_log
  download.rs              ← stays; trimmed (uses util::fmt, uses
                             webview::download_native::shared::extract_audio_after_download)
  ffmpeg.rs                ← stays; commented-out ensure_ffmpeg dead block deleted
  settings.rs              ← stays; AppSettings adds #[serde(rename_all = "camelCase")]
```

## Detailed Changes

### A. Rust: state types (`state.rs`)

```rust
pub enum ViewMode { Browser, Main }   // sub-view (downloads/settings)
                                      // is purely a frontend concern,
                                      // Rust doesn't need to track it
                                      // beyond the one-shot `view` arg
                                      // already passed to do_show_main_view.

pub struct AppState {
    pub download_manager: tokio::sync::Mutex<DownloadManager>,
    pub settings: std::sync::Mutex<AppSettings>,
    pub current_view_mode: std::sync::Mutex<ViewMode>,
    pub pending_downloads: std::sync::Mutex<HashMap<String, PendingBrowserDownload>>,
}
```

Five existing `*m == "browser"` / `*m == "main"` call sites get rewritten as
`matches!(*m, ViewMode::Browser)` / `matches!(*m, ViewMode::Main)`. Default
remains `ViewMode::Browser` (matches today). The `view: String` param of
`show_main_view` and the `serde_json::json!({"view": ...})` payload are
unchanged — keeps the inject-script / Svelte contract intact.

### B. Rust: util/fmt.rs

Single canonical implementation absorbed from `main.rs` (which has the more
complete `fmt_size`-as-fallback-ETA branch) and `download.rs::format_speed/
format_duration`:

```rust
pub fn fmt_speed(bps: f64) -> String { ... }
pub fn fmt_size(bytes: u64) -> String { ... }
pub fn fmt_duration(secs: f64) -> String { ... }   // h/m/s, matches download.rs
```

`download.rs` and the new `webview/download_native/{linux,fallback}.rs` all
import these.

### C. Rust: shared download-tail helpers (`webview/download_native/shared.rs`)

Two helpers replacing copy-pasted blocks:

```rust
/// Emit `download-complete` then optionally extract audio per current settings.
/// Called from BOTH the Linux GTK `connect_finished` callback and the
/// macOS/Windows reqwest fallback.
pub fn after_browser_download(app: &tauri::AppHandle, task_id: &str, filepath: &str);

/// If the current view-mode is not Browser, hide the browser-webview.
/// Replaces the inline mode-check + hide() blocks scattered through main.rs.
pub fn rehide_browser_if_not_browser_mode(app: &tauri::AppHandle);
```

`download.rs::download_file` already has its own m3u8→mp4 conversion step
that the browser download path does not (different upstream contract); it
keeps its own extraction call but switches to the same shared helper for the
emit, so the `audio-extract-complete` event payload is identical across all
three paths.

### D. Rust: `settings.rs` camelCase

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings { ... }   // field names unchanged (snake_case in Rust)
```

Persisted JSON file at `~/.config/pku-course-desktop/settings.json` will
flip from snake_case to camelCase keys. **Migration**: in
`settings::load_settings`, on `serde_json::from_str` failure attempt a
second parse with the legacy snake_case `#[serde(alias = ...)]` derives
(implemented via a private `LegacyAppSettings` struct with
`rename_all = "snake_case"`) and re-save in the new format. Users with an
existing settings file are not blown away.

### E. Rust: `ffmpeg.rs` cleanup

Delete the commented-out `ensure_ffmpeg` block (lines 124–149). No behavior
change.

### F. Rust: `download.rs` path bug

`download.rs::download_file` line 236-239 uses `String::replace` to
swap the extension via the result of `rsplit_once('.')`. If the filename
has no dot in the basename but the *path* contains a dot
(e.g. `~/.local/Downloads/foo`), this corrupts the path. Replace with
`std::path::Path::with_extension(audio_format)`.

### G. Rust: inject-script wiring stays put

`main.rs::setup` continues to:
- `include_str!("../inject-scripts/nav-bar.js")`
- `include_str!("../inject-scripts/video-detector.js")`
- `include_str!("../inject-scripts/hls.min.js")`
- `include_str!("../inject-scripts/hls-player.js")`

These four scripts are not touched in any way.

### H. Frontend: download queue (`src/lib/download-queue.ts` — new)

Move `startNextPendingDownload` plus the slot-check from `App.svelte`'s
`add-download-from-browser` listener into a single export:

```ts
// src/lib/download-queue.ts
export async function enqueueDownload(videoInfo: VideoInfo): Promise<void>;
export async function startNextPendingDownload(): Promise<void>;
```

Both use `get(downloadTasks)` / `get(settings)` instead of subscribing —
matches existing `download-utils.ts` style. `App.svelte` becomes purely
event-wiring: progress/complete/error listeners call
`startNextPendingDownload()`; `add-download-from-browser` calls
`enqueueDownload(payload)`.

### I. Frontend: `SettingsPanel.svelte` simplification

After step D, replace the manual snake_case map (lines 28-36) with
`await invoke('save_settings', { settings: $settings })`. Drop the
`snakeSettings` local. `console.log` debug line removed.

### J. Frontend: `Naming.ts` / `download-utils.ts` unchanged

These are clean and reused as-is.

## Critical files modified

| Path | Action |
|------|--------|
| `src-tauri/src/main.rs` | Shrink to entry+setup; move bodies out |
| `src-tauri/src/state.rs` | New — AppState + ViewMode |
| `src-tauri/src/commands/*.rs` | New — per-concern command modules |
| `src-tauri/src/webview/setup.rs` | New — extracted from `main.rs::setup` |
| `src-tauri/src/webview/layout.rs` | New — `do_show_main_view`, resize handler |
| `src-tauri/src/webview/on_download.rs` | New — `handle_download_event` |
| `src-tauri/src/webview/download_native/{linux,fallback,shared}.rs` | New |
| `src-tauri/src/ipc/{mod,routes,bridge}.rs` | New — protocol handler split |
| `src-tauri/src/util/{fmt,log}.rs` | New — unified helpers |
| `src-tauri/src/download.rs` | Use util::fmt, fix path-replace bug |
| `src-tauri/src/ffmpeg.rs` | Delete dead `ensure_ffmpeg` block |
| `src-tauri/src/settings.rs` | Add `rename_all = "camelCase"` + legacy migration |
| `src/App.svelte` | Drop queue logic, call `lib/download-queue` |
| `src/lib/download-queue.ts` | New |
| `src/components/SettingsPanel.svelte` | Drop manual snake_case map |

**Not modified:** `nav-bar.js`, `video-detector.js`, `hls-player.js`,
`hls.min.js`, `BrowserView.svelte`, `DownloadPanel.svelte`,
`DownloadTaskItem.svelte`, `VideoInfoCard.svelte`, `store.ts`, `types.ts`,
`naming.ts`, `theme.ts`, `download-utils.ts`, `tauri.conf.json`, `Cargo.toml`.

## Verification

End-to-end smoke (must pass on the user's primary platform — Linux —
before accepting):

1. **Build clean:** `cd src-tauri && cargo build` and `bun run build`
   from repo root, no new warnings.
2. **Dev launch:** `cargo tauri dev` opens the window, browser-webview
   loads `course.pku.edu.cn`, injected nav-bar appears.
3. **View switch round-trip:** click "下载管理" in the injected toolbar →
   main view appears with the empty downloads panel → click "浏览器" tab in
   the Svelte header → browser webview reappears at the correct position.
   Repeat for "设置". (Verifies `do_show_main_view` byte-equivalence and the
   `current_view_mode` enum migration.)
4. **Settings persist:** open Settings, toggle "同时提取音频文件", choose AAC,
   click 保存设置. Restart app. Setting still on. (Verifies camelCase JSON
   migration.)
5. **Settings legacy migration:** before the rebuild, manually edit
   `~/.config/pku-course-desktop/settings.json` to use snake_case keys
   (the pre-refactor format). After rebuild, launch the app — no crash,
   settings load, file is rewritten on next save.
6. **Download a video:** log in to a course, navigate to a video, click
   "下载视频" in the injected button. Task appears in the downloads panel,
   progress bar advances, file lands in the configured download path,
   `download-complete` event fires.
7. **Audio extraction:** with extract_audio enabled, the matching `.mp3`
   file appears next to the video after completion (verifies the shared
   `after_browser_download` helper works on Linux's GTK callback path).
8. **Concurrency:** set max-concurrent to 2, queue 3 videos rapidly. Two
   start, one stays "等待中". When one finishes the third begins.
   (Verifies `lib/download-queue.ts` extraction.)
9. **Resize:** drag the window edges in both browser and main view —
   webviews resize correctly. (Verifies the resize handler relocation.)

If the user has macOS or Windows hardware available, repeat steps 2/3/6/7
there since `browser_download_fallback` is the path actually used and
`view-switching` had macOS-specific timing fixes (`set_focus`,
`set_position(10000.0, 0.0)`, the 150 ms deferred emit) that must
survive the move.

## Out of scope

- Inject-script consolidation / shared `pku-ipc.js` module (off-limits per
  user constraint).
- Resume-on-disconnect support, m3u8 streaming-transcode, persistent
  download history (DESIGN.md §8.2 future work).
- The `m3u8Url: isM3u8 ? downloadUrl : null` discrepancy in
  `video-detector.js` line 308 (latent bug, but inside an off-limits file).
- Tauri capability hardening, CSP tightening — separate concern.

## Refactor Result

The refactor landed in full. `cargo build`, `cargo clippy`, and
`bun run build` (151 modules, 78.40 kB / gz 28.02 kB) all pass clean
with no new warnings. The four inject scripts
(`nav-bar.js`, `video-detector.js`, `hls.min.js`, `hls-player.js`) were
not touched, and the view-switching primitives (`show_browser_view`,
`do_show_main_view`, the 150 ms deferred `switch-to-main` emit, off-screen
positioning, the Resize handler, and the macOS `set_focus`/
`set_position(10000, 0)` belt-and-suspenders) are byte-for-byte preserved —
only relocated.

### Final layout (Rust)

| Path | Lines | Notes |
|------|------:|-------|
| `src-tauri/src/main.rs` | 64 | Builder wiring + `invoke_handler` only (was 1166 — **−94.5 %**) |
| `src-tauri/src/state.rs` | 45 | New: `AppState`, `PendingBrowserDownload`, `ViewMode` enum |
| `src-tauri/src/util/fmt.rs` | 37 | New: unified `fmt_speed`, `fmt_size`, `fmt_duration` |
| `src-tauri/src/util/log.rs` | 30 | New: `debug_log` |
| `src-tauri/src/util/mod.rs` | 2 | New |
| `src-tauri/src/commands/mod.rs` | 17 | New |
| `src-tauri/src/commands/settings_cmd.rs` | 35 | New |
| `src-tauri/src/commands/files.rs` | 41 | New |
| `src-tauri/src/commands/browser_nav.rs` | 60 | New |
| `src-tauri/src/commands/download_cmd.rs` | 82 | New: thin `browser_download` delegates to `webview::download_native::run` |
| `src-tauri/src/commands/media.rs` | 26 | New |
| `src-tauri/src/commands/view.rs` | 21 | New: thin wrappers over `webview::layout` |
| `src-tauri/src/ipc/mod.rs` | 8 | New: re-exports `routes::handle` |
| `src-tauri/src/ipc/routes.rs` | 118 | New: `pku-ipc://` per-route dispatch |
| `src-tauri/src/ipc/bridge.rs` | 54 | New: `IPC_BRIDGE_HTML` + `DONATION_QR_PNG` (`include_bytes!`) |
| `src-tauri/src/webview/mod.rs` | 8 | New |
| `src-tauri/src/webview/setup.rs` | 104 | New: setup() body + four `include_str!` inject-script lines (untouched) |
| `src-tauri/src/webview/layout.rs` | 166 | New: `show_browser_view`, `do_show_main_view`, `handle_window_resize` (logic byte-equivalent) |
| `src-tauri/src/webview/on_download.rs` | 108 | New: `handle_download_event` |
| `src-tauri/src/webview/download_native/mod.rs` | 33 | New: `cfg`-gated `run(...)` dispatch |
| `src-tauri/src/webview/download_native/linux.rs` | 151 | New: WebKitGTK `download_uri` path |
| `src-tauri/src/webview/download_native/fallback.rs` | 113 | New: `reqwest` fallback for macOS/Windows |
| `src-tauri/src/webview/download_native/shared.rs` | 83 | New: `after_browser_download` + `rehide_browser_if_not_browser_mode` |
| `src-tauri/src/download.rs` | 262 | Trimmed: uses `util::fmt`, calls shared `after_browser_download` for emit |
| `src-tauri/src/ffmpeg.rs` | 122 | Dead `ensure_ffmpeg` block removed |
| `src-tauri/src/settings.rs` | 110 | `#[serde(rename_all = "camelCase")]` + private `LegacyAppSettings` migration |

### Final layout (Frontend)

| Path | Lines | Notes |
|------|------:|-------|
| `src/lib/download-queue.ts` | 72 | New: `enqueueDownload` + `startNextPendingDownload` (`get(store)` style) |
| `src/App.svelte` | 279 | Slot-check + `startNextPendingDownload` body removed; pure event-wiring now |
| `src/components/SettingsPanel.svelte` | 363 | Manual snake_case map dropped; `$settings` passed through directly |

### Bugs / smells fixed

1. `download.rs::download_file` audio-path corruption when path contains a dot
   but the basename does not — switched from `String::replace` to
   `Path::with_extension`.
2. Triplicated audio-extraction tail (Linux GTK callback / fallback /
   `download.rs`) collapsed into `shared::after_browser_download` so the
   `audio-extract-complete` payload is identical across all three paths.
3. Two divergent format-helper sets (`main.rs::fmt_speed`/`fmt_size` vs
   `download.rs::format_speed`/`format_duration`) collapsed into one
   `util::fmt` module.
4. Stringly-typed `current_view_mode: StdMutex<String>` + five
   `*m == "browser"` comparisons replaced with a `ViewMode` enum and
   `matches!(*m, ViewMode::Browser)`.
5. `AppSettings` now serializes camelCase end-to-end; legacy snake_case
   `settings.json` files are auto-migrated on first load (private
   `LegacyAppSettings` fallback + rewrite on next save).
6. Commented-out `ensure_ffmpeg` block (≈25 dead lines) deleted.

### Verification status

| Step | Status |
|------|--------|
| `cargo build` clean | ✅ |
| `cargo clippy` clean | ✅ |
| `bun run build` clean | ✅ |
| Manual smoke (steps 2–9 in §Verification) | ⏳ pending — run on Linux primary, then macOS/Windows if available |

### Constraints honored

- **Inject scripts:** zero changes. The four `include_str!` lines now live
  in `webview/setup.rs` but resolve to the same files via the same paths.
- **View-switching semantics/timing:** `show_browser_view`,
  `do_show_main_view`, the deferred 150 ms `switch-to-main` emit, the
  off-screen `(10000.0, 0.0)` reposition, the Resize handler logic, and the
  macOS `set_focus()` calls are reproduced verbatim in `webview/layout.rs`.
  The `view: String` argument and `{"view": ...}` event payload are
  unchanged, so the inject-script ↔ Svelte contract is preserved.
