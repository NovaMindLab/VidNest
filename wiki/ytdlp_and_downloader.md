# 📥 yt-dlp 在线视频解析与音画混流引擎 (yt-dlp Downloader & Merger)

VidNest 内置企业级在线视频下载模块，支持从主流流媒体平台（YouTube、Bilibili、TikTok 等）高速提取高清视频与音频。

---

## 💥 一、YouTube 4K DASH 混流陷阱与修复

### 1. 核心问题背景
YouTube 1080P 及以上规格（2K/4K/8K）均采用 **DASH (Dynamic Adaptive Streaming over HTTP)** 协议，其视频流与音频流在服务器端是物理分离的：
- 单独下载高清视频流（如 `137` 格式）会导致生成的文件**完全没有声音**；
- 若下载器无法在系统 PATH 中找到 `ffmpeg`，尝试自动混流时会直接中断崩溃并报错：
  `ERROR: Postprocessing: ffprobe and ffmpeg not found. Please install or provide the path using --ffmpeg-location`。

### 2. VidNest 的工业级修复方案

#### (1) 自动注入 `--ffmpeg-location`
VidNest 的下载执行器 ([executor.rs](file:///Users/hou/hqx/hubproject/VidNest/src-tauri/src/downloader/executor.rs)) 实现了本地静态二进制文件的优先寻址：
```rust
pub fn resolve_ffmpeg_dir(app_data_path: &Path) -> Option<PathBuf> {
    let local_bin = app_data_path.join("bin");
    if local_bin.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }).exists() {
        return Some(local_bin);
    }
    None // 降级为系统 PATH
}
```
每次执行探测与下载时，自动追加 `--ffmpeg-location "<managed_bin_dir>"`，确保无论用户系统是否预装 ffmpeg，均能由 VidNest 自带的静态引擎完成音画自动混流。

#### (2) 智能音视频配对参数
若用户仅选择了单个无声音视频轨，下载器自动智能扩充参数为：
`-f "<video_id>+bestaudio/best"`，保证下载产物始终为声画同步的完整 MP4/MKV。

---

## ⚡ 二、子进程异步排水防死锁机制 (Non-blocking Stderr Draining)

传统使用 `std::process::Command` 或 `tokio::process::Command` 同步等待子进程结束时：
- 若 yt-dlp 产生大量详细调试日志，操作系统的 `stderr` 管道缓冲区（通常为 64KB）会被迅速填满；
- 子进程由于无法继续写入管道而被操作系统挂起阻塞，父进程等待子进程退出，发生**经典的管道死锁 (Pipe Deadlock)**，表现为下载进度卡在 99% 或初始阶段假死。

### 解决方案
VidNest 在子进程生成后，立即派发独立的 tokio 异步协程专门消费 `child.stderr`，实现边读边排，彻底终结进程假死：
```rust
let stderr_task = tokio::spawn(async move {
    let mut reader = BufReader::new(stderr).lines();
    let mut logs = Vec::new();
    while let Ok(Some(line)) = reader.next_line().await {
        logs.push(line);
    }
    logs.join("\n")
});
```

---

## 🌐 三、Node.js 运行时集成 (`--js-runtimes`)

YouTube 近年来升级了播放器签名算法（Player Signature Cipher），纯 Python 脚本解析经常触发 `Signature extraction failed`。
VidNest 在构建命令时动态检测宿主系统的 Node.js 路径并注入：
`--js-runtimes node:<node_path>`，使 yt-dlp 能够秒级解密复杂签名，确保流地址解析率达 100%。
