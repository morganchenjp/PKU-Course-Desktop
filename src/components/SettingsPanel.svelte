<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { settings } from "../lib/store";
  import { getDefaultNamingPatterns } from "../lib/naming";
  
  const namingPatterns = getDefaultNamingPatterns();
  
  async function selectDownloadPath() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: $settings.downloadPath,
      });
      
      if (selected) {
        settings.update(s => ({ ...s, downloadPath: selected }));
      }
    } catch (error) {
      console.error("Failed to select directory:", error);
    }
  }
  
  function saveSettings() {
    // Settings are automatically saved via store subscription
    alert("设置已保存");
  }
  
  function resetSettings() {
    if (confirm("确定要恢复默认设置吗？")) {
      settings.set({
        downloadPath: '',
        namingPattern: '{courseName} - {subTitle} - {lecturerName}',
        autoDownload: false,
        maxConcurrentDownloads: 3,
        defaultQuality: 'highest',
        extractAudio: false,
        audioFormat: 'mp3',
      });
    }
  }
</script>

<div class="settings-panel">
  <header class="panel-header">
    <h2 class="panel-title">设置</h2>
  </header>
  
  <div class="settings-content">
    <section class="setting-section">
      <h3 class="section-title">下载设置</h3>
      
      <div class="setting-item">
        <label class="setting-label" for="download-path">下载路径</label>
        <div class="setting-control path-control">
          <input 
            id="download-path"
            type="text" 
            value={$settings.downloadPath} 
            placeholder="默认下载到系统下载文件夹"
            readonly
          />
          <button class="btn btn-secondary" onclick={selectDownloadPath}>
            选择文件夹
          </button>
        </div>
      </div>
      
      <div class="setting-item">
        <label class="setting-label" for="naming-pattern">文件命名规则</label>
        <div class="setting-control">
          <select id="naming-pattern" bind:value={$settings.namingPattern}>
            {#each namingPatterns as pattern}
              <option value={pattern.value}>{pattern.label}</option>
            {/each}
          </select>
          <p class="setting-hint">
            可用变量：{'{courseName}'}, {'{subTitle}'}, {'{lecturerName}'}, {'{date}'}, {'{index}'}
          </p>
        </div>
      </div>
      
      <div class="setting-item">
        <label class="setting-label" for="max-concurrent">最大并发下载数</label>
        <div class="setting-control">
          <input 
            id="max-concurrent"
            type="number" 
            min="1" 
            max="10" 
            bind:value={$settings.maxConcurrentDownloads}
          />
        </div>
      </div>
      
      <div class="setting-item">
        <label class="setting-label">
          <input type="checkbox" bind:checked={$settings.autoDownload} />
          自动开始下载（检测到视频时自动添加到队列并开始）
        </label>
      </div>
    </section>
    
    <section class="setting-section">
      <h3 class="section-title">视频设置</h3>
      
      <div class="setting-item">
        <label class="setting-label" for="default-quality">默认视频质量</label>
        <div class="setting-control">
          <select id="default-quality" bind:value={$settings.defaultQuality}>
            <option value="highest">最高质量</option>
            <option value="high">高质量</option>
            <option value="medium">中等质量</option>
            <option value="low">低质量（节省空间）</option>
          </select>
        </div>
      </div>
      
      <div class="setting-item">
        <label class="setting-label">
          <input type="checkbox" bind:checked={$settings.extractAudio} />
          同时提取音频文件
        </label>
      </div>
      
      {#if $settings.extractAudio}
        <div class="setting-item indented">
          <label class="setting-label" for="audio-format">音频格式</label>
          <div class="setting-control">
            <select id="audio-format" bind:value={$settings.audioFormat}>
              <option value="mp3">MP3</option>
              <option value="aac">AAC</option>
              <option value="wav">WAV</option>
            </select>
          </div>
        </div>
      {/if}
    </section>
    
    <div class="about-donation-row">
      <section class="setting-section about-col">
        <h3 class="section-title">关于</h3>
        <div class="about-content">
          <p><strong>PKU Course Desktop</strong></p>
          <p>版本: 0.2.0</p>
          <p>开源北大课程视频下载工具 <a href="https://github.com/morganchenjp/PKU-Course-Desktop/" onclick={(e) => { e.preventDefault(); invoke('open_external_link', { url: 'https://github.com/morganchenjp/PKU-Course-Desktop/' }); }}>
              GitHub</a></p>
          <p>
            Inspired by <a href="https://github.com/zhuozhiyongde/PKU-Art" onclick={(e) => { e.preventDefault(); invoke('open_external_link', { url: 'https://github.com/zhuozhiyongde/PKU-Art' }); }}>
              PKU-Art project
            </a>
          </p>
        </div>
      </section>

      <section class="setting-section donation-col">
        <h3 class="section-title">Donation</h3>
        <div class="about-content donation-section">
          <p>Buy me a coffee via WeChat Pay</p>
          <img class="qrcode-img" src="/morgan-wechat-qrcode.png" alt="WeChat Pay QR Code" />
        </div>
      </section>
    </div>
  </div>
  
  <div class="settings-footer">
    <button class="btn btn-secondary" onclick={resetSettings}>
      恢复默认
    </button>
    <button class="btn btn-primary" onclick={saveSettings}>
      保存设置
    </button>
  </div>
</div>

<style>
  .settings-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    max-width: 800px;
    margin: 0 auto;
  }
  
  .panel-header {
    margin-bottom: 24px;
  }
  
  .panel-title {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }
  
  .settings-content {
    flex: 1;
    overflow-y: auto;
  }
  
  .setting-section {
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--border-color);
  }
  
  .setting-section:last-of-type {
    border-bottom: none;
  }
  
  .section-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 16px 0;
  }
  
  .setting-item {
    margin-bottom: 20px;
  }
  
  .setting-item.indented {
    margin-left: 24px;
  }
  
  .setting-label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 8px;
    cursor: pointer;
  }
  
  .setting-control {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  
  .path-control {
    flex-direction: row;
    gap: 8px;
  }
  
  .path-control input {
    flex: 1;
  }
  
  .setting-hint {
    font-size: 12px;
    color: var(--text-tertiary);
    margin: 0;
  }
  
  .about-content {
    font-size: 14px;
    line-height: 1.6;
  }
  
  .about-content p {
    margin: 0 0 8px 0;
  }
  
  .about-content a {
    color: var(--accent-color);
    text-decoration: none;
  }
  
  .about-content a:hover {
    text-decoration: underline;
  }

  .about-donation-row {
    display: flex;
    gap: 32px;
  }

  .about-col {
    flex: 1;
  }

  .donation-col {
    flex: 0 0 auto;
  }

  .donation-section {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .qrcode-img {
    width: 200px;
    height: auto;
    border-radius: 8px;
  }
  
  .settings-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding-top: 16px;
    border-top: 1px solid var(--border-color);
  }
  
  .btn {
    padding: 8px 20px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .btn-primary {
    background: var(--accent-color);
    color: white;
  }
  
  .btn-primary:hover {
    background: var(--accent-hover);
  }
  
  .btn-secondary {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  .btn-secondary:hover {
    background: var(--border-color);
  }
  
  input[type="text"],
  input[type="number"],
  select {
    padding: 8px 12px;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 14px;
  }
  
  input[type="checkbox"] {
    margin-right: 8px;
  }
  
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent-color);
  }
</style>
