use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: String,
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsInfo {
    pub os: String,
    pub arch: String,
    pub family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageStatus {
    pub app_data_dir: String,
    pub app_data_writable: bool,
    pub temp_dir: String,
    pub temp_writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthReport {
    pub app_name: String,
    pub app_version: String,
    pub status: String, // "healthy" | "degraded"
    pub os: OsInfo,
    pub tools: Vec<ToolStatus>,
    pub storage: StorageStatus,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YtDlpUpdateCheck {
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub executable_path: Option<String>,
    pub is_managed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YtDlpUpdateResult {
    pub success: bool,
    pub previous_version: Option<String>,
    pub current_version: Option<String>,
    pub executable_path: String,
    pub method: String, // "native_self_update" | "github_download" | "already_up_to_date"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub created_at: i64,
    pub video_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub rules_json: Option<String>,
    pub video_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Video {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_hash: String,
    pub file_size: u64,
    pub duration: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: String,
    pub thumbnail_path: Option<String>,
    pub thumbnail_data: Option<String>, // Base64 data URL for frontend rendering
    pub is_starred: bool,
    pub watch_status: String, // "unwatched" | "in_progress" | "watched"
    pub playback_progress: f64, // 0.0 ~ 1.0
    pub color_tag: Option<String>, // "red" | "yellow" | "green" | "blue"
    pub last_played_at: Option<i64>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoFilter {
    pub search: Option<String>,
    pub codec: Option<String>,
    pub sort_by: Option<String>, // "created_desc", "duration_desc", "size_desc", "name_asc"
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub format_category: Option<String>, // "all" | "shorts" | "movies" | "series" | "screen"
    pub quality: Option<String>,          // "all" | "4k" | "1080p" | "720p" | "sd"
    pub duration_range: Option<String>,   // "all" | "under_3m" | "3_to_30m" | "over_30m"
    pub watch_status: Option<String>,     // "all" | "unwatched" | "in_progress" | "watched"
    pub is_starred: Option<bool>,
    pub color_tag: Option<String>,
    pub tag_id: Option<i64>,
    pub collection_id: Option<i64>,
    pub source_folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceFolderStat {
    pub folder_path: String,
    pub folder_name: String,
    pub video_count: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategoryStats {
    pub total_videos: usize,
    pub total_size_bytes: u64,
    pub shorts_count: usize,
    pub movies_count: usize,
    pub series_count: usize,
    pub screen_count: usize,
    pub unwatched_count: usize,
    pub in_progress_count: usize,
    pub watched_count: usize,
    pub starred_count: usize,
    pub tags: Vec<Tag>,
    pub collections: Vec<Collection>,
    pub source_folders: Vec<SourceFolderStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoListResponse {
    pub items: Vec<Video>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub scanned_dir: String,
    pub total_files_examined: usize,
    pub video_files_found: usize,
    pub new_imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub phase: String, // "discovering" | "indexing" | "enriching" | "completed" | "cancelled"
    pub message: String,
    pub total_found: usize,
    pub processed: usize,
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormatOption {
    pub format_id: String,
    pub ext: Option<String>,
    pub resolution: Option<String>,
    pub note: Option<String>,
    pub filesize: Option<u64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub tbr: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UrlInspectResult {
    pub url: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub extractor: Option<String>,
    pub formats: Vec<FormatOption>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub platform: Option<String>,
    pub format_id: Option<String>,
    pub status: String, // "pending" | "downloading" | "paused" | "completed" | "error" | "cancelled"
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub speed_str: Option<String>,
    pub eta: Option<String>,
    pub file_path: Option<String>,
    pub error_log: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoSource {
    pub id: i64,
    pub video_id: i64,
    pub original_url: Option<String>,
    pub platform: Option<String>,
    pub author: Option<String>,
    pub download_task_id: Option<String>,
    pub downloaded_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppUpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_name: String,
    pub release_notes: String,
    pub download_url: String,
    pub is_differential: bool,
    pub download_size_bytes: u64,
    pub original_size_bytes: u64,
    pub sha256: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateProgressPayload {
    pub status: String, // "downloading" | "verifying" | "ready" | "error"
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub speed_mbps: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateInstallResult {
    pub success: bool,
    pub message: String,
    pub requires_restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateGroup {
    pub group_id: String,
    pub match_reason: String, // "exact_hash" | "metadata" | "fuzzy_name"
    pub match_label: String,
    pub total_size: u64,
    pub reclaimable_size: u64,
    pub videos: Vec<Video>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateResolutionAction {
    pub group_id: String,
    pub keep_video_id: i64,
    pub delete_video_ids: Vec<i64>,
    pub delete_disk_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolutionSummary {
    pub groups_processed: usize,
    pub videos_removed: usize,
    pub bytes_reclaimed: u64,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioExtractionOptions {
    pub video_path: String,
    pub video_id: Option<i64>,
    pub format: String, // "mp3" | "m4a" | "wav" | "flac"
    pub quality_bitrate: Option<String>, // "320k" | "192k" | "128k" | "256k"
    pub copy_stream: Option<bool>, // copy audio stream directly without re-encoding
    pub output_dir: Option<String>, // custom directory or default video folder
    pub custom_filename: Option<String>, // optional custom filename without extension
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioExtractionResult {
    pub success: bool,
    pub output_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub duration: Option<f64>,
    pub format: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedAudio {
    pub id: i64,
    pub video_id: Option<i64>,
    pub source_video_title: Option<String>,
    pub source_video_path: Option<String>,
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub duration: Option<f64>,
    pub format: String,
    pub quality_bitrate: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedAudioFilter {
    pub search: Option<String>,
    pub format: Option<String>,
    pub sort_by: Option<String>, // "created_desc", "duration_desc", "size_desc", "name_asc"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VideoClipSegment {
    pub id: String,
    pub title: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipVideoRequest {
    pub video_id: Option<i64>,
    pub video_path: String,
    pub segments: Vec<VideoClipSegment>,
    pub output_dir: Option<String>,
    pub export_mode: String, // "separate" | "merge" | "both"
    pub fast_copy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipResultItem {
    pub id: String,
    pub title: String,
    pub output_path: String,
    pub duration_sec: f64,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipVideoResponse {
    pub success: bool,
    pub clips: Vec<ClipResultItem>,
    pub merged_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRecommendation {
    pub id: String,
    pub title: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub reason: String,
    pub confidence: f32,
    #[serde(default)]
    pub is_highlight: bool,
    #[serde(default)]
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendSegmentsResponse {
    pub segments: Vec<SegmentRecommendation>,
    pub source: String,
    pub message: String,
    #[serde(default)]
    pub highlights: Vec<SegmentRecommendation>,
}





