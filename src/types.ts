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
  title: string;
  thumbnail?: string | null;
  status: DownloadStatus | string;
  progress: number;
  downloaded_bytes: number;
  total_bytes: number;
  speed_str?: string | null;
  eta_str?: string | null;
  error_message?: string | null;
  target_path?: string | null;
  created_at: number;
  updated_at: number;
}
