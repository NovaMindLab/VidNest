use crate::models::{HealthReport, OsInfo, StorageStatus, ToolStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Probes an external executable tool by executing it with a version flag.
/// Prevents console windows from flashing on Windows desktop targets.
pub fn probe_external_tool(name: &str, version_arg: &str, required: bool, description: &str) -> ToolStatus {
    let mut cmd = Command::new(name);
    cmd.arg(version_arg);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.output() {
        Ok(output) if output.status.success() => {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout_str
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .trim()
                .to_string();

            ToolStatus {
                name: name.to_string(),
                found: true,
                path: None, // Can be refined when custom paths are configured
                version: if first_line.is_empty() { None } else { Some(first_line) },
                required,
                description: description.to_string(),
            }
        }
        _ => ToolStatus {
            name: name.to_string(),
            found: false,
            path: None,
            version: None,
            required,
            description: description.to_string(),
        },
    }
}

/// Tests whether a directory exists (or can be created) and has write permissions.
pub fn verify_directory_writable(dir: &Path) -> bool {
    if !dir.exists() {
        if fs::create_dir_all(dir).is_err() {
            return false;
        }
    }

    let test_file = dir.join(format!(".write_probe_{}", std::process::id()));
    match fs::write(&test_file, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(test_file);
            true
        }
        Err(_) => false,
    }
}

/// Collects OS metadata.
pub fn get_os_info() -> OsInfo {
    OsInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        family: std::env::consts::FAMILY.to_string(),
    }
}

/// Collects storage status for app data and temp dirs.
pub fn check_storage_status(app_data_path: Option<PathBuf>) -> StorageStatus {
    let app_data = app_data_path.unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });

    let temp_dir = std::env::temp_dir();

    let app_data_writable = verify_directory_writable(&app_data);
    let temp_writable = verify_directory_writable(&temp_dir);

    StorageStatus {
        app_data_dir: app_data.to_string_lossy().to_string(),
        app_data_writable,
        temp_dir: temp_dir.to_string_lossy().to_string(),
        temp_writable,
    }
}

use std::sync::{OnceLock, RwLock};
use std::time::Instant;

fn get_health_cache() -> &'static RwLock<Option<(Instant, HealthReport)>> {
    static CACHE: OnceLock<RwLock<Option<(Instant, HealthReport)>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(None))
}

/// Performs a full health check across system environment, storage, and external CLI tools.
/// Features a 30-second TTL in-memory cache and parallel tool probing for instant response.
pub fn run_health_check(app_data_path: Option<PathBuf>) -> HealthReport {
    let cache = get_health_cache();
    if let Ok(guard) = cache.read() {
        if let Some((instant, ref report)) = *guard {
            if instant.elapsed() < std::time::Duration::from_secs(30) {
                return report.clone();
            }
        }
    }

    let os = get_os_info();
    let app_data = app_data_path.unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });

    let storage = check_storage_status(Some(app_data.clone()));

    let (active_ytdlp, is_managed) = crate::ytdlp::resolve_active_binary(Some(&app_data));
    let ytdlp_target = active_ytdlp
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "yt-dlp".to_string());

    let ytdlp_desc = if is_managed {
        "yt-dlp 视频下载引擎 (应用内置独立托管版本，支持自动更新)"
    } else {
        "yt-dlp 视频下载引擎 (系统全局版本)"
    };

    let managed_ffmpeg = app_data.join("bin").join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
    let ffmpeg_target = if managed_ffmpeg.exists() {
        managed_ffmpeg.to_string_lossy().to_string()
    } else {
        "ffmpeg".to_string()
    };

    let managed_ffprobe = app_data.join("bin").join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
    let ffprobe_target = if managed_ffprobe.exists() {
        managed_ffprobe.to_string_lossy().to_string()
    } else {
        "ffprobe".to_string()
    };

    // Parallel multi-process tool probing using std::thread::scope
    let (mut ytdlp_status, mut ffmpeg_status, mut ffprobe_status) = std::thread::scope(|s| {
        let yt_h = s.spawn(|| probe_external_tool(&ytdlp_target, "--version", true, ytdlp_desc));
        let ff_h = s.spawn(|| probe_external_tool(&ffmpeg_target, "-version", true, "FFmpeg 多媒体处理内核 (音视频编解码与缩略图抽帧)"));
        let fp_h = s.spawn(|| probe_external_tool(&ffprobe_target, "-version", true, "FFprobe 媒体探测器 (流信息与元数据解析)"));
        (
            yt_h.join().unwrap_or_else(|_| ToolStatus { name: "yt-dlp".to_string(), found: false, path: None, version: None, required: true, description: ytdlp_desc.to_string() }),
            ff_h.join().unwrap_or_else(|_| ToolStatus { name: "ffmpeg".to_string(), found: false, path: None, version: None, required: true, description: "FFmpeg".to_string() }),
            fp_h.join().unwrap_or_else(|_| ToolStatus { name: "ffprobe".to_string(), found: false, path: None, version: None, required: true, description: "FFprobe".to_string() }),
        )
    });

    ytdlp_status.name = "yt-dlp".to_string();
    if active_ytdlp.is_some() {
        ytdlp_status.path = Some(ytdlp_target);
    }

    ffmpeg_status.name = "ffmpeg".to_string();
    if ffmpeg_status.found {
        ffmpeg_status.path = Some(ffmpeg_target);
    }

    ffprobe_status.name = "ffprobe".to_string();
    if ffprobe_status.found {
        ffprobe_status.path = Some(ffprobe_target);
    }

    let tools = vec![ffmpeg_status, ffprobe_status, ytdlp_status];

    let all_required_tools_present = tools.iter().all(|t| !t.required || t.found);
    let storage_healthy = storage.app_data_writable && storage.temp_writable;

    let status = if all_required_tools_present && storage_healthy {
        "healthy".to_string()
    } else {
        "degraded".to_string()
    };

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let report = HealthReport {
        app_name: "Local Video Intelligence".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        status,
        os,
        tools,
        storage,
        timestamp_ms,
    };

    if let Ok(mut guard) = cache.write() {
        *guard = Some((Instant::now(), report.clone()));
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_info() {
        let os_info = get_os_info();
        assert!(!os_info.os.is_empty());
        assert!(!os_info.arch.is_empty());
        assert!(!os_info.family.is_empty());
    }

    #[test]
    fn test_verify_directory_writable() {
        let temp = std::env::temp_dir();
        assert!(verify_directory_writable(&temp));
    }

    #[test]
    fn test_tool_probe_missing() {
        let probe = probe_external_tool("nonexistent_binary_xyz_12345", "--version", false, "Test tool");
        assert!(!probe.found);
        assert!(probe.version.is_none());
        assert_eq!(probe.name, "nonexistent_binary_xyz_12345");
    }

    #[test]
    fn test_health_report_generation() {
        let report = run_health_check(None);
        assert_eq!(report.app_name, "Local Video Intelligence");
        assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
        assert!(report.timestamp_ms > 0);
        assert_eq!(report.tools.len(), 3);
        assert!(report.storage.temp_writable);
    }
}
