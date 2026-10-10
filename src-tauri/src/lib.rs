pub mod commands;
pub mod db;
pub mod dedup;
pub mod downloader;
pub mod health;
pub mod media;
pub mod models;
pub mod scanner;
pub mod updater;
pub mod ytdlp;

use commands::{
    cancel_download, cancel_download_task, cancel_full_disk_scan, check_app_update, check_ytdlp_update,
    clear_completed_tasks, delete_extracted_audio, delete_video, download_and_install_update, echo,
    extract_audio_from_video, find_duplicate_videos, get_download_tasks, get_player_init_video,
    get_video_detail, inspect_url, inspect_video_url, install_update_and_relaunch, list_extracted_audios,
    list_videos, open_update_installer_folder, open_video_player_window, open_ytd_window, pause_download,
    pause_download_task, ping, relaunch_app, resolve_duplicate_videos, resume_download, resume_download_task,
    reveal_in_explorer, scan_video_directory, set_player_always_on_top, start_download,
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
    let db_for_purge = db.clone();

    tauri::Builder::default()
        .setup(move |app| {
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

            // Quiet background purge after 2.5s delay to keep startup ultra-fast
            let db_bg = db_for_purge.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
                let _ = tokio::task::spawn_blocking(move || {
                    let conn_arc = db_bg.get_connection();
                    if let Ok(conn) = conn_arc.lock() {
                        let _ = crate::db::videos::purge_invalid_ts_records(&conn);
                    };
                }).await;
            });

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
            install_update_and_relaunch,
            open_update_installer_folder,
            relaunch_app,
            open_video_player_window,
            get_player_init_video,
            set_player_always_on_top,
            open_ytd_window,
            reveal_in_explorer,
            find_duplicate_videos,
            resolve_duplicate_videos,
            extract_audio_from_video,
            list_extracted_audios,
            delete_extracted_audio,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

