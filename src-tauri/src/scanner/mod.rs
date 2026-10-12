pub mod platform;

use crate::db::Database;
use crate::db::videos::{insert_or_update_video, update_video_metadata};
use crate::media::{extract_metadata, generate_thumbnail};
use crate::models::{ScanProgress, ScanSummary};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use walkdir::WalkDir;

pub use platform::{discover_all_videos, DEFAULT_MIN_SIZE_BYTES};

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Most common
    "mp4", "mkv", "avi", "mov", "flv", "wmv", "webm", "m4v",
    // Transport stream variants
    "ts", "m2ts", "mts",
    // Legacy / multimedia
    "rmvb", "rm", "divx", "xvid",
    // Disc / professional formats
    "vob", "ifo", "mxf",
    // Mobile / 3G formats
    "3gp", "3g2",
    // Other
    "ogv", "f4v", "asf", "mpg", "mpeg", "mp2", "m2v", "dv", "qt",
];

/// Global atomic cancellation flag for current scan operation.
pub fn get_scan_cancel_flag() -> Arc<AtomicBool> {
    static FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false))).clone()
}

/// Cancels the currently active scan operation.
pub fn cancel_scan() -> bool {
    let flag = get_scan_cancel_flag();
    flag.store(true, Ordering::Relaxed);
    true
}

/// Computes a fast deterministic SHA-256 hash.
/// Samples head and tail for large video files to avoid disk I/O bottlenecks.
pub fn compute_fast_file_hash(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open file for hashing {:?}: {}", path, e))?;

    let file_len = file
        .metadata()
        .map_err(|e| format!("Failed reading file metadata: {}", e))?
        .len();

    let mut hasher = Sha256::new();
    let sample_threshold = 8 * 1024 * 1024; // 8 MB

    if file_len <= sample_threshold {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed reading file bytes: {}", e))?;
        hasher.update(&buffer);
    } else {
        let sample_size = 2 * 1024 * 1024; // 2 MB
        let mut head_buf = vec![0u8; sample_size];
        file.read_exact(&mut head_buf)
            .map_err(|e| format!("Failed reading head sample: {}", e))?;
        hasher.update(&head_buf);

        file.seek(SeekFrom::End(-(sample_size as i64)))
            .map_err(|e| format!("Failed seeking tail: {}", e))?;
        let mut tail_buf = vec![0u8; sample_size];
        file.read_exact(&mut tail_buf)
            .map_err(|e| format!("Failed reading tail sample: {}", e))?;
        hasher.update(&tail_buf);

        hasher.update(&file_len.to_le_bytes());
    }

    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

/// Checks whether a given path is a valid supported video file.
/// For `.ts` files, rigorously disambiguates against TypeScript source code.
pub fn is_video_file(path: &Path) -> bool {
    let ext = match path.extension().and_then(|ext| ext.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };

    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return false;
    }

    // Crucial: Disambiguate .ts extension (TypeScript vs MPEG-TS)
    if ext == "ts" {
        return platform::is_mpeg_ts_video(path);
    }

    true
}

