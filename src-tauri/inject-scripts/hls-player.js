/**
 * PKU Course Desktop - HLS Player Integration Script
 * Injected into the video-webview to enable HLS playback via hls.js.
 *
 * This script:
 *   1. Provides a postMessage IPC relay (top frame only) since nav-bar.js
 *      is NOT injected into the video-webview.
 *   2. Overrides HTMLMediaElement.src setter to intercept m3u8 URLs and
 *      hand them to hls.js instead of native (broken) WebKitGTK playback.
 *   3. Overrides Element.setAttribute for the same purpose.
 *
 * Requires hls.min.js to be loaded first (defines window.Hls).
 */
(function () {
  'use strict';

  // ═══════════════════════════════════════════════════════════════════
  // IPC helper (same pattern as nav-bar.js / video-detector.js)
  // ═══════════════════════════════════════════════════════════════════
  var isWebView2 = !!(window.chrome && window.chrome.webview);
  var ipcBaseUrl = isWebView2 ? 'https://pku-ipc.localhost/' : 'pku-ipc://localhost/';

  function ipcSend(path, data) {
    try {
      var xhr = new XMLHttpRequest();
      xhr.open(data ? 'POST' : 'GET', ipcBaseUrl + path, true);
      if (data) {
        xhr.setRequestHeader('Content-Type', 'application/json');
        xhr.send(typeof data === 'string' ? data : JSON.stringify(data));
      } else {
        xhr.send();
      }
    } catch (e) { /* expected to fail in cross-origin iframes */ }
  }

  // ═══════════════════════════════════════════════════════════════════
  // TOP FRAME ONLY: Detect player iframe and navigate directly to it.
  //
  // Problem: The video-webview loads a course.pku.edu.cn wrapper page
  // which embeds the actual player (onlineroomse.pku.edu.cn/player)
  // in a cross-origin iframe.  Initialization scripts may not run
  // reliably in cross-origin iframes, so video-detector.js cannot
  // inject download buttons there.
  //
  // Solution: When we detect the player iframe in the wrapper page,
  // navigate the top frame directly to the player URL.  After
  // navigation the init scripts run again — this time the top frame
  // IS the player page, so video-detector.js matches and works.
  //
  // Also provides the postMessage IPC relay for any messages that
  // arrive before the navigation fires.
  // ═══════════════════════════════════════════════════════════════════
  if (window.self === window.top) {
    var _isPlayerPage = /onlineroomse\.pku\.edu\.cn\/player/.test(window.location.href);

    // PostMessage relay: forward IPC from player iframe to Rust
    window.addEventListener('message', function (event) {
      if (event.data && event.data.type === 'pku-desktop-ipc') {
        console.log('[hls-player] Relay forwarding IPC:', event.data.path);
        ipcSend(event.data.path, event.data.data);
      }
    });
    console.log('[hls-player] PostMessage IPC relay active (top frame)');

    // ── Iframe detection & auto-navigation (wrapper pages only) ──
    // When the browser-webview loads a course.pku.edu.cn wrapper page that
    // embeds the actual player (onlineroomse.pku.edu.cn/player) in a
    // cross-origin iframe, navigate the top frame directly to the player URL.
    // This allows video-detector.js to run on the player page and inject
    // download buttons.
    if (!_isPlayerPage) {
      var _navigating = false;

      function navigateToPlayer(src) {
        if (_navigating) return;
        _navigating = true;
        console.log('[hls-player] Detected player iframe, navigating directly:', src);
        window.location.href = src;
      }

      function checkIframeNode(node) {
        if (!node || node.nodeType !== 1) return;
        if (node.tagName === 'IFRAME') {
          var src = node.getAttribute('src') || '';
          if (/onlineroomse\.pku\.edu\.cn\/player/.test(src)) {
            navigateToPlayer(src);
          }
        }
        if (node.querySelectorAll) {
          var iframes = node.querySelectorAll('iframe');
          for (var i = 0; i < iframes.length; i++) {
            var s = iframes[i].getAttribute('src') || '';
            if (/onlineroomse\.pku\.edu\.cn\/player/.test(s)) {
              navigateToPlayer(s);
              return;
            }
          }
        }
      }

      var _iframeObs = new MutationObserver(function (mutations) {
        if (_navigating) return;
        for (var m = 0; m < mutations.length; m++) {
          var mut = mutations[m];
          if (mut.type === 'childList' && mut.addedNodes) {
            for (var n = 0; n < mut.addedNodes.length; n++) {
              checkIframeNode(mut.addedNodes[n]);
              if (_navigating) return;
            }
          }
          if (mut.type === 'attributes' && mut.target.tagName === 'IFRAME') {
            var src = mut.target.getAttribute('src') || '';
            if (/onlineroomse\.pku\.edu\.cn\/player/.test(src)) {
              navigateToPlayer(src);
              return;
            }
          }
        }
      });

      if (document.documentElement) {
        _iframeObs.observe(document.documentElement, {
          childList: true, subtree: true,
          attributes: true, attributeFilter: ['src']
        });
      }

      var _pollTimer = setInterval(function () {
        if (_navigating) { clearInterval(_pollTimer); return; }
        var iframes = document.querySelectorAll('iframe');
        for (var i = 0; i < iframes.length; i++) {
          var src = iframes[i].getAttribute('src') || '';
          if (/onlineroomse\.pku\.edu\.cn\/player/.test(src)) {
            clearInterval(_pollTimer);
            _iframeObs.disconnect();
            navigateToPlayer(src);
            return;
          }
        }
      }, 500);
      setTimeout(function () {
        clearInterval(_pollTimer);
        if (!_navigating) {
          console.warn('[hls-player] Timed out waiting for player iframe (30s)');
        }
      }, 30000);
    }
  }

  // ═══════════════════════════════════════════════════════════════════
  // HLS.js integration (ALL FRAMES)
  // ═══════════════════════════════════════════════════════════════════
  if (typeof Hls === 'undefined') {
    console.warn('[hls-player] Hls global not found. hls.min.js must be loaded first.');
    return;
  }

  if (!Hls.isSupported()) {
    console.warn('[hls-player] MediaSource Extensions not supported. HLS playback unavailable.');
    // Download buttons from video-detector.js still work as fallback.
    return;
  }

  console.log('[hls-player] Hls.js v' + Hls.version + ' loaded, MSE supported');

  // ─── Shared HLS attachment helper ───
  function attachHls(videoElement, url) {
    // Clean up previous instance
    if (videoElement._hlsInstance) {
      try { videoElement._hlsInstance.destroy(); } catch (e) { /**/ }
      videoElement._hlsInstance = null;
    }

    console.log('[hls-player] Attaching HLS to video element, src:', url);

    var hls = new Hls({
      debug: false,
      enableWorker: true,
      lowLatencyMode: false,
      // Reasonable defaults for course video playback
      maxBufferLength: 30,
      maxMaxBufferLength: 600,
      startLevel: -1  // auto quality selection
    });

    hls.loadSource(url);
    hls.attachMedia(videoElement);

    hls.on(Hls.Events.MANIFEST_PARSED, function () {
      console.log('[hls-player] Manifest parsed, starting playback');
      videoElement.play().catch(function (e) {
        console.log('[hls-player] Auto-play prevented:', e.message);
      });
    });

    hls.on(Hls.Events.ERROR, function (event, data) {
      if (data.fatal) {
        console.error('[hls-player] Fatal error:', data.type, data.details);
        switch (data.type) {
          case Hls.ErrorTypes.NETWORK_ERROR:
            console.log('[hls-player] Network error, attempting recovery...');
            hls.startLoad();
            break;
          case Hls.ErrorTypes.MEDIA_ERROR:
            console.log('[hls-player] Media error, attempting recovery...');
            hls.recoverMediaError();
            break;
          default:
            console.error('[hls-player] Unrecoverable error, destroying instance');
            hls.destroy();
            videoElement._hlsInstance = null;
            break;
        }
      }
    });

    videoElement._hlsInstance = hls;
  }

  // Helper: check if a URL is an m3u8 stream
  function isM3u8Url(url) {
    if (!url || typeof url !== 'string') return false;
    var lower = url.toLowerCase();
    return lower.indexOf('.m3u8') !== -1 ||
           lower.indexOf('mpegurl') !== -1;
  }

  // ─── Override HTMLMediaElement.prototype.src setter ───
  // This is the primary interception point. cmcPlayer.js typically does:
  //   video.src = "https://...playlist.m3u8"
  // We intercept this and route through hls.js.
  var origSrcDescriptor = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');

  if (origSrcDescriptor && origSrcDescriptor.set) {
    Object.defineProperty(HTMLMediaElement.prototype, 'src', {
      get: origSrcDescriptor.get,
      set: function (url) {
        if (this.tagName === 'VIDEO' && isM3u8Url(url)) {
          console.log('[hls-player] Intercepted video.src =', url);
          attachHls(this, url);
          return;
        }
        origSrcDescriptor.set.call(this, url);
      },
      configurable: true,
      enumerable: true
    });
    console.log('[hls-player] HTMLMediaElement.src setter override active');
  } else {
    console.warn('[hls-player] Could not get src property descriptor, trying fallback');
  }

  // ─── Override Element.prototype.setAttribute ───
  // Catches: video.setAttribute('src', 'm3u8url')
  var origSetAttribute = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function (name, value) {
    if (this.tagName === 'VIDEO' && name === 'src' && isM3u8Url(value)) {
      console.log('[hls-player] Intercepted video.setAttribute("src",', value, ')');
      attachHls(this, value);
      return;
    }
    origSetAttribute.call(this, name, value);
  };

  // ─── MutationObserver for <source> elements ───
  // Belt-and-suspenders: catches <source src="...m3u8" type="application/...">
  var observer = new MutationObserver(function (mutations) {
    mutations.forEach(function (mutation) {
      mutation.addedNodes.forEach(function (node) {
        // Check if a <source> was added to a <video>
        if (node.nodeType === 1 && node.tagName === 'SOURCE' &&
            node.parentElement && node.parentElement.tagName === 'VIDEO') {
          var src = node.getAttribute('src');
          var type = node.getAttribute('type') || '';
          if (isM3u8Url(src) || type.toLowerCase().indexOf('mpegurl') !== -1) {
            console.log('[hls-player] Intercepted <source> element:', src);
            attachHls(node.parentElement, src);
          }
        }
        // Check if a <video> was added with m3u8 src
        if (node.nodeType === 1 && node.tagName === 'VIDEO') {
          var videoSrc = node.getAttribute('src');
          if (isM3u8Url(videoSrc) && !node._hlsInstance) {
            console.log('[hls-player] Intercepted <video> with m3u8 src:', videoSrc);
            attachHls(node, videoSrc);
          }
        }
      });
    });
  });

  // Start observing as soon as possible
  if (document.documentElement) {
    observer.observe(document.documentElement, { childList: true, subtree: true });
  } else {
    document.addEventListener('DOMContentLoaded', function () {
      observer.observe(document.documentElement, { childList: true, subtree: true });
    });
  }

  console.log('[hls-player] HLS interception active (all frames)');
})();
