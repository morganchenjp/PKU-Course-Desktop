# Browser ↔ Main View Switching — IPC Design Change

## Context

The `PKU Course Desktop` app uses a dual-webview architecture:

- **main webview**: Svelte UI for downloads/settings panels
- **browser-webview**: WebView loading `https://course.pku.edu.cn` with injected nav-bar

View switching is triggered by two sources:
- **Svelte header tabs** (Browser/Download/Settings) — invoke Rust `show_browser_view` / `show_main_view` via `window.__TAURI__` IPC
- **Injected nav-bar buttons** (Download/Settings in browser-webview) — send IPC to Rust via a custom `pku-ipc://` URI scheme, because `window.__TAURI__` is blocked on remote HTTPS URLs

## Original Design (pre-2026-05-04)

The inject script (`nav-bar.js`) uses a **bridge-iframe** indirection for all IPC:

1. `nav-bar.js` creates a hidden iframe with `src = 'pku-ipc://localhost/bridge.html'`
2. The bridge HTML page receives `postMessage` from the parent (HTTPS origin)
3. Bridge page makes `XMLHttpRequest` to `pku-ipc://localhost/<route>`
4. Tauri `register_uri_scheme_protocol("pku-ipc", ...)` routes the request to Rust
5. Rust calls `do_show_main_view(app, view)`

This was designed to bypass macOS WKWebView mixed-content blocking (HTTPS → custom scheme XHR is blocked on WKWebView).

### Failure on Windows

On **WebView2 (Windows)**, `XMLHttpRequest` from an HTTPS page (`course.pku.edu.cn`) to a custom URI scheme (`pku-ipc://`) is **silently blocked** by mixed-content / CORB policies. The request never reaches Rust.

The bridge-iframe indirection also fails on WebView2 because:
- `postMessage` from HTTPS parent → `pku-ipc://` iframe may be dropped
- Even if received, same-origin XHR from the `pku-ipc://` iframe to `pku-ipc://localhost` is still blocked

### Symptom

- Linux / macOS: nav-bar buttons work, view switches correctly
- Windows: nav-bar buttons visible, but clicking them does nothing

## Iteration 1 — Windows iframe fallback (2026-05-04)

### Platform-detected fallback transport

The nav-bar IPC detects the webview engine at runtime and uses the appropriate transport:

```
┌─────────────────────────────────────────────────────────────────────┐
│  nav-bar.js ipcSend(path, data)                                     │
│                                                                     │
│  ┌──────────────────┐   ┌──────────────────┐   ┌────────────────┐  │
│  │ isWebView2?      │   │ bridgeReady?     │   │ Transport used │  │
│  ├──────────────────┤   ├──────────────────┤   ├────────────────┤  │
│  │ true  (Windows)  │ → │ — (ignored)      │ → │ iframe fallback│  │
│  │ false (macOS/Lin)│ → │ true             │ → │ bridge iframe  │  │
│  │ false (macOS/Lin)│ → │ false            │ → │ iframe fallback│  │
│  └──────────────────┘   └──────────────────┘   └────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### Windows transport — hidden iframe navigation

Instead of `XMLHttpRequest`, create a short-lived hidden iframe and set its `src` to the `pku-ipc://` URL:

```js
var iframe = document.createElement('iframe');
iframe.style.cssText = 'position:fixed;width:0;height:0;border:0;visibility:hidden;';
iframe.src = 'pku-ipc://localhost/show-main-view?view=downloads&_=<timestamp>';
document.body.appendChild(iframe);
setTimeout(() => iframe.remove(), 1000);
```

Why this was thought to work:
- WebView2 `WebResourceRequested` catches **navigation** requests (iframe `src` loads) just like XHR
- Navigation to a custom scheme is **not subject** to mixed-content blocking that blocks XHR
- The Rust handler receives the request and calls `do_show_main_view` identically

### Why it failed

The iframe-fallback approach also failed on Windows because Tauri's custom-protocol workaround on WebView2 is **not applied to iframe navigations**. The DevTools console shows:

```
Failed to launch 'pku-ipc://localhost/bridge.html' because the scheme does not have a registered handler.
```