/// Two-Phase ultra-low-overhead full-disk scan.
/// Phase 1: High-speed discovery + batch SQLite indexing (sub-second / few seconds to display UI).
/// Phase 2: Detached low-priority background worker for thumbnails & ffprobe.
pub fn scan_full_disk(
    app: &tauri::AppHandle,
    db: &Database,
    app_data_dir: &Path,
    min_size_bytes: u64,
) -> Result<ScanSummary, String> {
    let cancel_flag = get_scan_cancel_flag();
    cancel_flag.store(false, Ordering::Relaxed);
    let start_time = SystemTime::now();

    let _ = app.emit("scan:progress", ScanProgress {
        phase: "discovering".to_string(),
        message: "正在极速发现全盘视频...".to_string(),
        total_found: 0,
        processed: 0,
        current_file: None,
    });

    // Phase 1: High speed discovery
    let candidate_paths = platform::discover_all_videos(min_size_bytes, &cancel_flag)?;
    let total_found = candidate_paths.len();

    let _ = app.emit("scan:progress", ScanProgress {
        phase: "indexing".to_string(),
        message: format!("发现 {} 个视频文件，正在快速索引与入库...", total_found),
        total_found,
        processed: 0,
        current_file: None,
    });

    let conn_arc = db.get_connection();
    let conn = conn_arc.lock().map_err(|e| e.to_string())?;

    let mut new_imported = 0;
    let mut updated = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut processed = 0;

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for chunk in candidate_paths.chunks(100) {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        for file_path in chunk {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            processed += 1;
            let path_str = file_path.to_string_lossy().to_string();
            let file_name = file_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let metadata = match file_path.metadata() {
                Ok(m) => m,
                Err(_) => {
                    failed += 1;
                    continue;
                }
            };
            let file_size = metadata.len();

            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or_default();

            // Check existing record
            let existing_record: Option<(i64, i64, i64, Option<String>)> = conn
                .query_row(
                    "SELECT id, file_size, updated_at, thumbnail_path FROM videos WHERE file_path = ?1",
                    [&path_str],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .ok();

            if let Some((_, db_size, db_updated_at, _)) = &existing_record {
                if *db_size == file_size as i64 && modified_ms <= *db_updated_at {
                    skipped += 1;
                    continue;
                }
            }

            // Fast hash (2MB head + 2MB tail)
            let file_hash = match compute_fast_file_hash(file_path) {
                Ok(h) => h,
                Err(_) => {
                    failed += 1;
                    continue;
                }
            };

            let thumb_path = existing_record.as_ref().and_then(|r| r.3.as_deref());

            let insert_res = insert_or_update_video(
                &conn,
                &path_str,
                &file_name,
                &file_hash,
                file_size,
                None, // Duration enriched lazily in Phase 2
                None,
                None,
                None,
                thumb_path,
                now_ms,
            );

            match insert_res {
                Ok(_) => {
                    if existing_record.is_some() {
                        updated += 1;
                    } else {
                        new_imported += 1;
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        let _ = app.emit("scan:progress", ScanProgress {
            phase: "indexing".to_string(),
            message: format!("正在索引入库 ({}/{})", processed, total_found),
            total_found,
            processed,
            current_file: chunk.last().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()),
        });
    }

    let is_cancelled = cancel_flag.load(Ordering::Relaxed);
    let duration_ms = start_time.elapsed().unwrap_or_default().as_millis() as u64;

    let _ = app.emit("scan:progress", ScanProgress {
        phase: if is_cancelled { "cancelled".to_string() } else { "completed".to_string() },
        message: if is_cancelled {
            "全盘扫描已手动中止".to_string()
        } else {
            format!("全盘扫描完成！已同步 {} 个视频", new_imported + updated + skipped)
        },
        total_found,
        processed,
        current_file: None,
    });

    if !is_cancelled {
        spawn_background_enricher(app.clone(), db.clone(), app_data_dir.to_path_buf());
    }

    Ok(ScanSummary {
        scanned_dir: "全盘极速扫描 (Full Disk)".to_string(),
        total_files_examined: total_found,
        video_files_found: total_found,
        new_imported,
        updated,
        skipped,
        failed,
        duration_ms,
    })
}

/// Detached low-priority worker thread to enrich video metadata and generate thumbnails
/// in the background without causing CPU spikes or UI stutters.
pub fn spawn_background_enricher(
    app: tauri::AppHandle,
    db: Database,
    app_data_dir: PathBuf,
) {
    std::thread::spawn(move || {
        let cancel_flag = get_scan_cancel_flag();
        let pending: Vec<(i64, String, String)> = {
            let conn_arc = db.get_connection();
            let conn = match conn_arc.lock() {
                Ok(c) => c,
                Err(_) => return,
            };
            let mut stmt = match conn.prepare(
                "SELECT id, file_path, file_hash FROM videos WHERE duration IS NULL OR thumbnail_path IS NULL ORDER BY id DESC LIMIT 500"
            ) {
                Ok(s) => s,
                Err(_) => return,
            };
            let rows = match stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            }) {
                Ok(r) => r,
                Err(_) => return,
            };
            rows.flatten().collect()
        };

        for (id, file_path_str, file_hash) in pending {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            let file_path = Path::new(&file_path_str);
            if !file_path.exists() {
                continue;
            }

            // Extract metadata with ffprobe
            let media_meta = extract_metadata(file_path).unwrap_or_default();

            // Generate thumbnail with ffmpeg
            let thumb_path = generate_thumbnail(
                file_path,
                &file_hash,
                media_meta.duration,
                &app_data_dir,
            )
            .ok()
            .map(|p| p.to_string_lossy().to_string());

            // Update record in database
            {
                if let Ok(conn) = db.get_connection().lock() {
                    let _ = update_video_metadata(
                        &conn,
                        id,
                        media_meta.duration,
                        media_meta.width,
                        media_meta.height,
                        media_meta.codec.as_deref(),
                        thumb_path.as_deref(),
                    );
                }
            }

            let _ = app.emit("video:enriched", id);

            // Throttle with 50ms sleep between items for minimal CPU/IO footprint
            std::thread::sleep(Duration::from_millis(50));
        }
    });
}

/// Scans a specific directory recursively (single folder mode).
pub fn scan_directory(
    dir_path: &Path,
    db: &Database,
    app_data_dir: &Path,
) -> Result<ScanSummary, String> {
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Directory does not exist or is not a folder: {:?}", dir_path));
    }

    let start_time = SystemTime::now();
    let conn_arc = db.get_connection();
    let conn = conn_arc.lock().map_err(|e| e.to_string())?;

    let mut total_files_examined = 0;
    let mut video_files_found = 0;
    let mut new_imported = 0;
    let mut updated = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for entry in WalkDir::new(dir_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let file_name = e.file_name().to_string_lossy();
            !platform::is_blacklisted_dir(&file_name)
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        total_files_examined += 1;
        let file_path = entry.path();

        if !is_video_file(file_path) {
            continue;
        }

        video_files_found += 1;
        let path_str = file_path.to_string_lossy().to_string();
        let file_name = file_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let metadata = match file_path.metadata() {
            Ok(m) => m,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        let file_size = metadata.len();

        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default();

        // Check if file_path already in database
        let existing_record: Option<(i64, i64, i64)> = conn
            .query_row(
                "SELECT id, file_size, updated_at FROM videos WHERE file_path = ?1",
                [&path_str],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        if let Some((_, db_size, db_updated_at)) = existing_record {
            if db_size == file_size as i64 && modified_ms <= db_updated_at {
                skipped += 1;
                continue;
            }
        }

        // File is new or modified: compute hash
        let file_hash = match compute_fast_file_hash(file_path) {
            Ok(h) => h,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        // Extract media metadata via ffprobe
        let media_meta = extract_metadata(file_path).unwrap_or_default();

        // Generate thumbnail via ffmpeg
        let thumb_path = generate_thumbnail(
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

        let insert_res = insert_or_update_video(
            &conn,
            &path_str,
            &file_name,
            &file_hash,
            file_size,
            media_meta.duration,
            media_meta.width,
            media_meta.height,
            media_meta.codec.as_deref(),
            thumb_path.as_deref(),
            now_ms,
        );

        match insert_res {
            Ok(_) => {
                if existing_record.is_some() {
                    updated += 1;
                } else {
                    new_imported += 1;
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let duration_ms = start_time
        .elapsed()
        .unwrap_or_default()
        .as_millis() as u64;

    Ok(ScanSummary {
        scanned_dir: dir_path.to_string_lossy().to_string(),
        total_files_examined,
        video_files_found,
        new_imported,
        updated,
        skipped,
        failed,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("movie.mp4")));
        assert!(is_video_file(Path::new("video.MKV")));
        assert!(is_video_file(Path::new("clip.webm")));
        assert!(!is_video_file(Path::new("image.jpg")));
        assert!(!is_video_file(Path::new("doc.pdf")));
    }

    #[test]
    fn test_compute_fast_file_hash() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("test_video_hash_{}.mp4", std::process::id()));
        std::fs::write(&test_file, b"test video sample data content for hashing").expect("Write test file");

        let hash1 = compute_fast_file_hash(&test_file).expect("Hash 1");
        let hash2 = compute_fast_file_hash(&test_file).expect("Hash 2");
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());

        let _ = std::fs::remove_file(test_file);
    }
}
