# PKU Course Desktop - 设计文档

> 版本: 0.2.0 | 最后更新: 2026-04-08

## 1. 项目概述

PKU Course Desktop 是一款基于 Tauri 2.0 构建的北大课程视频下载桌面应用。通过内嵌浏览器加载课程平台，自动检测录播视频并提供一键下载能力，支持 HLS (m3u8) 流媒体播放与转码。

### 1.1 核心功能

- 内置浏览器，支持PKU IAAA 统一认证登录
- 自动检测录播视频，拦截视频元数据（课程名、讲师、M3U8 地址、JWT）
- 在浏览器内直接播放 HLS 视频（通过 HLS.js）
- 下载队列管理，支持并发下载与进度追踪
- M3U8 转 MP4（FFmpeg）
- 日间/夜间主题切换
- 可配置文件命名规则
- TODO:视频中的音频提取（MP3/AAC/WAV）

### 1.2 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 前端框架 | Svelte 5 + TypeScript | 5.0 / 5.6 |
| 构建工具 | Vite | 6.0 |
| 包管理器 | Bun | 1.0+ |
| 桌面框架 | Tauri | 2.10 |
| 后端语言 | Rust | 1.77+ |
| HTTP 客户端 | reqwest | 0.12 |
| 异步运行时 | tokio | 1.x |
| 视频处理 | FFmpeg | 外部依赖 |
| HLS 播放 | hls.js | 内嵌 |
| Linux WebView | WebKitGTK | 2.0 |

### 1.3 支持平台

| 平台 | 构建目标 | 产物格式 |
|------|----------|----------|
| macOS (Apple Silicon) | aarch64-apple-darwin | .dmg |
| macOS (Intel) | x86_64-apple-darwin | .dmg |
| Ubuntu 22.04+ | x86_64-unknown-linux-gnu | .AppImage, .deb |
| Windows 10+ | x86_64-pc-windows-msvc | .msi, .exe |

---

## 2. 系统架构

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────┐
│                  Tauri Window (main)                  │
│  ┌─────────────────────┐  ┌───────────────────────┐  │
│  │   Main WebView       │  │   Browser WebView      │  │
│  │   (Svelte App)       │  │   (course.pku.edu.cn)  │  │
│  │                      │  │                        │  │
│  │  ┌──────────────┐   │  │  ┌──────────────────┐  │  │
│  │  │ DownloadPanel│   │  │  │ nav-bar.js       │  │  │
│  │  │ SettingsPanel│   │  │  │ video-detector.js│  │  │
│  │  │ BrowserView  │   │  │  │ hls-player.js    │  │  │
│  │  │ VideoInfoCard│   │  │  │ hls.min.js       │  │  │
│  │  └──────────────┘   │  │  └──────────────────┘  │  │
│  └─────────────────────┘  └───────────────────────┘  │
│                                                       │
│  ┌───────────────────────────────────────────────┐   │
│  │              Rust Backend (Tauri)              │   │
│  │  main.rs | download.rs | ffmpeg.rs | settings │   │
│  │  IPC Handler | URI Scheme | Event Emitter     │   │
│  └───────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### 2.2 双 WebView 架构

应用采用双 WebView 架构，共享同一个 Tauri 窗口：

- **Main WebView**：运行 Svelte 前端 UI，负责下载管理、设置、视频信息展示
- **Browser WebView**：加载 PKU 课程平台（`course.pku.edu.cn`），注入 4 个脚本

两个 WebView 通过 Rust 后端控制可见性和尺寸，同一时间只显示一个。视图切换通过 `show_browser_view` / `show_main_view` 命令实现。

### 2.3 IPC 通信机制

由于 Browser WebView 加载的是远端 URL，无法使用 Tauri 的 `window.__TAURI__` API。因此采用以下通信方案：

#### 自定义 URI Scheme (`pku-ipc://`)

注入脚本通过 `XMLHttpRequest` 向 `pku-ipc://localhost/<route>` 发送请求，Rust 端通过 `register_uri_scheme_protocol` 注册处理器。

支持的路由：
| 路由 | 方法 | 功能 |
|------|------|------|
| `/download-diag` | POST | 下载诊断日志 |
| `/show-main-view` | POST | 切换到主视图（下载/设置） |
| `/video-info` | POST | 接收视频元数据，emit 到前端 |
| `/add-download` | POST | 添加下载任务 |
| `/open-external` | POST | 在系统浏览器中打开 URL |

