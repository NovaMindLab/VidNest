use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoCompatibilityStatus {
    pub is_compatible: bool,
    pub has_cached_transcode: bool,
    pub cached_path: Option<String>,
    pub suggested_action: String, // "direct_play" | "cached_play" | "needs_conversion"
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeResult {
    pub original_path: String,
    pub compatible_path: String,
    pub is_cached: bool,
    pub remuxed: bool,
    pub output_size_bytes: u64,
}

/// Identifies extensions that are typically rejected by HTML5 WebKit / WebView2 video players.
pub fn is_extension_web_incompatible(ext: &str) -> bool {
    let lower = ext.trim_start_matches('.').to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "avi" | "wmv" | "asf" | "flv" | "f4v" | "rmvb" | "rm" | "vob" | "mpg" | "mpeg" | "mkv" | "ts" | "divx"
    )
}

/// Generates a cache file path for transcoded output in app_data/transcode_cache/
pub fn get_transcode_cache_path(
    file_path: &Path,
    file_hash: Option<&str>,
    app_data_dir: &Path,
) -> PathBuf {
    let cache_dir = app_data_dir.join("transcode_cache");
    let name_stem = match file_hash {
        Some(h) if !h.is_empty() => h.to_string(),
        _ => {
            // Fallback: derive a pseudo hash from file path and size
            let meta_len = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
            let raw_str = format!("{}_{}", file_path.to_string_lossy(), meta_len);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            raw_str.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        }
    };
    cache_dir.join(format!("{}.mp4", name_stem))
}

