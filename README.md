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