#### PostMessage 中继

跨域 iframe（如 `onlineroomse.pku.edu.cn`）中的脚本无法直接发起 `pku-ipc://` 请求。解决方案：

1. iframe 内的脚本通过 `window.parent.postMessage()` 发送消息
2. 顶层页面的 `hls-player.js` 监听 `message` 事件
3. 中继转发到 `pku-ipc://` 端点

---

## 3. 模块详解

### 3.1 Rust 后端 (`src-tauri/src/`)

#### `main.rs` (~916 行)

核心入口文件，职责：

- **AppState 管理**：下载管理器、设置、视图模式、待处理下载
- **Tauri 命令**：15 个 `#[tauri::command]` 函数，涵盖下载、导航、设置、文件操作
- **自定义 IPC 协议**：`pku-ipc://` URI scheme handler
- **WebView 生命周期**：setup 中预创建 browser-webview，注入脚本
- **窗口事件**：resize handler 根据当前视图模式调整 WebView 尺寸
- **跨平台下载**：Linux 使用 WebKitGTK 原生 `download_uri()`，macOS/Windows 使用 reqwest 流式下载

```rust
pub struct AppState {
    download_manager: Mutex<DownloadManager>,
    settings: Mutex<AppSettings>,
    current_view_mode: StdMutex<String>,       // "browser" | "main"
    pending_downloads: StdMutex<HashMap<String, PendingBrowserDownload>>,
}
```

#### `download.rs` (~248 行)

下载任务管理：

- `DownloadManager`：管理活动下载，持有 abort handles
- `download_file()`：tokio 异步下载，支持 JWT 认证，500ms 节流进度上报
- 下载完成后自动触发 M3U8 → MP4 转码

#### `ffmpeg.rs` (~148 行)

FFmpeg 命令封装：

- `convert_m3u8_to_mp4()`：HLS 流转码，`-c copy` 无损拷贝，`-movflags +faststart`
- `extract_audio()`：提取音频，支持 MP3(VBR)/AAC(192kbps)/WAV
- JWT 认证 header 注入

#### `settings.rs` (~58 行)

设置持久化到 `~/.config/pku-course-desktop/settings.json`。

### 3.2 注入脚本 (`src-tauri/inject-scripts/`)

这些脚本通过 `initialization_script()` 注入到 Browser WebView，在每个页面加载时自动执行。

#### `nav-bar.js` (~314 行)

在浏览器页面顶部注入导航工具栏：

- 导航按钮（后退/前进/刷新/首页）
- URL 地址栏
- 视图切换按钮（下载管理、设置）
- 拦截 `target="_blank"` 链接和 `window.open()` 调用
- 视频 URL 检测：识别 `playVideo`、`bb-streammedia`、`onlineroomse.pku.edu.cn/player` 等模式
- PostMessage IPC 中继

#### `video-detector.js` (~343 行)

视频检测与下载按钮注入，分两阶段执行：

**阶段 1（所有页面）**：
- Override `HTMLVideoElement.prototype.canPlayType`
- 声称支持 HLS MIME 类型（`application/vnd.apple.mpegurl`）
- 欺骗 cmcPlayer.js 认为浏览器原生支持 HLS，触发 API 请求

**阶段 2（仅限 player 页面）**：
- 拦截 `XMLHttpRequest.send()`
- 从 `Authorization` header 捕获 JWT
- 解析 `get-sub-info-by-auth-data` API 响应
- 提取课程名、副标题、讲师名、M3U8 URL
- 将 M3U8 URL 转换为 Blackboard 下载 API 端点
- 注入 3 按钮 UI：下载视频、复制链接、在浏览器播放

#### `hls-player.js` (~299 行)

HLS.js 集成与 iframe 导航：

1. **PostMessage IPC 中继**：转发 iframe 消息到 Rust
2. **Iframe 自动导航**：检测包装页面中的 player iframe，自动导航到播放器 URL
   - MutationObserver 监听 DOM 变化
   - 500ms 轮询 fallback
   - 30 秒超时
3. **HLS.js 播放**：
   - 拦截 `video.src` setter 和 `setAttribute('src')`
   - 监听 `<source>` 元素变化
   - 对 `.m3u8` URL 启动 HLS.js 播放
   - 配置：自动质量选择，30-600s 缓冲

