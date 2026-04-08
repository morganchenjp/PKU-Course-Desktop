<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { downloadTasks } from "../lib/store";
  import type { DownloadTask } from "../lib/types";
  
  let { task }: { task: DownloadTask } = $props();
  
  let isRetrying = $state(false);
  
  function getStatusLabel(status: string): string {
    const labels: Record<string, string> = {
      pending: '等待中',
      downloading: '下载中',
      paused: '已暂停',
      completed: '已完成',
      error: '失败',
    };
    return labels[status] || status;
  }
  
  function getStatusClass(status: string): string {
    return `status-${status}`;
  }
  
  async function startDownload() {
    downloadTasks.update(tasks => 
      tasks.map(t => 
        t.id === task.id 
          ? { ...t, status: 'downloading', startedAt: Date.now() }
          : t
      )
    );
    
    try {
      await invoke('browser_download', {
        taskId: task.id,
        url: task.videoInfo.downloadUrl,
        filepath: task.filepath,
      });
    } catch (error) {
      console.error("Download failed:", error);
      downloadTasks.update(tasks => 
        tasks.map(t => 
          t.id === task.id 
            ? { ...t, status: 'error', error: String(error) }
            : t
        )
      );
    }
  }
  
  async function pauseDownload() {
    await invoke('pause_download', { taskId: task.id });
    downloadTasks.update(tasks => 
      tasks.map(t => 
        t.id === task.id ? { ...t, status: 'paused' } : t
      )
    );
  }
  
  async function retryDownload() {
    isRetrying = true;
    await startDownload();
    isRetrying = false;
  }
  
  function removeTask() {
    downloadTasks.update(tasks => tasks.filter(t => t.id !== task.id));
  }
  
  function openFileLocation() {
    invoke('open_file_location', { path: task.filepath });
  }
  
  const progressPercent = $derived(Math.round(task.progress));
  const isIndeterminate = $derived(task.progress < 0);
</script>

<div class="task-item">
  <div class="task-main">
    <div class="task-info">
      <div class="task-filename" title={task.filename}>
        {task.filename}
      </div>
      <div class="task-meta">
        <span class="status-badge {getStatusClass(task.status)}">
          {getStatusLabel(task.status)}
        </span>
        {#if task.status === 'downloading'}
          <span class="speed">{task.speed}</span>
          {#if isIndeterminate}
            <span class="eta">已下载 {task.eta}</span>
          {:else}
            <span class="eta">剩余 {task.eta}</span>
          {/if}
        {/if}
        {#if task.error}
          <span class="error-msg" title={task.error}>{task.error}</span>
        {/if}
      </div>
    </div>
    
    <div class="task-actions">
      {#if task.status === 'pending'}
        <button class="action-btn" onclick={startDownload} title="开始">
          ▶️
        </button>
      {:else if task.status === 'downloading'}
        <button class="action-btn" onclick={pauseDownload} title="暂停">
          ⏸️
        </button>
      {:else if task.status === 'paused'}
        <button class="action-btn" onclick={startDownload} title="继续">
          ▶️
        </button>
      {:else if task.status === 'error'}
        <button class="action-btn" onclick={retryDownload} disabled={isRetrying} title="重试">
          {isRetrying ? '...' : '🔄'}
        </button>
      {:else if task.status === 'completed'}
        <button class="action-btn" onclick={openFileLocation} title="打开位置">
          📂
        </button>
      {/if}
      
      <button class="action-btn delete" onclick={removeTask} title="删除">
        🗑️
      </button>
    </div>
  </div>
  
  {#if task.status === 'downloading' || task.status === 'completed'}
    <div class="progress-bar">
      {#if isIndeterminate && task.status === 'downloading'}
        <div class="progress-fill indeterminate"></div>
        <span class="progress-text">{task.eta}</span>
      {:else}
        <div class="progress-fill" style="width: {progressPercent}%"></div>
        <span class="progress-text">{progressPercent}%</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .task-item {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 12px;
  }
  
  .task-main {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  
  .task-info {
    flex: 1;
    min-width: 0;
  }
  
  .task-filename {
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .task-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  
  .status-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
  }
  
  .status-pending {
    background: var(--warning-color);
    color: white;
  }
  
  .status-downloading {
    background: var(--info-color);
    color: white;
  }
  
  .status-paused {
    background: var(--text-tertiary);
    color: white;
  }
  
  .status-completed {
    background: var(--success-color);
    color: white;
  }
  
  .status-error {
    background: var(--error-color);
    color: white;
  }
  
  .speed, .eta {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .error-msg {
    font-size: 12px;
    color: var(--error-color);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .task-actions {
    display: flex;
    gap: 4px;
  }
  
  .action-btn {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    transition: background 0.2s;
  }
  
  .action-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  
  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .action-btn.delete:hover {
    background: var(--danger-color);
    color: white;
  }
  
  .progress-bar {
    margin-top: 8px;
    height: 4px;
    background: var(--border-color);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }
  
  .progress-fill {
    height: 100%;
    background: var(--accent-color);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .progress-fill.indeterminate {
    width: 30%;
    animation: indeterminate 1.5s ease-in-out infinite;
  }

  @keyframes indeterminate {
    0%   { transform: translateX(-100%); }
    100% { transform: translateX(433%); }
  }
  
  .progress-text {
    position: absolute;
    right: 0;
    top: -18px;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
