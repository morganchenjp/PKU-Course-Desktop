/**
 * PKU Course Desktop - Video Detector Inject Script
 * Ported from PKU-Art's initializeDirectDownload() implementation.
 *
 * Architecture:
 *   The video player page (onlineroomse.pku.edu.cn/player) is embedded in a
 *   course.pku.edu.cn iframe. WebKitGTK does not natively support HLS, so
 *   cmcPlayer.js aborts with "不支持hls视频!" before making API calls.
 *
 *   Fix strategy:
 *   1. Override HTMLVideoElement.canPlayType to claim HLS support (all frames).
 *      This tricks cmcPlayer.js into proceeding with the API request.
 *   2. Intercept the XHR to get-sub-info-by-auth-data (player page only).
 *   3. Inject download buttons + "play in browser" into the page footer.
 *
 * IPC: Uses both direct pku-ipc:// XHR and postMessage relay to top frame.
 */
(function () {
  'use strict';

  // ═══════════════════════════════════════════════════════════════════
  // Phase 1: canPlayType override (runs in ALL frames, including iframes)
  // This must execute before any page script checks HLS support.
  // ═══════════════════════════════════════════════════════════════════
  try {
    var origCanPlayType = HTMLVideoElement.prototype.canPlayType;
    HTMLVideoElement.prototype.canPlayType = function (type) {
      if (type && (
        type.indexOf('mpegurl') !== -1 ||
        type.indexOf('mpegURL') !== -1 ||
        type.indexOf('x-mpegURL') !== -1 ||
        type === 'application/vnd.apple.mpegurl'
      )) {
        return 'probably';
      }
      return origCanPlayType.call(this, type);
    };
  } catch (e) {
    console.warn('[video-detector] canPlayType override failed:', e);
  }

  // ═══════════════════════════════════════════════════════════════════
  // Phase 2: Video detection and download buttons (player page only)
  // ═══════════════════════════════════════════════════════════════════
  var pageUrl = window.location.href;
  console.log('[video-detector] URL check:', pageUrl);
  if (!/onlineroomse\.pku\.edu\.cn\/player/.test(pageUrl)) {
    console.log('[video-detector] Not a player page, skipping button injection');
    return;
  }

  console.log('[PKU Course Desktop] Video detector active on player page');

  // ─── IPC helper ───
  function ipcSend(path, data) {
    var payload = (typeof data === 'string') ? data : JSON.stringify(data);

    // Channel 1: direct custom protocol XHR
    try {
      var xhr = new XMLHttpRequest();
      xhr.open(data ? 'POST' : 'GET', 'pku-ipc://localhost/' + path, true);
      if (data) { xhr.send(payload); } else { xhr.send(); }
    } catch (e) {
      // Expected to fail from cross-origin iframe; relay handles it.
    }

    // Channel 2: postMessage relay to top frame (nav-bar.js forwards to pku-ipc://)
    try {
      if (window.self !== window.top) {
        window.top.postMessage({ type: 'pku-desktop-ipc', path: path, data: data }, '*');
      }
    } catch (e2) { /* cross-origin restriction – ok */ }
  }

  // ─── State ───
  var downloadUrl = '';
  var downloadJson = null;
  var courseName = '';
  var subTitle = '';
  var lecturerName = '';
  var fileName = '';
  var JWT = '';
  var isM3u8 = false;
  var resourceId = '';

  // ─── XHR Interception (same approach as PKU-Art) ───
  var originalSend = XMLHttpRequest.prototype.send;
  var originalSetRequestHeader = XMLHttpRequest.prototype.setRequestHeader;

  XMLHttpRequest.prototype.setRequestHeader = function (header, value) {
    if (!this._pku_headers) { this._pku_headers = {}; }
    this._pku_headers[header] = value;
    originalSetRequestHeader.apply(this, arguments);
  };

  XMLHttpRequest.prototype.send = function () {
    this.addEventListener('load', function () {
      if (this.responseURL && this.responseURL.indexOf('get-sub-info-by-auth-data') !== -1) {
        try {
          downloadJson = JSON.parse(this.response);

          // Extract JWT
          if (this._pku_headers) {
            for (var h in this._pku_headers) {
              if (h.toLowerCase() === 'authorization') {
                JWT = this._pku_headers[h].split(' ')[1] || '';
                break;
              }
            }
          }
          if (JWT) {
            console.log('[PKU Course Desktop] JWT captured');
            try { sessionStorage.setItem('PKU_COURSE_DESKTOP_JWT', JWT); } catch (e) { /**/ }
          }

          console.log('[PKU Course Desktop] Video info intercepted');

          // Parse video details
          courseName = downloadJson.list[0].title;
          subTitle = downloadJson.list[0].sub_title;
          lecturerName = downloadJson.list[0].lecturer_name;
          fileName = courseName + ' - ' + subTitle + ' - ' + lecturerName + '.mp4';

          var filmContent = JSON.parse(downloadJson.list[0].sub_content);
          isM3u8 = filmContent.save_playback.is_m3u8 === 'yes';

          if (isM3u8) {
            var m3u8Url = filmContent.save_playback.contents;
            var m3u8Pattern =
              /https:\/\/resourcese\.pku\.edu\.cn\/play\/0\/harpocrates\/\d+\/\d+\/\d+\/([a-zA-Z0-9]+)(\/.+)\/playlist\.m3u8.*/;
            var match = m3u8Url.match(m3u8Pattern);
            if (match) {
              resourceId = match[1];
              downloadUrl =
                'https://course.pku.edu.cn/webapps/bb-streammedia-hqy-BBLEARN/downloadVideo.action?resourceId=' + resourceId +
                (JWT ? '&token=' + encodeURIComponent(JWT) : '');
              console.log('[PKU Course Desktop] M3U8 → download URL:', downloadUrl);
            } else {
              downloadUrl = m3u8Url;
            }
          } else {
            downloadUrl = filmContent.save_playback.contents;
            console.log('[PKU Course Desktop] Direct download URL:', downloadUrl);
          }

          // Notify Svelte app
          ipcSend('video-info', {
            courseName: courseName,
            subTitle: subTitle,
            lecturerName: lecturerName,
            downloadUrl: downloadUrl,
            isM3u8: isM3u8,
            m3u8Url: isM3u8 ? filmContent.save_playback.contents : null,
            resourceId: resourceId || null,
            jwt: JWT,
            fileName: fileName,
            timestamp: Date.now()
          });
        } catch (e) {
          console.error('[PKU Course Desktop] Error parsing video info:', e);
        }
      }
    });
    originalSend.apply(this, arguments);
  };

  // ─── Wait for data & footer, then inject buttons ───
  var OBSERVATION_MS = 5000;
  var injectionStart = Date.now();

  new Promise(function (resolve) {
    var timer = setInterval(function () {
      var footer = document.querySelector('.course-info__wrap .course-info__footer');

      if (downloadJson && footer) {
        clearInterval(timer);
        resolve(true);
        return;
      }
      if (footer && !downloadJson && Date.now() - injectionStart >= OBSERVATION_MS) {
        clearInterval(timer);
        console.warn('[PKU Course Desktop] Footer found but no video data yet');
        resolve(false);
      }
    }, 500);

    // Hard timeout
    setTimeout(function () { clearInterval(timer); resolve(!!downloadJson); }, 30000);
  }).then(function (captured) {
    var footer = document.querySelector('.course-info__wrap .course-info__footer');

    if (captured && downloadJson && footer) {
      injectButtons(footer);
    } else if (footer) {
      // Data not captured – show fallback with "open in browser" only
      injectFallback(footer);
    }
  });

  // ─── Button injection ───
  function injectButtons(footer) {
    // Update title
    var titleEl = document.querySelector('.course-info__wrap .course-info__header > span');
    if (titleEl) {
      titleEl.innerText = courseName + ' - ' + subTitle + ' - ' + lecturerName;
    }

    // Clear existing children
    while (footer.firstChild) footer.removeChild(footer.firstChild);

    // Grid layout (matching PKU-Art's courseVideoPlayFrame.css)
    footer.style.cssText =
      'display:grid!important;width:fit-content!important;' +
      'grid-template-columns:repeat(3,200px)!important;' +
      'justify-content:center!important;align-items:center!important;' +
      'gap:10px!important;margin:10px auto!important;';

    var btnCss =
      'height:35px;border:none;border-radius:4px;font-size:14px;font-weight:bold;' +
      'cursor:pointer;display:flex;align-items:center;justify-content:center;gap:8px;' +
      'padding:0 16px;transition:background 0.15s,color 0.15s;';

    // SVG icons
    var dlSvg = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>';
    var linkSvg = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>';
    var playSvg = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>';

    // Download button
    var dlBtn = document.createElement('button');
    dlBtn.style.cssText = btnCss + 'background:#2563eb;color:#fff;';
    dlBtn.innerHTML = dlSvg + '<span>\u4E0B\u8F7D\u89C6\u9891</span>';

    // Copy link button
    var copyBtn = document.createElement('button');
    copyBtn.style.cssText = btnCss + 'background:#374151;color:#e5e7eb;';
    copyBtn.innerHTML = linkSvg + '<span>\u590D\u5236\u94FE\u63A5</span>';

    // Open in browser button
    var browserBtn = document.createElement('button');
    browserBtn.style.cssText = btnCss + 'background:#059669;color:#fff;';
    browserBtn.innerHTML = playSvg + '<span>\u5728\u6D4F\u89C8\u5668\u64AD\u653E</span>';

    footer.appendChild(dlBtn);
    footer.appendChild(copyBtn);
    footer.appendChild(browserBtn);

    // Feedback area
    var tipDiv = document.createElement('div');
    tipDiv.style.cssText =
      'grid-column:1/-1;padding:10px 16px;border-radius:4px;font-size:13px;' +
      'font-weight:600;line-height:1.5;display:none;';
    footer.appendChild(tipDiv);

    // Handlers
    dlBtn.addEventListener('click', function () {
      ipcSend('add-download', {
        courseName: courseName, subTitle: subTitle, lecturerName: lecturerName,
        downloadUrl: downloadUrl, isM3u8: isM3u8,
        m3u8Url: isM3u8 ? downloadUrl : null,
        resourceId: resourceId || null, jwt: JWT,
        fileName: fileName, timestamp: Date.now()
      });
      tipDiv.style.display = 'block';
      tipDiv.style.background = '#065f46';
      tipDiv.style.color = '#6ee7b7';
      tipDiv.innerHTML = '\u2713 \u5DF2\u6DFB\u52A0\u5230\u4E0B\u8F7D\u961F\u5217\uFF1A' + esc(fileName);
    });

    copyBtn.addEventListener('click', function () {
      var url = downloadUrl;
      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(url).then(function () { showCopied(); });
        } else {
          copyFallback(url);
        }
      } catch (e) { copyFallback(url); }

      function showCopied() {
        tipDiv.style.display = 'block';
        tipDiv.style.background = '#1e3a5f';
        tipDiv.style.color = '#93c5fd';
        tipDiv.textContent = '\u2713 \u4E0B\u8F7D\u94FE\u63A5\u5DF2\u590D\u5236';
      }
      function copyFallback(u) {
        var ta = document.createElement('textarea');
        ta.value = u;
        ta.style.cssText = 'position:fixed;left:-9999px;';
        document.body.appendChild(ta);
        ta.select();
        document.execCommand('copy');
        document.body.removeChild(ta);
        showCopied();
      }
    });

    browserBtn.addEventListener('click', function () {
      ipcSend('open-external', { url: pageUrl });
    });

    console.log('[PKU Course Desktop] Download buttons injected (3-button layout)');
  }

  // Fallback: only show "open in browser" when video data was not captured
  function injectFallback(footer) {
    while (footer.firstChild) footer.removeChild(footer.firstChild);

    footer.style.cssText =
      'display:flex!important;justify-content:center!important;' +
      'align-items:center!important;gap:10px!important;margin:10px auto!important;';

    var browserBtn = document.createElement('button');
    browserBtn.style.cssText =
      'height:40px;border:none;border-radius:6px;font-size:15px;font-weight:bold;' +
      'cursor:pointer;display:flex;align-items:center;justify-content:center;gap:10px;' +
      'padding:0 24px;background:#059669;color:#fff;transition:background 0.15s;';
    browserBtn.innerHTML =
      '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>' +
      '<span>\u5728\u7CFB\u7EDF\u6D4F\u89C8\u5668\u4E2D\u64AD\u653E\u89C6\u9891</span>';

    browserBtn.addEventListener('click', function () {
      ipcSend('open-external', { url: pageUrl });
    });

    footer.appendChild(browserBtn);

    var hint = document.createElement('div');
    hint.style.cssText =
      'text-align:center;font-size:12px;color:#9ca3af;margin-top:6px;width:100%;';
    hint.textContent = 'WebKitGTK \u4E0D\u652F\u6301 HLS \u89C6\u9891\u64AD\u653E\uFF0C\u8BF7\u4F7F\u7528\u7CFB\u7EDF\u6D4F\u89C8\u5668\u89C2\u770B';
    footer.parentNode.appendChild(hint);

    console.log('[PKU Course Desktop] Fallback browser button injected');
  }

  function esc(s) {
    var d = document.createElement('div');
    d.appendChild(document.createTextNode(s || ''));
    return d.innerHTML;
  }

  console.log('[PKU Course Desktop] Video detector initialised');
})();
