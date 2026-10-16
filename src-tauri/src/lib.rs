pub mod ai;
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
    add_tag_to_video, add_videos_to_collection, batch_add_tag_to_videos, batch_update_videos_status,
    cancel_download, cancel_download_task, cancel_full_disk_scan, check_app_update, check_ytdlp_update,
    check_video_compatibility, convert_video_to_compatible,
    clear_completed_tasks, close_video_picker_window, create_collection, create_tag, delete_collection,
    delete_extracted_audio, delete_tag, delete_video, delete_video_subtitles,
    download_and_install_update, echo, export_subtitles_file, extract_audio_from_video,
    find_duplicate_videos, get_ai_config, get_category_stats, get_collections,
    get_download_tasks, get_player_init_video, get_source_folders, get_tags, get_video_detail,
    get_video_subtitles, inspect_url, inspect_video_url, install_update_and_relaunch,
    list_extracted_audios, list_videos, open_update_installer_folder, open_video_picker_window,
    open_video_player_window, open_ytd_window, pause_download, pause_download_task, ping, relaunch_app,
    remove_tag_from_video, remove_video_from_collection, resolve_duplicate_videos, resume_download,
    resume_download_task, reveal_in_explorer, save_ai_config, scan_video_directory,
    search_subtitles_snippets, set_player_always_on_top, set_video_color_tag, start_download,
    start_full_disk_scan, system_health_check, toggle_video_star, transcribe_video,
    update_video_watch_progress, update_ytdlp, PlayerState,
    apply_ai_tags_to_video, generate_video_ai_analysis, get_video_ai_analysis, get_videos_ai_status,
    open_app_bin_dir, open_app_models_dir,
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
            open_video_picker_window,
            close_video_picker_window,
            reveal_in_explorer,
            find_duplicate_videos,
            resolve_duplicate_videos,
            extract_audio_from_video,
            list_extracted_audios,
            delete_extracted_audio,
            get_category_stats,
            toggle_video_star,
            update_video_watch_progress,
            set_video_color_tag,
            batch_update_videos_status,
            get_tags,
            create_tag,
            delete_tag,
            add_tag_to_video,
            remove_tag_from_video,
            batch_add_tag_to_videos,
            get_collections,
            create_collection,
            delete_collection,
            add_videos_to_collection,
            remove_video_from_collection,
            get_source_folders,
            get_ai_config,
            save_ai_config,
            open_app_bin_dir,
            open_app_models_dir,
            transcribe_video,
            get_video_subtitles,
            delete_video_subtitles,
            export_subtitles_file,
            search_subtitles_snippets,
            check_video_compatibility,
            convert_video_to_compatible,
            get_video_ai_analysis,
            generate_video_ai_analysis,
            get_videos_ai_status,
            apply_ai_tags_to_video,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

