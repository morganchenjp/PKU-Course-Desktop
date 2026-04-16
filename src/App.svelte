<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import BrowserView from "./components/BrowserView.svelte";
  import DownloadPanel from "./components/DownloadPanel.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import { currentView, theme, downloadTasks, settings } from "./lib/store";
  import { initTheme } from "./lib/theme";
  import { createDownloadTask } from "./lib/download-utils";

  let unlistenSwitchToMain: (() => void) | null = null;
  let isTransitioning = false;
  let unlistenAddDownload: (() => void) | null = null;
  let unlistenDlProgress: (() => void) | null = null;
  let unlistenDlComplete: (() => void) | null = null;
  let unlistenDlError: (() => void) | null = null;

  onMount(async () => {
    initTheme();

    // On startup, if currentView is 'browser', ensure the browser webview is shown.
    // This is the only place show_browser_view should be called at startup.
    if ($currentView === 'browser') {
      console.log('[App] startup: showing browser view');
      try {
        await invoke('show_browser_view');
      } catch (e) {
        console.error('[App] startup show_browser_view failed:', e);
      }
    }

    // Listen for view-switch events from the browser webview's injected nav-bar.
    // When the user clicks "下载管理" or "设置" in the injected toolbar,
    // Rust emits this event after showing the main webview.
    // Only update local view state here — do NOT call invoke() since nav-bar IPC
    // already handled the full switch (view resizing + event emission).
    unlistenSwitchToMain = await listen("switch-to-main", (event: any) => {
      console.log('[DEBUG Svelte] switch-to-main event received:', JSON.stringify(event.payload));
      const view = event.payload?.view;
      console.log('[DEBUG Svelte] setting currentView to:', view);
      if (view === 'downloads' || view === 'settings') {
        currentView.set(view);
      }
    });

    // Listen for download requests from the browser webview's injected scripts
    // (download button on video player page, or nav-bar toast "添加到下载队列").
    unlistenAddDownload = await listen("add-download-from-browser", async (event: any) => {
      try {
        const videoInfo = event.payload;
        const task = await createDownloadTask(videoInfo);
        const downloadingCount = $downloadTasks.filter(t => t.status === 'downloading').length;
        if (downloadingCount < $settings.maxConcurrentDownloads) {
          // Under limit: start immediately
          const startedTask = { ...task, status: 'downloading' as const, startedAt: Date.now() };
          downloadTasks.update(tasks => [...tasks, startedTask]);
          console.log('[App] Download task added, starting:', task.filename);
          await invoke('browser_download', {
            taskId: startedTask.id,
            url: startedTask.videoInfo.downloadUrl,
            filepath: startedTask.filepath,
          });
        } else {
          // At limit: queue as pending
          const pendingTask = { ...task, status: 'pending' as const };
          downloadTasks.update(tasks => [...tasks, pendingTask]);
          console.log('[App] Download task added to queue (max concurrent reached):', task.filename);
        }
      } catch (e) {
        console.error('[App] Failed to add/start download task:', e);
      }
    });

    // Listen for download progress/completion/error events from Rust
    unlistenDlProgress = await listen("download-progress", (event: any) => {
      const { taskId, progress, speed, eta } = event.payload;
      downloadTasks.update(tasks =>
        tasks.map(t =>
          t.id === taskId ? { ...t, progress, speed, eta } : t
        )
      );
    });

    unlistenDlComplete = await listen("download-complete", (event: any) => {
      const { taskId } = event.payload;
      downloadTasks.update(tasks =>
        tasks.map(t =>
          t.id === taskId
            ? { ...t, status: 'completed' as const, progress: 100, completedAt: Date.now() }
            : t
        )
      );
      console.log('[App] Download completed:', taskId);
      // A slot opened — try to start the next pending download
      startNextPendingDownload();
    });

    unlistenDlError = await listen("download-error", (event: any) => {
      const { taskId, error } = event.payload;
      downloadTasks.update(tasks =>
        tasks.map(t =>
          t.id === taskId
            ? { ...t, status: 'error' as const, error }
            : t
        )
      );
      console.error('[App] Download error:', taskId, error);
      // A slot opened — try to start the next pending download
      startNextPendingDownload();
    });
  });

  onDestroy(() => {
    if (unlistenSwitchToMain) unlistenSwitchToMain();
    if (unlistenAddDownload) unlistenAddDownload();
    if (unlistenDlProgress) unlistenDlProgress();
    if (unlistenDlComplete) unlistenDlComplete();
    if (unlistenDlError) unlistenDlError();
  });

  async function startNextPendingDownload() {
    let tasks_snapshot: typeof $downloadTasks = [];
    downloadTasks.update(tasks => {
      tasks_snapshot = tasks;
      return tasks;
    });
    const downloadingCount = tasks_snapshot.filter(t => t.status === 'downloading').length;
    if (downloadingCount >= $settings.maxConcurrentDownloads) return;
    const pending = tasks_snapshot.find(t => t.status === 'pending');
    if (!pending) return;
    downloadTasks.update(tasks =>
      tasks.map(t =>
        t.id === pending.id
          ? { ...t, status: 'downloading' as const, startedAt: Date.now() }
          : t
      )
    );
    try {
      await invoke('browser_download', {
        taskId: pending.id,
        url: pending.videoInfo.downloadUrl,
        filepath: pending.filepath,
      });
      console.log('[App] Started queued download:', pending.filename);
    } catch (e) {
      console.error('[App] Failed to start queued download:', e);
    }
  }

  async function switchToBrowser() {
    console.log('[DEBUG Svelte] switchToBrowser clicked');
    try {
      await invoke('show_browser_view');
    } catch (e) {
      console.error('[App] show_browser_view failed:', e);
    }
    currentView.set('browser');
  }

  async function switchToDownloads() {
    console.log('[DEBUG Svelte] switchToDownloads clicked');
    try {
      await invoke('show_main_view', { view: 'downloads' });
    } catch (e) {
      console.error('[App] show_main_view failed:', e);
    }
    currentView.set('downloads');
  }

  async function switchToSettings() {
    console.log('[DEBUG Svelte] switchToSettings clicked');
    try {
      await invoke('show_main_view', { view: 'settings' });
    } catch (e) {
      console.error('[App] show_main_view failed:', e);
    }
    currentView.set('settings');
  }