/// Resolves the ffmpeg executable path.
fn resolve_ffmpeg_bin(app_data_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = crate::downloader::executor::resolve_ffmpeg_dir(app_data_dir) {
        let exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        let p = dir.join(exe_name);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

/// Checks whether a video can be natively played in Webview or if a transcoded cache already exists.
pub fn check_video_compatibility(
    file_path: &Path,
    file_hash: Option<&str>,
    app_data_dir: &Path,
) -> VideoCompatibilityStatus {
    let cache_path = get_transcode_cache_path(file_path, file_hash, app_data_dir);

    // 1. Check if transcode cache already exists and is non-empty
    if cache_path.exists() {
        if let Ok(meta) = fs::metadata(&cache_path) {
            if meta.len() > 1024 {
                return VideoCompatibilityStatus {
                    is_compatible: true,
                    has_cached_transcode: true,
                    cached_path: Some(cache_path.to_string_lossy().to_string()),
                    suggested_action: "cached_play".to_string(),
                    reason: None,
                };
            }
        }
    }

    // 2. Check file container extension
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    if is_extension_web_incompatible(ext) {
        return VideoCompatibilityStatus {
            is_compatible: false,
            has_cached_transcode: false,
            cached_path: None,
            suggested_action: "needs_conversion".to_string(),
            reason: Some(format!(
                "格式 .{} 为传统或非 Web 容器，内嵌播放器需兼容优化以实现流畅播放与 AI 台词联动",
                ext
            )),
        };
    }

    // 3. Compatible container
    VideoCompatibilityStatus {
        is_compatible: true,
        has_cached_transcode: false,
        cached_path: None,
        suggested_action: "direct_play".to_string(),
        reason: None,
    }
}

/// Converts an incompatible video into a web-compatible H.264/AAC MP4.
/// Automatically chooses zero-loss remux (-c:v copy) if the stream is already H.264,
/// or hardware-accelerated / fast transcoding otherwise.
pub fn convert_video_to_compatible(
    file_path: &Path,
    file_hash: Option<&str>,
    app_data_dir: &Path,
) -> Result<TranscodeResult, String> {
    if !file_path.exists() {
        return Err(format!("原视频文件不存在: {:?}", file_path));
    }

    let cache_dir = app_data_dir.join("transcode_cache");
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("无法创建转码缓存目录 {:?}: {}", cache_dir, e))?;

    let final_output = get_transcode_cache_path(file_path, file_hash, app_data_dir);

    // Cache hit: return immediately
    if final_output.exists() {
        if let Ok(meta) = fs::metadata(&final_output) {
            if meta.len() > 1024 {
                return Ok(TranscodeResult {
                    original_path: file_path.to_string_lossy().to_string(),
                    compatible_path: final_output.to_string_lossy().to_string(),
                    is_cached: true,
                    remuxed: false,
                    output_size_bytes: meta.len(),
                });
            }
        }
    }

    let ffmpeg_bin = resolve_ffmpeg_bin(Some(app_data_dir));

    // Extract codec info to see if we can perform a zero-loss Remux
    let meta = crate::media::extract_metadata(file_path).ok();
    let is_h264 = meta
        .as_ref()
        .and_then(|m| m.codec.as_deref())
        .map(|c| c.eq_ignore_ascii_case("h264") || c.eq_ignore_ascii_case("avc1"))
        .unwrap_or(false);

    let tmp_output = cache_dir.join(format!(
        "tmp_{}_{}.mp4",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    // Strategy A: Zero-loss Remux if stream is already H.264
    let mut remux_succeeded = false;
    if is_h264 {
        let mut cmd = Command::new(&ffmpeg_bin);
        cmd.args([
            "-i",
        ]);
        cmd.arg(file_path);
        cmd.args([
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
            "-y",
        ]);
        cmd.arg(&tmp_output);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if let Ok(output) = cmd.output() {
            if output.status.success() && tmp_output.exists() {
                if let Ok(meta) = fs::metadata(&tmp_output) {
                    if meta.len() > 1024 {
                        remux_succeeded = true;
                    }
                }
            }
        }
    }

    // Strategy B: Hardware-accelerated / fast transcode if not remuxed
    if !remux_succeeded {
        // Clean up partial tmp file if any
        let _ = fs::remove_file(&tmp_output);

        // Try macOS VideoToolbox hardware encoder first if on Apple Darwin
        #[cfg(target_os = "macos")]
        let mut hw_succeeded = false;
        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new(&ffmpeg_bin);
            cmd.args([
                "-i",
            ]);
            cmd.arg(file_path);
            cmd.args([
                "-c:v",
                "h264_videotoolbox",
                "-b:v",
                "4500k",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-movflags",
                "+faststart",
                "-y",
            ]);
            cmd.arg(&tmp_output);

            if let Ok(output) = cmd.output() {
                if output.status.success() && tmp_output.exists() {
                    if let Ok(meta) = fs::metadata(&tmp_output) {
                        if meta.len() > 1024 {
                            hw_succeeded = true;
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        let needs_cpu_fallback = !hw_succeeded;
        #[cfg(not(target_os = "macos"))]
        let needs_cpu_fallback = true;

        if needs_cpu_fallback {
            let _ = fs::remove_file(&tmp_output);
            let mut cmd = Command::new(&ffmpeg_bin);
            cmd.args([
                "-i",
            ]);
            cmd.arg(file_path);
            cmd.args([
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-crf",
                "22",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-movflags",
                "+faststart",
                "-y",
            ]);
            cmd.arg(&tmp_output);

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            let output = cmd.output().map_err(|e| {
                format!("无法执行 FFmpeg 转码命令 (路径: {:?}): {}", ffmpeg_bin, e)
            })?;

            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                let _ = fs::remove_file(&tmp_output);
                return Err(format!("FFmpeg 转码失败: {}", err_msg.trim()));
            }
        }
    }

    // Verify generated file
    let out_meta = fs::metadata(&tmp_output).map_err(|e| {
        format!("转码输出文件读取失败: {}", e)
    })?;
    if out_meta.len() < 1024 {
        let _ = fs::remove_file(&tmp_output);
        return Err("转码生成的文件体积异常（小于 1KB），可能转码失败".to_string());
    }

    // Atomically rename temporary file to final output path
    fs::rename(&tmp_output, &final_output).map_err(|e| {
        format!("重命名兼容视频文件失败: {}", e)
    })?;

    Ok(TranscodeResult {
        original_path: file_path.to_string_lossy().to_string(),
        compatible_path: final_output.to_string_lossy().to_string(),
        is_cached: false,
        remuxed: remux_succeeded,
        output_size_bytes: out_meta.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_extension_web_incompatible() {
        assert!(is_extension_web_incompatible("avi"));
        assert!(is_extension_web_incompatible(".avi"));
        assert!(is_extension_web_incompatible("AVI"));
        assert!(is_extension_web_incompatible("wmv"));
        assert!(is_extension_web_incompatible("flv"));
        assert!(is_extension_web_incompatible("mkv"));
        assert!(is_extension_web_incompatible("rmvb"));

        assert!(!is_extension_web_incompatible("mp4"));
        assert!(!is_extension_web_incompatible("MP4"));
        assert!(!is_extension_web_incompatible("webm"));
        assert!(!is_extension_web_incompatible("mov"));
    }

    #[test]
    fn test_get_transcode_cache_path() {
        let dummy_path = Path::new("/videos/sample.avi");
        let app_dir = Path::new("/tmp/vidnest_test");
        let cache_path = get_transcode_cache_path(dummy_path, Some("abc123hash"), app_dir);
        assert_eq!(
            cache_path,
            Path::new("/tmp/vidnest_test/transcode_cache/abc123hash.mp4")
        );
    }
}
