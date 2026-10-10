use crate::models::{AudioExtractionOptions, AudioExtractionResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Normalizes audio format name and determines appropriate file extension.
pub fn normalize_format(format: &str) -> (&'static str, &'static str) {
    match format.trim().to_lowercase().as_str() {
        "m4a" | "aac" => ("m4a", "m4a"),
        "wav" => ("wav", "wav"),
        "flac" => ("flac", "flac"),
        _ => ("mp3", "mp3"),
    }
}

/// Builds FFmpeg argument list for audio extraction.
pub fn build_audio_args(
    video_path: &Path,
    options: &AudioExtractionOptions,
    dest_path: &Path,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        video_path.to_string_lossy().to_string(),
        "-vn".to_string(),
    ];

    let (norm_fmt, _) = normalize_format(&options.format);

    if options.copy_stream.unwrap_or(false) {
        args.push("-c:a".to_string());
        args.push("copy".to_string());
    } else {
        match norm_fmt {
            "m4a" => {
                args.push("-c:a".to_string());
                args.push("aac".to_string());
                let bitrate = options
                    .quality_bitrate
                    .as_deref()
                    .unwrap_or("256k");
                args.push("-b:a".to_string());
                args.push(bitrate.to_string());
            }
            "wav" => {
                args.push("-c:a".to_string());
                args.push("pcm_s16le".to_string());
            }
            "flac" => {
                args.push("-c:a".to_string());
                args.push("flac".to_string());
            }
            _ => {
                // Default to MP3
                args.push("-c:a".to_string());
                args.push("libmp3lame".to_string());
                let bitrate = options
                    .quality_bitrate
                    .as_deref()
                    .unwrap_or("320k");
                args.push("-b:a".to_string());
                args.push(bitrate.to_string());
            }
        }
    }

    args.push(dest_path.to_string_lossy().to_string());
    args
}

/// Generates a non-conflicting destination file path for audio extraction.
pub fn resolve_unique_audio_path(
    video_path: &Path,
    custom_output_dir: Option<&Path>,
    custom_filename: Option<&str>,
    format: &str,
) -> PathBuf {
    let dest_dir = if let Some(dir) = custom_output_dir {
        if dir.as_os_str().is_empty() {
            video_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            dir.to_path_buf()
        }
    } else {
        video_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    let (_, ext) = normalize_format(format);

    let stem = if let Some(custom) = custom_filename {
        let trimmed = custom.trim();
        if trimmed.is_empty() {
            video_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("audio")
                .to_string()
        } else {
            // Strip format extension if user typed filename with extension
            let p = Path::new(trimmed);
            p.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(trimmed)
                .to_string()
        }
    } else {
        video_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("audio")
            .to_string()
    };

    let initial_path = dest_dir.join(format!("{}.{}", stem, ext));
    if !initial_path.exists() {
        return initial_path;
    }

    // Deduplicate with index _1, _2, etc.
    for index in 1..=9999 {
        let candidate = dest_dir.join(format!("{}_{}.{}", stem, index, ext));
        if !candidate.exists() {
            return candidate;
        }
    }

    dest_dir.join(format!("{}_{}.{}", stem, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0), ext))
}

