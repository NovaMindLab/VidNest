# 🎬 VidNest (巢视) - 本地优先 AI 视频智能中枢与媒体管理工作台

<p align="center">
  <img src="../../src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>新一代开源本地 AI 视频智能理解、内容分析与数字媒体资产管理工作台</strong><br>
  100% 隐私安全 · 离线 Whisper 语音转写 · AI 智能摘要与主题看点 · 毫秒级 FTS5 全文检索 · 原生画中画独立播放器 · 跨平台桌面驱动
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
      <a href="../../README.md">🇺🇸 English</a> &nbsp;|&nbsp; 
      <a href="README.zh-CN.md">🇨🇳 <b>简体中文</b></a> &nbsp;|&nbsp; 
      <a href="README.ja.md">🇯🇵 日本語</a> &nbsp;|&nbsp; 
      <a href="README.ko.md">🇰🇷 한국어</a> &nbsp;|&nbsp; 
      <a href="README.de.md">🇩🇪 Deutsch</a> &nbsp;|&nbsp; 
      <a href="README.es.md">🇪🇸 Español</a>
    </p>
  </details>
</div>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 官方推广站 & 下载中心</a> •
  <a href="../../wiki/README.md">📚 架构与开发文档</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ 最新安装包下载</a>
</p>

---

## 🌟 为什么选择 VidNest？

**VidNest** 是专为视频创作者、剪辑师、研究者与影视收藏家量身打造的极速、轻量、**本地优先（Local-First）AI 视频智能中枢与媒体管理桌面应用**。彻底摆脱商业软件高昂授权费、云端隐私泄露风险与传统 Electron 框架的沉重卡顿。

### 🚀 核心特性

* 🧠 **AI 视频智能中枢 (AI Intelligence Hub)**：集成零配置本地启发式 NLP 算法，并原生支持离线 Whisper 语音转写与云端 LLM（OpenAI/Groq/Ollama 等）。全自动提炼视频核心概要、生成带时间戳的主题看点与智能标签，点击时间戳瞬间直达对应画面。
* ✂️ **视频智能分段与高潮裁剪工坊 (Smart Clipper Studio)**：基于声学能量（RMS）、音量波动与 Whisper 语义流对齐算法，毫秒级提取高能精彩瞬间；提供直观的双向时间轴卡尺、分段预览与无损流式切片导出。
* 🎵 **音频替换混音与 Hi-Fi 提取**：在视频分段管理中支持为裁剪视频一键替换自定义背景音乐或混音融合；同时支持将任意视频瞬间无损提取为 MP3、M4A、WAV、FLAC 格式。
* ⚡ **毫秒级 FTS5 全文索引引擎**：内置 SQLite FTS5 全文检索，十万级视频库检索延迟 < 10ms，支持按文件名、标签、分辨率、码率以及 AI 转写台词脚本进行复合过滤。
* 🔍 **多维度智能视频查重去重**：融合快速哈希（头部/尾部采样）、精确元数据对比与中文名称智能模糊分词，秒级识别多盘符重复素材与冗余副本，一键合并标签并安全释放磁盘。
* 📥 **yt-dlp 在线视频解析与下载中心**：内置现代化多线程下载调度引擎，支持全球 1000+ 在线视频平台高清画质解析、音视频自动混流与后台断点续传。

---

## 🛠️ 技术架构

* **内核底座**：Tauri v2 + Rust 2021 内存安全引擎 (< 80MB 内存占用)
* **本地存储**：嵌入式 SQLite3 (启用 FTS5 全文搜索与 WAL 模式)
* **前端框架**：Vue 3.5, TypeScript, Vite
* **音视频与 AI 底层**：FFmpeg / FFprobe, Whisper (whisper.cpp), yt-dlp

---

## 📄 开源许可证

本项目基于 [MIT License](../../LICENSE) 协议永久开源免费使用。
