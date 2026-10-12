use crate::db::Database;
use crate::downloader::parser::{extract_platform_from_url, parse_progress_line, ProgressUpdate};
use crate::models::{FormatOption, UrlInspectResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncBufReadExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskControlSignal {
    Run,
    Pause,
    Cancel,
}

#[derive(Debug)]
pub enum DownloadOutcome {
    Success { file_path: PathBuf },
    Paused,
    Cancelled,
    Failed { error: String },
}

/// Locates directory containing ffmpeg and ffprobe.
pub fn resolve_ffmpeg_dir(app_data_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = app_data_dir {
        let bin_dir = dir.join("bin");
        let ffmpeg_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        if bin_dir.join(ffmpeg_name).exists() {
            return Some(bin_dir);
        }
    }

    #[cfg(unix)]
    {
        for p in &["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin", "/usr/bin"] {
            let path = Path::new(p);
            if path.join("ffmpeg").exists() {
                return Some(path.to_path_buf());
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for part in path_var.split(sep) {
            let p = Path::new(part);
            let ffmpeg_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
            if p.join(ffmpeg_name).exists() {
                return Some(p.to_path_buf());
            }
        }
    }

    None
}

/// Locates a viable JavaScript runtime (Deno, Bun, or Node.js) for yt-dlp.
pub fn resolve_js_runtime() -> Option<String> {
    #[cfg(unix)]
    {
        // 1. Check well-known system locations
        for p in &[
            "/opt/homebrew/bin/deno",
            "/usr/local/bin/deno",
            "/opt/homebrew/bin/bun",
            "/usr/local/bin/bun",
            "/opt/homebrew/bin/node",
            "/usr/local/bin/node",
            "/usr/bin/node",
        ] {
            let path = Path::new(p);
            if path.exists() {
                let kind = if p.contains("deno") {
                    "deno"
                } else if p.contains("bun") {
                    "bun"
                } else {
                    "node"
                };
                return Some(format!("{}:{}", kind, p));
            }
        }

        // 2. Check user home directories (.deno, .bun, .nvm, .volta)
        if let Ok(home) = std::env::var("HOME") {
            let home_path = Path::new(&home);

            let deno_bin = home_path.join(".deno/bin/deno");
            if deno_bin.exists() {
                return Some(format!("deno:{}", deno_bin.to_string_lossy()));
            }

            let bun_bin = home_path.join(".bun/bin/bun");
            if bun_bin.exists() {
                return Some(format!("bun:{}", bun_bin.to_string_lossy()));
            }

            let nvm_versions = home_path.join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
                let mut node_candidates = Vec::new();
                for entry in entries.flatten() {
                    let bin = entry.path().join("bin/node");
                    if bin.exists() {
                        node_candidates.push(bin);
                    }
                }
                node_candidates.sort();
                if let Some(latest) = node_candidates.last() {
                    return Some(format!("node:{}", latest.to_string_lossy()));
                }
            }

            let volta_bin = home_path.join(".volta/bin/node");
            if volta_bin.exists() {
                return Some(format!("node:{}", volta_bin.to_string_lossy()));
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for part in path_var.split(sep) {
            let p = Path::new(part);
            let deno_name = if cfg!(windows) { "deno.exe" } else { "deno" };
            if p.join(deno_name).exists() {
                return Some(format!("deno:{}", p.join(deno_name).to_string_lossy()));
            }
            let bun_name = if cfg!(windows) { "bun.exe" } else { "bun" };
            if p.join(bun_name).exists() {
                return Some(format!("bun:{}", p.join(bun_name).to_string_lossy()));
            }
            let node_name = if cfg!(windows) { "node.exe" } else { "node" };
            if p.join(node_name).exists() {
                return Some(format!("node:{}", p.join(node_name).to_string_lossy()));
            }
        }
    }

    #[cfg(windows)]
    {
        for p in &[
            r"C:\Program Files\nodejs\node.exe",
            r"C:\Program Files (x86)\nodejs\node.exe",
        ] {
            let path = Path::new(p);
            if path.exists() {
                return Some(format!("node:{}", p));
            }
        }
    }

    None
}

/// Builds the safe arguments array for yt-dlp download execution.
/// Guarantees that arguments are passed as discrete vector elements without shell concatenation.
pub fn build_download_args(
    url: &str,
    format_id: Option<&str>,
    output_template: &str,
    ffmpeg_location: Option<&Path>,
    js_runtime: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "--newline".to_string(),
        "--no-colors".to_string(),
        "--no-warnings".to_string(),
        "--no-update".to_string(),
        "--no-playlist".to_string(),
        "--no-check-certificates".to_string(),
        "--retries".to_string(),
        "5".to_string(),
        "--fragment-retries".to_string(),
        "5".to_string(),
        "--progress".to_string(),
        "--no-quiet".to_string(),
        "--no-simulate".to_string(),
        "--concurrent-fragments".to_string(),
        "5".to_string(),
        "--throttled-rate".to_string(),
        "100K".to_string(),
        "--compat-options".to_string(),
        "no-youtube-unavailable-videos".to_string(),
        "--print".to_string(),
        "after_move:filepath".to_string(),
        "-o".to_string(),
        output_template.to_string(),
    ];

    if let Some(ff_loc) = ffmpeg_location {
        args.push("--ffmpeg-location".to_string());
        args.push(ff_loc.to_string_lossy().to_string());
    }

    if let Some(js_rt) = js_runtime {
        let trimmed = js_rt.trim();
        if !trimmed.is_empty() {
            args.push("--js-runtimes".to_string());
            args.push(trimmed.to_string());
        }
    }

    if let Some(fid) = format_id {
        let trimmed = fid.trim();
        if !trimmed.is_empty() {
            args.push("-f".to_string());
            if !trimmed.contains('+') && !trimmed.contains('/') && !trimmed.starts_with("bestaudio") && !trimmed.starts_with("ba") {
                args.push(format!("{}+bestaudio/best/{}", trimmed, trimmed));
            } else {
                args.push(trimmed.to_string());
            }
        }
    }

    args.push(url.to_string());
    args
}

/// Inspects URL metadata using yt-dlp --dump-single-json.
pub async fn inspect_url(url: &str, app_data_dir: Option<&Path>) -> Result<UrlInspectResult, String> {
    let (active_bin, _) = crate::ytdlp::resolve_active_binary(app_data_dir);
    let bin_path = active_bin.ok_or_else(|| {
        "未找到可用的 yt-dlp 可执行文件，请检查系统 PATH 或在设置中完成更新安装".to_string()
    })?;

    let mut cmd = tokio::process::Command::new(&bin_path);
    let mut args = vec![
        "--dump-single-json".to_string(),
        "--no-playlist".to_string(),
        "--no-warnings".to_string(),
        "--no-update".to_string(),
        "--no-colors".to_string(),
        "--skip-download".to_string(),
        "--no-check-certificates".to_string(),
    ];

    if let Some(ff_loc) = resolve_ffmpeg_dir(app_data_dir) {
        args.push("--ffmpeg-location".to_string());
        args.push(ff_loc.to_string_lossy().to_string());
    }

    if let Some(js_rt) = resolve_js_runtime() {
        args.push("--js-runtimes".to_string());
        args.push(js_rt);
    }

    args.push(url.to_string());
    cmd.args(&args);

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("执行 yt-dlp 探测元数据失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp 探测元数据失败: {}", stderr.trim()));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let json_val: serde_json::Value = serde_json::from_str(&stdout_str)
        .map_err(|e| format!("解析 yt-dlp 元数据 JSON 失败: {}", e))?;

    let title = json_val
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled Video")
        .to_string();

    let uploader = json_val
        .get("uploader")
        .or_else(|| json_val.get("channel"))
        .or_else(|| json_val.get("uploader_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let duration = json_val.get("duration").and_then(|v| v.as_f64());
    let thumbnail = json_val.get("thumbnail").and_then(|v| v.as_str()).map(|s| s.to_string());
    let extractor = json_val
        .get("extractor_key")
        .or_else(|| json_val.get("extractor"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| Some(extract_platform_from_url(url)));
    let description = json_val.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

    let mut formats = Vec::new();
    if let Some(formats_arr) = json_val.get("formats").and_then(|f| f.as_array()) {
        for f in formats_arr {
            let format_id = match f.get("format_id").and_then(|v| v.as_str()) {
                Some(id) if !id.trim().is_empty() => id.to_string(),
                _ => continue,
            };

            let ext = f.get("ext").and_then(|v| v.as_str()).map(|s| s.to_string());
            let note = f.get("format_note").and_then(|v| v.as_str()).map(|s| s.to_string());
            let resolution = f
                .get("resolution")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    let w = f.get("width").and_then(|v| v.as_u64());
                    let h = f.get("height").and_then(|v| v.as_u64());
                    match (w, h) {
                        (Some(width), Some(height)) => Some(format!("{}x{}", width, height)),
                        _ => None,
                    }
                });

            let filesize = f
                .get("filesize")
                .and_then(|v| v.as_u64())
                .or_else(|| f.get("filesize_approx").and_then(|v| v.as_u64()));

            let vcodec = f
                .get("vcodec")
                .and_then(|v| v.as_str())
                .filter(|c| *c != "none")
                .map(|s| s.to_string());

            let acodec = f
                .get("acodec")
                .and_then(|v| v.as_str())
                .filter(|c| *c != "none")
                .map(|s| s.to_string());

            let tbr = f.get("tbr").and_then(|v| v.as_f64());

            formats.push(FormatOption {
                format_id,
                ext,
                resolution,
                note,
                filesize,
                vcodec,
                acodec,
                tbr,
            });
        }
    }

    Ok(UrlInspectResult {
        url: url.to_string(),
        title,
        uploader,
        duration,
        thumbnail,
        extractor,
        formats,
        description,
    })
}

/// Executes yt-dlp to download the video, parsing real-time progress and watching for control signals.
pub async fn execute_download<F>(
    bin_path: &Path,
    args: &[String],
    downloads_dir: &Path,
    mut control_rx: tokio::sync::watch::Receiver<TaskControlSignal>,
    mut on_progress: F,
) -> DownloadOutcome
where
    F: FnMut(ProgressUpdate) + Send + 'static,
{
    let mut cmd = tokio::process::Command::new(bin_path);
    cmd.args(args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return DownloadOutcome::Failed {
                error: format!("启动 yt-dlp 下载进程失败: {}", e),
            };
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            return DownloadOutcome::Failed {
                error: "无法捕获 yt-dlp 输出流".to_string(),
            };
        }
    };

    let stderr = child.stderr.take();
    let captured_stderr = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let captured_stderr_clone = captured_stderr.clone();

    if let Some(err_stream) = stderr {
        tokio::spawn(async move {
            let mut err_reader = tokio::io::BufReader::new(err_stream).lines();
            while let Ok(Some(line)) = err_reader.next_line().await {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let mut lock = captured_stderr_clone.lock().await;
                    if lock.len() < 100 {
                        lock.push(trimmed.to_string());
                    }
                }
            }
        });
    }

    let mut reader = tokio::io::BufReader::new(stdout).lines();
    let mut captured_filepath: Option<PathBuf> = None;
    let mut is_paused = false;
    let mut is_cancelled = false;

    loop {
        tokio::select! {
            changed_res = control_rx.changed() => {
                if changed_res.is_ok() {
                    let sig = *control_rx.borrow();
                    match sig {
                        TaskControlSignal::Pause => {
                            is_paused = true;
                            let _ = child.kill().await;
                            break;
                        }
                        TaskControlSignal::Cancel => {
                            is_cancelled = true;
                            let _ = child.kill().await;
                            break;
                        }
                        TaskControlSignal::Run => {}
                    }
                }
            }
            line_res = reader.next_line() => {
                match line_res {
                    Ok(Some(line)) => {
                        for subline in line.split('\r') {
                            let trimmed = subline.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            // Check if subline is directly a filepath printed by `--print after_move:filepath`
                            if !trimmed.starts_with('[') {
                                let path = PathBuf::from(trimmed);
                                if path.is_file() || trimmed.ends_with(".mp4") || trimmed.ends_with(".mkv") || trimmed.ends_with(".webm") {
                                    captured_filepath = Some(path);
                                }
                            }

                            if let Some(update) = parse_progress_line(trimmed) {
                                if let Some(ref fp) = update.file_path {
                                    captured_filepath = Some(PathBuf::from(fp));
                                }
                                on_progress(update);
                            }
                        }
                    }
                    Ok(None) => break, // EOF
                    Err(_) => break,
                }
            }
        }
    }

    let status_res = child.wait().await;

    if is_paused {
        return DownloadOutcome::Paused;
    }
    if is_cancelled {
        return DownloadOutcome::Cancelled;
    }

    match status_res {
        Ok(status) if status.success() => {
            // Find the completed file path
            let final_file = if let Some(path) = captured_filepath {
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            };

            let resolved_path = final_file.or_else(|| {
                // Scan downloads_dir for latest non-.part video file
                find_latest_file_in_dir(downloads_dir)
            });

            match resolved_path {
                Some(fp) => DownloadOutcome::Success { file_path: fp },
                None => DownloadOutcome::Failed {
                    error: "下载完成但无法定位输出文件".to_string(),
                },
            }
        }
        Ok(status) => {
            let stderr_lines = captured_stderr.lock().await;
            let mut err_msg = format!("yt-dlp 退出码: {:?}", status.code());
            if !stderr_lines.is_empty() {
                let last_lines = if stderr_lines.len() > 6 {
                    &stderr_lines[stderr_lines.len() - 6..]
                } else {
                    &stderr_lines[..]
                };
                err_msg = format!("{}: {}", err_msg, last_lines.join(" | "));
            }
            DownloadOutcome::Failed { error: err_msg }
        }
        Err(e) => DownloadOutcome::Failed {
            error: format!("等待 yt-dlp 进程异常: {}", e),
        },
    }
}

/// Scans the directory for the most recently modified video file.
fn find_latest_file_in_dir(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut latest: Option<(PathBuf, SystemTime)> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
            if ext != "part" && ext != "ytdl" && crate::scanner::is_video_file(&path) {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mod_time) = meta.modified() {
                        if latest.as_ref().map_or(true, |(_, t)| mod_time > *t) {
                            latest = Some((path, mod_time));
                        }
                    }
                }
            }
        }
    }

    latest.map(|(p, _)| p)
}