</script>

<div class="app" data-theme={$theme}>
  <header class="app-header">
    <div class="logo">
      <img class="logo-icon" src="/app-icon.png" alt="Logo" />
      <span class="logo-text">PKU Course Desktop</span>
    </div>
    <nav class="nav-tabs">
      <button 
        class="nav-tab" 
        class:active={$currentView === 'browser'}
        onclick={switchToBrowser}
      >
        浏览器
      </button>
      <button 
        class="nav-tab" 
        class:active={$currentView === 'downloads'}
        onclick={switchToDownloads}
      >
        下载管理
      </button>
      <button 
        class="nav-tab" 
        class:active={$currentView === 'settings'}
        onclick={switchToSettings}
      >
        设置
      </button>
    </nav>
    <div class="window-controls">
      <button class="theme-toggle" onclick={() => theme.update(t => t === 'light' ? 'dark' : 'light')}>
        {$theme === 'light' ? '🌙' : '☀️'}
      </button>
    </div>
  </header>

  <main class="app-main" class:browser-mode={$currentView === 'browser'}>
    {#if $currentView === 'browser'}
      <BrowserView />
    {:else if $currentView === 'downloads'}
      <DownloadPanel />
    {:else if $currentView === 'settings'}
      <SettingsPanel />
    {/if}
  </main>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .app-header {
    display: flex;
    align-items: center;
    padding: 0 16px;
    height: 48px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    -webkit-app-region: drag;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 16px;
    -webkit-app-region: no-drag;
  }

  .logo-icon {
    width: 24px;
    height: 24px;
    object-fit: contain;
  }

  .nav-tabs {
    display: flex;
    gap: 4px;
    margin-left: 32px;
    -webkit-app-region: no-drag;
  }

  .nav-tab {
    padding: 6px 16px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    border-radius: 6px;
    transition: all 0.2s;
  }

  .nav-tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-tab.active {
    background: var(--accent-color);
    color: white;
  }

  .window-controls {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
  }

  .theme-toggle {
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    font-size: 18px;
    cursor: pointer;
    border-radius: 6px;
    transition: background 0.2s;
  }

  .theme-toggle:hover {
    background: var(--bg-hover);
  }

  .app-main {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  /* When browser view is active, make main webview transparent and non-interactive
     so clicks pass through to the browser webview underneath. The nav-bar buttons
     are behind the Svelte header, so we must remove input handling entirely. */
  .app-main.browser-mode {
    display: none;
  }
</style>
