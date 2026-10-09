use base64::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub duration: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
}

/// Extracts media metadata using ffprobe via JSON serialization.
pub fn extract_metadata(video_path: &Path) -> Result<MediaMetadata, String> {
    let mut cmd = Command::new("ffprobe");
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
    ]);
    cmd.arg(video_path);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke ffprobe on {:?}: {}", video_path, e))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe exited with status {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json_text = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&json_text)
        .map_err(|e| format!("Failed parsing ffprobe JSON output: {}", e))?;

    let mut meta = MediaMetadata::default();

    // Duration from format
    if let Some(format_obj) = val.get("format") {
        if let Some(dur_str) = format_obj.get("duration").and_then(|d| d.as_str()) {
            meta.duration = dur_str.parse::<f64>().ok();
        }
    }

    // Video stream: width, height, codec
    if let Some(streams) = val.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let codec_type = stream.get("codec_type").and_then(|c| c.as_str());
            if codec_type == Some("video") {
                if meta.width.is_none() {
                    meta.width = stream.get("width").and_then(|w| w.as_u64()).map(|w| w as u32);
                }
                if meta.height.is_none() {
                    meta.height = stream.get("height").and_then(|h| h.as_u64()).map(|h| h as u32);
                }
                if meta.codec.is_none() {
                    meta.codec = stream
                        .get("codec_name")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_uppercase());
                }
                // Fallback duration if missing in format
                if meta.duration.is_none() {
                    if let Some(dur_str) = stream.get("duration").and_then(|d| d.as_str()) {
                        meta.duration = dur_str.parse::<f64>().ok();
                    }
                }
                break;
            }
        }
    }

    Ok(meta)
}

/// Generates a video thumbnail frame using ffmpeg and caches it into app_data/thumbnails/{file_hash}.jpg.
pub fn generate_thumbnail(
    video_path: &Path,
    file_hash: &str,
    duration: Option<f64>,
    app_data_dir: &Path,
) -> Result<PathBuf, String> {
    let thumb_dir = app_data_dir.join("thumbnails");
    fs::create_dir_all(&thumb_dir)
        .map_err(|e| format!("Failed to create thumbnails cache dir: {}", e))?;

    let output_file = thumb_dir.join(format!("{}.jpg", file_hash));

    // Cache hit: if already exists and is non-empty, return cached file path
    if output_file.exists() {
        if let Ok(meta) = fs::metadata(&output_file) {
            if meta.len() > 0 {
                return Ok(output_file);
            }
        }
    }

    let seek_sec = match duration {
        Some(d) if d > 10.0 => 5.0,
        Some(d) if d > 2.0 => d * 0.25,
        _ => 0.0,
    };
    let seek_str = format!("{:.2}", seek_sec);

    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-ss",
        &seek_str,
        "-i",
    ]);
    cmd.arg(video_path);
    cmd.args([
        "-vframes",
        "1",
        "-q:v",
        "2",
        "-vf",
        "scale=640:-1",
        "-y",
    ]);
    cmd.arg(&output_file);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke ffmpeg: {}", e))?;

    if !output.status.success() || !output_file.exists() {
        return Err(format!(
            "ffmpeg thumbnail extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output_file)
}

/// Loads a thumbnail image file and encodes it as a base64 Data URL.
pub fn read_thumbnail_base64(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:image/jpeg;base64,{}", encoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_thumbnail_base64_not_found() {
        let fake = Path::new("nonexistent_thumb_file.jpg");
        assert!(read_thumbnail_base64(fake).is_none());
    }
}