/// Executes the closed-loop ingestion after download completion:
/// 1. Computes SHA-256 hash
/// 2. Calls ffprobe for duration, dimensions, codec
/// 3. Calls ffmpeg to generate video thumbnail
/// 4. Inserts/updates `videos` table
/// 5. Inserts record into `video_sources` table
/// 6. Updates FTS5 full-text index
pub fn ingest_downloaded_video(
    file_path: &Path,
    task_id: &str,
    original_url: &str,
    title: Option<&str>,
    uploader: Option<&str>,
    platform: Option<&str>,
    db: &Database,
    app_data_dir: &Path,
) -> Result<i64, String> {
    if !file_path.exists() {
        return Err(format!("下载的目标文件不存在: {:?}", file_path));
    }

    let file_path_str = file_path.to_string_lossy().to_string();
    let file_name = file_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "downloaded_video".to_string());

    let metadata = fs::metadata(file_path)
        .map_err(|e| format!("读取文件属性失败: {}", e))?;
    let file_size = metadata.len();

    // 1. Compute hash
    let file_hash = crate::scanner::compute_fast_file_hash(file_path)?;

    // 2. ffprobe metadata
    let media_meta = crate::media::extract_metadata(file_path).unwrap_or_default();

    // 3. ffmpeg thumbnail
    let thumbnail_path = crate::media::generate_thumbnail(
        file_path,
        &file_hash,
        media_meta.duration,
        app_data_dir,
    )
    .ok()
    .map(|p| p.to_string_lossy().to_string());

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let conn_arc = db.get_connection();
    let conn = conn_arc.lock().map_err(|e| e.to_string())?;

    // 4. Insert into videos table
    let video_id = crate::db::videos::insert_or_update_video(
        &conn,
        &file_path_str,
        &file_name,
        &file_hash,
        file_size,
        media_meta.duration,
        media_meta.width,
        media_meta.height,
        media_meta.codec.as_deref(),
        thumbnail_path.as_deref(),
        now_ms,
    )?;

    // 5. Insert into video_sources table
    let resolved_platform = platform
        .map(|p| p.to_string())
        .unwrap_or_else(|| extract_platform_from_url(original_url));

    crate::db::tasks::insert_video_source(
        &conn,
        video_id,
        Some(original_url),
        Some(&resolved_platform),
        uploader,
        Some(task_id),
        now_ms,
    )?;

    // 6. Synchronize FTS5 index with video title and author tags
    let indexed_title = title.unwrap_or(&file_name);
    let indexed_tags = format!(
        "{},{}",
        resolved_platform,
        uploader.unwrap_or("")
    );

    let _ = conn.execute(
        "DELETE FROM fts_index WHERE video_id = ?1",
        [video_id.to_string()],
    );
    let _ = conn.execute(
        "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, '', '', ?3)",
        rusqlite::params![video_id.to_string(), indexed_title, indexed_tags],
    );

    Ok(video_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_download_args() {
        let args = build_download_args(
            "https://www.youtube.com/watch?v=123",
            Some("1080p"),
            "downloads/%(title)s.%(ext)s",
            Some(Path::new("/dummy/bin")),
            Some("node:/usr/bin/node"),
        );
        assert!(args.contains(&"--newline".to_string()));
        assert!(args.contains(&"--ffmpeg-location".to_string()));
        assert!(args.contains(&"/dummy/bin".to_string()));
        assert!(args.contains(&"--js-runtimes".to_string()));
        assert!(args.contains(&"node:/usr/bin/node".to_string()));
        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"downloads/%(title)s.%(ext)s".to_string()));
        assert!(args.contains(&"-f".to_string()));
        assert!(args.contains(&"1080p+bestaudio/best/1080p".to_string()));
        assert_eq!(args.last(), Some(&"https://www.youtube.com/watch?v=123".to_string()));
    }

    #[test]
    fn test_ingest_downloaded_video_in_memory() {
        let db = Database::init_in_memory().expect("in-memory db");
        let temp_dir = std::env::temp_dir();
        let test_video_file = temp_dir.join(format!("test_ingest_{}.mp4", std::process::id()));
        fs::write(&test_video_file, b"sample binary video content").expect("write test file");

        let video_id = ingest_downloaded_video(
            &test_video_file,
            "task_abc",
            "https://www.bilibili.com/video/BV123",
            Some("My Great Video"),
            Some("ChannelOwner"),
            Some("bilibili"),
            &db,
            &temp_dir,
        )
        .expect("ingest video");

        assert!(video_id > 0);

        // Verify video_sources record
        let conn = db.get_connection();
        let conn_guard = conn.lock().unwrap();
        let source = crate::db::tasks::get_video_source_by_video_id(&conn_guard, video_id)
            .expect("query source")
            .expect("source exists");

        assert_eq!(source.video_id, video_id);
        assert_eq!(source.platform, Some("bilibili".to_string()));
        assert_eq!(source.author, Some("ChannelOwner".to_string()));
        assert_eq!(source.download_task_id, Some("task_abc".to_string()));

        // Verify FTS5 query works with author tag
        let fts_count: i64 = conn_guard
            .query_row(
                "SELECT COUNT(*) FROM fts_index WHERE fts_index MATCH 'ChannelOwner*'",
                [],
                |r| r.get(0),
            )
            .expect("fts query");
        assert_eq!(fts_count, 1);

        let _ = fs::remove_file(test_video_file);
    }
}
