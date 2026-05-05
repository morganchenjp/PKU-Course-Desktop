# PKU Course Desktop

基于 Tauri 构建的北大课程视频下载桌面应用。

## 功能特性

- 🌐 内置浏览器，支持 IAAA 统一认证登录
- 📹 自动检测录播视频，一键添加到下载队列
- 🚀 批量下载管理，支持并发下载
- 🎨 支持日间/夜间主题切换
- 📝 智能文件命名规则
- 🔄 m3u8 转码为 MP4（内置 FFmpeg 支持）
- 🎵 通过 FFmpeg 从MP4视频提取音频为 MP3/AAC/WAV

## 技术栈

- **Frontend**: Svelte 5 + TypeScript
- **Backend**: Rust + Tauri 2.0
- **Build Tool**: Vite
- **Package Manager**: Bun

## 简要使用说明
### 必须先安装 FFmpeg

**macOS:**
```bash
brew install ffmpeg
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install ffmpeg
## also need install gstreamer to support online playing HLS(m3u8) video
sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-alsa
```

**Windows:**
下载 FFmpeg 并添加到系统 PATH: https://ffmpeg.org/download.html 
或者更直接的下载链接： https://www.gyan.dev/ffmpeg/builds/ ， 下载 ffmpeg-git-essentials.7z 文件即可， 
然后把这个文件解压到 c:\ffmpeg 目录， 同时把 c:\ffmpeg\bin  目录添加到 PATH 中。 

### App 的安装使用
下载安装对应OS的发布包， 正常安装后启动， 
App启动后会自动打开PKU教学网， 正常用“校园卡用户“身份登陆， 然后找到你的课程， 然后找到“课堂实录”， 再点击对应日期的课堂实录， （建议把App全屏使用）
会在App内置的浏览器中自动打开一个视频播放View, 在Video的下方可以看到3个Button，如下图：
<img width="668" height="161" alt="video-downloader" src="https://github.com/user-attachments/assets/e506b0e0-a8f8-4ec0-aeff-dcb9c34fd3a8" />
直接点击“下载视频” 即可， 然后点击右上角的“下载管理”， 可以查看当前的下载任务，文件缺省下载到系统的“Download” 目录下。
<img width="169" height="58" alt="nav-buttons" src="https://github.com/user-attachments/assets/f952ca12-df03-4976-a534-076e637d8e39" />
如果希望下载视频MP4 之后， 自动把MP4中的音频提取为MP3文件， 方便后续把MP3文件上传到阿里的”通义听悟“ 之类的AI工具中， 把语音直接转化为课堂笔记，请点击右上角的”设置“ Button,
<img width="230" height="275" alt="extract-audio" src="https://github.com/user-attachments/assets/c4ba41f9-8c60-4b0a-b61a-5f79a376130c" />
如上图，打开“同时提取音频文件”开关， 然后点击底部的“保存设置” 即可。


## 开发环境要求

- [Rust](https://rustup.rs/) (1.77.0+)
- [Node.js](https://nodejs.org/) (18+)
- [Bun](https://bun.sh/) (1.0+)
- [FFmpeg](https://ffmpeg.org/) (用于视频转码)

### 1. 克隆项目

```bash
git clone https://github.com/zhuozhiyongde/PKU-Course-Desktop.git
cd PKU-Course-Desktop
```

### 2. 安装依赖

```bash
bun install
```

### 3. 安装 Tauri CLI

```bash
cargo install tauri-cli
```

### 4. 启动开发服务器

```bash
bun run tauri:dev
```

### 5. 构建生产版本

```bash
# 构建所有平台
bun run tauri:build

# 或特定平台
cargo tauri build --target x86_64-pc-windows-msvc
cargo tauri build --target x86_64-apple-darwin
cargo tauri build --target aarch64-apple-darwin
cargo tauri build --target x86_64-unknown-linux-gnu
```

## 项目结构

```
PKU-Course-Desktop/
├── src/                    # 前端源代码
│   ├── components/         # Svelte 组件
│   ├── lib/               # 工具库和 store
│   ├── styles/            # CSS 样式
│   ├── App.svelte         # 主应用组件
│   └── main.ts            # 前端入口
├── src-tauri/             # Tauri/Rust 后端
│   ├── src/               # Rust 源代码
│   │   ├── main.rs        # 主入口
│   │   ├── download.rs    # 下载管理
│   │   ├── ffmpeg.rs      # FFmpeg 封装
│   │   └── settings.rs    # 设置管理
│   ├── inject-scripts/    # 注入网页的脚本
│   └── Cargo.toml         # Rust 依赖
├── package.json           # Node.js 依赖
├── Cargo.toml            # Rust 工作区配置
└── tauri.conf.json       # Tauri 配置
```

## 核心功能实现

### 视频检测

应用通过注入脚本到内置浏览器来拦截视频信息：

1. 拦截 `XMLHttpRequest` 和 `fetch` 请求
2. 检测 `get-sub-info-by-auth-data` API 调用
3. 解析响应数据提取视频元信息
4. 通过 Tauri IPC 发送到前端

### 下载管理

- 使用 `reqwest` 库进行 HTTP 下载
- 支持 JWT 认证头
- 实时进度更新
- 断点续传支持（TODO）

### 视频转码

- 使用 FFmpeg 进行 m3u8 到 MP4 的转码
- 支持音频提取 (TODO) ,从而可以把音频文件上传到 通义千悟 之类的语音转文字笔记AI 应用，方便整理课堂笔记。
- 硬件加速支持（TODO）

## 跨平台支持

| 平台 | 状态 | 说明 |
|------|------|------|
| Windows 10+ | ✅ 支持 | MSI 安装包 |
| macOS 11+ | ✅ 支持 | DMG 安装包 |
| Ubuntu 24+ | ✅ 支持 | AppImage 包 |

## 开发计划

### Phase 1: MVP (当前)
- [x] 基础框架搭建
- [x] 内嵌浏览器
- [x] 视频检测与拦截
- [x] 单视频下载
- [x] 主题切换

### Phase 2: 核心功能
- [ ] 批量下载队列
- [ ] 下载进度管理
- [ ] 本地设置持久化
- [ ] 下载历史记录

### Phase 3: 增强功能
- [ ] 课程资料自动同步
- [ ] 音频提取功能
- [ ] 系统托盘最小化
- [ ] 自动更新

## 许可证

GPL-3.0 License

## 致谢

- [PKU-Art](https://github.com/zhuozhiyongde/PKU-Art) - 原始 PKU Art 用户脚本项目
- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Svelte](https://svelte.dev/) - 前端框架
