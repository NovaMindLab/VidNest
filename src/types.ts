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
}

export interface VideoFilter {
  search?: string;
  codec?: string;
  sort_by?: string;
  page?: number;
  page_size?: number;
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


