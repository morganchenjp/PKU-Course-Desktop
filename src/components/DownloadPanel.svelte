<script lang="ts">
  import { downloadTasks } from "../lib/store";
  import DownloadTaskItem from "./DownloadTaskItem.svelte";
  import { derived } from 'svelte/store';
  
  let filterStatus = $state('all');
  
  const filteredTasks = derived(downloadTasks, $tasks => {
    if (filterStatus === 'all') return $tasks;
    return $tasks.filter(task => task.status === filterStatus);
  });
  
  const stats = derived(downloadTasks, $tasks => ({
    total: $tasks.length,
    pending: $tasks.filter(t => t.status === 'pending').length,
    downloading: $tasks.filter(t => t.status === 'downloading').length,
    completed: $tasks.filter(t => t.status === 'completed').length,
    error: $tasks.filter(t => t.status === 'error').length,
  }));
  
  function clearCompleted() {
    downloadTasks.update(tasks => tasks.filter(t => t.status !== 'completed'));
  }
  
  function clearAll() {
    if (confirm('确定要清空所有下载任务吗？')) {
      downloadTasks.set([]);
    }
  }
</script>

<div class="download-panel">
  <header class="panel-header">
    <h2 class="panel-title">下载管理</h2>
    <div class="stats-bar">
      <span class="stat-item">总计: {$stats.total}</span>
      <span class="stat-item pending">待下载: {$stats.pending}</span>
      <span class="stat-item downloading">下载中: {$stats.downloading}</span>
      <span class="stat-item completed">已完成: {$stats.completed}</span>
      {#if $stats.error > 0}
        <span class="stat-item error">失败: {$stats.error}</span>
      {/if}
    </div>
  </header>
  
  <div class="panel-toolbar">
    <div class="filter-tabs">
      <button 
        class="filter-tab" 
        class:active={filterStatus === 'all'}
        onclick={() => filterStatus = 'all'}
      >
        全部
      </button>
      <button 
        class="filter-tab" 
        class:active={filterStatus === 'downloading'}
        onclick={() => filterStatus = 'downloading'}
      >
        下载中
      </button>
      <button 
        class="filter-tab" 
        class:active={filterStatus === 'pending'}
        onclick={() => filterStatus = 'pending'}
      >
        等待中
      </button>
      <button 
        class="filter-tab" 
        class:active={filterStatus === 'completed'}
        onclick={() => filterStatus = 'completed'}
      >
        已完成
      </button>
      <button 
        class="filter-tab" 
        class:active={filterStatus === 'error'}
        onclick={() => filterStatus = 'error'}
      >
        失败
      </button>
    </div>
    
    <div class="toolbar-actions">
      <button class="btn btn-secondary" onclick={clearCompleted}>
        清空已完成
      </button>
      <button class="btn btn-danger" onclick={clearAll}>
        清空全部
      </button>
    </div>
  </div>
  
  <div class="task-list">
    {#if $filteredTasks.length === 0}
      <div class="empty-state">
        <div class="empty-icon">📭</div>
        <p class="empty-text">
          {filterStatus === 'all' ? '暂无下载任务' : '该分类下暂无任务'}
        </p>
        <p class="empty-hint">在浏览器中访问录播页面即可添加下载</p>
      </div>
    {:else}
      {#each $filteredTasks as task (task.id)}
        <DownloadTaskItem {task} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .download-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
  }
  
  .panel-header {
    margin-bottom: 16px;
  }
  
  .panel-title {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 12px 0;
  }
  
  .stats-bar {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }
  
  .stat-item {
    font-size: 13px;
    color: var(--text-secondary);
  }
  
  .stat-item.pending { color: var(--warning-color); }
  .stat-item.downloading { color: var(--info-color); }
  .stat-item.completed { color: var(--success-color); }
  .stat-item.error { color: var(--error-color); }
  
  .panel-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-color);
  }
  
  .filter-tabs {
    display: flex;
    gap: 4px;
  }
  
  .filter-tab {
    padding: 6px 12px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    border-radius: 6px;
    transition: all 0.2s;
  }
  
  .filter-tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .filter-tab.active {
    background: var(--accent-color);
    color: white;
  }
  
  .toolbar-actions {
    display: flex;
    gap: 8px;
  }
  
  .btn {
    padding: 6px 12px;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .btn-secondary {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .btn-secondary:hover {
    background: var(--border-color);
  }
  
  .btn-danger {
    background: var(--danger-color);
    color: white;
  }
  
  .btn-danger:hover {
    background: var(--danger-hover);
  }
  
  .task-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
  
  .empty-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }
  
  .empty-text {
    font-size: 16px;
    margin: 0 0 8px 0;
  }
  
  .empty-hint {
    font-size: 13px;
    color: var(--text-tertiary);
  }
</style>
