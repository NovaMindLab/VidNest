export interface ToolStatus {
  name: string;
  found: boolean;
  path: string | null;
  version: string | null;
  required: boolean;
  description: string;
}

export interface OsInfo {
  os: string;
  arch: string;
  family: string;
}

export interface StorageStatus {
  app_data_dir: string;
  app_data_writable: boolean;
  temp_dir: string;
  temp_writable: boolean;
}

export interface HealthReport {
  app_name: string;
  app_version: string;
  status: 'healthy' | 'degraded';
  os: OsInfo;
  tools: ToolStatus[];
  storage: StorageStatus;
  timestamp_ms: number;
}

export interface YtDlpUpdateCheck {
  current_version: string | null;
  latest_version: string | null;
  has_update: boolean;
  executable_path: string | null;
  is_managed: boolean;
  message: string;
}

export interface YtDlpUpdateResult {
  success: boolean;
  previous_version: string | null;
  current_version: string | null;
  executable_path: string;
  method: string;
  message: string;
}

export interface Tag {
  id: number;
  name: string;
  color?: string | null;
  created_at: number;
  video_count?: number;
}

export interface Collection {
  id: number;
  name: string;
  description?: string | null;
  is_smart: boolean;
  rules_json?: string | null;
  video_count: number;
  created_at: number;
  updated_at: number;
}

export interface SourceFolderStat {
  folder_path: string;
  folder_name: string;
  video_count: number;
  total_size_bytes: number;
}

export interface CategoryStats {
  total_videos: number;
  total_size_bytes: number;
  shorts_count: number;
  movies_count: number;
  series_count: number;
  screen_count: number;
  unwatched_count: number;
  in_progress_count: number;
  watched_count: number;
  starred_count: number;
  tags: Tag[];
  collections: Collection[];
  source_folders: SourceFolderStat[];
}

export interface Video {
  id: number;
  file_path: string;
  file_name: string;
  file_hash: string;
  file_size: number;
  duration: number | null;
  width: number | null;
  height: number | null;
  codec: string | null;
  created_at: number;
  updated_at: number;
  status: string;
  thumbnail_path: string | null;
  thumbnail_data: string | null;
  is_starred: boolean;
  watch_status: 'unwatched' | 'in_progress' | 'watched';
  playback_progress: number;
  color_tag?: string | null;
  last_played_at?: number | null;
  tags: Tag[];
}

export interface VideoFilter {
  search?: string;
  codec?: string;
  sort_by?: string;
  page?: number;
  page_size?: number;
  format_category?: string;
  quality?: string;
  duration_range?: string;
  watch_status?: string;
  is_starred?: boolean;
  color_tag?: string;
  tag_id?: number;
  collection_id?: number;
  source_folder?: string;
}

