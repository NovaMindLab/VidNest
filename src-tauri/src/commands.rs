use crate::ai::whisper::{
    check_whisper_engine_status, load_ai_config, save_ai_config as persist_ai_config,
    transcribe_video_pipeline, AiTranscriptionConfig, LocalWhisperStatus,
};
use crate::db::subtitles::{
    delete_video_subtitles as db_delete_subtitles, export_to_srt, export_to_vtt,
    get_video_subtitles as db_get_subtitles, save_video_subtitles,
    search_subtitles_snippets as db_search_subtitles_snippets, SubtitleSegment,
    SubtitleSnippetMatch,
};
use crate::db::Database;
use crate::health::run_health_check;
use crate::media::read_thumbnail_base64;
use crate::models::{
    AudioExtractionOptions, AudioExtractionResult, CategoryStats, ClipVideoRequest,
    ClipVideoResponse, Collection, DownloadTask, DuplicateGroup, DuplicateResolutionAction,
    ExtractedAudio, ExtractedAudioFilter, HealthReport, RecommendSegmentsResponse, ResolutionSummary,
    ScanSummary, SourceFolderStat, Tag, UrlInspectResult, Video,
    VideoFilter, VideoListResponse, YtDlpUpdateCheck, YtDlpUpdateResult,
};
use crate::scanner::scan_directory;
use crate::ytdlp;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn system_health_check(app: AppHandle) -> Result<HealthReport, String> {
    let app_data_path = app.path().app_data_dir().ok();
    tokio::task::spawn_blocking(move || {
        let report = run_health_check(app_data_path);
        Ok(report)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn check_ytdlp_update(app: AppHandle) -> Result<YtDlpUpdateCheck, String> {
    let app_data_path = app.path().app_data_dir().ok().unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    ytdlp::check_ytdlp_update(Some(&app_data_path)).await
}

#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<YtDlpUpdateResult, String> {
    let app_data_path = app.path().app_data_dir().ok().unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    ytdlp::update_ytdlp(&app_data_path).await
}

#[tauri::command]
pub async fn scan_video_directory(
    path: String,
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<ScanSummary, String> {
    let app_data_path = app.path().app_data_dir().unwrap_or_else(|_| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });

    let dir_path = PathBuf::from(path);
    // Run CPU-bound directory scanning on blocking thread pool
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        scan_directory(&dir_path, &db_clone, &app_data_path)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn start_full_disk_scan(
    min_size_mb: Option<u64>,
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<ScanSummary, String> {
    let app_data_path = app.path().app_data_dir().unwrap_or_else(|_| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });

    let min_bytes = min_size_mb
        .map(|mb| mb * 1024 * 1024)
        .unwrap_or(crate::scanner::DEFAULT_MIN_SIZE_BYTES);

    let db_clone = (*db).clone();
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        crate::scanner::scan_full_disk(&app_clone, &db_clone, &app_data_path, min_bytes)
    })
    .await
    .map_err(|e| format!("Full disk scan task error: {}", e))?
}

#[tauri::command]
pub fn cancel_full_disk_scan() -> Result<bool, String> {
    Ok(crate::scanner::cancel_scan())
}

#[tauri::command]
pub async fn list_videos(
    filter: Option<VideoFilter>,
    db: State<'_, Database>,
) -> Result<VideoListResponse, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;

        let mut response = crate::db::videos::list_videos(&conn, filter.unwrap_or_default())?;

        // Populate thumbnail base64 data for the page items (cached in memory)
        for item in &mut response.items {
            if let Some(thumb_path) = &item.thumbnail_path {
                item.thumbnail_data = read_thumbnail_base64(Path::new(thumb_path));
            }
        }

        Ok(response)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_video_detail(
    id: i64,
    db: State<'_, Database>,
) -> Result<Option<Video>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;

        let mut video = crate::db::videos::get_video_by_id(&conn, id)?;
        if let Some(ref mut v) = video {
            if let Some(thumb_path) = &v.thumbnail_path {
                v.thumbnail_data = read_thumbnail_base64(Path::new(thumb_path));
            }
        }

        Ok(video)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_video(
    id: i64,
    delete_file: Option<bool>,
    deleteFile: Option<bool>,
    db: State<'_, Database>,
) -> Result<bool, String> {
    let should_delete = delete_file.or(deleteFile).unwrap_or(false);
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;

        crate::db::videos::delete_video(&conn, id, should_delete)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn reveal_in_explorer(app: AppHandle, path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let trimmed = path.trim().trim_matches('"').trim_matches('\'');
        let p = std::path::PathBuf::from(trimmed);

        if !p.exists() {
            // If the specific file doesn't exist, check if parent directory exists
            if let Some(parent) = p.parent() {
                if parent.exists() {
                    use tauri_plugin_opener::OpenerExt;
                    let _ = app.opener().open_path(parent.to_string_lossy().to_string(), None::<&str>);
                    return Err(format!("文件不存在，已定位至所在目录: {}", parent.display()));
                }
            }
            return Err("文件路径在本地磁盘上不存在".to_string());
        }

        // 1. Resolve canonical path if possible, stripping Windows \\?\ extended prefix
        let clean_path = if let Ok(canonical) = p.canonicalize() {
            let s = canonical.to_string_lossy().to_string();
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                stripped.to_string()
            } else {
                s
            }
        } else {
            p.to_string_lossy().to_string()
        };

        // 2. Primary: Try native tauri_plugin_opener (uses Windows SHOpenFolderAndSelectItems API)
        use tauri_plugin_opener::OpenerExt;
        if app.opener().reveal_item_in_dir(&clean_path).is_ok() {
            return Ok(());
        }

        // 3. Fallback: Platform-specific command invocation
        #[cfg(target_os = "windows")]
        {
            let win_path = clean_path.replace('/', "\\");
            use std::os::windows::process::CommandExt;
            let mut cmd = std::process::Command::new("explorer");
            // raw_arg ensures /select,"C:\..." is passed without Rust auto-wrapping outer quotes
            cmd.raw_arg(format!("/select,\"{}\"", win_path));
            if let Ok(mut child) = cmd.spawn() {
                let _ = child.wait();
                return Ok(());
            }

            // Secondary fallback: open parent folder if selection fails
            if let Some(parent) = std::path::Path::new(&win_path).parent() {
                let _ = std::process::Command::new("explorer").arg(parent).spawn();
            }
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("open")
                .arg("-R")
                .arg(&clean_path)
                .status();
            if let Ok(st) = status {
                if st.success() {
                    return Ok(());
                }
            }
            if let Some(parent) = std::path::Path::new(&clean_path).parent() {
                let _ = std::process::Command::new("open").arg(parent).status();
            }
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            let parent = std::path::Path::new(&clean_path).parent().unwrap_or(std::path::Path::new(&clean_path));
            let _ = std::process::Command::new("xdg-open").arg(parent).status();
            return Ok(());
        }

        #[allow(unreachable_code)]
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}

#[tauri::command]
pub fn echo(message: String) -> String {
    format!("Echo from Rust backend: {}", message)
}

#[tauri::command]
pub async fn inspect_video_url(url: String, app: AppHandle) -> Result<UrlInspectResult, String> {
    let app_data_path = app.path().app_data_dir().unwrap_or_else(|_| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    crate::downloader::inspect_url(&url, Some(&app_data_path)).await
}

#[tauri::command]
pub async fn start_download(
    url: String,
    format_id: Option<String>,
    title: Option<String>,
    author: Option<String>,
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<DownloadTask, String> {
    let app_data_path = app.path().app_data_dir().unwrap_or_else(|_| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    let manager = crate::downloader::DownloadManager::global();
    manager
        .start_download(url, format_id, title, author, app_data_path, (*db).clone())
        .await
}

#[tauri::command]
pub async fn pause_download(task_id: String) -> Result<bool, String> {
    let manager = crate::downloader::DownloadManager::global();
    manager.pause_task(&task_id).await
}

#[tauri::command]
pub async fn resume_download(
    task_id: String,
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<bool, String> {
    let app_data_path = app.path().app_data_dir().unwrap_or_else(|_| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    let manager = crate::downloader::DownloadManager::global();
    manager
        .resume_task(&task_id, app_data_path, (*db).clone())
        .await
}

#[tauri::command]
pub async fn cancel_download(task_id: String) -> Result<bool, String> {
    let manager = crate::downloader::DownloadManager::global();
    manager.cancel_task(&task_id).await
}

#[tauri::command]
pub async fn get_download_tasks() -> Result<Vec<DownloadTask>, String> {
    let manager = crate::downloader::DownloadManager::global();
    Ok(manager.get_tasks().await)
}

#[tauri::command]
pub async fn clear_completed_tasks() -> Result<usize, String> {
    let manager = crate::downloader::DownloadManager::global();
    Ok(manager.clear_completed().await)
}
#[tauri::command]
pub async fn inspect_url(url: String, app: AppHandle) -> Result<UrlInspectResult, String> {
    inspect_video_url(url, app).await
}

#[tauri::command]
pub async fn pause_download_task(task_id: String) -> Result<bool, String> {
    pause_download(task_id).await
}

#[tauri::command]
pub async fn resume_download_task(
    task_id: String,
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<bool, String> {
    resume_download(task_id, app, db).await
}

#[tauri::command]
pub async fn cancel_download_task(task_id: String) -> Result<bool, String> {
    cancel_download(task_id).await
}

#[tauri::command]
pub async fn check_app_update(mock: Option<bool>) -> Result<crate::models::AppUpdateInfo, String> {
    let mock_mode = mock.unwrap_or(false);
    crate::updater::check_app_update_internal(mock_mode).await
}

#[tauri::command]
pub async fn download_and_install_update(
    app: AppHandle,
    download_url: String,
    total_bytes: u64,
    sha256: Option<String>,
) -> Result<crate::models::UpdateInstallResult, String> {
    crate::updater::download_and_install_update_internal(app, download_url, total_bytes, sha256).await
}

#[tauri::command]
pub fn install_update_and_relaunch(app: AppHandle) -> Result<(), String> {
    let installer_path = {
        let lock = crate::updater::DOWNLOADED_INSTALLER_PATH.lock().map_err(|e| e.to_string())?;
        lock.clone()
    };

    let path = match installer_path {
        Some(p) if p.exists() => p,
        _ => {
            let temp_dir = std::env::temp_dir();
            #[cfg(target_os = "windows")]
            let candidate = temp_dir.join("vidnest-update-setup.exe");
            #[cfg(target_os = "macos")]
            let candidate = temp_dir.join("vidnest-update-setup.dmg");
            #[cfg(target_os = "linux")]
            let candidate = temp_dir.join("vidnest-update-setup.AppImage");

            if candidate.exists() {
                candidate
            } else {
                app.restart();
            }
        }
    };

    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("Local Video Intelligence.exe"));
        let clean_installer = path.to_string_lossy().replace('/', "\\");
        let clean_exe = current_exe.to_string_lossy().replace('/', "\\");

        // Write a dedicated, robust updater batch script in temp dir
        // This avoids any cmd.exe /C inline argument quoting and backslash parsing bugs
        let temp_dir = std::env::temp_dir();
        let bat_path = temp_dir.join("vidnest_silent_updater.bat");

        let bat_lines = [
            "@echo off",
            "chcp 65001 >nul",
            "timeout /t 1 /nobreak >nul",
            "taskkill /F /IM \"Local Video Intelligence.exe\" >nul 2>&1",
            "taskkill /F /IM \"local-video-intelligence.exe\" >nul 2>&1",
            &format!("start /wait \"\" \"{}\" /S", clean_installer),
            "timeout /t 1 /nobreak >nul",
            &format!("start \"\" \"{}\"", clean_exe),
            "del \"%~f0\" >nul 2>&1",
        ];
        let bat_content = bat_lines.join("\r\n");

        if let Ok(_) = std::fs::write(&bat_path, bat_content) {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let mut cmd = std::process::Command::new("cmd.exe");
            cmd.args(["/C", &bat_path.to_string_lossy().to_string()]);
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.spawn();
            std::process::exit(0);
        } else {
            // Fallback: direct spawn of installer with /S
            let _ = std::process::Command::new(&path).arg("/S").spawn();
            std::process::exit(0);
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS 自动直接覆盖安装：
        // 若为 dmg 则静默挂载并直接覆盖替换 /Applications/ 下的应用，跳过手动拖拽，完成后直接拉起新版
        let script = format!(
            r#"
            sleep 1
            if echo "{}" | grep -q "\.dmg$"; then
                MOUNT_DIR=$(mktemp -d -t vidnest_mount)
                if hdiutil attach -nobrowse "{}" -mountpoint "$MOUNT_DIR" >/dev/null 2>&1; then
                    APP_DIR=$(find "$MOUNT_DIR" -maxdepth 2 -name "*.app" | head -n 1)
                    if [ -n "$APP_DIR" ]; then
                        APP_NAME=$(basename "$APP_DIR")
                        DEST="/Applications/$APP_NAME"
                        rm -rf "$DEST"
                        cp -R "$APP_DIR" "/Applications/"
                        xattr -rd com.apple.quarantine "$DEST" 2>/dev/null || true
                        hdiutil detach "$MOUNT_DIR" -force >/dev/null 2>&1 || true
                        rm -rf "$MOUNT_DIR"
                        open "$DEST"
                        exit 0
                    fi
                    hdiutil detach "$MOUNT_DIR" -force >/dev/null 2>&1 || true
                    rm -rf "$MOUNT_DIR"
                fi
            fi
            open "{}"
            "#,
            path.display(),
            path.display(),
            path.display()
        );

        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .spawn();

        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("LocalVideoIntelligence"));
        let script = format!(
            r#"
            sleep 1
            chmod +x "{}"
            cp -f "{}" "{}"
            chmod +x "{}"
            "{}" &
            "#,
            path.display(),
            path.display(),
            current_exe.display(),
            current_exe.display(),
            current_exe.display()
        );

        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .spawn();

        std::process::exit(0);
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[tauri::command]
pub async fn open_update_installer_folder() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        let installer_path = {
            let lock = crate::updater::DOWNLOADED_INSTALLER_PATH.lock().map_err(|e| e.to_string())?;
            lock.clone()
        };
        let target = installer_path.unwrap_or_else(|| std::env::temp_dir());
        let folder = if target.is_file() {
            target.parent().unwrap_or(&target).to_path_buf()
        } else {
            target
        };

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(&folder).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&folder).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn relaunch_app(app: AppHandle) {
    if let Err(_) = install_update_and_relaunch(app.clone()) {
        app.restart();
    }
}

#[derive(Default)]
pub struct PlayerState {
    pub current_video: Mutex<Option<Value>>,
}

#[tauri::command]
pub async fn open_video_player_window(
    app: AppHandle,
    state: State<'_, PlayerState>,
    video: Value,
) -> Result<(), String> {
    // 1. Update managed player state
    {
        let mut cur = state.current_video.lock().map_err(|e| e.to_string())?;
        *cur = Some(video.clone());
    }

    let win_title = video
        .get("file_name")
        .and_then(|v| v.as_str())
        .map(|n| format!("VidNest 播放器 - {}", n))
        .unwrap_or_else(|| "VidNest 独立播放器".to_string());

    // 2. If window already exists, activate and emit event
    if let Some(win) = app.get_webview_window("video-player") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        let _ = win.set_title(&win_title);
        let _ = win.emit("play-video", &video);
        return Ok(());
    }

    // 3. Otherwise, create new dedicated WebviewWindow
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "video-player",
        tauri::WebviewUrl::App("index.html?window=player".into()),
    )
    .title(&win_title)
    .inner_size(1080.0, 720.0)
    .min_inner_size(600.0, 400.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("创建独立播放器窗口失败: {}", e))?;

    let _ = win.set_focus();
    Ok(())
}

#[tauri::command]
pub async fn get_player_init_video(state: State<'_, PlayerState>) -> Result<Option<Value>, String> {
    let cur = state.current_video.lock().map_err(|e| e.to_string())?;
    Ok(cur.clone())
}

#[tauri::command]
pub async fn set_player_always_on_top(app: AppHandle, always_on_top: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("video-player") {
        win.set_always_on_top(always_on_top)
            .map_err(|e| format!("设置置顶状态失败: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn open_ytd_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("ytd-window") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return Ok(());
    }

    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "ytd-window",
        tauri::WebviewUrl::App("index.html?window=ytd".into()),
    )
    .title("VidNest - 视频嗅探与下载中枢")
    .inner_size(980.0, 750.0)
    .min_inner_size(680.0, 520.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("创建独立下载窗口失败: {}", e))?;

    let _ = win.set_focus();
    Ok(())
}

#[tauri::command]
pub async fn open_video_picker_window(
    app: AppHandle,
    initial_url: Option<String>,
) -> Result<(), String> {
    crate::downloader::open_video_picker_window_sync(&app, initial_url)
}

#[tauri::command]
pub async fn close_video_picker_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("video-picker-window") {
        let _ = win.close();
    }
    Ok(())
}

#[tauri::command]
pub async fn find_duplicate_videos(
    strategy: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<DuplicateGroup>, String> {
    let strat = strategy.unwrap_or_else(|| "exact_hash".to_string());
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        let mut groups = crate::dedup::find_duplicates(&conn, &strat)?;
        for grp in &mut groups {
            for v in &mut grp.videos {
                if let Some(ref thumb_path) = v.thumbnail_path {
                    v.thumbnail_data = read_thumbnail_base64(Path::new(thumb_path));
                }
            }
        }
        Ok(groups)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_duplicate_videos(
    actions: Vec<DuplicateResolutionAction>,
    db: State<'_, Database>,
) -> Result<ResolutionSummary, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let mut conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::dedup::resolve_duplicates(&mut conn, actions)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn extract_audio_from_video(
    app: AppHandle,
    db: State<'_, Database>,
    options: AudioExtractionOptions,
) -> Result<AudioExtractionResult, String> {
    let app_data_path = app.path().app_data_dir().ok().unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });

    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let res = crate::media::extract_audio(options.clone(), Some(&app_data_path))?;

        // Automatically record into extracted_audios table
        let conn_arc = db_clone.get_connection();
        if let Ok(conn) = conn_arc.lock() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            // Resolve source video info
            let (source_id, source_title) = if options.video_id.is_some() {
                let id = options.video_id;
                let title: Option<String> = id.and_then(|vid| {
                    conn.query_row(
                        "SELECT file_name FROM videos WHERE id = ?1",
                        [vid],
                        |row| row.get(0),
                    ).ok()
                });
                (id, title)
            } else {
                let queried: Option<(i64, String)> = conn
                    .query_row(
                        "SELECT id, file_name FROM videos WHERE file_path = ?1",
                        [&options.video_path],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .ok();
                if let Some((vid, title)) = queried {
                    (Some(vid), Some(title))
                } else {
                    let fallback_title = Path::new(&options.video_path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string());
                    (None, fallback_title)
                }
            };

            let _ = crate::db::audios::insert_or_update_extracted_audio(
                &conn,
                source_id,
                source_title.as_deref(),
                Some(&options.video_path),
                &res.output_path,
                &res.file_name,
                res.file_size,
                res.duration,
                &res.format,
                options.quality_bitrate.as_deref(),
                now_ms,
            );
        }

        Ok(res)
    })
    .await
    .map_err(|e| format!("执行音频提取任务失败: {}", e))?
}

#[tauri::command]
pub async fn list_extracted_audios(
    filter: Option<ExtractedAudioFilter>,
    db: State<'_, Database>,
) -> Result<Vec<ExtractedAudio>, String> {
    let db_clone = (*db).clone();
    let f = filter.unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::audios::list_extracted_audios(&conn, &f)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_extracted_audio(
    id: i64,
    delete_disk_file: bool,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::audios::delete_extracted_audio(&conn, id, delete_disk_file)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ==========================================
// Video Classification & Organization Commands
// ==========================================

#[tauri::command]
pub async fn get_category_stats(db: State<'_, Database>) -> Result<CategoryStats, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::get_category_stats(&conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn toggle_video_star(video_id: i64, db: State<'_, Database>) -> Result<bool, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::toggle_video_star(&conn, video_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_video_watch_progress(
    video_id: i64,
    status: String,
    progress: f64,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::update_video_watch_progress(&conn, video_id, &status, progress)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_video_color_tag(
    video_id: i64,
    color: Option<String>,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::set_video_color_tag(&conn, video_id, color.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn batch_update_videos_status(
    video_ids: Vec<i64>,
    watch_status: Option<String>,
    is_starred: Option<bool>,
    color_tag: Option<String>,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::batch_update_videos_status(
            &conn,
            &video_ids,
            watch_status.as_deref(),
            is_starred,
            color_tag.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_tags(db: State<'_, Database>) -> Result<Vec<Tag>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::get_all_tags(&conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_tag(
    name: String,
    color: Option<String>,
    db: State<'_, Database>,
) -> Result<Tag, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::create_tag(&conn, &name, color.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_tag(id: i64, db: State<'_, Database>) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::delete_tag(&conn, id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_tag_to_video(
    video_id: i64,
    tag_id: i64,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::add_tag_to_video(&conn, video_id, tag_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_tag_from_video(
    video_id: i64,
    tag_id: i64,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::remove_tag_from_video(&conn, video_id, tag_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn batch_add_tag_to_videos(
    video_ids: Vec<i64>,
    tag_id: i64,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::batch_add_tag_to_videos(&conn, &video_ids, tag_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_collections(db: State<'_, Database>) -> Result<Vec<Collection>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::get_all_collections(&conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_collection(
    name: String,
    description: Option<String>,
    is_smart: Option<bool>,
    rules_json: Option<String>,
    db: State<'_, Database>,
) -> Result<Collection, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::create_collection(
            &conn,
            &name,
            description.as_deref(),
            is_smart.unwrap_or(false),
            rules_json.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_collection(id: i64, db: State<'_, Database>) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::delete_collection(&conn, id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_videos_to_collection(
    collection_id: i64,
    video_ids: Vec<i64>,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::add_videos_to_collection(&conn, collection_id, &video_ids)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_video_from_collection(
    collection_id: i64,
    video_id: i64,
    db: State<'_, Database>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::remove_video_from_collection(&conn, collection_id, video_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_source_folders(db: State<'_, Database>) -> Result<Vec<SourceFolderStat>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::db::classification::get_source_folders(&conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ==========================================
// AI Speech Transcription & Subtitles Commands
// ==========================================

#[tauri::command]
pub async fn get_ai_config(app_handle: AppHandle) -> Result<AiTranscriptionConfig, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    Ok(load_ai_config(&app_data_dir))
}

#[tauri::command]
pub async fn save_ai_config(
    app_handle: AppHandle,
    config: AiTranscriptionConfig,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    persist_ai_config(&app_data_dir, &config)
}

#[tauri::command]
pub async fn open_app_bin_dir(app_handle: AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    let bin_dir = app_data_dir.join("bin");
    let _ = std::fs::create_dir_all(&bin_dir);
    use tauri_plugin_opener::OpenerExt;
    let path_str = bin_dir.to_string_lossy().to_string();
    let _ = app_handle.opener().open_path(&path_str, None::<&str>);
    Ok(path_str)
}

#[tauri::command]
pub async fn open_app_models_dir(app_handle: AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    let models_dir = app_data_dir.join("models");
    let _ = std::fs::create_dir_all(&models_dir);
    use tauri_plugin_opener::OpenerExt;
    let path_str = models_dir.to_string_lossy().to_string();
    let _ = app_handle.opener().open_path(&path_str, None::<&str>);
    Ok(path_str)
}

#[tauri::command]
pub async fn check_local_whisper_status(
    app_handle: AppHandle,
    model_name: Option<String>,
) -> Result<LocalWhisperStatus, String> {
    let app_data_dir = app_handle.path().app_data_dir().ok();
    let resource_dir = app_handle.path().resource_dir().ok();
    Ok(check_whisper_engine_status(
        model_name.as_deref(),
        app_data_dir.as_deref(),
        resource_dir.as_deref(),
    ))
}

#[tauri::command]
pub async fn transcribe_video(
    db: State<'_, Database>,
    app_handle: AppHandle,
    video_id: i64,
    config: Option<AiTranscriptionConfig>,
) -> Result<Vec<SubtitleSegment>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let active_config = match config {
        Some(cfg) => {
            let _ = persist_ai_config(&app_data_dir, &cfg);
            cfg
        }
        None => load_ai_config(&app_data_dir),
    };

    // 1. Fetch video file path
    let db_clone = (*db).clone();
    let video_path = tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        let video = crate::db::videos::get_video_by_id(&conn, video_id)?
            .ok_or_else(|| format!("Video ID {} not found", video_id))?;
        Ok::<PathBuf, String>(PathBuf::from(video.file_path))
    })
    .await
    .map_err(|e| e.to_string())??;

    // 2. Perform speech transcription
    let resource_dir = app_handle.path().resource_dir().ok();
    let segments = transcribe_video_pipeline(
        &video_path,
        video_id,
        &active_config,
        &app_data_dir,
        resource_dir.as_deref(),
    )
    .await?;

    // 3. Save to database & update FTS5
    let db_clone2 = (*db).clone();
    let segments_to_save = segments.clone();
    let lang = active_config.language.clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone2.get_connection();
        let mut conn = conn_arc.lock().map_err(|e| e.to_string())?;
        save_video_subtitles(
            &mut conn,
            video_id,
            Some(&lang),
            Some("vtt"),
            &segments_to_save,
        )?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(segments)
}

#[tauri::command]
pub async fn get_video_subtitles(
    db: State<'_, Database>,
    video_id: i64,
) -> Result<Vec<SubtitleSegment>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        db_get_subtitles(&conn, video_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_video_subtitles(
    db: State<'_, Database>,
    video_id: i64,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let mut conn = conn_arc.lock().map_err(|e| e.to_string())?;
        db_delete_subtitles(&mut conn, video_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_subtitles_file(
    db: State<'_, Database>,
    video_id: i64,
    format: String,
    target_path: String,
) -> Result<String, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        let segments = db_get_subtitles(&conn, video_id)?;
        if segments.is_empty() {
            return Err("该视频暂无字幕内容可导出".to_string());
        }

        let content = if format.to_lowercase() == "srt" {
            export_to_srt(&segments)
        } else {
            export_to_vtt(&segments)
        };

        std::fs::write(&target_path, content)
            .map_err(|e| format!("写入字幕文件失败: {}", e))?;

        Ok(target_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_subtitles_snippets(
    db: State<'_, Database>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SubtitleSnippetMatch>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        db_search_subtitles_snippets(&conn, &query, limit.unwrap_or(20))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn check_video_compatibility(
    app: AppHandle,
    file_path: String,
    file_hash: Option<String>,
) -> Result<crate::media::VideoCompatibilityStatus, String> {
    let app_data_path = app.path().app_data_dir().ok().unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    let p = PathBuf::from(file_path);
    let status = crate::media::check_video_compatibility(&p, file_hash.as_deref(), &app_data_path);
    Ok(status)
}

#[tauri::command]
pub async fn convert_video_to_compatible(
    app: AppHandle,
    file_path: String,
    file_hash: Option<String>,
) -> Result<crate::media::TranscodeResult, String> {
    let app_data_path = app.path().app_data_dir().ok().unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });
    let p = PathBuf::from(file_path);
    tokio::task::spawn_blocking(move || {
        crate::media::convert_video_to_compatible(&p, file_hash.as_deref(), &app_data_path)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ==========================================
// AI Video Intelligence & Analysis Commands
// ==========================================

#[tauri::command]
pub async fn get_video_ai_analysis(
    db: State<'_, Database>,
    video_id: i64,
) -> Result<Option<crate::ai::summary::AiAnalysis>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::ai::summary::get_video_ai_analysis(&conn, video_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn generate_video_ai_analysis(
    db: State<'_, Database>,
    app_handle: AppHandle,
    video_id: i64,
    force: Option<bool>,
) -> Result<crate::ai::summary::AiAnalysis, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    let config = load_ai_config(&app_data_dir);

    let force_regen = force.unwrap_or(false);

    // 1. Check existing analysis if not forced
    let db_clone = (*db).clone();
    if !force_regen {
        let existing = tokio::task::spawn_blocking(move || {
            let conn_arc = db_clone.get_connection();
            let conn = conn_arc.lock().map_err(|e| e.to_string())?;
            crate::ai::summary::get_video_ai_analysis(&conn, video_id)
        })
        .await
        .map_err(|e| e.to_string())??;

        if let Some(analysis) = existing {
            return Ok(analysis);
        }
    }

    // 2. Load subtitles, video title and duration
    let db_clone2 = (*db).clone();
    let (video_title, video_duration_ms, segments) = tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone2.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        let video = crate::db::videos::get_video_by_id(&conn, video_id)?
            .ok_or_else(|| format!("Video ID {} not found", video_id))?;
        let segs = crate::db::subtitles::get_video_subtitles(&conn, video_id)?;
        let dur_ms = video.duration.map(|d| (d * 1000.0) as i64);
        Ok::<(String, Option<i64>, Vec<crate::db::subtitles::SubtitleSegment>), String>((video.file_name, dur_ms, segs))
    })
    .await
    .map_err(|e| e.to_string())??;

    // 3. Generate analysis pipeline (works smoothly with or without subtitles)
    let mut analysis = crate::ai::summary::generate_ai_analysis_pipeline(
        video_id,
        &video_title,
        video_duration_ms,
        &segments,
        &config,
    )
    .await?;

    // 4. Save to database
    let db_clone3 = (*db).clone();
    let analysis_to_save = analysis.clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone3.get_connection();
        let mut conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::ai::summary::save_video_ai_analysis(&mut conn, &analysis_to_save)
    })
    .await
    .map_err(|e| e.to_string())??;

    // Refresh id from db if needed
    let db_clone4 = (*db).clone();
    if let Ok(Ok(Some(saved))) = tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone4.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::ai::summary::get_video_ai_analysis(&conn, video_id)
    })
    .await
    {
        analysis = saved;
    }

    Ok(analysis)
}

#[tauri::command]
pub async fn get_videos_ai_status(
    db: State<'_, Database>,
    video_ids: Vec<i64>,
) -> Result<std::collections::HashMap<i64, crate::ai::summary::VideoAiStatus>, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        crate::ai::summary::get_videos_ai_status(&conn, &video_ids)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_ai_tags_to_video(
    db: State<'_, Database>,
    video_id: i64,
    tags: Vec<String>,
) -> Result<(), String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
        for t in tags {
            let t_clean = t.trim().trim_start_matches('#');
            if t_clean.is_empty() {
                continue;
            }
            let tag_id: i64 = match conn.query_row(
                "SELECT id FROM tags WHERE name = ?1",
                rusqlite::params![t_clean],
                |row| row.get(0),
            ) {
                Ok(id) => id,
                Err(_) => {
                    let created = crate::db::classification::create_tag(&conn, t_clean, None)?;
                    created.id
                }
            };
            let _ = crate::db::classification::add_tag_to_video(&conn, video_id, tag_id);
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn clip_video_segments(
    app_handle: AppHandle,
    request: ClipVideoRequest,
) -> Result<ClipVideoResponse, String> {
    let app_data_dir = app_handle.path().app_data_dir().ok();
    tokio::task::spawn_blocking(move || {
        crate::media::clip_video_segments(request, app_data_dir.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn recommend_video_segments(
    db: State<'_, Database>,
    app_handle: AppHandle,
    video_id: Option<i64>,
    video_path: Option<String>,
) -> Result<RecommendSegmentsResponse, String> {
    let app_data_dir = app_handle.path().app_data_dir().ok();
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn_guard = conn_arc.lock().ok();
        let conn_ref = conn_guard.as_deref();

        let resolved_path = if let Some(vp) = video_path {
            PathBuf::from(vp)
        } else if let (Some(v_id), Some(conn)) = (video_id, conn_ref) {
            if let Ok(Some(video)) = crate::db::videos::get_video_by_id(conn, v_id) {
                PathBuf::from(video.file_path)
            } else {
                PathBuf::new()
            }
        } else {
            PathBuf::new()
        };

        let duration_ms = if let (Some(v_id), Some(conn)) = (video_id, conn_ref) {
            if let Ok(Some(video)) = crate::db::videos::get_video_by_id(conn, v_id) {
                video.duration.map(|d| (d * 1000.0) as i64)
            } else {
                None
            }
        } else {
            None
        };

        crate::media::recommend_video_segments(
            video_id,
            &resolved_path,
            duration_ms,
            conn_ref,
            app_data_dir.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}


