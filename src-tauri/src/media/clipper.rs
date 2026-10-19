use crate::models::{
    ClipResultItem, ClipVideoRequest, ClipVideoResponse, RecommendSegmentsResponse,
    SegmentRecommendation,
};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Resolves ffmpeg executable path
pub fn resolve_ffmpeg_bin(app_data_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = crate::downloader::executor::resolve_ffmpeg_dir(app_data_dir) {
        let exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        dir.join(exe_name)
    } else {
        PathBuf::from(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
    }
}

/// Sanitizes a string for safe usage in file names
pub fn sanitize_filename_part(text: &str) -> String {
    let sanitized: String = text
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || ('\u{4e00}'..='\u{9fa5}').contains(&c) {
                c
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "clip".to_string()
    } else if trimmed.chars().count() > 30 {
        trimmed.chars().take(30).collect()
    } else {
        trimmed.to_string()
    }
}

/// Generates a unique destination file path
pub fn resolve_unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let mut candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let mut counter = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{}_{}{}", stem, counter, ext));
        counter += 1;
    }
    candidate
}

/// Clips a video into one or multiple segments based on the request
pub fn clip_video_segments(
    request: ClipVideoRequest,
    app_data_dir: Option<&Path>,
) -> Result<ClipVideoResponse, String> {
    let input_path = PathBuf::from(&request.video_path);
    if !input_path.exists() {
        return Err(format!("源视频文件不存在: {:?}", input_path));
    }

    if request.segments.is_empty() {
        return Err("裁剪片段列表不能为空，请至少指定一个片段。".to_string());
    }

    let ffmpeg_bin = resolve_ffmpeg_bin(app_data_dir);

    // Determine output directory
    let output_dir = if let Some(ref dir_str) = request.output_dir {
        let p = PathBuf::from(dir_str.trim());
        if !p.as_os_str().is_empty() {
            let _ = fs::create_dir_all(&p);
            p
        } else {
            input_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        }
    } else {
        input_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");

    let mut clip_results = Vec::new();

    // 1. Export each individual segment
    for (idx, seg) in request.segments.iter().enumerate() {
        let start_ms = seg.start_ms.max(0);
        let end_ms = seg.end_ms.max(start_ms + 100);
        let start_sec = start_ms as f64 / 1000.0;
        let duration_sec = (end_ms - start_ms) as f64 / 1000.0;

        let clean_title = sanitize_filename_part(&seg.title);
        let file_name = format!("{}_clip_{:02}_{}.{}", stem, idx + 1, clean_title, ext);
        let output_file = resolve_unique_path(&output_dir, &file_name);

        let mut cmd = Command::new(&ffmpeg_bin);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if request.fast_copy {
            // Fast Stream Copy (Lossless & lightning fast)
            cmd.args([
                "-y",
                "-ss",
                &format!("{:.3}", start_sec),
                "-i",
                &input_path.to_string_lossy(),
                "-t",
                &format!("{:.3}", duration_sec),
                "-c",
                "copy",
                "-avoid_negative_ts",
                "make_zero",
                &output_file.to_string_lossy(),
            ]);
        } else {
            // Frame-accurate re-encode
            cmd.args([
                "-y",
                "-ss",
                &format!("{:.3}", start_sec),
                "-i",
                &input_path.to_string_lossy(),
                "-t",
                &format!("{:.3}", duration_sec),
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-crf",
                "19",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                &output_file.to_string_lossy(),
            ]);
        }

        let output = cmd.output().map_err(|e| {
            format!("执行 FFmpeg 裁剪失败 (片段 {}): {}", idx + 1, e)
        })?;

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            // If stream copy failed, try fallback re-encode
            if request.fast_copy {
                let mut retry_cmd = Command::new(&ffmpeg_bin);
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    retry_cmd.creation_flags(CREATE_NO_WINDOW);
                }
                retry_cmd.args([
                    "-y",
                    "-ss",
                    &format!("{:.3}", start_sec),
                    "-i",
                    &input_path.to_string_lossy(),
                    "-t",
                    &format!("{:.3}", duration_sec),
                    "-c:v",
                    "libx264",
                    "-preset",
                    "fast",
                    "-crf",
                    "20",
                    "-c:a",
                    "aac",
                    &output_file.to_string_lossy(),
                ]);
                let retry_output = retry_cmd.output().map_err(|e| e.to_string())?;
                if !retry_output.status.success() {
                    return Err(format!(
                        "片段 {} 裁剪失败: {}",
                        idx + 1,
                        String::from_utf8_lossy(&retry_output.stderr)
                    ));
                }
            } else {
                return Err(format!("片段 {} 裁剪失败: {}", idx + 1, stderr_str));
            }
        }

        let file_size = fs::metadata(&output_file).map(|m| m.len()).unwrap_or(0);
        clip_results.push(ClipResultItem {
            id: seg.id.clone(),
            title: seg.title.clone(),
            output_path: output_file.to_string_lossy().to_string(),
            duration_sec,
            file_size,
            start_ms,
            end_ms,
        });
    }

    // 2. Handle Merging if requested ("merge" or "both")
    let mut merged_path_opt = None;
    if (request.export_mode == "merge" || request.export_mode == "both")
        && clip_results.len() > 1
    {
        let merged_name = format!("{}_merged_highlights.{}", stem, ext);
        let merged_file = resolve_unique_path(&output_dir, &merged_name);

        let temp_list_file = output_dir.join(format!(
            "vidnest_concat_{}_{}.txt",
            stem,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));

        let mut concat_content = String::new();
        for item in &clip_results {
            // Escape single quotes for ffmpeg concat demuxer
            let clean_p = item.output_path.replace('\'', "'\\''");
            concat_content.push_str(&format!("file '{}'\n", clean_p));
        }

        if let Ok(_) = fs::write(&temp_list_file, concat_content) {
            let mut merge_cmd = Command::new(&ffmpeg_bin);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                merge_cmd.creation_flags(CREATE_NO_WINDOW);
            }
            merge_cmd.args([
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                &temp_list_file.to_string_lossy(),
                "-c",
                "copy",
                &merged_file.to_string_lossy(),
            ]);

            if let Ok(m_out) = merge_cmd.output() {
                if m_out.status.success() {
                    merged_path_opt = Some(merged_file.to_string_lossy().to_string());
                } else {
                    // Try re-encoding concat fallback
                    let mut re_merge = Command::new(&ffmpeg_bin);
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        re_merge.creation_flags(CREATE_NO_WINDOW);
                    }
                    re_merge.args([
                        "-y",
                        "-f",
                        "concat",
                        "-safe",
                        "0",
                        "-i",
                        &temp_list_file.to_string_lossy(),
                        "-c:v",
                        "libx264",
                        "-preset",
                        "fast",
                        "-crf",
                        "19",
                        "-c:a",
                        "aac",
                        &merged_file.to_string_lossy(),
                    ]);
                    if let Ok(rm_out) = re_merge.output() {
                        if rm_out.status.success() {
                            merged_path_opt = Some(merged_file.to_string_lossy().to_string());
                        }
                    }
                }
            }
            let _ = fs::remove_file(&temp_list_file);
        }

        // If user strictly requested "merge" only (not "both"), remove the temporary individual clips
        if request.export_mode == "merge" && merged_path_opt.is_some() {
            for item in &clip_results {
                let _ = fs::remove_file(&item.output_path);
            }
            clip_results.clear();
        }
    }

    Ok(ClipVideoResponse {
        success: true,
        clips: clip_results,
        merged_path: merged_path_opt,
        message: "视频裁剪与阶段导出已顺利完成".to_string(),
    })
}

