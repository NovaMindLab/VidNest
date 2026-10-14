use crate::db::subtitles::{parse_srt_or_vtt, SubtitleSegment};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTranscriptionConfig {
    pub engine: String, // "local" | "api"
    pub api_url: String, // e.g. "https://api.openai.com/v1" or "http://127.0.0.1:8000/v1"
    pub api_key: String, // Bearer token / api key
    pub model: String,   // "tiny", "base", "small", "whisper-1"
    pub language: String, // "auto", "zh", "en", "ja"
    pub prompt: Option<String>,
    pub local_model_path: Option<String>,
    #[serde(default)]
    pub llm_api_url: Option<String>,
    #[serde(default)]
    pub llm_api_key: Option<String>,
    #[serde(default)]
    pub llm_model: Option<String>,
}

impl Default for AiTranscriptionConfig {
    fn default() -> Self {
        Self {
            engine: "local".to_string(),
            api_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "base".to_string(),
            language: "auto".to_string(),
            prompt: None,
            local_model_path: None,
            llm_api_url: None,
            llm_api_key: None,
            llm_model: Some("gpt-4o-mini".to_string()),
        }
    }
}

/// Loads AI transcription settings from app_data/ai_config.json, with safe fallback.
pub fn load_ai_config(app_data_dir: &Path) -> AiTranscriptionConfig {
    let config_path = app_data_dir.join("ai_config.json");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<AiTranscriptionConfig>(&content) {
                return config;
            }
        }
    }
    AiTranscriptionConfig::default()
}

/// Saves AI transcription settings to app_data/ai_config.json.
pub fn save_ai_config(app_data_dir: &Path, config: &AiTranscriptionConfig) -> Result<(), String> {
    let config_path = app_data_dir.join("ai_config.json");
    if let Some(parent) = config_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize AI config: {}", e))?;
    fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write AI config to {:?}: {}", config_path, e))?;
    Ok(())
}

/// Resolves the local whisper binary location:
/// 1. App-managed bin directory (app_data/bin/whisper-cli or whisper)
/// 2. System PATH (whisper-cli, whisper, whisper.cpp)
pub fn resolve_whisper_binary(app_data_dir: Option<&Path>) -> Option<PathBuf> {
    let candidate_names = if cfg!(windows) {
        vec!["whisper-cli.exe", "whisper.exe", "main.exe"]
    } else {
        vec!["whisper-cli", "whisper", "whisper-cpp", "main"]
    };

    if let Some(dir) = app_data_dir {
        let bin_dir = dir.join("bin");
        for name in &candidate_names {
            let p = bin_dir.join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // Check system PATH
    for name in &candidate_names {
        let p = PathBuf::from(name);
        let mut cmd = Command::new(&p);
        cmd.arg("-h");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        if let Ok(output) = cmd.output() {
            if output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
                return Some(p);
            }
        }
    }

    None
}

/// Extracts a 16kHz mono 16-bit PCM WAV from a video using ffmpeg for Whisper digestion.
pub fn extract_audio_for_whisper(
    video_path: &Path,
    output_wav: &Path,
    app_data_dir: Option<&Path>,
) -> Result<(), String> {
    let ffmpeg_bin = if let Some(dir) = crate::downloader::executor::resolve_ffmpeg_dir(app_data_dir) {
        let exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        dir.join(exe_name)
    } else {
        PathBuf::from(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
    };

    if let Some(parent) = output_wav.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut cmd = Command::new(&ffmpeg_bin);
    cmd.arg("-y")
        .arg("-i")
        .arg(video_path)
        .arg("-vn")
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(output_wav);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute ffmpeg for audio extraction: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "FFmpeg audio extraction failed (status: {}): {}",
            output.status,
            stderr.lines().last().unwrap_or("")
        ));
    }

    if !output_wav.exists() {
        return Err("FFmpeg completed but output audio WAV file was not found".to_string());
    }

    Ok(())
}

