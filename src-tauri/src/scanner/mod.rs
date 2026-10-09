use crate::db::Database;
use crate::db::videos::insert_or_update_video;
use crate::media::{extract_metadata, generate_thumbnail};
use crate::models::ScanSummary;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "flv", "wmv", "webm", "m4v", "ts", "rmvb", "m2ts",
];

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

/// Checks whether a given path has a supported video extension.
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            SUPPORTED_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// Scans a directory recursively, extracts metadata, generates thumbnails, and updates SQLite.
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
            !file_name.starts_with('.') && file_name != "$RECYCLE.BIN" && file_name != "System Volume Information"
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