wry converts `pku-ipc://` → `http://pku-ipc.localhost/` for the **top-level** navigation only. When JS assigns `iframe.src = 'pku-ipc://...'`, WebView2 sees the raw custom scheme and rejects it because it has no OS-level handler.

### Linux / macOS transport — bridge iframe (unchanged)

- `bridgeReady` is true after the bridge iframe loads and signals via `postMessage`
- `sendViaBridge` posts message to the bridge iframe
- Bridge iframe makes same-origin XHR to `pku-ipc://localhost/<route>`
- This path is preserved because it works reliably on WebKitGTK and WKWebView

## Current Design — WebView2 https:// workaround (2026-05-04)

### Root cause

Tauri 2 on Windows does not register a true custom URI scheme with WebView2. Instead it intercepts `http(s)://<scheme>.localhost/*` via `WebResourceRequested`. This interception is added via `AddWebResourceRequestedFilter` (or `AddWebResourceRequestedFilterWithRequestSourceKinds` on WebView2 ≥ 122). However, the **navigation to the raw `pku-ipc://` scheme** itself is not intercepted; only requests to the workaround URL are.

### Fix

1. **Rust (`setup.rs`)**: Set `.use_https_scheme(true)` on the browser webview builder. This tells Tauri/wry to use `https://pku-ipc.localhost/` instead of `http://pku-ipc.localhost/` as the internal mapped URL.

2. **Inject scripts (`nav-bar.js`, `video-detector.js`, `hls-player.js`)**: Detect WebView2 at runtime (`window.chrome.webview`). When detected, construct IPC URLs as `https://pku-ipc.localhost/<path>` and send them via **direct XHR** (not iframe navigation). Because the page is `https://course.pku.edu.cn` and the target is `https://pku-ipc.localhost/`, mixed-content blocking does not apply. Tauri's `WebResourceRequested` handler intercepts the XHR, reverts the URL back to `pku-ipc://localhost/...`, and routes it to the same Rust handler.

3. **POST requests** (`video-info`, `add-download`) work unchanged: the browser sends a CORS preflight OPTIONS, our `routes.rs` already responds with `Access-Control-Allow-Origin: *`, then the POST body is delivered to the handler.

### Platform matrix

| Platform | `use_https_scheme` | Transport | URL format | Result |
|----------|-------------------|-----------|------------|--------|
| Linux    | no-op (ignored)   | bridge iframe | `pku-ipc://localhost/...` | Works |
| macOS    | no-op (ignored)   | bridge iframe | `pku-ipc://localhost/...` | Works |
| Windows  | `true`            | direct XHR | `https://pku-ipc.localhost/...` | Works |

### Detection logic

```js
var isWebView2 = !!(window.chrome && window.chrome.webview);
```

- `window.chrome` exists in all Chromium-based engines (Chrome, Edge, WebView2)
- `window.chrome.webview` exists **only** in WebView2 (injected by the WebView2 runtime)
- Regular Edge browser does **not** expose `window.chrome.webview`
- This detection is reliable and does not depend on user-agent sniffing

## Files modified

| File | Change |
|------|--------|
| `src-tauri/src/webview/setup.rs` | Added `.use_https_scheme(true)` to browser webview builder |
| `src-tauri/inject-scripts/nav-bar.js` | Added `isWebView2` detection; use `https://pku-ipc.localhost/` + direct XHR on WebView2; bridge iframe preserved for macOS/Linux |
| `src-tauri/inject-scripts/video-detector.js` | Same detection + URL change; donation-qr uses same base URL |
| `src-tauri/inject-scripts/hls-player.js` | Same detection + URL change |

## Backwards compatibility

- Linux: no behavior change — bridge path still used after first click
- macOS: no behavior change — bridge path still used after first click
- Windows: buttons now work — uses direct XHR to `https://pku-ipc.localhost/` for every click

## Notes

- The `pku-desktop-bridge` iframe is still created on Windows (for potential future use and for cross-origin iframe relay from `video-detector.js`), but view-switch messages bypass it
- `do_show_main_view` in Rust is idempotent — duplicate requests are harmless