export interface VideoListResponse {
  items: Video[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface ScanSummary {
  scanned_dir: string;
  total_files_examined: number;
  video_files_found: number;
  new_imported: number;
  updated: number;
  skipped: number;
  failed: number;
  duration_ms: number;
}

export interface ScanProgress {
  phase: 'discovering' | 'indexing' | 'enriching' | 'completed' | 'cancelled';
  message: string;
  total_found: number;
  processed: number;
  current_file?: string | null;
}

export interface FormatOption {
  format_id: string;
  ext: string;
  resolution: string;
  note?: string | null;
  filesize?: number | null;
}

export interface UrlInspectResult {
  url: string;
  title: string;
  uploader?: string | null;
  duration?: number | null;
  thumbnail?: string | null;
  extractor?: string | null;
  formats: FormatOption[];
}

export type DownloadStatus = 'pending' | 'downloading' | 'paused' | 'completed' | 'failed' | 'cancelled';

export interface DownloadTask {
  id: string;
  url: string;
  title?: string | null;
  author?: string | null;
  platform?: string | null;
  format_id?: string | null;
  thumbnail?: string | null;
  status: DownloadStatus | string;
  progress: number;
  downloaded_bytes: number;
  total_bytes: number;
  speed?: number;
  speed_str?: string | null;
  eta?: string | null;
  eta_str?: string | null;
  file_path?: string | null;
  error_log?: string | null;
  error_message?: string | null;
  target_path?: string | null;
  created_at: number;
  updated_at: number;
}

export interface AppUpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  release_name: string;
  release_notes: string;
  download_url: string;
  is_differential: boolean;
  download_size_bytes: number;
  original_size_bytes: number;
  sha256: string | null;
  published_at: string | null;
}

export interface UpdateProgressPayload {
  status: 'downloading' | 'verifying' | 'ready' | 'error';
  downloaded_bytes: number;
  total_bytes: number;
  percentage: number;
  speed_mbps: number;
  message: string;
}

export interface UpdateInstallResult {
  success: boolean;
  message: string;
  requires_restart: boolean;
}

export type DedupStrategy = 'exact_hash' | 'metadata' | 'fuzzy_name';

export interface DuplicateGroup {
  group_id: string;
  match_reason: 'exact_hash' | 'metadata' | 'fuzzy_name' | string;
  match_label: string;
  total_size: number;
  reclaimable_size: number;
  videos: Video[];
}

export interface DuplicateResolutionAction {
  groupId: string;
  keepVideoId: number;
  deleteVideoIds: number[];
  deleteDiskFile: boolean;
}

export interface ResolutionSummary {
  groups_processed: number;
  videos_removed: number;
  bytes_reclaimed: number;
  success: boolean;
  message: string;
}

export interface AudioExtractionOptions {
  videoPath: string;
  videoId?: number | null;
  format: 'mp3' | 'm4a' | 'wav' | 'flac' | string;
  qualityBitrate?: string;
  copyStream?: boolean;
  outputDir?: string;
  customFilename?: string;
}

export interface AudioExtractionResult {
  success: boolean;
  outputPath: string;
  fileName: string;
  fileSize: number;
  duration?: number | null;
  format: string;
  message: string;
}

export interface ExtractedAudio {
  id: number;
  videoId?: number | null;
  sourceVideoTitle?: string | null;
  sourceVideoPath?: string | null;
  filePath: string;
  fileName: string;
  fileSize: number;
  duration?: number | null;
  format: string;
  qualityBitrate?: string | null;
  createdAt: number;
}

export interface ExtractedAudioFilter {
  search?: string;
  format?: string;
  sortBy?: 'created_desc' | 'duration_desc' | 'size_desc' | 'name_asc' | string;
}

export interface SubtitleSegment {
  id: number;
  video_id: number;
  language?: string | null;
  format?: string | null;
  start_time_ms: number;
  end_time_ms: number;
  content: string;
}

export interface SubtitleSnippetMatch {
  video_id: number;
  segment_id: number;
  start_time_ms: number;
  end_time_ms: number;
  snippet: string;
  full_content: string;
}

export interface LocalWhisperStatus {
  binary_available: boolean;
  binary_path?: string | null;
  model_available: boolean;
  model_path?: string | null;
  ready: boolean;
}

export interface AiTranscriptionConfig {
  engine: 'local' | 'api' | string;
  api_url: string;
  api_key: string;
  model: string;
  language: string;
  prompt: string;
  local_model_path: string;
  llm_api_url?: string;
  llm_api_key?: string;
  llm_model?: string;
}

export interface AiTopicItem {
  timestamp_ms: number;
  title: string;
  point: string;
}

export interface AiAnalysis {
  id?: number | null;
  video_id: number;
  summary: string;
  keywords: string[];
  tags: string[];
  topics: AiTopicItem[];
  model_name: string;
  created_at: number;
}

export interface VideoAiStatus {
  video_id: number;
  has_subtitles: boolean;
  subtitle_count: number;
  has_analysis: boolean;
  summary_preview?: string | null;
  tags: string[];
}

export interface VideoCompatibilityStatus {
  is_compatible: boolean;
  has_cached_transcode: boolean;
  cached_path: string | null;
  suggested_action: 'direct_play' | 'cached_play' | 'needs_conversion' | string;
  reason?: string | null;
}

export interface TranscodeResult {
  original_path: string;
  compatible_path: string;
  is_cached: boolean;
  remuxed: boolean;
  output_size_bytes: number;
}

export interface VideoClipSegment {
  id: string;
  title: string;
  startMs: number;
  endMs: number;
}

export interface ClipVideoRequest {
  videoId?: number | null;
  videoPath: string;
  segments: VideoClipSegment[];
  outputDir?: string | null;
  exportMode: 'separate' | 'merge' | 'both';
  fastCopy: boolean;
}

export interface ClipResultItem {
  id: string;
  title: string;
  outputPath: string;
  durationSec: number;
  fileSize: number;
  startMs?: number;
  endMs?: number;
}

export interface ClipVideoResponse {
  success: boolean;
  clips: ClipResultItem[];
  mergedPath?: string | null;
  message: string;
}

export interface SegmentRecommendation {
  id: string;
  title: string;
  startMs: number;
  endMs: number;
  reason: string;
  confidence: number;
  isHighlight?: boolean;
  score?: number | null;
}

export interface RecommendSegmentsResponse {
  segments: SegmentRecommendation[];
  source: 'ai_climax' | 'ai_topics' | 'subtitles' | 'audio_peaks' | 'silence_detect' | 'pacing' | string;
  message: string;
  highlights?: SegmentRecommendation[];
}

export interface ClippedVideo {
  id: number;
  videoId?: number | null;
  sourceVideoTitle?: string | null;
  sourceVideoPath: string;
  filePath: string;
  fileName: string;
  title: string;
  startMs: number;
  endMs: number;
  duration?: number | null;
  fileSize: number;
  isMerged: boolean;
  createdAt: number;
}

export interface MasterVideoClipsGroup {
  sourceVideoPath: string;
  sourceVideoTitle: string;
  videoId?: number | null;
  totalClips: number;
  totalDuration: number;
  totalSize: number;
  latestCreatedAt: number;
  clips: ClippedVideo[];
}

export interface ClippedVideoFilter {
  search?: string;
  sourceVideoPath?: string;
  isMergedOnly?: boolean;
  sortBy?: 'created_desc' | 'created_asc' | 'duration_desc' | 'title_asc' | string;
}


