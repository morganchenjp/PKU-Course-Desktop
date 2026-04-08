<script lang="ts">
  import { onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { videoUrl } from "../lib/store";

  onDestroy(async () => {
    try {
      await invoke('destroy_video_view');
      console.log('[VideoView] destroy_video_view on unmount');
    } catch (_) { /* may already be destroyed */ }
  });
</script>

<div class="video-view">
  {#if $videoUrl}
    <div class="video-status">
      <span class="status-dot"></span>
      <span class="status-text">正在播放视频</span>
    </div>
  {:else}
    <div class="video-empty">
      <div class="empty-icon">🎬</div>
      <div class="empty-title">在线视频</div>
      <div class="empty-desc">在浏览器中点击视频链接后，视频将在此处播放</div>
    </div>
  {/if}
</div>

<style>
  .video-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    position: relative;
  }

  .video-empty {
    text-align: center;
    color: var(--text-secondary);
  }

  .empty-icon {
    font-size: 64px;
    margin-bottom: 16px;
  }

  .empty-title {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .empty-desc {
    font-size: 14px;
    max-width: 300px;
    line-height: 1.5;
  }

  .video-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    background: var(--bg-secondary);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4caf50;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
