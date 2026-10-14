# 🎬 VidNest (巢视) - 本地 AI 视频智能中枢

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>新一代开源本地 AI 视频管理与内容理解工作台</strong><br>
  100% 隐私安全 · AI 智能摘要与看点 · 毫秒级全文检索 · 独立画中画播放 · 跨平台桌面原生驱动
</p>

<p align="center">
  <a href="https://github.com/NovaMindLab/VidNest/releases"><img src="https://img.shields.io/github/v/release/NovaMindLab/VidNest?style=flat-square&color=blue" alt="Release"></a>
  <a href="https://github.com/NovaMindLab/VidNest/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg?style=flat-square" alt="License"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://vuejs.org/"><img src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square&logo=vue.js&logoColor=white" alt="Vue"></a>
</p>

<div align="center">
  <details>
    <summary>🌐 <b>语言选择 / Language / 言語 / 언어 / Sprache / Idioma (6 种语言)</b></summary>
    <br/>
    <p>
      <a href="README.md">🇺🇸 English</a> &nbsp;|&nbsp; 
      <a href="README.zh-CN.md">🇨🇳 <b>简体中文</b></a> &nbsp;|&nbsp; 
      <a href="docs/i18n/README.ja.md">🇯🇵 日本語</a> &nbsp;|&nbsp; 
      <a href="docs/i18n/README.ko.md">🇰🇷 한국어</a> &nbsp;|&nbsp; 
      <a href="docs/i18n/README.de.md">🇩🇪 Deutsch</a> &nbsp;|&nbsp; 
      <a href="docs/i18n/README.es.md">🇪🇸 Español</a>
    </p>
  </details>
</div>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 官方推广站 & 下载中心</a> •
  <a href="wiki/README.md">📚 架构与开发文档</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ 最新安装包下载</a>
</p>

---

## 💡 为什么需要 VidNest？

在视频自媒体、特效剪辑与影视制作时代，每位创作者的硬盘中都沉淀了成百上千 GB 的视频素材：
- 🔍 **找素材像大海捞针**：按文件名根本找不到某个特定镜头，更记不清主角哪一秒说了哪句台词。
- 🤖 **AI 能力难以落地**：市面上的 AI 剪辑工具强制联网上传，上百 G 素材上传耗时数小时且有泄密风险。
- 🐢 **传统软件沉重卡顿**：Electron 应用动辄占用 1GB+ 内存，导入上万文件直接假死。
- 💸 **高昂授权费与破坏性目录**：商业素材软件收取昂贵买断费，且强行重组你的磁盘目录。

**VidNest 为解决上述痛点而生！** 纯本地优先（Local-First）原则，就地毫秒级索引，极致轻巧安全，开箱即用 AI 视频内容理解。

---

## ✨ 核心特性

### 1. 🧠 AI 视频智能中枢 (AI Video Intelligence)
- **AI 智能摘要与主题提炼**：
  - 自动提炼视频 1-2 句核心全局概述；
  - 提取带时间戳的核心看点与话题时间轴（如 `▶ 02:15 讨论系统核心架构`），**点击时间戳直接秒级跳播对应画面**；
  - 智能推荐分类标签，支持**「一键绑定至视频」**，自动同步至媒体库分类筛选树。
- **本地双引擎驱动 (Hybrid AI Engine)**：
  - **零配置本地算法**：免网络、免配置，开箱即用，通过语义分词、TF-IDF 与时间轴语义聚类生成高质量看点；
  - **大模型 API 模式**：兼容 OpenAI、DeepSeek、Groq 与本地 Ollama（如 `qwen2.5`、`llama3`），服务异常自动平滑降级。
- **显性化状态徽章**：网格卡片与列表直观展示 `🧠 AI 摘要` 与 `✨ XX句台词` 发光状态标签，卡片 ✨ 按钮直达智能抽屉。

### 2. 💬 对齐字幕台词与语音识别
- 集成 Whisper 语音转写引擎（支持本地 CLI 与兼容 API）；
- 全量字幕流式呈现与实时关键词过滤检索；
- 媒体库搜索直接匹配对白，展示 `🗣️ 命中台词: "..." ▶ 01:25` 气泡，点击精确跳播；
- 支持一键导出标准 `SRT` 和 `VTT` 字幕文件。

### 3. ⚡ 全盘毫秒级 FTS5 智能索引
- 内置 **SQLite + FTS5** 全文检索底座，支持万级视频零延迟就地索引。
- 原地扫描，**不改动、不复制、不破坏** 原始硬盘目录结构。
- 复合过滤检索：按分辨率（4K/1080P/720P）、码率、时长、编码格式（H.264/HEVC/AV1）秒级筛出目标。

### 4. 🪟 独立原生弹窗画中画播放器
- 支持一键分离为独立的 **系统级原生 WebviewWindow**。
- 置顶悬浮模式（Pin to Top）、平滑时间轴精确定位、键盘极速操控（空格/方向键/全屏/置顶）。
- 专为剪辑对比设计，一边在剪辑软件调色，一边悬浮对照参考视频。

