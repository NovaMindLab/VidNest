pub mod commands;
pub mod db;
pub mod downloader;
pub mod health;
pub mod media;
pub mod models;
pub mod scanner;
pub mod updater;
pub mod ytdlp;

use commands::{
    cancel_download, cancel_download_task, cancel_full_disk_scan, check_app_update, check_ytdlp_update,
    clear_completed_tasks, delete_video, download_and_install_update, echo, get_download_tasks,
    get_player_init_video, get_video_detail, inspect_url, inspect_video_url, list_videos,
    open_video_player_window, pause_download, pause_download_task, ping, relaunch_app, resume_download,
    resume_download_task, scan_video_directory, set_player_always_on_top, start_download,
    start_full_disk_scan, system_health_check, update_ytdlp, PlayerState,
};
use db::Database;
use std::path::PathBuf;
use tauri::Manager;

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
        .setup(|app| {
            if let Ok(app_data) = app.path().app_data_dir() {
                let bin_dir = app_data.join("bin");
                let _ = std::fs::create_dir_all(&bin_dir);
                let bin_str = bin_dir.to_string_lossy().to_string();
                let current_path = std::env::var("PATH").unwrap_or_default();
                let mut paths = vec![bin_str];
                #[cfg(unix)]
                {
                    for p in &["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin"] {
                        if std::path::Path::new(p).exists() {
                            paths.push(p.to_string());
                        }
                    }
                }
                paths.push(current_path);
                std::env::set_var("PATH", paths.join(":"));
            }
            Ok(())
        })
        .manage(db)
        .manage(download_manager)
        .manage(PlayerState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            system_health_check,
            check_ytdlp_update,
            update_ytdlp,
            scan_video_directory,
            start_full_disk_scan,
            cancel_full_disk_scan,
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
            check_app_update,
            download_and_install_update,
            relaunch_app,
            open_video_player_window,
            get_player_init_video,
            set_player_always_on_top,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

