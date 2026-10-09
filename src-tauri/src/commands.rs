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
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn system_health_check(app: AppHandle) -> Result<HealthReport, String> {
    let app_data_path = app.path().app_data_dir().ok();
    let report = run_health_check(app_data_path);
    Ok(report)
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
pub fn list_videos(
    filter: Option<VideoFilter>,
    db: State<'_, Database>,
) -> Result<VideoListResponse, String> {
    let conn_arc = db.get_connection();
    let conn = conn_arc.lock().map_err(|e| e.to_string())?;

    let mut response = crate::db::videos::list_videos(&conn, filter.unwrap_or_default())?;

    // Populate thumbnail base64 data for the page items
    for item in &mut response.items {
        if let Some(thumb_path) = &item.thumbnail_path {
            item.thumbnail_data = read_thumbnail_base64(Path::new(thumb_path));
        }
    }

    Ok(response)
}

#[tauri::command]
pub fn get_video_detail(
    id: i64,
    db: State<'_, Database>,
) -> Result<Option<Video>, String> {
    let conn_arc = db.get_connection();
    let conn = conn_arc.lock().map_err(|e| e.to_string())?;

    let mut video = crate::db::videos::get_video_by_id(&conn, id)?;
    if let Some(ref mut v) = video {
        if let Some(thumb_path) = &v.thumbnail_path {
            v.thumbnail_data = read_thumbnail_base64(Path::new(thumb_path));
        }
    }

    Ok(video)
}

#[tauri::command]
pub fn delete_video(
    id: i64,
    delete_file: bool,
    db: State<'_, Database>,
) -> Result<bool, String> {
    let conn_arc = db.get_connection();
    let conn = conn_arc.lock().map_err(|e| e.to_string())?;

    crate::db::videos::delete_video(&conn, id, delete_file)
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
