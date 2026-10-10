use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

pub const DEFAULT_MIN_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

pub const BLACKLIST_DIR_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    ".Trash",
    "$RECYCLE.BIN",
    "System Volume Information",
    "Library",
    "Applications",
    "System",
    "AppData",
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    ".cargo",
    ".rustup",
    ".gradle",
    ".vscode",
    ".idea",
    "Pods",
    "DerivedData",
    ".npm",
    ".yarn",
    ".pnpm-store",
    "Caches",
    ".cache",
    "dist",
    "build",
    "target",
    "out",
    ".next",
    ".nuxt",
    ".turbo",
    "coverage",
    "typings",
    "@types",
    "vendor",
    "packages",
];

/// Checks if a folder name matches common system/cache/dependency blacklists.
pub fn is_blacklisted_dir(dir_name: &str) -> bool {
    let lower = dir_name.to_lowercase();
    for item in BLACKLIST_DIR_NAMES {
        if lower == item.to_lowercase() {
            return true;
        }
    }
    dir_name.starts_with('.')
}

/// Checks whether a `.ts` file is actually an MPEG-TS video file
/// instead of a TypeScript source code file.
pub fn is_mpeg_ts_video(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|f| f.to_str()) {
        Some(name) => name.to_lowercase(),
        None => return false,
    };

    // 1. Filename heuristic: TypeScript declaration, test, or config files
    if file_name.ends_with(".d.ts")
        || file_name.ends_with(".spec.ts")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".config.ts")
        || file_name.ends_with(".routes.ts")
        || file_name.ends_with(".controller.ts")
        || file_name.ends_with(".service.ts")
    {
        return false;
    }

    // 2. Size heuristic: real video streams are almost never below 64 KB
    let metadata = match path.metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let file_len = metadata.len();
    if file_len < 64 * 1024 {
        return false;
    }

    // 3. Content heuristic: check binary vs text
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut sample_buf = [0u8; 1024];
    let n = match file.read(&mut sample_buf) {
        Ok(bytes) if bytes >= 188 => bytes,
        _ => return false,
    };

    // Quick text check: if the head looks like source code (contains text keywords or mostly printable ASCII)
    if let Ok(sample_str) = std::str::from_utf8(&sample_buf[..n]) {
        let code_keywords = [
            "import ", "export ", "function ", "interface ", "class ",
            "const ", "let ", "var ", "type ", "declare ", "return ",
            "//", "/*", "<script", "<template",
        ];
        for kw in &code_keywords {
            if sample_str.contains(kw) {
                return false;
            }
        }
    }

    // 4. Strict MPEG-TS packet sync byte check
    // In MPEG-TS, packets are 188 bytes long (or 204 with FEC).
    // The first byte of each packet must be 0x47 ('G').
    // We verify at least 3 consecutive packet sync bytes.
    let packet_sizes = [188usize, 204usize];
    let mut valid_ts = false;

    for &pkt_size in &packet_sizes {
        if n >= pkt_size * 3 {
            let mut all_sync = true;
            let mut count = 0;
            for offset in (0..n).step_by(pkt_size) {
                if offset < n {
                    count += 1;
                    if sample_buf[offset] != 0x47 {
                        all_sync = false;
                        break;
                    }
                }
            }
            if all_sync && count >= 3 {
                valid_ts = true;
                break;
            }
        }
    }

    valid_ts
}

/// Filters candidate video path for valid extension, minimum size, and false-positive checks.
pub fn validate_video_file(path: &Path, min_size_bytes: u64) -> bool {
    if !path.is_file() {
        return false;
    }

    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };

    if !super::SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return false;
    }

    // Check file size
    if let Ok(meta) = path.metadata() {
        if meta.len() < min_size_bytes {
            return false;
        }
    } else {
        return false;
    }

    // Disambiguate .ts extension (TypeScript vs MPEG-TS)
    if ext == "ts" && !is_mpeg_ts_video(path) {
        return false;
    }

    // Skip paths that contain blacklisted components
    for comp in path.components() {
        if let Some(comp_str) = comp.as_os_str().to_str() {
            if is_blacklisted_dir(comp_str) {
                return false;
            }
        }
    }

    true
}

