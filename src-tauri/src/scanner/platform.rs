use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

pub const DEFAULT_MIN_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

/// Directory names that should NEVER contain user media — system internals, package managers, IDEs.
/// Keep this list conservative: only skip directories that are CERTAIN to be non-user-media.
/// Broad terms like "dist", "build", "out", "target" must NOT be here — users routinely name
/// their video folders with those words.
pub const BLACKLIST_DIR_NAMES: &[&str] = &[
    // OS Trash / Recycle
    ".Trash",
    "$RECYCLE.BIN",
    "System Volume Information",
    // macOS OS directories
    "Applications",
    "Library",           // macOS /Library, not user-facing
    "System",
    // Windows OS directories
    "Windows",
    "WinSxS",
    // Windows App data (hidden developer paths)
    "AppData",
    "ProgramData",
    "Program Files",
    "Program Files (x86)",
    // Version control
    ".git",
    ".svn",
    ".hg",
    // Dependency / build tool directories (with strong signals these are dev artifacts)
    "node_modules",
    ".cargo",
    ".rustup",
    ".gradle",
    ".npm",
    ".yarn",
    ".pnpm-store",
    ".cache",
    // IDE config
    ".vscode",
    ".idea",
    // iOS / macOS dev
    "Pods",
    "DerivedData",
    // Framework magic dirs
    ".next",
    ".nuxt",
    ".turbo",
    // Test coverage output
    "coverage",
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

    // Check file size — but relax for disc format containers (ifo, vob can be small menus)
    let size_threshold = if ext == "ifo" || ext == "vob" {
        // Only skip tiny IFO/VOB files (< 512 bytes are clearly metadata stubs)
        512
    } else {
        min_size_bytes
    };

    if let Ok(meta) = path.metadata() {
        if meta.len() < size_threshold {
            return false;
        }
    } else {
        return false;
    }

    // Disambiguate .ts extension (TypeScript vs MPEG-TS)
    if ext == "ts" && !is_mpeg_ts_video(path) {
        return false;
    }

    // Skip paths that traverse a blacklisted directory component.
    // Only check normal directory Name components (skip drive roots like "C:", prefix slashes, etc.)
    use std::path::Component;
    for comp in path.components() {
        let dir_name = match comp {
            Component::Normal(os_str) => os_str.to_string_lossy(),
            _ => continue, // Skip CurDir, ParentDir, Prefix (drive letters), RootDir
        };
        // Only blacklist non-file components (i.e., skip checking the filename itself)
        // We skip the last component which is the filename
        if is_blacklisted_dir(&dir_name) {
            // Make sure this component is actually a directory segment, not the filename
            if path.file_name().map(|f| f == comp.as_os_str()).unwrap_or(false) {
                continue; // Don't reject a file just because its name looks like a blacklisted dir
            }
            return false;
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
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut seen_roots = std::collections::HashSet::<PathBuf>::new();

    let mut add_root = |p: PathBuf| {
        if p.exists() && seen_roots.insert(p.clone()) {
            roots.push(p);
        }
    };

    // 1. Current user profile — prioritize these so they appear first in results
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(&user_profile);
        for sub in &[
            "Videos", "Downloads", "Documents", "Desktop",
            "Music", "Pictures", "3D Objects", "Saved Games",
        ] {
            add_root(p.join(sub));
        }
        // OneDrive personal folder (and variants like "OneDrive - Personal" / "OneDrive - Company")
        if let Ok(entries) = std::fs::read_dir(&p) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("OneDrive") && entry.path().is_dir() {
                    add_root(entry.path());
                }
            }
        }
    }

    // 2. Scan all user profiles under C:\Users\ (covers multi-user PCs and Public folder)
    let users_dir = PathBuf::from("C:\\Users");
    if users_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&users_dir) {
            for entry in entries.flatten() {
                let user_path = entry.path();
                if !user_path.is_dir() { continue; }
                let user_name = entry.file_name().to_string_lossy().to_string();
                // Skip system pseudo-accounts
                if matches!(user_name.as_str(), "Default" | "Default User" | "All Users") {
                    continue;
                }
                for sub in &["Videos", "Downloads", "Documents", "Desktop", "Music", "Pictures"] {
                    add_root(user_path.join(sub));
                }
                // OneDrive per-user profile
                if let Ok(entries2) = std::fs::read_dir(&user_path) {
                    for e2 in entries2.flatten() {
                        let n = e2.file_name().to_string_lossy().to_string();
                        if n.starts_with("OneDrive") && e2.path().is_dir() {
                            add_root(e2.path());
                        }
                    }
                }
            }
        }
    }

    // 3. Discover all drive letters A:\..Z:\ — catches USB drives, external HDDs, SD cards
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = PathBuf::from(&drive);
        if !path.exists() { continue; }
        // For C drive, user directories have already been prioritized above.
        if letter != b'C' {
            add_root(path);
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
        .follow_links(true)  // Follow symlinks to catch OneDrive, NAS mounts, junction points
        .same_file_system(false) // Allow crossing filesystem boundaries (external drives)
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
