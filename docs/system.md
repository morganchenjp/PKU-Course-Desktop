# PKU Course Desktop - System Specification

> 本文档作为后续开发的高级准则，记录技术栈选型、目录结构原则和核心业务逻辑。

## 1. 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 桌面框架 | Tauri 2.10 | 窗口管理、WebView 编排、IPC 命令 |
| 前端框架 | Svelte 5 + TypeScript | 状态管理、UI 渲染 |
| 构建工具 | Vite 6 + Bun 1.x | 极速前端构建与热更新 |
| 后端语言 | Rust 1.77+ | Tauri 命令、下载管理、FFmpeg 调用 |
| HTTP 客户端 | reqwest 0.12 | 带 cookies 的 HTTP 流式下载 |
| 异步运行时 | tokio 1.x | 并发下载任务管理 |
| 视频处理 | FFmpeg (外部依赖) | m3u8 → MP4 转码、音频提取 |
| HLS 播放 | hls.js (内嵌) | 绕过 WebKitGTK 原生 HLS 不支持的限制 |
| Linux WebView | WebKitGTK 2.0 | Linux 平台 WebView 引擎 |

### 平台支持

| 平台 | 构建目标 | 产物格式 |
|------|----------|----------|
| macOS (Apple Silicon / Intel) | aarch64/x86_64-apple-darwin | .dmg |
| Ubuntu 22.04+ | x86_64-unknown-linux-gnu | .AppImage, .deb |
| Windows 10+ | x86_64-pc-windows-msvc | .msi, .exe |

---

## 2. 目录结构

```
PKU-Course-Desktop/
├── src/                          # Svelte 前端 (TypeScript)
│   ├── App.svelte                # 根组件：视图路由、事件监听
│   ├── main.ts                   # 前端入口
│   ├── components/               # Svelte 组件
│   │   ├── BrowserView.svelte    # 浏览器视图（仅事件监听，UI 在注入脚本）
│   │   ├── DownloadPanel.svelte  # 下载队列管理面板
│   │   ├── DownloadTaskItem.svelte # 单个下载任务行
│   │   ├── SettingsPanel.svelte  # 设置面板（下载路径、命名规则、并发数）
│   │   └── VideoInfoCard.svelte  # 视频信息卡（捐赠按钮 + QR 码）
│   ├── lib/                      # 工具模块
│   │   ├── store.ts              # Svelte store（currentView, downloadTasks, settings, theme）
│   │   ├── types.ts              # TypeScript 类型定义
│   │   ├── download-utils.ts     # createDownloadTask() 辅助函数
│   │   ├── naming.ts             # 文件命名规则解析
│   │   └── theme.ts              # 主题初始化与 CSS 变量注入
│   └── styles/                   # 全局样式
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 入口：webview 创建、IPC 协议、命令注册
│   │   ├── download.rs            # DownloadManager、HTTP 流式下载
│   │   ├── ffmpeg.rs              # m3u8→MP4 转码、音频提取
│   │   └── settings.rs            # 配置文件的加载与保存
│   ├── inject-scripts/           # 注入到 browser-webview 的 JS
│   │   ├── nav-bar.js             # 导航栏：前进/后退/刷新/主页、视图切换、URL 追踪
│   │   ├── video-detector.js      # 视频检测：拦截 XHR、注入下载/捐赠按钮
│   │   ├── hls.min.js             # HLS.js 库（内嵌，不依赖 CDN）
│   │   └── hls-player.js          # HLS 播放协调：canPlayType override、播放器注入
│   ├── Cargo.toml                # Rust 依赖
│   └── tauri.conf.json           # Tauri 配置（窗口、权限、资源、图标）
├── public/                       # 前端静态资源
│   ├── morgan-wechat-qrcode.png  # 捐赠二维码（嵌入到应用）
│   └── app-icon.png              # 应用图标
└── .specs/                      # 设计规格文档
    └── system.md                 # 本文件
```

### 目录结构原则

- **`src/`**：纯前端，Svelte 组件和 TypeScript 逻辑，不含 Rust 代码
- **`src-tauri/`**：Rust 后端和注入脚本，与前端完全隔离
- **`inject-scripts/`**：`src-tauri/inject-scripts/` 目录的 JS 文件通过 `initialization_script` 注入到 browser-webview，运行在远程 origin 下
- **`public/`**：Vite 静态资源，打包时复制到输出目录
- **`.specs/`**：架构决策记录，不参与构建

---

## 3. 核心业务逻辑

### 3.1 双 WebView 架构

应用共享一个 Tauri 窗口，内含两个 WebView：

```
┌──────────────────────────────────────────────────┐
│                Tauri Window (main)                │
│  ┌──────────────────┐  ┌──────────────────────┐ │
│  │  Main WebView     │  │  Browser WebView      │ │
│  │  (Svelte UI)     │  │  (course.pku.edu.cn)  │ │
│  │                  │  │                        │ │
│  │  下载管理面板     │  │  nav-bar.js (注入)    │ │
│  │  设置面板        │  │  video-detector.js     │ │
│  │  VideoInfoCard   │  │  hls-player.js        │ │
│  └──────────────────┘  └──────────────────────┘ │
└──────────────────────────────────────────────────┘
```

**视图切换**（由 Rust 控制）：
- 切换到浏览器视图：`browser.set_position(0, 48)` + `browser.show()` + `main.hide()`
- 切换到主视图：`browser.hide()` + `browser.set_position(10000, 48)` + `main.show()` + `main.set_position(0, 0)`
- Svelte 侧 `currentView` store 只负责 UI 状态，不直接控制 webview 可见性