/// Discovers video files using macOS Spotlight (`mdfind`) for sub-second zero-I/O discovery.
#[cfg(target_os = "macos")]
pub fn discover_videos_macos(
    min_size_bytes: u64,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<Vec<PathBuf>, String> {
    let query = format!(
        "kMDItemContentTypeTree == 'public.movie' && kMDItemFSSize >= {}",
        min_size_bytes
    );

    let output = Command::new("mdfind")
        .arg(&query)
        .output()
        .map_err(|e| format!("Failed to execute mdfind: {}", e))?;

    if !output.status.success() {
        return Err("mdfind query failed".to_string());
    }

    let out_str = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in out_str.lines() {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let p = PathBuf::from(trimmed);
        if validate_video_file(&p, min_size_bytes) {
            results.push(p);
        }
    }

    Ok(results)
}

/// Discovers video files on Windows:
/// Enumerates logical drives and walks user profiles & data drives with aggressive pruning.
#[cfg(target_os = "windows")]
pub fn discover_videos_windows(
    min_size_bytes: u64,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<Vec<PathBuf>, String> {
    let mut results = Vec::new();
    let mut roots = Vec::new();

    // 1. Prioritize user profile media directories
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(user_profile);
        for sub in &["Videos", "Downloads", "Documents", "Desktop"] {
            let user_dir = p.join(sub);
            if user_dir.exists() {
                roots.push(user_dir);
            }
        }
    }

    // 2. Discover available drive letters (C:\, D:\, etc.)
    for letter in b'C'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = PathBuf::from(&drive);
        if path.exists() {
            // For C drive, user dirs are already prioritized; for secondary data drives, scan root
            if letter != b'C' {
                roots.push(path);
            }
        }
    }

    for root in roots {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }
        walk_and_collect(&root, min_size_bytes, cancel_flag, &mut results);
    }

    Ok(results)
}

/// Fallback / Generic discovery for all platforms:
/// Smartly discovers videos by scanning user home and mounted external drives with pruning.
pub fn discover_videos_generic(
    min_size_bytes: u64,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<Vec<PathBuf>, String> {
    let mut results = Vec::new();
    let mut roots = Vec::new();

    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let home_path = PathBuf::from(home);
        for sub in &["Movies", "Videos", "Downloads", "Documents", "Desktop"] {
            let target = home_path.join(sub);
            if target.exists() {
                roots.push(target);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Scan external mounted volumes under /Volumes
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let p = entry.path();
                // Exclude root Macintosh HD symlink
                if p != Path::new("/Volumes/Macintosh HD") && p.is_dir() {
                    roots.push(p);
                }
            }
        }
    }

    for root in roots {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }
        walk_and_collect(&root, min_size_bytes, cancel_flag, &mut results);
    }

    Ok(results)
}

/// Walks a directory with aggressive directory-level pruning to keep I/O and CPU minimal.
fn walk_and_collect(
    root: &Path,
    min_size_bytes: u64,
    cancel_flag: &Arc<AtomicBool>,
    results: &mut Vec<PathBuf>,
) {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !is_blacklisted_dir(&name)
        });

    for entry in walker {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            let path = entry.path();
            if validate_video_file(path, min_size_bytes) {
                results.push(path.to_path_buf());
            }
        }
    }
}

/// High-level platform dispatcher for full-disk video discovery.
pub fn discover_all_videos(
    min_size_bytes: u64,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<Vec<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        // Try Spotlight mdfind first (sub-second zero I/O)
        match discover_videos_macos(min_size_bytes, cancel_flag) {
            Ok(v) if !v.is_empty() => return Ok(v),
            _ => {
                // If Spotlight is disabled or yielded 0 items, fallback to generic
                return discover_videos_generic(min_size_bytes, cancel_flag);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        return discover_videos_windows(min_size_bytes, cancel_flag);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return discover_videos_generic(min_size_bytes, cancel_flag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blacklisted_dir() {
        assert!(is_blacklisted_dir("node_modules"));
        assert!(is_blacklisted_dir(".git"));
        assert!(is_blacklisted_dir("AppData"));
        assert!(is_blacklisted_dir("Library"));
        assert!(!is_blacklisted_dir("Movies"));
        assert!(!is_blacklisted_dir("MyVacationVideos"));
    }

    #[test]
    fn test_is_mpeg_ts_video_on_source_code() {
        // A TypeScript source file shouldn't be detected as an MPEG-TS video
        let ts_file = Path::new("../src/main.ts");
        if ts_file.exists() {
            assert!(!is_mpeg_ts_video(ts_file));
        }
    }

    #[test]
    fn test_discover_videos_cancel() {
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let results = discover_all_videos(1024 * 1024 * 1024, &cancel_flag);
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }
}
