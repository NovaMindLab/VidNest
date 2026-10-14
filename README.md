# 🎬 VidNest - Local-First AI Video Intelligence & Media Hub

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="VidNest Logo" />
</p>

<p align="center">
  <strong>Next-Generation Open-Source Desktop Workspace for Video Intelligence, Content Understanding & Asset Management</strong><br>
  100% Private & Local-First · AI Executive Summary & Topic Timeline · Millisecond Full-Text Search · Standalone Picture-in-Picture · Cross-Platform Desktop Native
</p>

<p align="center">
  <a href="https://github.com/NovaMindLab/VidNest/releases"><img src="https://img.shields.io/github/v/release/NovaMindLab/VidNest?style=flat-square&color=blue" alt="Release"></a>
  <a href="https://github.com/NovaMindLab/VidNest/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg?style=flat-square" alt="License"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://vuejs.org/"><img src="https://img.shields.io/badge/Vue-3.5-4FC08D?style=flat-square&logo=vue.js&logoColor=white" alt="Vue"></a>
</p>

<p align="center">
  <a href="README.md"><strong>English</strong></a> • <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://novamindlab.github.io/VidNest/">🌐 Official Portal & Download Center</a> •
  <a href="wiki/README.md">📚 Architecture & Developer Docs</a> •
  <a href="https://github.com/NovaMindLab/VidNest/releases/latest">⚡ Download Latest Release</a>
</p>

---

## 💡 Why VidNest?

In the era of 4K content creation, courseware, and video archiving, creators store hundreds of gigabytes of footage:
- 🔍 **Finding shots is like searching a needle in an ocean**: File names don't tell you who said what or where a specific scene is located.
- 🤖 **Cloud AI tools compromise privacy**: Uploading hundreds of gigabytes to cloud servers is slow, expensive, and risks leaks of private footage.
- 🐢 **Bloated Electron apps lag**: Traditional apps easily consume 1GB+ RAM and freeze when indexing 10,000+ videos.
- 💸 **Costly licenses & proprietary formats**: Commercial tools force you to duplicate files into proprietary vaults (`.library`).

**VidNest solves these challenges!** Guided by the **Local-First** principle, it delivers millisecond in-place indexing, extreme lightweight performance (~60MB RAM), and ready-to-use AI video understanding directly on your machine.

---

## ✨ Key Features

### 1. 🧠 AI Video Intelligence Hub
- **Executive Summaries & Chapters**:
  - Automatically distills a 1-2 sentence core overview of your video;
  - Extracts key takeaways and a clickable **Topic Timeline** (e.g., `▶ 02:15 Architectural Discussion`) — **clicking any timestamp immediately starts playback at that exact second**;
  - Generates recommended tags with **one-click binding** to your local video library categories.
- **Hybrid AI Engine**:
  - **Zero-Config Local NLP**: 100% offline heuristic semantic extractor using word frequency, stopword filtering, and timeline clustering;
  - **External LLM / Ollama**: Plug-and-play compatibility with OpenAI, DeepSeek, Groq, and local Ollama (`qwen2.5`, `llama3`). Automatically falls back to local algorithms if offline.
- **Prominent Card Badges**: Visual indicators (`🧠 AI Summary` and `✨ XX Subtitles`) let you identify AI-ready assets at a glance.

### 2. 💬 Speech Recognition & Dialogue Search
- Powered by the Whisper transcription pipeline (supports local binary and OpenAI-compatible endpoints);
- Search dialogue lines across your entire library in milliseconds;
- Subtitle snippet match pills (`🗣️ "..." ▶ 01:25`) jump straight to the spoken dialogue;
- Export standard `SRT` and `VTT` subtitle files with one click.

### 3. ⚡ Zero-Copy Millisecond Full-Text Indexing
- Built upon **SQLite + FTS5** full-text search engine for instant retrieval across tens of thousands of videos;
- In-place indexing: **does not move, copy, or alter** your existing directory structure;
- Filter effortlessly by resolution (4K / 1080P / 720P), duration, codec (H.264 / HEVC / AV1 / AVI), tags, and collections.

### 4. 🪟 Standalone Picture-in-Picture Floating Player
- Detach into a dedicated **native WebviewWindow** with a single click;
- Always-on-Top (Pin to Top) floating mode, keyboard shortcuts (Space, Arrow keys, Fullscreen, Mute);
- Designed for video editing workflows: preview reference clips in a floating window while grading in your NLE.