/// Core function to extract audio from video using FFmpeg.
pub fn extract_audio(
    options: AudioExtractionOptions,
    app_data_dir: Option<&Path>,
) -> Result<AudioExtractionResult, String> {
    let video_path = PathBuf::from(&options.video_path);
    if !video_path.exists() || !video_path.is_file() {
        return Err(format!("原视频文件不存在: {}", options.video_path));
    }

    let custom_dir = options.output_dir.as_ref().map(PathBuf::from);
    if let Some(ref dir) = custom_dir {
        if !dir.exists() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("创建目标保存文件夹失败: {}", e))?;
        }
    }

    let dest_path = resolve_unique_audio_path(
        &video_path,
        custom_dir.as_deref(),
        options.custom_filename.as_deref(),
        &options.format,
    );

    let ffmpeg_bin = if let Some(dir) = crate::downloader::executor::resolve_ffmpeg_dir(app_data_dir) {
        let exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        let p = dir.join(exe_name);
        if p.exists() {
            p
        } else {
            PathBuf::from(exe_name)
        }
    } else {
        PathBuf::from(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
    };

    let args = build_audio_args(&video_path, &options, &dest_path);

    let mut cmd = Command::new(&ffmpeg_bin);
    // Note: args already includes dest_path as last element
    cmd.args(&args);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().map_err(|e| {
        format!(
            "无法启动 FFmpeg 执行音频提取 (可执行文件: {:?}): {}",
            ffmpeg_bin, e
        )
    })?;

    if !output.status.success() {
        let stderr_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "FFmpeg 音频提取失败 (退出码 {:?}): {}",
            output.status.code(),
            stderr_msg.trim()
        ));
    }

    if !dest_path.exists() {
        return Err("提取已完成但未找到生成的音频文件".to_string());
    }

    let file_size = fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);
    if file_size == 0 {
        return Err("提取出的音频文件内容为空".to_string());
    }

    let duration = crate::media::extract_metadata(&dest_path)
        .ok()
        .and_then(|m| m.duration);

    let file_name = dest_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("audio")
        .to_string();

    let (norm_fmt, _) = normalize_format(&options.format);

    Ok(AudioExtractionResult {
        success: true,
        output_path: dest_path.to_string_lossy().to_string(),
        file_name,
        file_size,
        duration,
        format: norm_fmt.to_string(),
        message: "音频提取成功".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_format() {
        assert_eq!(normalize_format("mp3"), ("mp3", "mp3"));
        assert_eq!(normalize_format("M4A"), ("m4a", "m4a"));
        assert_eq!(normalize_format("aac"), ("m4a", "m4a"));
        assert_eq!(normalize_format("WAV"), ("wav", "wav"));
        assert_eq!(normalize_format("flac"), ("flac", "flac"));
        assert_eq!(normalize_format("unknown"), ("mp3", "mp3"));
    }

    #[test]
    fn test_build_audio_args_mp3() {
        let v = Path::new("/path/to/video.mp4");
        let d = Path::new("/path/to/video.mp3");
        let opts = AudioExtractionOptions {
            video_path: "/path/to/video.mp4".to_string(),
            video_id: None,
            format: "mp3".to_string(),
            quality_bitrate: Some("192k".to_string()),
            copy_stream: Some(false),
            output_dir: None,
            custom_filename: None,
        };
        let args = build_audio_args(v, &opts, d);
        assert_eq!(args[0], "-y");
        assert_eq!(args[1], "-i");
        assert_eq!(args[2], "/path/to/video.mp4");
        assert_eq!(args[3], "-vn");
        assert_eq!(args[4], "-c:a");
        assert_eq!(args[5], "libmp3lame");
        assert_eq!(args[6], "-b:a");
        assert_eq!(args[7], "192k");
        assert_eq!(args[8], "/path/to/video.mp3");
    }

    #[test]
    fn test_build_audio_args_copy_stream() {
        let v = Path::new("/path/to/video.mp4");
        let d = Path::new("/path/to/video.m4a");
        let opts = AudioExtractionOptions {
            video_path: "/path/to/video.mp4".to_string(),
            video_id: None,
            format: "m4a".to_string(),
            quality_bitrate: None,
            copy_stream: Some(true),
            output_dir: None,
            custom_filename: None,
        };
        let args = build_audio_args(v, &opts, d);
        assert_eq!(args[3], "-vn");
        assert_eq!(args[4], "-c:a");
        assert_eq!(args[5], "copy");
        assert_eq!(args[6], "/path/to/video.m4a");
    }

    #[test]
    fn test_build_audio_args_wav_flac() {
        let v = Path::new("/path/to/video.mp4");
        let d_wav = Path::new("/path/to/audio.wav");
        let opts_wav = AudioExtractionOptions {
            video_path: "/path/to/video.mp4".to_string(),
            video_id: None,
            format: "wav".to_string(),
            quality_bitrate: None,
            copy_stream: None,
            output_dir: None,
            custom_filename: None,
        };
        let args_wav = build_audio_args(v, &opts_wav, d_wav);
        assert_eq!(args_wav[4], "-c:a");
        assert_eq!(args_wav[5], "pcm_s16le");

        let d_flac = Path::new("/path/to/audio.flac");
        let opts_flac = AudioExtractionOptions {
            video_path: "/path/to/video.mp4".to_string(),
            video_id: None,
            format: "flac".to_string(),
            quality_bitrate: None,
            copy_stream: None,
            output_dir: None,
            custom_filename: None,
        };
        let args_flac = build_audio_args(v, &opts_flac, d_flac);
        assert_eq!(args_flac[4], "-c:a");
        assert_eq!(args_flac[5], "flac");
    }

    #[test]
    fn test_resolve_unique_audio_path_basic() {
        let v = Path::new("/tmp/sample_video_test.mp4");
        let out = resolve_unique_audio_path(v, None, None, "mp3");
        assert_eq!(out.file_name().unwrap(), "sample_video_test.mp3");

        let out_custom = resolve_unique_audio_path(v, None, Some("my_audio.mp3"), "m4a");
        assert_eq!(out_custom.file_name().unwrap(), "my_audio.m4a");
    }
}