### 5. 🔍 智能查重与精准防误判
- **剧集分集防误判保护**：自动识别 `EP01`、`S01E02`、`第1集` 等分卷编号，彻底避免电视剧不同集数误合并。
- **真实视频缩略图预览**：每组重复视频均展现高清视频封面截图。
- **单组独立合并**：支持单组独立确认合并清理，不再局限于全局一键操作。

### 6. 🎵 视频母带级高质音频提取
- 支持原声母带流极速直接抽取（零重新编码、秒级导出无损音轨）；
- 支持转码为 MP3、M4A、WAV、FLAC 等主流音频格式；
- 内置独立提取音频资产库，便捷重放与分类管理。

### 7. 📥 智能嗅探与视频下载中心
- 开箱集成最新 **yt-dlp + FFmpeg** 后端。
- 专注于国际主流视频平台（YouTube、TED、Vimeo 等）一键解析、多线程下载、自动混流与标签就地归档。

### 8. 🛡️ 100% 本地隐私 (Local-First) 与极致轻量
- 零云端数据交互、零账号体系、零网络上报。
- 运行内存仅需约 **~60MB**，冷启动毫秒级响应。
- 针对 Apple Silicon (M系列)、Intel Mac、Windows 与 Linux 深度编译优化。

---

## 📊 产品特性对比

| 功能维度 | 🎬 VidNest (巢视) | 🦅 Eagle 等设计素材库 | ✂️ 剪映 / CapCut 素材库 |
| :--- | :--- | :--- | :--- |
| **价格与开源** | **100% 永久免费开源 (MIT)** | 商业单设备买断 ($29.95) | 免费基础 + 付费高级 VIP |
| **AI 视频内容理解** | **内置 AI 摘要、时间轴看点、台词检索** | 无 AI 语义理解 | 仅限云端字幕识别 |
| **存储方式** | **原地零拷贝就地挂载** | 必须复制进专用 `.library` 库 (双倍占盘) | 强绑定工程草稿缓存 |
| **内存与性能** | **Tauri 2 + Rust (~60MB)** | Electron 架构 (~600MB+) | 宿主捆绑 (~1GB+) |
| **播放体验** | **系统级独立画中画弹窗 + 浮窗置顶** | 仅应用内固定面板预览 | 仅工程内预览 |
| **隐私保障** | **100% 离线脱网可用，零上传** | 依赖三方网盘，本地存储 | 强制云端同步分析 |
| **搜索响应** | **SQLite FTS5 (<10ms)** | 文件数超10万时易卡顿 | 仅支持简单工程内搜索 |
| **跨平台** | **Windows / macOS (Apple+Intel) / Linux** | Windows / macOS | Windows / macOS / 移动端 |

---

## 🚀 极速下载与安装

前往 [GitHub Releases](https://github.com/NovaMindLab/VidNest/releases/latest) 下载对应平台的安装程序：

- **macOS (Apple Silicon M1/M2/M3/M4)**: `Local.Video.Intelligence_x.x.x_aarch64.dmg`
- **macOS (Intel x86_64)**: `Local.Video.Intelligence_x.x.x_x64.dmg`
- **Windows (x64)**: `Local.Video.Intelligence_x.x.x_x64-setup.exe`
- **Linux (x64)**: `Local.Video.Intelligence_x.x.x_amd64.AppImage` / `.deb`

---

## 🛠️ 本地开发与构建

### 1. 准备环境
- Node.js >= 20
- Rust 工具链 (rustc & cargo stable)
- 系统依赖 (按 Tauri 2 官方指引)

### 2. 安装依赖并启动
```bash
# 安装前端依赖
npm install

# 运行自动化测试
cd src-tauri && cargo test && cd ..

# 启动开发调试模式
npm run tauri dev
```

### 3. 构建生产包
```bash
npm run tauri build
```

---

## 📚 详细文档索引

更多架构细节、技术内幕与部署运维蓝皮书，请查阅项目 [Wiki 知识库](wiki/README.md)：

- 📐 [系统总体架构与设计理念](wiki/architecture_and_design.md)
- ⚡ [跨平台全盘扫描与毫秒级索引算法](wiki/full_disk_scanner_and_indexing.md)
- 🎬 [媒体资产库与独立弹窗视频播放器实现](wiki/video_library_and_player.md)
- 🔄 [应用内差分增量升级指南](wiki/in_app_differential_update.md)
- 📥 [yt-dlp 在线视频解析与混流引擎](wiki/ytdlp_and_downloader.md)
- 🚀 [autodeploy 全自动发版与全渠道推广实践指南](wiki/autodeploy_guide.md)
- 🌐 [全自动发版与全渠道分发实战蓝皮书](wiki/universal_release_and_promotion_blueprint.md)

---

## 🤝 贡献与社区

欢迎通过以下方式参与 VidNest 的建设：
- 🌟 在 GitHub 点亮 Star 支持我们！
- 🐛 提交 [Issue](https://github.com/NovaMindLab/VidNest/issues) 反馈 Bug 或建议。
- 🔀 提交 Pull Request 贡献代码。

## 📄 开源许可证

本项目基于 [MIT License](LICENSE) 开源。