#### `hls.min.js` (~532 KB)

HLS.js 库，提供 `window.Hls` 全局对象，用于 MediaSource API 客户端 HLS 播放。

### 3.3 Svelte 前端 (`src/`)

#### `App.svelte` - 根组件

- 3 个视图标签：浏览器、下载管理、设置
- 顶部导航栏带 Logo 和主题切换
- 事件监听：`switch-to-main`、`add-download-from-browser`、`download-progress/complete/error`

#### `components/BrowserView.svelte` - 浏览器视图

- 调用 `show_browser_view` 显示 browser-webview
- 监听 `webview-message` 事件，展示 `VideoInfoCard`

#### `components/DownloadPanel.svelte` - 下载管理

- 过滤标签（全部/下载中/待处理/已完成/失败）
- 实时统计数据
- 清理操作（清空已完成/全部清空）

#### `components/DownloadTaskItem.svelte` - 下载任务项

- 状态徽章、进度条、速度/ETA 显示
- 操作按钮（开始/暂停/重试/打开位置/删除）

#### `components/SettingsPanel.svelte` - 设置面板

- 下载设置：路径、命名规则、并发数、自动下载
- 视频设置：默认质量、音频提取、音频格式
- 关于信息与 Donation（左右并排布局）

#### `components/VideoInfoCard.svelte` - 视频信息卡片

- 底部右侧滑入动画
- 显示课程名、讲师、格式标签
- 一键添加到下载队列

### 3.4 状态管理 (`src/lib/store.ts`)

使用 Svelte writable store：

| Store | 类型 | 用途 |
|-------|------|------|
| `currentView` | `'browser' \| 'downloads' \| 'settings'` | 当前视图 |
| `theme` | `'light' \| 'dark'` | 主题模式 |
| `downloadTasks` | `DownloadTask[]` | 下载任务列表 |
| `currentVideoInfo` | `VideoInfo \| null` | 当前检测到的视频 |
| `settings` | `AppSettings` | 用户设置 |
| `browserState` | `{ url, canGoBack, ... }` | 浏览器状态 |

### 3.5 样式系统 (`src/styles/`)

使用 CSS 自定义属性实现主题系统：

