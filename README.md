# Local Video Intelligence

开源本地 AI 视频管理工具（PC 桌面端），基于 Tauri 2 + Rust + Vue 3 + TypeScript 构建。

## 技术栈与架构
- **桌面框架**：Tauri 2 (Rust 原生内核 + 独立轻量 WebView)
- **前端技术**：Vue 3 + TypeScript + Vite
- **后端语言**：Rust 2021 Edition
- **媒体与下载引擎**：FFmpeg / FFprobe + yt-dlp
- **数据存储**：SQLite + FTS5 全文索引

## 开发与构建

### 1. 安装前端依赖
```bash
npm install
```

### 2. 运行单元测试
```bash
cd src-tauri
cargo test
```

### 3. 开发环境启动
```bash
npm run tauri dev
```

### 4. 生产包打包构建
```bash
npm run tauri build
```

## 📚 项目官方 Wiki 知识库

全套工程架构、核心算法、媒体流播放与自动化发布运维文档请参阅 [wiki/](wiki/README.md)：

- 📐 [系统架构与设计理念](wiki/architecture_and_design.md)
- ⚡ [跨平台全盘极速扫描与毫秒级索引](wiki/full_disk_scanner_and_indexing.md)
- 🎬 [媒体资产库与独立弹窗视频播放器](wiki/video_library_and_player.md)
- 🔄 [应用内差分增量升级指南](wiki/in_app_differential_update.md)
- 📥 [yt-dlp 在线视频解析与混流引擎](wiki/ytdlp_and_downloader.md)
- 🚀 [autodeploy 全自动发版与全渠道推广实践指南](wiki/autodeploy_guide.md)
- 🌐 [全自动发版与推广实战蓝皮书](wiki/universal_release_and_promotion_blueprint.md)

