# 🎬 VidNest (巢视) - 本地 AI 视频智能中枢

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>新一代开源本地 AI 视频管理与智能检索桌面工作台</strong><br>
  100% 隐私安全 · 毫秒级全文索引 · 独立弹窗画中画播放 · 跨平台桌面原生驱动
</p>

<p align="center">
  <a href="https://github.com/NovaMindLab/VidNest/releases"><img src="https://img.shields.io/github/v/release/NovaMindLab/VidNest?style=flat-square&color=blue" alt="Release"></a>
  <a href="https://github.com/NovaMindLab/VidNest/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg?style=flat-square" alt="License"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://vuejs.org/"><img src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square&logo=vue.js&logoColor=white" alt="Vue"></a>
</p>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 官方推广站 & 下载中心</a> •
  <a href="wiki/README.md">📚 架构与开发文档</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ 最新安装包下载</a>
</p>

---

## 💡 为什么需要 VidNest？

在视频自媒体、特效剪辑与影视制作时代，每位创作者的硬盘中都沉淀了成百上千 GB 的视频素材：
- 🔍 **找素材像大海捞针**：按文件名根本找不到想要的某个片段或特定分辨率镜头。
- 🐢 **传统软件沉重卡顿**：Electron 应用动辄占用 1GB+ 内存，导入上万文件直接假死。
- 💸 **高昂授权费与垄断**：商业素材软件收取昂贵买断费，且强行破坏性重组你的磁盘目录。
- 🔒 **隐私担忧**：商业剪辑软件强制上传云端同步，商业机密与私密视频面临审核泄密风险。

**VidNest 为解决上述痛点而生！** 纯本地优先（Local-First）原则，就地毫秒级索引，极致轻巧安全。

---

## ✨ 核心特性

### 1. ⚡ 全盘毫秒级 FTS5 智能索引
- 内置 **SQLite + FTS5** 全文检索底座，支持万级视频零延迟就地索引。
- 原地扫描，**不改动、不复制、不破坏** 原始硬盘目录结构。
- 复合过滤检索：按分辨率（4K/1080P/720P）、码率、时长、编码格式（H.264/HEVC/AV1）秒级筛出目标。

### 2. 🪟 独立原生弹窗画中画播放器
- 支持一键分离为独立的 **系统级原生 WebviewWindow**。
- 置顶悬浮模式（Pin to Top）、平滑时间轴精确定位、键盘极速操控（空格/方向键/全屏/置顶）。
- 专为剪辑对比设计，一边在剪辑软件调色，一边悬浮对照参考视频。

### 3. 🛡️ 100% 本地隐私 (Local-First)
- 零云端数据交互、零账号体系、零网络上报。
- 你的所有视频素材、数据库与标签全部保留在你的机器上，彻底保障商业与个人隐私。

### 4. 📥 智能嗅探与视频下载中心
- 开箱集成最新 **yt-dlp + FFmpeg** 后端。
- 支持主流音视频网站链接一键解析、多线程下载、自动混流与标签就地归档。

### 5. 🔄 应用内无感差分增量升级
- 针对桌面端深度打磨的应用内升级体系。
- 自动拉取 GitHub Releases 差分增量包（仅下载补丁，省带宽极速更新），一键无缝重启。

### 6. 🦀 Rust + Tauri 2 极致性能
- 运行内存仅需约 **~60MB**，冷启动毫秒级响应。
- 针对 Apple Silicon (M系列)、Intel Mac、Windows 与 Linux 深度编译优化。

---

## 📊 产品特性对比

| 功能维度 | 🎬 VidNest (巢视) | 🦅 Eagle 等设计素材库 | ✂️ 剪映 / CapCut 素材库 |
| :--- | :--- | :--- | :--- |
| **价格与开源** | **100% 永久免费开源 (MIT)** | 商业单设备买断 ($29.95) | 免费基础 + 付费高级 VIP |
| **存储方式** | **原地零拷贝就地挂载** | 必须复制进专用 `.library` 库 (双倍占盘) | 强绑定工程草稿缓存 |
| **内存与性能** | **Tauri 2 + Rust (~60MB)** | Electron 架构 (~600MB+) | 宿主捆绑 (~1GB+) |
| **播放体验** | **系统级独立画中画弹窗 + 浮窗置顶** | 仅应用内固定面板预览 | 仅工程内预览 |
| **隐私保障** | **100% 离线脱网可用，零上传** | 依赖三方网盘，本地存储 | 强制云端同步分析 |
| **搜索响应** | **SQLite FTS5 (<10ms)** | 文件数超10万时易卡顿 | 仅支持简单工程内搜索 |
| **跨平台** | **Windows / macOS (Apple+Intel) / Linux** | Windows / macOS | Windows / macOS / 移动端 |

---

## 🚀 极速下载与安装

前往 [GitHub Releases](https://github.com/NovaMindLab/VidNest/releases/latest) 下载对应平台的安装程序：

- **macOS (Apple Silicon M1/M2/M3/M4)**: `VidNest_x.x.x_aarch64.dmg`
- **macOS (Intel x86_64)**: `VidNest_x.x.x_x64.dmg`
- **Windows (x64)**: `VidNest_x.x.x_x64-setup.exe`
- **Linux (x64)**: `VidNest_x.x.x_amd64.AppImage` / `.deb`

> 💡 **包管理器安装（即将正式支持）**：
> ```bash
> # macOS Homebrew
> brew install --cask vidnest
> 
> # Windows WinGet
> winget install NovaMindLab.VidNest
> ```

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