### 3.2 注入脚本与 IPC 通信

由于注入脚本运行在 `course.pku.edu.cn` 远程 origin 下，`window.__TAURI__` 被 CSP 限制无法使用。解决方案：**自定义 `pku-ipc://` URI 协议**。

```
browser-webview (course.pku.edu.cn)
    │
    │ XMLHttpRequest POST/GET
    ▼
pku-ipc://localhost/<path>
    │
    │ Rust register_uri_scheme_protocol
    ▼
Rust IPC Handler (main.rs)
    │
    ├── show-main-view?view=downloads|settings
    ├── video-info  (body: VideoInfo JSON)
    ├── add-download (body: VideoInfo JSON)
    ├── open-external (body: { url })
    └── donation-qr  → 返回 morgan-wechat-qrcode.png
```

**所有 IPC 调用必须包含 cache-busting 查询参数**（`&_=<timestamp>.<random>`），因为 WebKitGTK 会缓存同路径的响应。

### 3.3 视频检测流程

1. `video-detector.js` 拦截 `XMLHttpRequest`（`get-sub-info-by-auth-data` API）获取视频元数据
2. 注入下载按钮（3 按钮布局：下载视频 / 复制链接 / ☕ 捐赠）和视频信息卡
3. 用户点击"下载视频" → `ipcSend('add-download')` → Rust `add-download-from-browser` 事件 → Svelte 创建下载任务
4. HLS 播放：override `HTMLVideoElement.canPlayType` 返回 `'probably'` → cmcPlayer.js 发起 API 请求 → `video-detector.js` 拦截响应 → 注入 hls.js 播放器

### 3.4 下载队列与并发控制

- 下载队列存储在 Svelte `downloadTasks` store（内存，关闭后丢失）
- 并发数由 Settings 中的 `maxConcurrentDownloads` 控制（默认 2，最大 2）
- 新任务加入时：若 `downloading` 数量 < 限额，立即开始；否则标记为 `pending` 等待
- 下载完成或出错时，自动从 pending 队列取出下一个任务开始下载

### 3.5 m3u8 下载与转码流程

```
下载阶段（reqwest / WebKitGTK download_uri）
    │
    ├── 下载 m3u8 播放列表
    ├── 遍历 ts segment URL，顺序下载所有 .ts 文件
    └── 保存为 .m3u8（原始分片）

转码阶段（FFmpeg external process）
    │
    └── ffmpeg -i input.m3u8 -c copy output.mp4
        （不重新编码，只拼接 ts 分片，速度快）
```

### 3.6 设置持久化

- 存储路径：`$HOME/.local/share/pku-course-desktop/settings.json`（Linux）/ `PKU Course Desktop.app/Contents/MacOS/settings.json`（macOS）
- Rust 启动时加载，Svelte 通过 `settings` store 读写
- 配置项：下载路径、命名规则、自动下载、并发数、音质偏好、音频格式

---

## 4. 关键设计决策

### 4.1 为什么用自定义 URI 协议而不是 Tauri invoke？

Tauri 的 `invoke()` 基于 `window.__TAURI__`，该 API 在远程 origin（course.pku.edu.cn）下因 CSP 和 capability 限制不可用。`pku-ipc://` 协议通过浏览器原生 XMLHttpRequest 发起，本质上是合法的 HTTP 请求，绕过了 CSP 限制。

### 4.2 为什么注入脚本不用 CDN 依赖？

- `hls.min.js`：WebKitGTK 在离线环境下无法访问 CDN，必须内嵌
- 其他脚本（nav-bar、video-detector）：直接注入到页面 DOM，不依赖外部资源

### 4.3 为什么不用 WebView2 自带下载而用 reqwest？

- Windows / macOS：使用 WebKitGTK `download_uri()` 保留 JSESSIONID cookie（认证依赖）
- Linux：`download_uri()` 无回调，无法获取进度，使用 reqwest 替代

### 4.4 为什么不用 Tauri 的 `@tauri-apps/plugin-fs` 存储下载进度？

当前实现选择简单性：下载任务状态存于内存（`downloadTasks` store），关闭即丢失。未来可接入 `tauri-plugin-fs` 实现断点续传。

---

## 5. 已知限制

| 限制 | 说明 | 影响 |
|------|------|------|
| 无断点续传 | 下载中断需从头开始 | 大文件下载风险 |
| 无历史持久化 | 关闭后队列丢失 | 每次启动重新添加 |
| 无系统托盘 | 未实现 minimize to tray | 无法后台下载 |
| FFmpeg 外部依赖 | 用户需自行安装 | 非自带，配置复杂 |
| Linux WebKitGTK HLS | 需 hls.js 注入绕过 | 兼容性和性能风险 |
| 并发上限 2 | 设置项最大值硬编码 | 大批量下载速度受限 |

---

## 6. 变更日志决策点（供后续参考）

- **v0.2.0**：删除 video-webview 导致 iframe 自动导航代码被误删 → 修复：恢复 hls-player.js 中的 `navigateToPlayer()`
- **WebKitGTK XHR 缓存**：同一 `pku-ipc://` 路径被缓存 → 修复：每次请求附加 `&_=<timestamp>.<random>`
- **Linux hide() Z-order**：`webview.hide()` 不降低 Z 轴 → 修复：改用 `set_position(10000, 48)` 将 browser webview 移出可视区域