const CLIMAX_KEYWORDS: &[&str] = &[
    "核心", "高潮", "关键", "巅峰", "反转", "绝了", "太强", "厉害", "卧槽", "逆天", "爆", "燃",
    "秘密", "第一", "注意", "真相", "竟然", "没想到", "终于", "突破", "成功", "惊呆", "神了",
    "一定要", "爆料", "绝招", "精华", "重点", "瞬间", "炸裂", "王炸", "结论", "精选", "亮点",
];

/// Extracts 1-second sampled RMS audio volume energy timeline via FFmpeg astats.
pub fn extract_audio_energy_timeline(
    video_path: &Path,
    app_data_dir: Option<&Path>,
) -> Vec<(f64, f32)> {
    if !video_path.exists() {
        return Vec::new();
    }
    let ffmpeg_bin = resolve_ffmpeg_bin(app_data_dir);
    let mut cmd = Command::new(&ffmpeg_bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args([
        "-i",
        &video_path.to_string_lossy(),
        "-vn",
        "-af",
        "asetnsamples=n=44100,astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.RMS_level:file=-",
        "-f",
        "null",
        "-",
    ]);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut points = Vec::new();
    let mut cur_sec = 0.0;

    for line in stdout_str.lines() {
        if let Some(pos) = line.find("pts_time:") {
            let rest = &line[pos + 9..];
            if let Some(sec_str) = rest.split_whitespace().next() {
                if let Ok(s) = sec_str.parse::<f64>() {
                    cur_sec = s;
                }
            }
        } else if let Some(pos) = line.find("lavfi.astats.Overall.RMS_level=") {
            let val_str = line[pos + 31..].trim();
            let db = if val_str == "-inf" {
                -90.0
            } else if let Ok(v) = val_str.parse::<f32>() {
                v
            } else {
                -90.0
            };
            points.push((cur_sec, db));
        }
    }

    points
}

/// Detects climax / highlight segments using multi-modal signals:
/// 1. Audio RMS loudness bursts & energy peaks;
/// 2. Linguistic sentiment, exclamation marks, and high-energy keywords in subtitles;
/// 3. Sentence boundary snapping for clean start and cut;
/// 4. AI Topic core highlights fallback.
pub fn detect_climax_highlights(
    video_id: Option<i64>,
    video_path: &Path,
    total_duration_ms: i64,
    conn: Option<&Connection>,
    app_data_dir: Option<&Path>,
) -> Vec<SegmentRecommendation> {
    let mut candidates: Vec<SegmentRecommendation> = Vec::new();

    // A. Check if we have subtitles in SQLite
    let subs = if let (Some(v_id), Some(c)) = (video_id, conn) {
        crate::db::subtitles::get_video_subtitles(c, v_id).unwrap_or_default()
    } else {
        Vec::new()
    };

    // B. Check if we have AI analysis topics in SQLite
    let ai_analysis = if let (Some(v_id), Some(c)) = (video_id, conn) {
        crate::ai::summary::get_video_ai_analysis(c, v_id).ok().flatten()
    } else {
        None
    };

    // C. Extract audio energy curve
    let energy_timeline = extract_audio_energy_timeline(video_path, app_data_dir);
    let total_sec = total_duration_ms as f64 / 1000.0;

    if !energy_timeline.is_empty() && total_sec >= 15.0 {
        // Window sliding configuration
        let window_len_sec = if total_sec < 60.0 {
            15.0
        } else if total_sec < 300.0 {
            20.0
        } else {
            30.0
        };
        let step_sec = 4.0;

        let mut scored_windows: Vec<(f64, f64, f32, f32)> = Vec::new(); // (start_sec, end_sec, score, peak_db)
        let mut t = 0.0;
        while t + 8.0 <= total_sec {
            let win_end = (t + window_len_sec).min(total_sec);
            let samples: Vec<f32> = energy_timeline
                .iter()
                .filter(|(s, _)| *s >= t && *s <= win_end)
                .map(|(_, db)| *db)
                .collect();

            if !samples.is_empty() {
                let count = samples.len() as f32;
                let sum: f32 = samples.iter().sum();
                let avg_rms = sum / count;
                let peak_rms = samples.iter().cloned().fold(-90.0f32, f32::max);

                // Audio energy scoring: normal voice is -28dB, peaks reach -14dB
                let norm_avg = ((avg_rms + 42.0) / 28.0).clamp(0.0, 1.0);
                let norm_peak = ((peak_rms + 36.0) / 24.0).clamp(0.0, 1.0);
                let mut score = norm_avg * 45.0 + norm_peak * 35.0; // 0..80

                // Linguistic keyword & exclamation scoring if subtitles are present
                if !subs.is_empty() {
                    let win_start_ms = (t * 1000.0) as i64;
                    let win_end_ms = (win_end * 1000.0) as i64;
                    let mut win_text = String::new();
                    let mut exclaim_count = 0;
                    let mut keyword_hits = 0;

                    for s in &subs {
                        if s.start_time_ms < win_end_ms && s.end_time_ms > win_start_ms {
                            win_text.push_str(&s.content);
                            win_text.push(' ');
                            for ch in s.content.chars() {
                                if ch == '!' || ch == '！' || ch == '?' || ch == '？' {
                                    exclaim_count += 1;
                                }
                            }
                            for kw in CLIMAX_KEYWORDS {
                                if s.content.contains(kw) {
                                    keyword_hits += 1;
                                }
                            }
                        }
                    }

                    score += (keyword_hits as f32 * 4.5).min(18.0);
                    score += (exclaim_count as f32 * 2.5).min(10.0);

                    // Dialogue density bonus
                    let duration_s = win_end - t;
                    if duration_s > 0.0 {
                        let chars_per_sec = win_text.chars().count() as f64 / duration_s;
                        if chars_per_sec >= 4.0 {
                            score += 5.0;
                        }
                    }
                }

                scored_windows.push((t, win_end, score.clamp(30.0, 99.0), peak_rms));
            }

            t += step_sec;
        }

        // Sort by score descending
        scored_windows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Non-maximum suppression (NMS) to eliminate heavily overlapping windows
        let mut selected_windows: Vec<(f64, f64, f32, f32)> = Vec::new();
        let max_highlights = if total_sec < 60.0 { 2 } else if total_sec < 600.0 { 3 } else { 4 };

        for win in scored_windows {
            let (w_start, w_end, w_score, w_peak) = win;
            let overlaps = selected_windows.iter().any(|(s, e, _, _)| {
                let inter = (w_end.min(*e) - w_start.max(*s)).max(0.0);
                let union = (w_end.max(*e) - w_start.min(*s)).max(1.0);
                inter / union > 0.35 // IoU threshold
            });

            if !overlaps {
                selected_windows.push((w_start, w_end, w_score, w_peak));
                if selected_windows.len() >= max_highlights {
                    break;
                }
            }
        }

        // Sort selected windows by chronological time
        selected_windows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Convert into SegmentRecommendation with smart sentence boundary snapping
        for (i, (start_s, end_s, score, peak_db)) in selected_windows.into_iter().enumerate() {
            let mut start_ms = (start_s * 1000.0) as i64;
            let mut end_ms = (end_s * 1000.0) as i64;
            let mut punchy_title = String::new();

            // Snap to nearest subtitle sentence boundary if subtitles exist
            if !subs.is_empty() {
                // Find subtitle near start_ms
                let best_start_sub = subs
                    .iter()
                    .filter(|s| (s.start_time_ms - start_ms).abs() <= 4000)
                    .min_by_key(|s| (s.start_time_ms - start_ms).abs());
                if let Some(ss) = best_start_sub {
                    start_ms = ss.start_time_ms;
                }

                // Find subtitle near end_ms
                let best_end_sub = subs
                    .iter()
                    .filter(|s| (s.end_time_ms - end_ms).abs() <= 4000)
                    .min_by_key(|s| (s.end_time_ms - end_ms).abs());
                if let Some(es) = best_end_sub {
                    end_ms = es.end_time_ms;
                }

                // Pick the most dramatic sentence for title
                let touching_subs: Vec<&crate::db::subtitles::SubtitleSegment> = subs
                    .iter()
                    .filter(|s| s.start_time_ms < end_ms && s.end_time_ms > start_ms)
                    .collect();

                let keyword_sub = touching_subs
                    .iter()
                    .find(|s| CLIMAX_KEYWORDS.iter().any(|kw| s.content.contains(kw)));

                if let Some(ks) = keyword_sub {
                    punchy_title = ks.content.clone();
                } else if let Some(first_sub) = touching_subs.first() {
                    punchy_title = first_sub.content.clone();
                }
            }

            let clean_title_body = if punchy_title.chars().count() > 16 {
                format!("{}...", punchy_title.chars().take(16).collect::<String>())
            } else if !punchy_title.is_empty() {
                punchy_title
            } else {
                format!("峰值 {:.1}dB 核心高能", peak_db)
            };

            let title = format!("🔥 高潮高能 {:02}: {}", i + 1, clean_title_body);
            let reason = if peak_db > -90.0 {
                format!("声学能量爆发峰值 ({:.1}dB) 与高密度精彩节点", peak_db)
            } else {
                "核心高能高潮片段分析".to_string()
            };

            candidates.push(SegmentRecommendation {
                id: format!("climax_clip_{}", i + 1),
                title,
                start_ms: start_ms.max(0),
                end_ms: end_ms.min(total_duration_ms).max(start_ms + 3000),
                reason,
                confidence: 0.94,
                is_highlight: true,
                score: Some(score),
            });
        }
    }

    // D. If candidates are empty (e.g. no audio or FFmpeg failed), fallback to AI analysis climax topic
    if candidates.is_empty() {
        if let Some(ref analysis) = ai_analysis {
            for (i, topic) in analysis.topics.iter().enumerate() {
                let is_climax_topic = CLIMAX_KEYWORDS.iter().any(|kw| topic.title.contains(kw) || topic.point.contains(kw));
                if is_climax_topic || i == analysis.topics.len() / 2 {
                    let start = topic.timestamp_ms.max(0);
                    let end = if i + 1 < analysis.topics.len() {
                        analysis.topics[i + 1].timestamp_ms.min(start + 60_000)
                    } else {
                        (start + 45_000).min(total_duration_ms)
                    };
                    candidates.push(SegmentRecommendation {
                        id: format!("ai_climax_{}", i + 1),
                        title: format!("🔥 核心高潮: {}", topic.title),
                        start_ms: start,
                        end_ms: end.max(start + 5000),
                        reason: format!("AI 深度语义识别出的高能焦点: {}", topic.point),
                        confidence: 0.92,
                        is_highlight: true,
                        score: Some(95.0),
                    });
                    if candidates.len() >= 3 {
                        break;
                    }
                }
            }
        }
    }

    // E. Golden Ratio Fallback (Narrative climax typically at ~68% - 75% of runtime)
    if candidates.is_empty() && total_duration_ms > 20_000 {
        let center_ms = (total_duration_ms as f64 * 0.70) as i64;
        let half_win = 15_000;
        let start = (center_ms - half_win).max(0);
        let end = (center_ms + half_win).min(total_duration_ms);
        candidates.push(SegmentRecommendation {
            id: "pacing_climax_1".to_string(),
            title: "🔥 黄金节点核心高潮 (叙事峰值)".to_string(),
            start_ms: start,
            end_ms: end,
            reason: "剧作黄金分割率核心高能高潮区间 (70% 节点)".to_string(),
            confidence: 0.80,
            is_highlight: true,
            score: Some(88.0),
        });
    }

    candidates
}

/// AI-assisted smart segmentation recommender with Climax/Highlight detection:
/// 1. Detects Climax / High-energy moments (Audio peaks, exclamation subtitles, sentiment)
/// 2. Uses existing AI Analysis Topics (semantic chapters with titles)
/// 3. Uses Subtitle pause clustering (natural sentence boundaries)
/// 4. Uses FFmpeg silence detection
/// 5. Fallback to smart duration pacing
pub fn recommend_video_segments(
    video_id: Option<i64>,
    video_path: &Path,
    duration_ms: Option<i64>,
    conn: Option<&Connection>,
    app_data_dir: Option<&Path>,
    mode: Option<&str>,
) -> Result<RecommendSegmentsResponse, String> {
    let resolved_duration_ms = duration_ms.unwrap_or_else(|| {
        if let Ok(meta) = crate::media::extract_metadata(video_path) {
            (meta.duration.unwrap_or(60.0) * 1000.0) as i64
        } else {
            60_000
        }
    });

    let highlights = detect_climax_highlights(video_id, video_path, resolved_duration_ms, conn, app_data_dir);

    // If pure highlights mode is requested, return the extracted climax clips directly
    if mode == Some("highlights") || mode == Some("climax") {
        if !highlights.is_empty() {
            return Ok(RecommendSegmentsResponse {
                segments: highlights.clone(),
                source: "ai_climax".to_string(),
                message: format!("已成功智能截取 {} 个核心高能高潮精彩片段", highlights.len()),
                highlights,
            });
        }
    }

    let mark_highlights_on_chapters = |chapters: &mut [SegmentRecommendation]| {
        for seg in chapters.iter_mut() {
            for h in &highlights {
                let overlap_start = seg.start_ms.max(h.start_ms);
                let overlap_end = seg.end_ms.min(h.end_ms);
                if overlap_end > overlap_start + 4000 {
                    seg.is_highlight = true;
                    seg.score = h.score.or(Some(92.0));
                    break;
                }
            }
        }
    };

    // -------------------------------------------------------------
    // Strategy 1: AI Topic Chapters from SQLite ai_analyses
    // -------------------------------------------------------------
    if let (Some(v_id), Some(c)) = (video_id, conn) {
        if let Ok(Some(analysis)) = crate::ai::summary::get_video_ai_analysis(c, v_id) {
            if analysis.topics.len() >= 2 {
                let mut recs = Vec::new();
                for i in 0..analysis.topics.len() {
                    let cur = &analysis.topics[i];
                    let start = cur.timestamp_ms.max(0);
                    let end = if i + 1 < analysis.topics.len() {
                        analysis.topics[i + 1].timestamp_ms.max(start + 500)
                    } else {
                        resolved_duration_ms.max(start + 1000)
                    };

                    let title = if !cur.title.trim().is_empty() {
                        cur.title.clone()
                    } else {
                        format!("第 {} 章节", i + 1)
                    };

                    let reason = if !cur.point.trim().is_empty() {
                        format!("AI 深度语义主题: {}", cur.point)
                    } else {
                        "AI 深度语义主题与核心要点".to_string()
                    };

                    recs.push(SegmentRecommendation {
                        id: format!("ai_topic_{}", i + 1),
                        title,
                        start_ms: start,
                        end_ms: end,
                        reason,
                        confidence: 0.95,
                        is_highlight: false,
                        score: None,
                    });
                }

                if !recs.is_empty() {
                    mark_highlights_on_chapters(&mut recs);
                    return Ok(RecommendSegmentsResponse {
                        segments: recs,
                        source: "ai_topics".to_string(),
                        message: "已基于 AI 深度语义分析生成智能章节推荐".to_string(),
                        highlights,
                    });
                }
            }
        }
    }

    // -------------------------------------------------------------
    // Strategy 2: Subtitle Speech Clustering (Natural pauses & gaps)
    // -------------------------------------------------------------
    if let (Some(v_id), Some(c)) = (video_id, conn) {
        if let Ok(subs) = crate::db::subtitles::get_video_subtitles(c, v_id) {
            if subs.len() >= 3 {
                let mut recs = Vec::new();
                let mut chunk_start = subs[0].start_time_ms;
                let mut chunk_end = subs[0].end_time_ms;
                let mut first_sentence = subs[0].content.clone();
                let mut chunk_index = 1;

                for i in 1..subs.len() {
                    let prev_end = subs[i - 1].end_time_ms;
                    let cur_start = subs[i].start_time_ms;
                    let pause_gap = cur_start - prev_end;
                    let current_duration = cur_start - chunk_start;

                    // If speech pause > 1.8s or block duration exceeds 45s, split into a new chapter
                    if (pause_gap >= 1800 && current_duration >= 10000) || current_duration >= 45000 {
                        let clean_title = if first_sentence.chars().count() > 18 {
                            format!("{}...", first_sentence.chars().take(18).collect::<String>())
                        } else {
                            first_sentence.clone()
                        };

                        recs.push(SegmentRecommendation {
                            id: format!("speech_seg_{}", chunk_index),
                            title: format!("阶段 {}: {}", chunk_index, clean_title),
                            start_ms: chunk_start,
                            end_ms: chunk_end,
                            reason: "AI 语音语义与静音停顿聚类".to_string(),
                            confidence: 0.88,
                            is_highlight: false,
                            score: None,
                        });

                        chunk_index += 1;
                        chunk_start = cur_start;
                        first_sentence = subs[i].content.clone();
                    }

                    chunk_end = subs[i].end_time_ms;
                }

                // Add the last segment
                if chunk_end > chunk_start {
                    let clean_title = if first_sentence.chars().count() > 18 {
                        format!("{}...", first_sentence.chars().take(18).collect::<String>())
                    } else {
                        first_sentence
                    };

                    recs.push(SegmentRecommendation {
                        id: format!("speech_seg_{}", chunk_index),
                        title: format!("阶段 {}: {}", chunk_index, clean_title),
                        start_ms: chunk_start,
                        end_ms: chunk_end.max(resolved_duration_ms),
                        reason: "AI 语音语义与静音停顿聚类".to_string(),
                        confidence: 0.88,
                        is_highlight: false,
                        score: None,
                    });
                }

                if recs.len() >= 2 {
                    mark_highlights_on_chapters(&mut recs);
                    return Ok(RecommendSegmentsResponse {
                        segments: recs,
                        source: "subtitles".to_string(),
                        message: "已基于 AI 语音转写与自然断句停顿生成分段推荐".to_string(),
                        highlights,
                    });
                }
            }
        }
    }

    // -------------------------------------------------------------
    // Strategy 3: Silence Detection via FFmpeg
    // -------------------------------------------------------------
    if video_path.exists() {
        let ffmpeg_bin = resolve_ffmpeg_bin(app_data_dir);
        let mut probe_cmd = Command::new(&ffmpeg_bin);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            probe_cmd.creation_flags(CREATE_NO_WINDOW);
        }
        probe_cmd.args([
            "-i",
            &video_path.to_string_lossy(),
            "-af",
            "silencedetect=noise=-30dB:d=0.7",
            "-f",
            "null",
            "-",
        ]);

        if let Ok(output) = probe_cmd.output() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let mut cut_points_ms = Vec::new();

            for line in stderr_str.lines() {
                if let Some(pos) = line.find("silence_end:") {
                    let rest = &line[pos + 12..];
                    if let Some(sec_str) = rest.split_whitespace().next() {
                        if let Ok(sec) = sec_str.parse::<f64>() {
                            let ms = (sec * 1000.0) as i64;
                            if ms > 5000 && ms < (resolved_duration_ms - 5000) {
                                cut_points_ms.push(ms);
                            }
                        }
                    }
                }
            }

            // If we detected between 1 and 12 silence points, construct clean segments
            if !cut_points_ms.is_empty() && cut_points_ms.len() <= 12 {
                let mut recs = Vec::new();
                let mut prev_ms = 0;
                let mut idx = 1;

                for cut in cut_points_ms {
                    if cut - prev_ms >= 8000 {
                        recs.push(SegmentRecommendation {
                            id: format!("silence_cut_{}", idx),
                            title: format!("视频阶段 {:02}", idx),
                            start_ms: prev_ms,
                            end_ms: cut,
                            reason: "音频静音间歇自适应划分".to_string(),
                            confidence: 0.78,
                            is_highlight: false,
                            score: None,
                        });
                        prev_ms = cut;
                        idx += 1;
                    }
                }

                if resolved_duration_ms - prev_ms >= 3000 {
                    recs.push(SegmentRecommendation {
                        id: format!("silence_cut_{}", idx),
                        title: format!("视频阶段 {:02} (尾段)", idx),
                        start_ms: prev_ms,
                        end_ms: resolved_duration_ms,
                        reason: "音频静音间歇自适应划分".to_string(),
                        confidence: 0.78,
                        is_highlight: false,
                        score: None,
                    });
                }

                if recs.len() >= 2 {
                    mark_highlights_on_chapters(&mut recs);
                    return Ok(RecommendSegmentsResponse {
                        segments: recs,
                        source: "silence_detect".to_string(),
                        message: "已基于音频静音与停顿检测生成分段建议".to_string(),
                        highlights,
                    });
                }
            }
        }
    }

    // -------------------------------------------------------------
    // Strategy 4: Smart Rhythm / Pacing Fallback
    // -------------------------------------------------------------
    let total_sec = resolved_duration_ms as f64 / 1000.0;
    let parts_count = if total_sec < 30.0 {
        2
    } else if total_sec < 120.0 {
        3
    } else if total_sec < 600.0 {
        4
    } else {
        5
    };

    let titles = [
        "精彩开场与引言",
        "核心焦点展开",
        "高潮片段与实操",
        "精选细节与进阶",
        "总结归纳与结语",
    ];

    let chunk_ms = resolved_duration_ms / (parts_count as i64);
    let mut recs = Vec::new();

    for i in 0..parts_count {
        let start = (i as i64) * chunk_ms;
        let end = if i == parts_count - 1 {
            resolved_duration_ms
        } else {
            ((i + 1) as i64) * chunk_ms
        };

        let t = if i < titles.len() {
            titles[i].to_string()
        } else {
            format!("重点阶段 {}", i + 1)
        };

        recs.push(SegmentRecommendation {
            id: format!("pacing_seg_{}", i + 1),
            title: t,
            start_ms: start,
            end_ms: end,
            reason: "智能时长节奏黄金比例切片".to_string(),
            confidence: 0.68,
            is_highlight: false,
            score: None,
        });
    }

    mark_highlights_on_chapters(&mut recs);

    Ok(RecommendSegmentsResponse {
        segments: recs,
        source: "pacing".to_string(),
        message: "已基于视频黄金时长比例生成智能分段建议".to_string(),
        highlights,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_part() {
        assert_eq!(sanitize_filename_part("精彩镜头-01"), "精彩镜头-01");
        assert_eq!(sanitize_filename_part("hello world!@#$%"), "hello_world");
        assert_eq!(sanitize_filename_part(""), "clip");
    }

    #[test]
    fn test_resolve_unique_path() {
        let temp_dir = std::env::temp_dir();
        let path1 = resolve_unique_path(&temp_dir, "test_clip_unit.mp4");
        assert!(path1.to_string_lossy().contains("test_clip_unit.mp4"));
    }

    #[test]
    fn test_recommend_video_segments_fallback() {
        let dummy_path = PathBuf::from("non_existent_video.mp4");
        let res = recommend_video_segments(None, &dummy_path, Some(120_000), None, None, None).unwrap();
        assert_eq!(res.segments.len(), 4);
        assert_eq!(res.source, "pacing");
        assert_eq!(res.segments[0].start_ms, 0);
        assert_eq!(res.segments[3].end_ms, 120_000);
        assert!(!res.highlights.is_empty());
    }

    #[test]
    fn test_recommend_video_highlights_mode() {
        let dummy_path = PathBuf::from("non_existent_video.mp4");
        let res = recommend_video_segments(None, &dummy_path, Some(120_000), None, None, Some("highlights")).unwrap();
        assert!(!res.segments.is_empty());
        assert!(res.segments[0].is_highlight);
        assert_eq!(res.source, "ai_climax");
    }
}