- **Light 主题**：浅灰背景（#f6f8fa），北大红强调色（#9b0000）
- **Dark 主题**：深色背景（#0d1117），珊瑚红强调色（#e44c47）
- 语义化颜色变量：success(#52c41a)、warning(#faad14)、error(#f5222d)、info(#1890ff)

---

## 4. 数据流

### 4.1 视频检测流程

```
用户浏览课程页面
    │
    ▼
nav-bar.js 检测到视频链接
    │
    ▼
window.location.href 页内导航到播放器
    │
    ▼
hls-player.js 检测到包装页面中的 player iframe
    │
    ▼
自动导航到 onlineroomse.pku.edu.cn/player
    │
    ▼
video-detector.js Phase 1: canPlayType override
    │  (让 cmcPlayer.js 认为浏览器支持 HLS)
    ▼
video-detector.js Phase 2: XHR 拦截
    │  (捕获 JWT + 视频元数据)
    ▼
video-detector.js 注入下载按钮到页面
    │
    ├──> pku-ipc:///video-info → Rust emit → 前端 VideoInfoCard
    │
    └──> 用户点击"下载视频"
         │
         ▼
         pku-ipc:///add-download → Rust emit → 前端添加任务
```

### 4.2 下载流程

```
前端 createDownloadTask()
    │  (生成 UUID、文件名、路径)
    ▼
invoke('browser_download', { taskId, url, filepath })
    │
    ├── Linux: WebKitGTK download_uri() (保留 session cookies)
    │
    └── macOS/Windows: reqwest 流式下载 (JWT 认证)
        │
        ▼
    每 500ms emit download-progress { taskId, progress, speed, eta }
        │
        ▼
    下载完成 → emit download-complete
        │
        ▼
    如果是 M3U8 → FFmpeg convert_m3u8_to_mp4()
```

### 4.3 视图切换流程

```
Browser WebView 可见时:
  nav-bar.js "下载管理" 按钮
      │
      ▼
  pku-ipc:///show-main-view { view: "downloads" }
      │
      ▼
  Rust: browser-webview.hide() + main-webview.show()
      │
      ▼
  Rust: emit("switch-to-main", { view: "downloads" })
      │
      ▼
  App.svelte: currentView.set("downloads")

Main WebView 可见时:
  App.svelte "浏览器" 标签
      │
      ▼
  invoke('show_browser_view')
      │
      ▼
  Rust: main-webview.hide() + browser-webview.show()
```

---

## 5. 构建与部署

### 5.1 CI/CD 流水线

`.github/workflows/build.yml` 配置了 tag 触发的多平台构建：

- 触发条件：`v*` tag push 或手动 dispatch
- 构建矩阵：4 个目标（macOS aarch64/x86_64, Ubuntu, Windows）
- 步骤：Checkout → Node.js → Bun → Rust → 平台依赖 → Build → Release
- Release 产物自动上传到 GitHub Release

### 5.2 本地开发

```bash
bun install          # 安装前端依赖
bun run tauri:dev    # 启动开发服务器
bun run tauri:build  # 构建生产版本
```

### 5.3 图标生成

```bash
cargo tauri icon /path/to/source-icon.png -o src-tauri/icons
```

自动生成所有平台所需格式：PNG (32/128/256px)、ICO、ICNS。

---

## 6. 配置文件说明

### 6.1 `tauri.conf.json` 关键配置

```json
{
  "identifier": "ink.arthals.pku-course-desktop",
  "app": {
    "enableGTKAppId": true,          // Linux Wayland Dock 图标支持
    "windows": [{
      "width": 1400, "height": 900,
      "minWidth": 1000, "minHeight": 700
    }],
    "security": {
      "csp": {
        "script-src": "'self' 'unsafe-inline' 'unsafe-eval'",
        "img-src": "'self' https: http: data: blob:"
      }
    }
  },
  "bundle": {
    "targets": ["app", "appimage", "dmg", "msi"],
    "resources": ["inject-scripts/**/*"]
  }
}
```

### 6.2 `Cargo.toml` 关键依赖

```toml
tauri = { version = "2", features = ["devtools", "unstable", "image-png"] }
# image-png: 启用 Image::from_bytes PNG 解码，用于设置窗口图标
# unstable: 多 WebView 支持
# devtools: 开发工具
```

---

## 7. 调试问题记录

本节记录开发过程中遇到的关键问题、排查过程和最终解决方案。

### 7.1 跨平台编译失败 - WebKitGTK 条件编译

**问题描述**：
v0.1.0 发布时，在 macOS 和 Windows 的 GitHub Actions 构建失败，报错找不到 `webkit2gtk` 相关的 API。`download_uri()` 等函数是 WebKitGTK 专属的，在其他平台不存在。

**原因分析**：
`main.rs` 中的 `browser_download` 命令直接调用了 WebKitGTK 的 API，没有做条件编译。Linux 上开发测试通过，但 macOS/Windows 编译失败。

**解决方案**：
使用 `#[cfg(target_os = "linux")]` 条件编译，为不同平台实现不同的下载策略：
- **Linux**：使用 WebKitGTK 原生 `download_uri()`，保留 session cookies
- **macOS/Windows**：使用 `reqwest` 流式下载，通过 JWT URL 参数认证

**相关提交**：`fix: add conditional compilation for cross-platform builds`

---

### 7.2 Video WebView 布局空白问题 → 移除 Video WebView

**问题描述**：
v0.1.0 采用三 WebView 架构（main / browser / video）。当用户点击视频链接时，创建独立的 video-webview 加载播放器页面。但 Svelte header (48px) 和视频播放区域之间有大片空白，因为 video-webview 加载的 `course.pku.edu.cn` 包装页面自带 Blackboard UI 头部。

**尝试方案**：
1. ~~方案 1：调整 video-webview 的 offset 跳过头部~~ - 不可行，包装页面头部高度不固定
2. **方案 2（采用）：完全移除 video-webview** - 视频在 browser-webview 中直接播放

**解决方案**：
移除了整个 video-webview 架构（约 270 行 Rust 代码）：
- 删除 `do_create_video_view()`、`do_destroy_video_view()` 等函数
- 将 HLS 脚本（`hls.min.js` + `hls-player.js`）注入到 browser-webview
- 视频链接从 IPC 调用改为 `window.location.href` 页内导航
- 从 Svelte UI 移除"在线视频"标签页和 `VideoView.svelte`

**相关提交**：`feat: remove video-webview, play videos in browser-webview (v0.2.0)`

---

### 7.3 移除 Video WebView 后下载按钮不注入

**问题描述**：
移除 video-webview 后，视频可以在 browser-webview 中正常播放，但下载按钮（由 `video-detector.js` 注入）不再出现。DevTool Console 显示：
```
[video-detector] URL check: https://course.pku.edu.cn/webapps/...
[video-detector] Not a player page, skipping button injection
[hls-player] PostMessage IPC relay active (top frame)
```

**原因分析**：
`video-detector.js` 的下载按钮注入逻辑（Phase 2）只在 `onlineroomse.pku.edu.cn/player` 页面上执行。在旧架构中，`hls-player.js` 会检测包装页面中的 player iframe 并自动导航顶层页面到播放器 URL。

在移除 video-webview 的过程中，按照计划步骤 10（"移除 iframe 自动导航"），**错误地**将 `hls-player.js` 中的 iframe 自动导航代码一并删除了。然而这段代码不仅服务于旧的 video-webview 架构，更是 video-detector.js 工作的前提条件：

```
包装页面 (course.pku.edu.cn)
    └── iframe (onlineroomse.pku.edu.cn/player)
            └── video-detector.js 只在这个 URL 注入按钮

需要 hls-player.js 的 iframe 自动导航:
    检测到 player iframe → window.location.href = player URL
    → 顶层页面变成 player URL → video-detector.js 识别并注入按钮
```

**关键教训**：在大规模重构中，删除代码前需要仔细分析每段代码的所有调用方和依赖方，而不仅仅是看它属于哪个架构模块。

**解决方案**：
恢复 `hls-player.js` 中的完整 iframe 自动导航代码块，包括：
- `navigateToPlayer()` 函数
- `checkIframeNode()` 函数
- `MutationObserver` iframe 检测
- 轮询 fallback（500ms 间隔，30 秒超时）

---

### 7.4 Browser WebView 注入脚本无法使用 Tauri API

**问题描述**：
注入到 browser-webview（远端 URL `course.pku.edu.cn`）的脚本无法使用 `window.__TAURI__` 进行 IPC 通信。

**原因分析**：
Tauri 2.0 的 capability 系统限制了远端 URL 对 Tauri API 的访问。即使设置了 `withGlobalTauri: true`，远端页面的安全策略也阻止了 `__TAURI__` 注入。

**解决方案**：
设计了自定义 `pku-ipc://` URI scheme 协议：
- 注入脚本通过 `XMLHttpRequest` 发送请求到 `pku-ipc://localhost/<route>`
- Rust 端通过 `register_uri_scheme_protocol` 注册处理器
- 支持 CORS preflight（`OPTIONS` 请求返回正确的 headers）
- 对于跨域 iframe 中的脚本，使用 `postMessage` + 顶层中继的方式

---

### 7.5 WebKitGTK 不支持原生 HLS 播放

**问题描述**：
在 Ubuntu (WebKitGTK) 上，视频播放页面的 `<video>` 元素无法播放 M3U8 HLS 流。macOS 的 WebView (WKWebView) 原生支持 HLS，但 Linux 上需要额外处理。

**原因分析**：
WebKitGTK 的 `<video>` 元素不支持 `application/vnd.apple.mpegurl` MIME 类型，需要 MediaSource Extensions (MSE) 来实现 HLS 播放。

**解决方案**：
注入 hls.js 库到 WebView：
1. `hls.min.js`：提供 `window.Hls` 全局对象
2. `hls-player.js`：
   - 拦截 `video.src` setter 和 `setAttribute('src', ...)`
   - 监听 `<source>` 元素 DOM mutation
   - 对 `.m3u8` URL 自动创建 HLS.js 实例
   - 配置自动质量选择、30-600s 缓冲区

同时，`video-detector.js` override `canPlayType` 返回 `'maybe'` 让 cmcPlayer.js 认为浏览器支持 HLS，从而触发正常的 API 请求流程。

---

### 7.6 Linux Wayland Dock 图标显示为默认齿轮

**问题描述**：
自定义 App 图标后，在 Ubuntu (Wayland/GNOME) 的 Dock 上仍然显示默认的齿轮图标，而不是自定义图标。已经在 `setup` 中调用了 `main_window.set_icon()`，但无效。

**原因分析**：
在 Wayland 环境下，`Window::set_icon()` 只影响窗口标题栏图标（如果有的话），不影响 Dock/Taskbar 图标。Wayland 的 Dock 图标是通过以下机制确定的：
1. 应用设置 GTK Application ID
2. 桌面环境根据 app_id 查找匹配的 `.desktop` 文件
3. `.desktop` 文件中的 `Icon` 字段指向图标

**尝试方案**：
1. ~~仅调用 `Window::set_icon()`~~ - 在 Wayland 下对 Dock 无效
2. **正确方案（采用）**：三步配置

**解决方案**：
1. **`tauri.conf.json`**：添加 `"enableGTKAppId": true`，让 Tauri 使用 `identifier`（`ink.arthals.pku-course-desktop`）作为 GTK Application ID
2. **安装 `.desktop` 文件**：`~/.local/share/applications/ink.arthals.pku-course-desktop.desktop`
   ```ini
   [Desktop Entry]
   Name=PKU Course Desktop
   Icon=ink.arthals.pku-course-desktop
   StartupWMClass=ink.arthals.pku-course-desktop
   ```
3. **安装图标到系统路径**：`~/.local/share/icons/hicolor/128x128/apps/ink.arthals.pku-course-desktop.png`
4. 同时保留 `set_icon()` 调用，确保 X11 环境和窗口标题栏图标也正确显示

**关键知识**：Linux 下的应用图标由 freedesktop.org 规范管理，需要 `.desktop` 文件 + 系统图标路径，而不仅是 API 调用。`enableGTKAppId` 是 Tauri 2.0 专门为此提供的配置项。

---

### 7.7 Git Push 被拒绝 - 远端有新提交

**问题描述**：
本地完成 v0.2.0 代码后 `git push` 失败，提示 remote 有新的提交。

**原因分析**：
有人在 GitHub 上直接编辑了 `README.md`（`Update README.md` commit），导致本地和远端分叉。

**解决方案**：
```bash
git pull --rebase origin main
git push origin main
```

使用 rebase 而不是 merge，保持线性提交历史。

---

## 8. 已知限制与后续规划

### 8.1 当前限制

- 不支持断点续传（下载中断需要重新开始）
- M3U8 转码发生在下载后，不支持边下边转
- 没有下载历史持久化（关闭应用后下载记录丢失）
- FFmpeg 需要用户自行安装

### 8.2 后续规划

- 断点续传支持
- 批量下载队列管理（并发限制真正生效）
- 下载历史持久化
- 系统托盘最小化
- 自动更新机制
- 课程资料自动同步
- 硬件加速视频编码

---

## 9. 文件清单

| 文件 | 行数 | 职责 |
|------|------|------|
| `src-tauri/src/main.rs` | ~916 | Tauri 入口、IPC、WebView 管理 |
| `src-tauri/src/download.rs` | ~248 | 下载管理器、流式下载 |
| `src-tauri/src/ffmpeg.rs` | ~148 | FFmpeg 命令封装 |
| `src-tauri/src/settings.rs` | ~58 | 设置持久化 |
| `src-tauri/inject-scripts/nav-bar.js` | ~314 | 导航工具栏注入 |
| `src-tauri/inject-scripts/video-detector.js` | ~343 | 视频检测与下载按钮 |
| `src-tauri/inject-scripts/hls-player.js` | ~299 | HLS.js 集成与 iframe 导航 |
| `src/App.svelte` | ~259 | 根组件、视图路由、事件监听 |
| `src/components/BrowserView.svelte` | ~93 | 浏览器视图 |
| `src/components/DownloadPanel.svelte` | ~245 | 下载管理面板 |
| `src/components/DownloadTaskItem.svelte` | ~291 | 下载任务项 |
| `src/components/SettingsPanel.svelte` | ~359 | 设置面板 |
| `src/components/VideoInfoCard.svelte` | ~221 | 视频信息卡片 |
| `src/lib/store.ts` | ~35 | Svelte 状态管理 |
| `src/lib/types.ts` | ~50 | TypeScript 类型定义 |
| `src/lib/download-utils.ts` | ~46 | 下载工具函数 |
| `src/lib/naming.ts` | ~63 | 文件命名工具 |
| `src/lib/theme.ts` | ~29 | 主题管理 |
| `src/styles/global.css` | ~186 | 全局样式 |
| `src/styles/theme.css` | ~86 | 主题变量 |
| `.github/workflows/build.yml` | ~76 | CI/CD 构建流水线 |