/// Structure for parsing OpenAI /v1/audio/transcriptions verbose_json response
#[derive(Debug, Deserialize)]
struct OpenAiVerboseSegment {
    #[serde(default)]
    pub start: f64,
    #[serde(default)]
    pub end: f64,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiTranscriptionResponse {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub segments: Option<Vec<OpenAiVerboseSegment>>,
    #[serde(default)]
    pub language: Option<String>,
}

/// Transcribes a 16kHz WAV audio file via OpenAI-compatible /v1/audio/transcriptions API.
pub async fn transcribe_via_api(
    wav_path: &Path,
    video_id: i64,
    config: &AiTranscriptionConfig,
) -> Result<Vec<SubtitleSegment>, String> {
    let file_bytes = tokio::fs::read(wav_path)
        .await
        .map_err(|e| format!("Failed to read temporary WAV file: {}", e))?;

    let file_part = multipart::Part::bytes(file_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to create multipart file: {}", e))?;

    let mut form = multipart::Form::new().part("file", file_part);

    // Model name: if user specified tiny/base/small on cloud, default to whisper-1 or use specified
    let model_name = if config.model == "tiny" || config.model == "base" || config.model == "small" {
        if config.api_url.contains("openai.com") {
            "whisper-1".to_string()
        } else {
            config.model.clone()
        }
    } else {
        config.model.clone()
    };
    form = form.text("model", model_name);

    form = form.text("response_format", "verbose_json");

    if config.language != "auto" && !config.language.trim().is_empty() {
        form = form.text("language", config.language.clone());
    }

    if let Some(ref prompt) = config.prompt {
        if !prompt.trim().is_empty() {
            form = form.text("prompt", prompt.clone());
        }
    }

    let base_url = config.api_url.trim_end_matches('/');
    let endpoint = if base_url.ends_with("/audio/transcriptions") {
        base_url.to_string()
    } else if base_url.ends_with("/v1") {
        format!("{}/audio/transcriptions", base_url)
    } else {
        format!("{}/v1/audio/transcriptions", base_url)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10 minutes timeout for long files
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut request = client.post(&endpoint);
    if !config.api_key.trim().is_empty() {
        let auth_val = if config.api_key.starts_with("Bearer ") {
            config.api_key.clone()
        } else {
            format!("Bearer {}", config.api_key)
        };
        request = request.header("Authorization", auth_val);
    }

    let response = request
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("API transcription request failed: {}", e))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "API returned error (HTTP {}): {}",
            status,
            body_text.chars().take(400).collect::<String>()
        ));
    }

    // Try parsing as verbose_json
    if let Ok(parsed) = serde_json::from_str::<OpenAiTranscriptionResponse>(&body_text) {
        if let Some(segments) = parsed.segments {
            if !segments.is_empty() {
                let mut result = Vec::new();
                for seg in segments {
                    let content = seg.text.trim().to_string();
                    if content.is_empty() {
                        continue;
                    }
                    result.push(SubtitleSegment {
                        id: None,
                        video_id,
                        language: parsed.language.clone().or_else(|| Some(config.language.clone())),
                        format: Some("vtt".to_string()),
                        start_time_ms: (seg.start * 1000.0).round() as i64,
                        end_time_ms: (seg.end * 1000.0).round() as i64,
                        content,
                    });
                }
                return Ok(result);
            }
        }

        // Fallback: full text with one segment if no segments array
        if let Some(full_text) = parsed.text {
            if !full_text.trim().is_empty() {
                return Ok(vec![SubtitleSegment {
                    id: None,
                    video_id,
                    language: parsed.language.or_else(|| Some(config.language.clone())),
                    format: Some("vtt".to_string()),
                    start_time_ms: 0,
                    end_time_ms: 5000,
                    content: full_text.trim().to_string(),
                }]);
            }
        }
    }

    // Fallback: Check if response text is directly SRT or VTT format
    if body_text.contains("-->") {
        let segs = parse_srt_or_vtt(&body_text, video_id);
        if !segs.is_empty() {
            return Ok(segs);
        }
    }

    Err(format!(
        "Failed to parse segments from API response: {}",
        body_text.chars().take(200).collect::<String>()
    ))
}

