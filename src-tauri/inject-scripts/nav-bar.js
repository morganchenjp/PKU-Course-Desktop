/**
 * PKU Course Desktop - Injected Navigation Bar
 * Injected into the browser webview via initialization_script().
 * Creates a fixed toolbar at the top of every page with navigation controls
 * and view-switching buttons.
 *
 * Communication with the Rust backend uses the custom "pku-ipc" URI scheme
 * instead of window.__TAURI__ APIs to avoid capability/permission issues
 * on remote URLs.
 */
(function () {
  'use strict';

  // Only run in the top-level frame.
  // Initialization scripts are injected into ALL frames (including iframes).
  // The video player at onlineroomse.pku.edu.cn is loaded inside an iframe;
  // injecting the toolbar there would break its layout and prevent playback.
  if (window.self !== window.top) return;

  // Idempotency guard: don't create the navbar twice on the same page
  if (document.getElementById('pku-desktop-navbar')) return;

  var TOOLBAR_HEIGHT = 44;
  var START_URL = 'https://course.pku.edu.cn';
  var currentUrl = location.href;

  // ─── IPC helper: send commands to Rust via custom URI scheme ───
  function ipcSend(path, data) {
    try {
      var xhr = new XMLHttpRequest();
      var url = 'pku-ipc://localhost/' + path;
      xhr.open(data ? 'POST' : 'GET', url, true);
      if (data) {
        xhr.send(typeof data === 'string' ? data : JSON.stringify(data));
      } else {
        xhr.send();
      }
    } catch (e) {
      console.warn('[nav-bar] IPC send failed:', path, e);
    }
  }

  // ─── Inject Styles ───
  var style = document.createElement('style');
  style.id = 'pku-desktop-navbar-style';
  style.textContent = [
    'html { margin-top: ' + TOOLBAR_HEIGHT + 'px !important; }',
    '#pku-desktop-navbar {',
    '  position: fixed; top: 0; left: 0; right: 0;',
    '  height: ' + TOOLBAR_HEIGHT + 'px;',
    '  z-index: 2147483647;',
    '  display: flex; align-items: center;',
    '  padding: 0 12px; gap: 8px;',
    '  background: #1a1a2e; color: #e0e0e0;',
    '  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;',
    '  font-size: 13px;',
    '  box-shadow: 0 1px 4px rgba(0,0,0,0.4);',
    '  box-sizing: border-box;',
    '  -webkit-app-region: drag;',
    '}',
    '#pku-desktop-navbar * { box-sizing: border-box; }',
    '#pku-desktop-navbar button {',
    '  -webkit-app-region: no-drag;',
    '  border: none; background: transparent; color: #e0e0e0;',
    '  cursor: pointer; border-radius: 4px;',
    '  transition: background 0.15s;',
    '  font-size: 14px; padding: 0;',
    '  display: flex; align-items: center; justify-content: center;',
    '}',
    '#pku-desktop-navbar button:hover { background: rgba(255,255,255,0.1); }',
    '#pku-desktop-navbar button:disabled { opacity: 0.3; cursor: not-allowed; }',
    '#pku-desktop-navbar button:disabled:hover { background: transparent; }',
    '.pku-nav-btn { width: 28px; height: 28px; }',
    '.pku-nav-center {',
    '  flex: 1; display: flex; align-items: center;',
    '  -webkit-app-region: no-drag;',
    '}',
    '.pku-url-input {',
    '  width: 100%; padding: 5px 12px;',
    '  border: 1px solid #3a3a4e; border-radius: 16px;',
    '  background: #2a2a3e; color: #e0e0e0;',
    '  font-size: 13px; outline: none;',
    '  font-family: inherit;',
    '}',
    '.pku-url-input:focus { border-color: #9b0000; }',
    '.pku-nav-action {',
    '  -webkit-app-region: no-drag;',
    '  padding: 4px 10px !important;',
    '  font-size: 12px !important;',
    '  border-radius: 6px !important;',
    '  background: rgba(155,0,0,0.25) !important;',
    '  color: #ff9999 !important;',
    '  white-space: nowrap;',
    '}',
    '.pku-nav-action:hover {',
    '  background: rgba(155,0,0,0.45) !important;',
    '}',
  ].join('\n');
  (document.head || document.documentElement).appendChild(style);

  // ─── Create Toolbar DOM ───
  var navbar = document.createElement('div');
  navbar.id = 'pku-desktop-navbar';

  // Navigation buttons
  var btnBack = createBtn('pku-nav-btn', '\u2190', '\u540E\u9000');
  var btnFwd = createBtn('pku-nav-btn', '\u2192', '\u524D\u8FDB');
  var btnReload = createBtn('pku-nav-btn', '\u21BB', '\u5237\u65B0');

  // URL input
  var centerDiv = document.createElement('div');
  centerDiv.className = 'pku-nav-center';
  var urlInput = document.createElement('input');
  urlInput.type = 'text';
  urlInput.className = 'pku-url-input';
  urlInput.value = location.href;
  centerDiv.appendChild(urlInput);

  // Home button
  var btnHome = createBtn('pku-nav-btn', '\uD83C\uDFE0', '\u9996\u9875');

  // View-switch buttons
  var btnDownloads = createBtn('pku-nav-action', '\uD83D\uDCE5 \u4E0B\u8F7D\u7BA1\u7406');
  var btnSettings = createBtn('pku-nav-action', '\u2699 \u8BBE\u7F6E');

  navbar.appendChild(btnBack);
  navbar.appendChild(btnFwd);
  navbar.appendChild(btnReload);
  navbar.appendChild(centerDiv);
  navbar.appendChild(btnHome);
  navbar.appendChild(btnDownloads);
  navbar.appendChild(btnSettings);

  // Insert navbar as early as possible
  function insertNavbar() {
    if (document.body) {
      document.body.insertBefore(navbar, document.body.firstChild);
    } else {
      document.documentElement.appendChild(navbar);
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', insertNavbar);
  } else {
    insertNavbar();
  }

  // ─── Button Event Handlers ───
  // Use direct browser APIs instead of Tauri invoke() – no IPC needed for navigation.
  btnBack.addEventListener('click', function () {
    window.history.back();
  });
  btnFwd.addEventListener('click', function () {
    window.history.forward();
  });
  btnReload.addEventListener('click', function () {
    window.location.reload();
  });
  btnHome.addEventListener('click', function () {
    window.location.href = START_URL;
  });

  // View-switch buttons use the pku-ipc custom protocol to ask Rust to
  // hide the browser webview and show the Svelte main view.
  btnDownloads.addEventListener('click', function () {
    ipcSend('show-main-view?view=downloads');
  });
  btnSettings.addEventListener('click', function () {
    ipcSend('show-main-view?view=settings');
  });

  // URL input: navigate on Enter
  urlInput.addEventListener('keydown', function (e) {
    if (e.key === 'Enter') {
      var url = urlInput.value.trim();
      if (url && !url.match(/^https?:\/\//)) {
        url = 'https://' + url;
      }
      if (url) {
        window.location.href = url;
      }
    }
  });

  // ─── URL Tracking ───
  function updateUrlDisplay() {
    var newUrl = location.href;
    if (newUrl !== currentUrl) {
      currentUrl = newUrl;
      urlInput.value = currentUrl;
    }
  }

  // Intercept pushState and replaceState
  var origPushState = history.pushState;
  var origReplaceState = history.replaceState;
  history.pushState = function () {
    var result = origPushState.apply(this, arguments);
    updateUrlDisplay();
    return result;
  };
  history.replaceState = function () {
    var result = origReplaceState.apply(this, arguments);
    updateUrlDisplay();
    return result;
  };

  window.addEventListener('popstate', updateUrlDisplay);
  window.addEventListener('hashchange', updateUrlDisplay);

  // Fallback polling for URL changes
  setInterval(updateUrlDisplay, 800);

  // ─── IPC relay for cross-origin iframes ───
  // The video-detector.js runs inside the onlineroomse.pku.edu.cn iframe.
  // Direct XHR to pku-ipc:// may not work from a cross-origin iframe,
  // so the iframe posts messages to the top frame and we relay them.
  window.addEventListener('message', function (event) {
    if (event.data && event.data.type === 'pku-desktop-ipc') {
      ipcSend(event.data.path, event.data.data);
    }
  });

  // ─── Helpers ───
  function createBtn(className, text, title) {
    var btn = document.createElement('button');
    btn.className = className;
    btn.textContent = text;
    if (title) btn.title = title;
    return btn;
  }

  function escapeHtml(str) {
    var div = document.createElement('div');
    div.appendChild(document.createTextNode(str));
    return div.innerHTML;
  }

  // ─── Video URL detection ───
  // Video player pages should open in the system browser because WebKitGTK
  // does not support HLS playback.
  function isVideoUrl(url) {
    if (!url) return false;
    // course.pku.edu.cn video embed page (contains playVideo in path)
    if (/course\.pku\.edu\.cn\/webapps\/.*playVideo/i.test(url)) return true;
    // course.pku.edu.cn streammedia pages
    if (/bb-streammedia-hqy-BBLEARN/i.test(url)) return true;
    // Direct player page
    if (/onlineroomse\.pku\.edu\.cn\/player/i.test(url)) return true;
    return false;
  }

  // ─── Handle target="_blank" links ───
  // Video links → open in system browser (HLS not supported in WebKitGTK).
  // Other links → navigate in the current webview (desktop app has no tabs).
  document.addEventListener('click', function (e) {
    var anchor = e.target;
    // Walk up the DOM to find the nearest <a> element
    while (anchor && anchor.tagName !== 'A') {
      anchor = anchor.parentElement;
    }
    if (!anchor) return;

    var href = anchor.getAttribute('href');
    if (!href) return;

    var target = (anchor.getAttribute('target') || '').toLowerCase();
    if (target === '_blank') {
      e.preventDefault();
      e.stopPropagation();
      var resolved;
      try {
        resolved = new URL(href, location.href).href;
      } catch (_) {
        resolved = href;
      }

      if (isVideoUrl(resolved)) {
        // Open video in app's video-webview with hls.js support
        ipcSend('open-video', { url: resolved });
        console.log('[nav-bar] Video link opened in video-webview:', resolved);
      } else {
        // Non-video links navigate in-place
        window.location.href = resolved;
      }
    }
  }, true); // useCapture to intercept before the page's own handlers

  // Override window.open so that JS-triggered popups are handled correctly.
  // Video URLs → system browser; others → navigate in-place.
  var origWindowOpen = window.open;
  window.open = function (url) {
    if (url) {
      var resolved;
      try {
        resolved = new URL(url, location.href).href;
      } catch (_) {
        resolved = url;
      }

      if (isVideoUrl(resolved)) {
        ipcSend('open-video', { url: resolved });
        console.log('[nav-bar] window.open video → video-webview:', resolved);
      } else {
        window.location.href = resolved;
      }
    }
    // Return a minimal mock so callers don't throw on .focus()/.close() etc.
    return window;
  };

  console.log('[PKU Course Desktop] Navigation bar injected');
})();