### 5. 🔍 False-Positive-Safe Video Deduplication
- **Series/Episode Conflict Defense**: Accurately recognizes episode patterns (`EP01` vs `EP02`, `S01E01`, `Part 1`) to eliminate false merges of TV shows and courses;
- **Real Video Thumbnails**: Displays actual video frame snapshots for every duplicate candidate;
- **Independent Group Merging**: Resolve duplicate groups one by one or batch-merge with clear space reclamation previews.

### 6. 🎵 Lossless Audio Extraction
- Direct master-stream extraction (zero re-encoding, instant export of original audio track);
- Transcode to MP3, M4A, WAV, or FLAC with custom bitrates;
- Dedicated Audio Library for seamless playback and management.

### 7. 📥 International Video Sniffer & Downloader
- Integrated **yt-dlp + FFmpeg** engine out of the box;
- Designed for leading international video platforms (YouTube, TED, Vimeo, etc.) with 403 anti-blocking;
- Multi-resolution stream muxing and auto-import into your local library.

### 8. 🛡️ 100% Local Privacy & Lightweight Performance
- Zero cloud uploads, zero account requirements, zero telemetry;
- Idle memory footprint of only **~60MB**, cold boots in milliseconds;
- Native binaries tuned for Apple Silicon (M-series), Intel Mac, Windows 10/11, and Linux.

---

## 📊 Feature Comparison

| Feature Dimension | 🎬 VidNest | 🦅 Eagle & Asset Managers | ✂️ CapCut / NLE Vaults |
| :--- | :--- | :--- | :--- |
| **Pricing & License** | **100% Free & Open Source (MIT)** | Commercial one-off ($29.95) | Free basic + recurring VIP |
| **AI Video Understanding** | **AI Summary, Timeline Chapters, Subtitles** | No AI semantic understanding | Cloud-only subtitle transcription |
| **Storage Architecture** | **In-place Zero-Copy mounting** | Duplicates into proprietary `.library` | Locked to editor draft cache |
| **RAM Footprint** | **Tauri 2 + Rust (~60MB)** | Electron architecture (~600MB+) | Editor bundle (~1GB+) |
| **Playback Experience** | **Standalone native PiP window + Pin** | Fixed panel preview only | In-timeline preview only |
| **Privacy Guarantee** | **100% Offline & Local-First** | Local vault, third-party sync | Mandatory cloud telemetry |
| **Search Latency** | **SQLite FTS5 (<10ms)** | Lags on 100,000+ files | Limited project search |
| **Cross-Platform** | **Windows / macOS (Apple+Intel) / Linux** | Windows / macOS | Windows / macOS / Mobile |

---

## 🚀 Quick Download & Installation

Grab the latest installer for your system from [GitHub Releases](https://github.com/NovaMindLab/VidNest/releases/latest):

- **macOS (Apple Silicon M1/M2/M3/M4)**: `Local.Video.Intelligence_x.x.x_aarch64.dmg`
- **macOS (Intel x86_64)**: `Local.Video.Intelligence_x.x.x_x64.dmg`
- **Windows (x64)**: `Local.Video.Intelligence_x.x.x_x64-setup.exe`
- **Linux (x64)**: `Local.Video.Intelligence_x.x.x_amd64.AppImage` / `.deb`

---

## 🛠️ Local Development & Build

### 1. Prerequisites
- Node.js >= 20
- Rust toolchain (`rustc` & `cargo` stable)
- Platform dependencies (see official Tauri 2 prerequisites)

### 2. Setup & Development
```bash
# Install frontend dependencies
npm install

# Run automated tests
cd src-tauri && cargo test && cd ..

# Launch development window
npm run tauri dev
```

### 3. Build Production Bundle
```bash
npm run tauri build
```

---

## 📚 Technical Documentation

Explore detailed architectural blueprints and deep dives in our [Wiki](wiki/README.md):

- 📐 [System Architecture & Design Philosophy](wiki/architecture_and_design.md)
- ⚡ [Cross-Platform Scanner & Millisecond Indexing](wiki/full_disk_scanner_and_indexing.md)
- 🎬 [Media Library & Standalone Player](wiki/video_library_and_player.md)
- 🔄 [In-App Differential Updates](wiki/in_app_differential_update.md)
- 📥 [yt-dlp Downloader & Muxing Engine](wiki/ytdlp_and_downloader.md)
- 🚀 [Automated Release & Promotion Guide](wiki/autodeploy_guide.md)

---

## 🤝 Community & Contributing

Contributions are warmly welcomed!
- 🌟 Star the repository on GitHub to show your support!
- 🐛 Open an [Issue](https://github.com/NovaMindLab/VidNest/issues) for bug reports and feature suggestions.
- 🔀 Submit a Pull Request to contribute code or translations.

## 📄 License

VidNest is distributed under the [MIT License](LICENSE).
