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

## New Design (2026-05-04)

### Platform-detected fallback transport

The nav-bar IPC now detects the webview engine at runtime and uses the appropriate transport:

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

Why this works:
- WebView2 `WebResourceRequested` catches **navigation** requests (iframe `src` loads) just like XHR
- Navigation to a custom scheme is **not subject** to mixed-content blocking that blocks XHR
- The Rust handler receives the request and calls `do_show_main_view` identically

### Linux / macOS transport — bridge iframe (unchanged)

- `bridgeReady` is true after the bridge iframe loads and signals via `postMessage`
- `sendViaBridge` posts message to the bridge iframe
- Bridge iframe makes same-origin XHR to `pku-ipc://localhost/<route>`
- This path is preserved because it works reliably on WebKitGTK and WKWebView

## Detection logic

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
| `src-tauri/inject-scripts/nav-bar.js` | Added `isWebView2` detection; modified `ipcSend` to bypass bridge on Windows; replaced `tryDirectXhr` with hidden-iframe navigation |

## Backwards compatibility

- Linux: no behavior change — bridge path still used after first click
- macOS: no behavior change — bridge path still used after first click
- Windows: buttons now work — uses iframe fallback for every click

## Notes

- The `pku-desktop-bridge` iframe is still created on Windows (for potential future use and for cross-origin iframe relay from `video-detector.js`), but view-switch messages bypass it
- The fallback iframe is auto-removed after 1 second to avoid DOM clutter during rapid switching
- `do_show_main_view` in Rust is idempotent — duplicate requests from overlapping iframes are harmless
