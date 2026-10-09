pub mod commands;
pub mod db;
pub mod downloader;
pub mod health;
pub mod media;
pub mod models;
pub mod scanner;
pub mod ytdlp;

use commands::{
    cancel_download, cancel_download_task, check_ytdlp_update, clear_completed_tasks, delete_video, echo,
    get_download_tasks, get_video_detail, inspect_url, inspect_video_url, list_videos, pause_download,
    pause_download_task, ping, resume_download, resume_download_task, scan_video_directory, start_download,
    system_health_check, update_ytdlp,
};
use db::Database;
use std::path::PathBuf;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_data_dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LocalVideoIntelligence");

    let db = Database::init(&app_data_dir)
        .expect("Failed to initialize SQLite database with migrations");

    let download_manager = downloader::DownloadManager::global();

    tauri::Builder::default()
        .manage(db)
        .manage(download_manager)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            system_health_check,
            check_ytdlp_update,
            update_ytdlp,
            scan_video_directory,
            list_videos,
            get_video_detail,
            delete_video,
            inspect_video_url,
            inspect_url,
            start_download,
            pause_download,
            pause_download_task,
            resume_download,
            resume_download_task,
            cancel_download,
            cancel_download_task,
            get_download_tasks,
            clear_completed_tasks,
            ping,
            echo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

