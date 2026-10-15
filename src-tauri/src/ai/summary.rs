use crate::ai::whisper::AiTranscriptionConfig;
use crate::db::subtitles::SubtitleSegment;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTopicItem {
    pub timestamp_ms: i64,
    pub title: String,
    pub point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysis {
    pub id: Option<i64>,
    pub video_id: i64,
    pub summary: String,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub topics: Vec<AiTopicItem>,
    pub model_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAiStatus {
    pub video_id: i64,
    pub has_subtitles: bool,
    pub subtitle_count: usize,
    pub has_analysis: bool,
    pub summary_preview: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSummaryPayload {
    pub text: String,
    #[serde(default)]
    pub topics: Vec<AiTopicItem>,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ==========================================
// 1. Database Operations for ai_analyses
// ==========================================

pub fn get_video_ai_analysis(
    conn: &Connection,
    video_id: i64,
) -> Result<Option<AiAnalysis>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, video_id, summary, keywords, tags, model_name, created_at
             FROM ai_analyses
             WHERE video_id = ?1",
        )
        .map_err(|e| format!("Failed to prepare get_video_ai_analysis: {}", e))?;

    let mut rows = stmt
        .query(params![video_id])
        .map_err(|e| format!("Failed to query ai_analyses: {}", e))?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let id: i64 = row.get(0).map_err(|e| e.to_string())?;
        let v_id: i64 = row.get(1).map_err(|e| e.to_string())?;
        let raw_summary: String = row.get(2).unwrap_or_default();
        let raw_keywords: String = row.get(3).unwrap_or_default();
        let raw_tags: String = row.get(4).unwrap_or_default();
        let model_name: String = row
            .get(5)
            .unwrap_or_else(|_| "local-heuristic".to_string());
        let created_at: i64 = row.get(6).unwrap_or_else(|_| now_millis());

        // Parse summary & topics
        let (summary_text, topics) = if raw_summary.trim_start().starts_with('{') {
            if let Ok(payload) = serde_json::from_str::<StoredSummaryPayload>(&raw_summary) {
                (payload.text, payload.topics)
            } else {
                (raw_summary, Vec::new())
            }
        } else {
            (raw_summary, Vec::new())
        };

        // Parse keywords
        let keywords: Vec<String> = if raw_keywords.trim_start().starts_with('[') {
            serde_json::from_str(&raw_keywords).unwrap_or_default()
        } else {
            raw_keywords
                .split(|c| c == ',' || c == '，' || c == ';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        // Parse tags
        let tags: Vec<String> = if raw_tags.trim_start().starts_with('[') {
            serde_json::from_str(&raw_tags).unwrap_or_default()
        } else {
            raw_tags
                .split(|c| c == ',' || c == '，' || c == ';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        Ok(Some(AiAnalysis {
            id: Some(id),
            video_id: v_id,
            summary: summary_text,
            keywords,
            tags,
            topics,
            model_name,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

pub fn save_video_ai_analysis(
    conn: &mut Connection,
    analysis: &AiAnalysis,
) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to begin tx: {}", e))?;

    let payload = StoredSummaryPayload {
        text: analysis.summary.clone(),
        topics: analysis.topics.clone(),
    };
    let summary_json = serde_json::to_string(&payload)
        .map_err(|e| format!("Failed to serialize summary payload: {}", e))?;

    let keywords_json = serde_json::to_string(&analysis.keywords).unwrap_or_default();
    let tags_json = serde_json::to_string(&analysis.tags).unwrap_or_default();

    tx.execute(
        "INSERT INTO ai_analyses (video_id, summary, keywords, tags, model_name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(video_id) DO UPDATE SET
             summary = excluded.summary,
             keywords = excluded.keywords,
             tags = excluded.tags,
             model_name = excluded.model_name,
             created_at = excluded.created_at",
        params![
            analysis.video_id,
            summary_json,
            keywords_json,
            tags_json,
            analysis.model_name,
            analysis.created_at,
        ],
    )
    .map_err(|e| format!("Failed to insert/update ai_analyses: {}", e))?;

    // Update FTS5 index (preserve title & subtitles)
    let (title, subtitles): (String, String) = tx
        .query_row(
            "SELECT COALESCE(title, ''), COALESCE(subtitles, '')
             FROM fts_index WHERE video_id = ?1",
            params![analysis.video_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .or_else(|_| {
            tx.query_row(
                "SELECT file_name FROM videos WHERE id = ?1",
                params![analysis.video_id],
                |row| Ok((row.get(0)?, String::new())),
            )
        })
        .unwrap_or_else(|_| (String::new(), String::new()));

    let _ = tx.execute(
        "DELETE FROM fts_index WHERE video_id = ?1",
        params![analysis.video_id.to_string()],
    );

    let mut searchable_summary = analysis.summary.clone();
    for t in &analysis.topics {
        searchable_summary.push(' ');
        searchable_summary.push_str(&t.title);
        searchable_summary.push(' ');
        searchable_summary.push_str(&t.point);
    }
    for kw in &analysis.keywords {
        searchable_summary.push(' ');
        searchable_summary.push_str(kw);
    }

    let joined_tags = analysis.tags.join(" ");

    tx.execute(
        "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            analysis.video_id.to_string(),
            title,
            subtitles,
            searchable_summary,
            joined_tags,
        ],
    )
    .map_err(|e| format!("Failed to update FTS5 ai_analyses: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit ai_analyses: {}", e))?;

    Ok(())
}

pub fn get_videos_ai_status(
    conn: &Connection,
    video_ids: &[i64],
) -> Result<HashMap<i64, VideoAiStatus>, String> {
    let mut map: HashMap<i64, VideoAiStatus> = HashMap::new();
    if video_ids.is_empty() {
        return Ok(map);
    }

    // Initialize map entries
    for id in video_ids {
        map.insert(
            *id,
            VideoAiStatus {
                video_id: *id,
                has_subtitles: false,
                subtitle_count: 0,
                has_analysis: false,
                summary_preview: None,
                tags: Vec::new(),
            },
        );
    }

    // 1. Fetch subtitle counts
    let id_placeholders = video_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sub_sql = format!(
        "SELECT video_id, COUNT(*) FROM subtitles WHERE video_id IN ({}) GROUP BY video_id",
        id_placeholders
    );

    let mut stmt = conn
        .prepare(&sub_sql)
        .map_err(|e| format!("Failed to prepare subtitle count query: {}", e))?;

    let sub_params = rusqlite::params_from_iter(video_ids.iter());
    let mut rows = stmt
        .query(sub_params)
        .map_err(|e| format!("Failed to query subtitle counts: {}", e))?;

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let v_id: i64 = row.get(0).map_err(|e| e.to_string())?;
        let count: i64 = row.get(1).map_err(|e| e.to_string())?;
        if let Some(st) = map.get_mut(&v_id) {
            st.has_subtitles = count > 0;
            st.subtitle_count = count as usize;
        }
    }

    // 2. Fetch AI analyses
    let ai_sql = format!(
        "SELECT video_id, summary, tags FROM ai_analyses WHERE video_id IN ({})",
        id_placeholders
    );
    let mut stmt2 = conn
        .prepare(&ai_sql)
        .map_err(|e| format!("Failed to prepare ai_analyses query: {}", e))?;

    let ai_params = rusqlite::params_from_iter(video_ids.iter());
    let mut rows2 = stmt2
        .query(ai_params)
        .map_err(|e| format!("Failed to query ai_analyses: {}", e))?;

    while let Some(row) = rows2.next().map_err(|e| e.to_string())? {
        let v_id: i64 = row.get(0).map_err(|e| e.to_string())?;
        let raw_summary: String = row.get(1).unwrap_or_default();
        let raw_tags: String = row.get(2).unwrap_or_default();

        let (summary_text, _) = if raw_summary.trim_start().starts_with('{') {
            if let Ok(payload) = serde_json::from_str::<StoredSummaryPayload>(&raw_summary) {
                (payload.text, payload.topics)
            } else {
                (raw_summary, Vec::new())
            }
        } else {
            (raw_summary, Vec::new())
        };

        let tags: Vec<String> = if raw_tags.trim_start().starts_with('[') {
            serde_json::from_str(&raw_tags).unwrap_or_default()
        } else {
            raw_tags
                .split(|c| c == ',' || c == '，' || c == ';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        if let Some(st) = map.get_mut(&v_id) {
            st.has_analysis = !summary_text.trim().is_empty();
            st.summary_preview = if st.has_analysis {
                let preview = summary_text.chars().take(80).collect::<String>();
                Some(preview)
            } else {
                None
            };
            st.tags = tags;
        }
    }

    Ok(map)
}

// ==========================================
// 2. Extractive & LLM AI Analysis Engine
// ==========================================

/// Generates an AI analysis using either an external LLM (Ollama / OpenAI / compatible API)
/// or high-quality local heuristic extractive summarizer.
pub async fn generate_ai_analysis_pipeline(
    video_id: i64,
    video_title: &str,
    video_duration_ms: Option<i64>,
    segments: &[SubtitleSegment],
    config: &AiTranscriptionConfig,
) -> Result<AiAnalysis, String> {
    // 1. Check if LLM API is available and try calling it
    let llm_url = config
        .llm_api_url
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if !config.api_url.trim().is_empty() && config.engine == "api" {
                Some(config.api_url.as_str())
            } else {
                None
            }
        });

    if let Some(api_base) = llm_url {
        let api_key = config
            .llm_api_key
            .as_deref()
            .unwrap_or(config.api_key.as_str());
        let model = config
            .llm_model
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("gpt-4o-mini");

        match call_llm_summarize(api_base, api_key, model, video_title, video_duration_ms, segments).await {
            Ok(mut analysis) => {
                analysis.video_id = video_id;
                analysis.created_at = now_millis();
                return Ok(analysis);
            }
            Err(e) => {
                eprintln!(
                    "[AI Summarizer] LLM API call failed ({}), falling back to local extractor: {}",
                    api_base, e
                );
            }
        }
    }

    // 2. Fallback: High-quality local extractive NLP summarizer (zero-config, 100% offline!)
    Ok(generate_local_extractive_analysis(
        video_id,
        video_title,
        video_duration_ms,
        segments,
    ))
}

/// Fallback local NLP heuristic summarizer:
/// Extracts keywords using word frequency and stopword filtering,
/// clusters timeline intervals to generate topics with timestamps,
/// and builds a cohesive summary and tags.
pub fn generate_local_extractive_analysis(
    video_id: i64,
    video_title: &str,
    video_duration_ms: Option<i64>,
    segments: &[SubtitleSegment],
) -> AiAnalysis {
    let stopwords: HashSet<&'static str> = [
        "的", "了", "是", "在", "我", "有", "和", "就", "不", "人", "都", "一", "一个", "上",
        "也", "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好", "自己", "这",
        "它", "他们", "我们", "你们", "这个", "那个", "因为", "所以", "如果", "但是", "然后",
        "可以", "或者", "进行", "通过", "对于", "关于", "作为", "并且", "以及", "还是", "为了",
        "之后", "之前", "这样", "那样", "现在", "已经", "其实", "大家", "可能", "应该", "就是",
        "the", "a", "an", "is", "are", "was", "were", "to", "of", "and", "in", "that", "have",
        "for", "it", "on", "with", "as", "at", "this", "but", "his", "by", "from", "they", "we",
        "say", "her", "she", "or", "will", "my", "one", "all", "would", "there", "their", "what",
        "so", "up", "out", "if", "about", "who", "get", "which", "go", "me", "when", "make", "can",
        "like", "time", "no", "just", "him", "know", "take", "people", "into", "year", "your", "good",
        "1080p", "720p", "4k", "2k", "x264", "x265", "hevc", "aac", "mp3", "h264", "bluray", "web-dl",
        "remux", "hdr", "10bit", "dvdrip", "hdrip", "bdrip", "mp4", "mkv", "avi", "mov", "flv", "wmv",
    ]
    .iter()
    .cloned()
    .collect();

    let clean_title = video_title
        .rsplit_once('.')
        .map(|(t, _)| t)
        .unwrap_or(video_title);

    let mut word_counts: HashMap<String, usize> = HashMap::new();
    let total_segs = segments.len();

    if total_segs > 0 {
        // 1. Word frequency extraction from subtitle dialogue
        for seg in segments {
            let text = seg.content.trim();
            if text.is_empty() {
                continue;
            }

            let chars: Vec<char> = text.chars().collect();
            let len = chars.len();

            // 2-gram to 3-gram for Chinese
            for n in 2..=3 {
                for i in 0..len {
                    if i + n <= len {
                        let gram: String = chars[i..i + n].iter().collect();
                        let gram_lower = gram.to_lowercase();
                        if !stopwords.contains(gram_lower.as_str())
                            && !gram.chars().any(|c| c.is_ascii_punctuation() || c.is_whitespace() || is_zh_punct(c))
                        {
                            *word_counts.entry(gram).or_insert(0) += 1;
                        }
                    }
                }
            }

            // English words
            for word in text.split(|c: char| !c.is_alphanumeric()) {
                let w = word.trim().to_lowercase();
                if w.len() >= 3 && !stopwords.contains(w.as_str()) {
                    *word_counts.entry(w).or_insert(0) += 1;
                }
            }
        }
    } else {
        // Fallback: extract keywords and tokens from cleaned video title
        let chars: Vec<char> = clean_title.chars().collect();
        let len = chars.len();

        for n in 2..=4 {
            for i in 0..len {
                if i + n <= len {
                    let gram: String = chars[i..i + n].iter().collect();
                    let gram_lower = gram.to_lowercase();
                    if !stopwords.contains(gram_lower.as_str())
                        && !gram.chars().any(|c| c.is_ascii_punctuation() || c.is_whitespace() || is_zh_punct(c))
                    {
                        *word_counts.entry(gram).or_insert(0) += 1;
                    }
                }
            }
        }

        for word in clean_title.split(|c: char| !c.is_alphanumeric()) {
            let w = word.trim().to_lowercase();
            if w.len() >= 2 && !stopwords.contains(w.as_str()) {
                *word_counts.entry(w).or_insert(0) += 1;
            }
        }
    }

    // Rank top keywords
    let mut sorted_words: Vec<(String, usize)> = word_counts.into_iter().collect();
    sorted_words.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.len().cmp(&a.0.len())));

    let mut keywords: Vec<String> = sorted_words
        .iter()
        .map(|(w, _)| w.clone())
        .take(6)
        .collect();

    if keywords.is_empty() {
        if clean_title.chars().count() >= 2 {
            keywords.push(clean_title.to_string());
        } else {
            keywords.push("视频内容".to_string());
        }
    }

    // 2. Timeline cluster & topic key takeaways
    let mut topics = Vec::new();

    if total_segs > 0 {
        let first_seg = &segments[0];
        let last_seg = &segments[total_segs - 1];
        let total_duration_ms = (last_seg.end_time_ms - first_seg.start_time_ms).max(1000);

        let num_topics = if total_duration_ms > 15 * 60 * 1000 {
            5
        } else if total_duration_ms > 5 * 60 * 1000 {
            4
        } else if total_segs >= 6 {
            3
        } else {
            1
        };

        let chunk_size = (total_segs + num_topics - 1) / num_topics;

        let default_titles = [
            "开场与背景引入",
            "核心议题阐述",
            "重点细节与案例展开",
            "关键观点与技术解析",
            "成果总结与未来展望",
        ];

        for i in 0..num_topics {
            let start_idx = i * chunk_size;
            let end_idx = ((i + 1) * chunk_size).min(total_segs);
            if start_idx >= total_segs {
                break;
            }

            let slice = &segments[start_idx..end_idx];
            if slice.is_empty() {
                continue;
            }

            // Pick the best representative sentence in this chunk
            let best_seg = slice
                .iter()
                .max_by_key(|s| {
                    let content = s.content.trim();
                    let kw_score = keywords
                        .iter()
                        .filter(|kw| content.contains(kw.as_str()))
                        .count()
                        * 10;
                    let len_score = if content.len() >= 10 && content.len() <= 60 {
                        5
                    } else {
                        1
                    };
                    kw_score + len_score
                })
                .unwrap_or(&slice[0]);

            let title_label = if i < default_titles.len() {
                default_titles[i].to_string()
            } else {
                format!("阶段节点 {}", i + 1)
            };

            topics.push(AiTopicItem {
                timestamp_ms: best_seg.start_time_ms,
                title: title_label,
                point: best_seg.content.trim().to_string(),
            });
        }
    } else {
        // Generate topics based on video duration milestone percentages
        let dur_ms = video_duration_ms.unwrap_or(0);
        let kw_sample = keywords.first().cloned().unwrap_or_else(|| clean_title.to_string());
        let kw_sample2 = keywords.get(1).cloned().unwrap_or_else(|| kw_sample.clone());

        if dur_ms >= 60_000 {
            topics.push(AiTopicItem {
                timestamp_ms: 0,
                title: "开篇与内容导引".to_string(),
                point: format!("视频起始位置，引入《{}》核心议题与场景背景", clean_title),
            });
            topics.push(AiTopicItem {
                timestamp_ms: dur_ms / 4,
                title: "核心主题展开".to_string(),
                point: format!("重点推进，深入呈现「{}」相关内容", kw_sample),
            });
            topics.push(AiTopicItem {
                timestamp_ms: dur_ms / 2,
                title: "重点细节与演示".to_string(),
                point: format!("关键展示，围绕「{}」核心细节展开演示与深入剖析", kw_sample2),
            });
            topics.push(AiTopicItem {
                timestamp_ms: (dur_ms * 3) / 4,
                title: "总结回顾与尾声".to_string(),
                point: "全片重点回顾，核心要点总结与收尾片段".to_string(),
            });
        } else if dur_ms > 0 {
            topics.push(AiTopicItem {
                timestamp_ms: 0,
                title: "视频精彩开场".to_string(),
                point: format!("视频起始位置，快速引入《{}》", clean_title),
            });
            topics.push(AiTopicItem {
                timestamp_ms: dur_ms / 2,
                title: "核心亮点展现".to_string(),
                point: format!("精彩高光推进，展示「{}」核心细节", kw_sample),
            });
        } else {
            topics.push(AiTopicItem {
                timestamp_ms: 0,
                title: "视频主体全貌".to_string(),
                point: format!("视频起始片段与《{}》核心画面呈现", clean_title),
            });
        }
    }

    // 3. Build summary
    let summary = if total_segs > 0 {
        let topic_summary = topics
            .iter()
            .take(3)
            .map(|t| t.point.as_str())
            .collect::<Vec<_>>()
            .join("；");
        if !keywords.is_empty() {
            format!(
                "本视频《{}》围绕「{}」等核心展开，涵盖 {}，共包含 {} 句对白台词。",
                clean_title,
                keywords.iter().take(3).cloned().collect::<Vec<_>>().join("、"),
                topic_summary,
                total_segs
            )
        } else {
            format!(
                "本视频《{}》包含 {} 句对话台词，重点涉及：{}。",
                clean_title, total_segs, topic_summary
            )
        }
    } else {
        let dur_ms = video_duration_ms.unwrap_or(0);
        let kw_text = keywords.iter().take(3).cloned().collect::<Vec<_>>().join("、");
        if dur_ms > 0 {
            let dur_sec = dur_ms / 1000;
            let dur_str = format!("{:02}:{:02}", dur_sec / 60, dur_sec % 60);
            format!(
                "本视频《{}》（时长 {}）聚焦于「{}」等核心要点。VidNest 本地智能分析引擎已根据文件特征与元数据生成全局概览与时间轴节点。若需提取精确到毫秒的逐句对白台词，可点击下方「🎙️ 语音转写台词」进行全文声学识别。",
                clean_title, dur_str, kw_text
            )
        } else {
            format!(
                "本视频《{}》聚焦于「{}」等核心要点。VidNest 本地智能分析引擎已根据文件特征与元数据完成快速分析。若需提取精确到毫秒的逐句对白台词，可点击下方「🎙️ 语音转写台词」进行全文声学识别。",
                clean_title, kw_text
            )
        }
    };

    // 4. Generate recommended tags
    let mut tags = Vec::new();
    for kw in &keywords {
        if kw.chars().count() >= 2 && kw.chars().count() <= 8 && !tags.contains(kw) {
            tags.push(kw.clone());
        }
        if tags.len() >= 4 {
            break;
        }
    }
    if tags.is_empty() {
        tags.push("精选视频".to_string());
    }

    let model_name = if total_segs > 0 {
        "内置智能分析引擎".to_string()
    } else {
        "内置智能分析引擎 (本地元数据)".to_string()
    };

    AiAnalysis {
        id: None,
        video_id,
        summary,
        keywords,
        tags,
        topics,
        model_name,
        created_at: now_millis(),
    }
}

fn is_zh_punct(c: char) -> bool {
    matches!(
        c,
        '，' | '。' | '！' | '？' | '；' | '：' | '“' | '”' | '‘' | '’' | '（' | '）' | '【' | '】' | '《' | '》' | '、'
    )
}

// ==========================================
// 3. LLM API Integration (OpenAI / Ollama / DeepSeek)
// ==========================================

#[derive(Serialize)]
struct ChatCompletionMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatCompletionMessage>,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessageContent,
}

#[derive(Deserialize)]
struct ChatCompletionMessageContent {
    content: Option<String>,
}

#[derive(Deserialize)]
struct LlmAnalysisOutput {
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub topics: Vec<LlmTopicOutput>,
}

#[derive(Deserialize)]
struct LlmTopicOutput {
    pub timestamp_ms: Option<i64>,
    pub title: String,
    pub point: String,
}

async fn call_llm_summarize(
    api_base: &str,
    api_key: &str,
    model: &str,
    video_title: &str,
    video_duration_ms: Option<i64>,
    segments: &[SubtitleSegment],
) -> Result<AiAnalysis, String> {
    let prompt = if segments.is_empty() {
        let dur_hint = if let Some(d) = video_duration_ms {
            let sec = d / 1000;
            format!("，时长约 {} 分 {} 秒", sec / 60, sec % 60)
        } else {
            String::new()
        };
        format!(
            "你是一个专业的本地视频内容智能分析助手。根据以下视频的文件名和基本信息{}，推测并生成该视频的内容概述、阶段性核心看点时间轴（使用合理的相对时间戳毫秒数）与分类标签。\n\n\
            【视频标题】: {}\n\n\
            请务必仅返回符合以下 JSON 结构的纯 JSON 文本（不要包含多余文字）：\n\
            {{\n  \
              \"summary\": \"1至2句视频全局核心概述与可能内容介绍\",\n  \
              \"keywords\": [\"关键词1\", \"关键词2\", \"关键词3\"],\n  \
              \"tags\": [\"分类标签1\", \"分类标签2\"],\n  \
              \"topics\": [\n    \
                {{\"timestamp_ms\": 0, \"title\": \"阶段小标题\", \"point\": \"该段落核心内容点\"}}\n  \
              ]\n\
            }}",
            dur_hint, video_title
        )
    } else {
        // Format transcript with timestamps
        let mut transcript_lines = Vec::new();
        for seg in segments.iter().take(300) {
            let sec = seg.start_time_ms / 1000;
            let mm = sec / 60;
            let ss = sec % 60;
            transcript_lines.push(format!("[{:02}:{:02}] {}", mm, ss, seg.content.trim()));
        }
        let transcript_text = transcript_lines.join("\n");

        format!(
            "你是一个专业的本地视频内容智能分析助手。根据以下视频的标题与字幕时间轴，生成视频的内容概述、核心看点时间轴与分类标签。\n\n\
            【视频标题】: {}\n\n\
            【字幕时间轴台词】:\n{}\n\n\
            请务必仅返回符合以下 JSON 结构的纯 JSON 文本（不要包含多余文字）：\n\
            {{\n  \
              \"summary\": \"1至2句视频全局核心概述\",\n  \
              \"keywords\": [\"关键词1\", \"关键词2\", \"关键词3\"],\n  \
              \"tags\": [\"分类标签1\", \"分类标签2\"],\n  \
              \"topics\": [\n    \
                {{\"timestamp_ms\": 0, \"title\": \"阶段小标题\", \"point\": \"该段落核心内容点\"}}\n  \
              ]\n\
            }}",
            video_title, transcript_text
        )
    };

    let endpoint = if api_base.ends_with('/') {
        format!("{}chat/completions", api_base)
    } else {
        format!("{}/chat/completions", api_base)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.post(&endpoint);
    if !api_key.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
    }

    let payload = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatCompletionMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        temperature: 0.3,
    };

    let resp = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", endpoint, e))?;

    if !resp.status().is_success() {
        let err_txt = resp.text().await.unwrap_or_default();
        return Err(format!("LLM API returned error: {}", err_txt));
    }

    let parsed_resp: ChatCompletionResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM JSON response: {}", e))?;

    let raw_text = parsed_resp
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .ok_or_else(|| "LLM returned empty choices".to_string())?;

    // Strip markdown code block if present
    let json_text = raw_text
        .trim()
        .strip_prefix("```json")
        .or_else(|| raw_text.trim().strip_prefix("```"))
        .unwrap_or(&raw_text)
        .trim()
        .strip_suffix("```")
        .unwrap_or(&raw_text)
        .trim();

    let llm_data: LlmAnalysisOutput = serde_json::from_str(json_text)
        .map_err(|e| format!("Failed to parse structured LLM JSON: {}. Raw: {}", e, json_text))?;

    let topics = llm_data
        .topics
        .into_iter()
        .map(|t| AiTopicItem {
            timestamp_ms: t.timestamp_ms.unwrap_or(0),
            title: t.title,
            point: t.point,
        })
        .collect();

    Ok(AiAnalysis {
        id: None,
        video_id: 0,
        summary: llm_data.summary,
        keywords: llm_data.keywords,
        tags: llm_data.tags,
        topics,
        model_name: format!("LLM ({})", model),
        created_at: now_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE videos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                duration REAL,
                width INTEGER,
                height INTEGER,
                codec TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE subtitles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                video_id INTEGER NOT NULL,
                language TEXT,
                format TEXT,
                start_time_ms INTEGER NOT NULL,
                end_time_ms INTEGER NOT NULL,
                content TEXT NOT NULL
            );
            CREATE TABLE ai_analyses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                video_id INTEGER NOT NULL UNIQUE REFERENCES videos(id) ON DELETE CASCADE,
                summary TEXT,
                keywords TEXT,
                tags TEXT,
                model_name TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE fts_index USING fts5(
                video_id UNINDEXED,
                title,
                subtitles,
                summary,
                tags
            );
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_local_extractive_analysis() {
        let segs = vec![
            SubtitleSegment {
                id: Some(1),
                video_id: 1,
                language: Some("zh".into()),
                format: Some("vtt".into()),
                start_time_ms: 1000,
                end_time_ms: 4000,
                content: "欢迎收看本地智能视频分析系统发布会。".into(),
            },
            SubtitleSegment {
                id: Some(2),
                video_id: 1,
                language: Some("zh".into()),
                format: Some("vtt".into()),
                start_time_ms: 5000,
                end_time_ms: 9000,
                content: "今天我们将深度演示视频AI内容理解和字幕搜索。".into(),
            },
            SubtitleSegment {
                id: Some(3),
                video_id: 1,
                language: Some("zh".into()),
                format: Some("vtt".into()),
                start_time_ms: 10000,
                end_time_ms: 15000,
                content: "无论是否联网，本地算法都能秒级提取核心要点和视频标签。".into(),
            },
        ];

        let analysis = generate_local_extractive_analysis(1, "VidNest_Intro.mp4", Some(15000), &segs);
        assert!(!analysis.summary.is_empty());
        assert!(!analysis.topics.is_empty());
        assert!(!analysis.tags.is_empty());
        assert_eq!(analysis.video_id, 1);
        assert_eq!(analysis.model_name, "内置智能分析引擎");
    }

    #[test]
    fn test_local_extractive_analysis_without_subtitles() {
        let segs: Vec<SubtitleSegment> = Vec::new();
        let analysis = generate_local_extractive_analysis(
            2,
            "2024_VidNest_AI产品发布会与架构演进_1080p.mp4",
            Some(120_000),
            &segs,
        );
        assert!(!analysis.summary.is_empty());
        assert!(!analysis.topics.is_empty());
        assert!(!analysis.tags.is_empty());
        assert_eq!(analysis.video_id, 2);
        assert_eq!(analysis.model_name, "内置智能分析引擎 (本地元数据)");
        // Topics should have 4 milestone sections for 120s video
        assert_eq!(analysis.topics.len(), 4);
        assert_eq!(analysis.topics[0].timestamp_ms, 0);
        assert_eq!(analysis.topics[1].timestamp_ms, 30_000);
    }

    #[test]
    fn test_db_save_and_retrieve_ai_analysis() {
        let mut conn = setup_test_db();
        conn.execute(
            "INSERT INTO videos (id, file_path, file_name, file_hash, file_size, created_at)
             VALUES (1, '/path/vid.mp4', 'vid.mp4', 'hash123', 1024, 1000)",
            [],
        )
        .unwrap();

        let analysis = AiAnalysis {
            id: None,
            video_id: 1,
            summary: "这是一个测试视频摘要".to_string(),
            keywords: vec!["测试".to_string(), "架构".to_string()],
            tags: vec!["开发".to_string(), "VidNest".to_string()],
            topics: vec![AiTopicItem {
                timestamp_ms: 5000,
                title: "第一章".to_string(),
                point: "开场介绍内容".to_string(),
            }],
            model_name: "test-model".to_string(),
            created_at: 12345678,
        };

        save_video_ai_analysis(&mut conn, &analysis).unwrap();

        let retrieved = get_video_ai_analysis(&conn, 1).unwrap().expect("Should exist");
        assert_eq!(retrieved.summary, "这是一个测试视频摘要");
        assert_eq!(retrieved.keywords.len(), 2);
        assert_eq!(retrieved.tags.len(), 2);
        assert_eq!(retrieved.topics.len(), 1);
        assert_eq!(retrieved.topics[0].timestamp_ms, 5000);

        // Test status query
        let status_map = get_videos_ai_status(&conn, &[1]).unwrap();
        let st = status_map.get(&1).expect("Status exists");
        assert!(st.has_analysis);
        assert_eq!(st.tags.len(), 2);
    }
}
