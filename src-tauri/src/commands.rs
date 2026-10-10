use crate::db::Database;
use crate::health::run_health_check;
use crate::media::read_thumbnail_base64;
use crate::models::{
    DownloadTask, HealthReport, ScanSummary, UrlInspectResult, Video, VideoFilter,
    VideoListResponse, YtDlpUpdateCheck, YtDlpUpdateResult,
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
pub async fn delete_video(
    id: i64,
    delete_file: bool,
    db: State<'_, Database>,
) -> Result<bool, String> {
    let db_clone = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_clone.get_connection();
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;

        crate::db::videos::delete_video(&conn, id, delete_file)
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
        .start_download(url, format_id, app_data_path, (*db).clone())
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
        std::process::Command::new(&path)
            .spawn()
            .map_err(|e| format!("启动安装程序失败: {}", e))?;
        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开安装包失败: {}", e))?;
        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&path, perms);
            }
        }
        std::process::Command::new(&path)
            .spawn()
            .map_err(|e| format!("启动新版本失败: {}", e))?;
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


