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

/// AI-assisted smart segmentation recommender:
/// 1. Uses existing AI Analysis Topics (semantic chapters with titles)
/// 2. Uses Subtitle pause clustering (natural sentence boundaries)
/// 3. Uses FFmpeg silence detection
/// 4. Fallback to smart duration pacing
pub fn recommend_video_segments(
    video_id: Option<i64>,
    video_path: &Path,
    duration_ms: Option<i64>,
    conn: Option<&Connection>,
    app_data_dir: Option<&Path>,
) -> Result<RecommendSegmentsResponse, String> {
    let resolved_duration_ms = duration_ms.unwrap_or_else(|| {
        if let Ok(meta) = crate::media::extract_metadata(video_path) {
            (meta.duration.unwrap_or(60.0) * 1000.0) as i64
        } else {
            60_000
        }
    });

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
                    });
                }

                if !recs.is_empty() {
                    return Ok(RecommendSegmentsResponse {
                        segments: recs,
                        source: "ai_topics".to_string(),
                        message: "已基于 AI 深度语义分析生成智能章节推荐".to_string(),
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
                    });
                }

                if recs.len() >= 2 {
                    return Ok(RecommendSegmentsResponse {
                        segments: recs,
                        source: "subtitles".to_string(),
                        message: "已基于 AI 语音转写与自然断句停顿生成分段推荐".to_string(),
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

            // If we detected between 1 and 8 silence points, construct clean segments
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
                    });
                }

                if recs.len() >= 2 {
                    return Ok(RecommendSegmentsResponse {
                        segments: recs,
                        source: "silence_detect".to_string(),
                        message: "已基于音频静音与停顿检测生成分段建议".to_string(),
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
        });
    }

    Ok(RecommendSegmentsResponse {
        segments: recs,
        source: "pacing".to_string(),
        message: "已基于视频黄金时长比例生成智能分段建议".to_string(),
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
        let res = recommend_video_segments(None, &dummy_path, Some(120_000), None, None).unwrap();
        assert_eq!(res.segments.len(), 4);
        assert_eq!(res.source, "pacing");
        assert_eq!(res.segments[0].start_ms, 0);
        assert_eq!(res.segments[3].end_ms, 120_000);
    }
}
