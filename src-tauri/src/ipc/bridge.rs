//! Compile-time-baked constants used by the `pku-ipc://` protocol handler.
//!
//! - `IPC_BRIDGE_HTML`: the HTML page loaded in a hidden iframe inside the
//!   browser-webview.  The parent page (course.pku.edu.cn) sends messages to
//!   this iframe via postMessage and the iframe forwards them to Rust via
//!   XHR to the `pku-ipc://` scheme.  This indirection bypasses macOS
//!   WKWebView mixed-content blocking which prevents XHR from HTTPS pages
//!   to custom schemes.
//! - `DONATION_QR_PNG`: a small donation QR code PNG served via
//!   `pku-ipc://localhost/donation-qr` so the Svelte UI can render it
//!   without needing a runtime file path.

/// The HTML for the in-iframe IPC bridge.  See module docs.
pub const IPC_BRIDGE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>PKU IPC Bridge</title>
<script>
(function() {
  'use strict';
  var ALLOWED_ORIGINS = ['https://course.pku.edu.cn', 'https://onlineroomse.pku.edu.cn'];
  function isAllowedOrigin(origin) {
    if (!origin) return false;
    for (var i = 0; i < ALLOWED_ORIGINS.length; i++) {
      if (origin === ALLOWED_ORIGINS[i]) return true;
    }
    return /\.pku\.edu\.cn$/.test(origin);
  }
  window.addEventListener('message', function(event) {
    if (!isAllowedOrigin(event.origin)) {
      console.warn('[ipc-bridge] rejected message from', event.origin);
      return;
    }
    if (event.data && event.data.type === 'pku-desktop-ipc') {
      var path = event.data.path;
      var data = event.data.data;
      var nocache = '&_=' + Date.now() + '.' + Math.random();
      var xhr = new XMLHttpRequest();
      xhr.open('POST', 'pku-ipc://localhost/' + path + nocache, true);
      xhr.send(data ? JSON.stringify(data) : null);
    }
  });
  if (window.parent !== window) {
    window.parent.postMessage({ type: 'pku-desktop-bridge-ready' }, '*');
  }
})();
</script>
</head>
<body></body>
</html>"#;

/// Donation QR code PNG baked in at compile time — no runtime file paths needed.
pub const DONATION_QR_PNG: &[u8] = include_bytes!("../../../public/morgan-wechat-qrcode.png");
