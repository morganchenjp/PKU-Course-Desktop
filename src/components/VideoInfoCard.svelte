<script lang="ts">
  import { downloadTasks } from "../lib/store";
  import { createDownloadTask } from "../lib/download-utils";
  import type { VideoInfo } from "../lib/types";

  interface Props {
    videoInfo: VideoInfo;
    onClose: () => void;
    onDownload: () => void;
  }
  
  let { videoInfo, onClose, onDownload }: Props = $props();
  
  let isAddingToQueue = $state(false);
  let addSuccess = $state(false);
  
  async function addToDownloadQueue() {
    isAddingToQueue = true;
    
    try {
      const task = await createDownloadTask(videoInfo);
      downloadTasks.update(tasks => [...tasks, task]);
      
      addSuccess = true;
      setTimeout(() => {
        addSuccess = false;
        onClose();
        onDownload();
      }, 1000);
      
    } catch (error) {
      console.error("Failed to add download task:", error);
      alert("添加到下载队列失败");
    } finally {
      isAddingToQueue = false;
    }
  }
  
  function getVideoTypeLabel(isM3u8: boolean): string {
    return isM3u8 ? 'M3U8 (需转码)' : 'MP4';
  }
</script>

<div class="video-info-card animate-slide-in">
  <div class="card-header">
    <h3 class="card-title">🎥 检测到视频</h3>
    <button class="close-btn" onclick={onClose}>×</button>
  </div>
  
  <div class="card-body">
    <div class="info-row">
      <span class="info-label">课程</span>
      <span class="info-value" title={videoInfo.courseName}>
        {videoInfo.courseName}
      </span>
    </div>
    
    <div class="info-row">
      <span class="info-label">讲次</span>
      <span class="info-value" title={videoInfo.subTitle}>
        {videoInfo.subTitle}
      </span>
    </div>
    
    <div class="info-row">
      <span class="info-label">讲师</span>
      <span class="info-value">{videoInfo.lecturerName}</span>
    </div>
    
    <div class="info-row">
      <span class="info-label">格式</span>
      <span class="info-value type-badge" class:m3u8={videoInfo.isM3u8}>
        {getVideoTypeLabel(videoInfo.isM3u8)}
      </span>
    </div>
  </div>
  
  <div class="card-footer">
    <button class="btn btn-secondary" onclick={onClose}>
      忽略
    </button>
    <button 
      class="btn btn-primary" 
      onclick={addToDownloadQueue}
      disabled={isAddingToQueue}
    >
      {#if isAddingToQueue}
        添加中...
      {:else if addSuccess}
        ✓ 已添加
      {:else}
        添加到下载队列
      {/if}
    </button>
  </div>
</div>

<style>
  .video-info-card {
    position: absolute;
    bottom: 20px;
    right: 20px;
    width: 320px;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    box-shadow: var(--shadow-lg);
    z-index: 100;
  }
  
  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-color);
  }
  
  .card-title {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }
  
  .close-btn {
    width: 24px;
    height: 24px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    font-size: 18px;
    color: var(--text-secondary);
    line-height: 1;
    transition: all 0.2s;
  }
  
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .card-body {
    padding: 12px 16px;
  }
  
  .info-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 6px 0;
  }
  
  .info-label {
    width: 48px;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  
  .info-value {
    flex: 1;
    font-size: 13px;
    word-break: break-all;
  }
  
  .type-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--success-color);
    color: white;
    font-size: 12px;
  }
  
  .type-badge.m3u8 {
    background: var(--warning-color);
  }
  
  .card-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border-color);
  }
  
  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .btn-primary {
    background: var(--accent-color);
    color: white;
  }
  
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  
  .btn-secondary {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .btn-secondary:hover {
    background: var(--border-color);
  }
  
  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