/// Transcribes via local whisper executable (e.g. whisper.cpp / whisper-cli).
pub fn transcribe_via_local_whisper(
    wav_path: &Path,
    video_id: i64,
    config: &AiTranscriptionConfig,
    whisper_bin: &Path,
    app_data_dir: &Path,
) -> Result<Vec<SubtitleSegment>, String> {
    // Resolve model file path
    let model_path = if let Some(ref custom_path) = config.local_model_path {
        let p = PathBuf::from(custom_path);
        if p.exists() {
            p
        } else {
            return Err(format!("Specified local model file does not exist: {}", custom_path));
        }
    } else {
        let model_dir = app_data_dir.join("models");
        let model_name = format!("ggml-{}.bin", config.model);
        let p = model_dir.join(&model_name);
        if p.exists() {
            p
        } else {
            return Err(format!(
                "Local Whisper model file '{model_name}' not found in {:?}.\n\
                 请先将模型文件放入该目录，或在设置中切换为兼容 API 转写模式。",
                model_dir
            ));
        }
    };

    let temp_out_prefix = wav_path.with_extension("");
    let mut cmd = Command::new(whisper_bin);
    cmd.arg("-m")
        .arg(&model_path)
        .arg("-f")
        .arg(wav_path)
        .arg("--output-vtt")
        .arg("-of")
        .arg(&temp_out_prefix);

    if config.language != "auto" && !config.language.trim().is_empty() {
        cmd.arg("-l").arg(&config.language);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to launch whisper-cli: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Whisper execution failed (code: {:?}): {}",
            output.status.code(),
            stderr.lines().last().unwrap_or("Unknown error")
        ));
    }

    let vtt_path = temp_out_prefix.with_extension("vtt");
    if vtt_path.exists() {
        let content = fs::read_to_string(&vtt_path)
            .map_err(|e| format!("Failed to read generated VTT file: {}", e))?;
        let _ = fs::remove_file(&vtt_path);
        let segments = parse_srt_or_vtt(&content, video_id);
        return Ok(segments);
    }

    // Try parsing stdout directly if file was not created
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("-->") {
        let segments = parse_srt_or_vtt(&stdout, video_id);
        if !segments.is_empty() {
            return Ok(segments);
        }
    }

    Err("Whisper finished but no valid subtitle segments or VTT output were produced".to_string())
}

/// Full orchestration: extracts 16kHz WAV, invokes selected engine, and returns parsed SubtitleSegments.
pub async fn transcribe_video_pipeline(
    video_path: &Path,
    video_id: i64,
    config: &AiTranscriptionConfig,
    app_data_dir: &Path,
) -> Result<Vec<SubtitleSegment>, String> {
    if !video_path.exists() {
        return Err(format!("Video file does not exist: {:?}", video_path));
    }

    // 1. Prepare temp output directory
    let temp_dir = app_data_dir.join("temp");
    let _ = fs::create_dir_all(&temp_dir);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let temp_wav = temp_dir.join(format!("transcribe_{}_{}.wav", video_id, now));

    // 2. Extract 16kHz audio slice
    extract_audio_for_whisper(video_path, &temp_wav, Some(app_data_dir))?;

    // 3. Dispatch to selected engine
    let segments_res = if config.engine == "api" {
        transcribe_via_api(&temp_wav, video_id, config).await
    } else {
        // Local engine
        let whisper_bin = resolve_whisper_binary(Some(app_data_dir));
        match whisper_bin {
            Some(bin) => transcribe_via_local_whisper(&temp_wav, video_id, config, &bin, app_data_dir),
            None => {
                // If local binary is not found, check if API is available and fallback or return clear guide
                if !config.api_key.trim().is_empty() {
                    transcribe_via_api(&temp_wav, video_id, config).await
                } else {
                    Err("未检测到本地 whisper-cli 内核。您可以在设置中配置 API 转录模式（如本地 Ollama/Faster-Whisper 或云端 API），或在 app_data/bin 目录下放置 whisper-cli 可执行文件。".to_string())
                }
            }
        }
    };

    // 4. Always clean up temporary WAV file
    let _ = fs::remove_file(&temp_wav);

    segments_res
}
