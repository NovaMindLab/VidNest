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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoFilter {
    pub search: Option<String>,
    pub codec: Option<String>,
    pub sort_by: Option<String>, // "created_desc", "duration_desc", "size_desc", "name_asc"
    pub page: Option<usize>,
    pub page_size: Option<usize>,
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


