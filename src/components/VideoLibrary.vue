<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Video, VideoFilter, VideoListResponse, ScanSummary } from '../types';

const videos = ref<Video[]>([]);
const totalVideos = ref<number>(0);
const currentPage = ref<number>(1);
const totalPages = ref<number>(1);
const pageSize = ref<number>(18);

const isLoading = ref<boolean>(false);
const isScanning = ref<boolean>(false);
const scanPath = ref<string>('D:\\AI_serach_image');
const scanSummary = ref<ScanSummary | null>(null);
const scanError = ref<string>('');

const searchText = ref<string>('');
const selectedCodec = ref<string>('');
const selectedSort = ref<string>('created_desc');
const viewMode = ref<'grid' | 'list'>('grid');

// Active selected video for detail modal
const selectedVideo = ref<Video | null>(null);

const availableCodecs = computed(() => {
  const set = new Set<string>();
  videos.value.forEach((v) => {
    if (v.codec) set.add(v.codec.toUpperCase());
  });
  return Array.from(set);
});

async function fetchVideos(page = 1) {
  isLoading.value = true;
  currentPage.value = page;
  try {
    const filter: VideoFilter = {
      search: searchText.value.trim() || undefined,
      codec: selectedCodec.value || undefined,
      sort_by: selectedSort.value || undefined,
      page: currentPage.value,
      page_size: pageSize.value,
    };
    const res = await invoke<VideoListResponse>('list_videos', { filter });
    videos.value = res.items;
    totalVideos.value = res.total;
    totalPages.value = res.total_pages;
  } catch (err: unknown) {
    console.error('Failed fetching videos:', err);
  } finally {
    isLoading.value = false;
  }
}

async function triggerScan() {
  if (!scanPath.value.trim()) {
    scanError.value = '请输入有效的扫描目录路径';
    return;
  }
  isScanning.value = true;
  scanError.value = '';
  scanSummary.value = null;
  try {
    const res = await invoke<ScanSummary>('scan_video_directory', {
      path: scanPath.value.trim(),
    });
    scanSummary.value = res;
    await fetchVideos(1);
  } catch (err: unknown) {
    scanError.value = `扫描失败: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    isScanning.value = false;
  }
}

async function removeVideo(v: Video, deleteFile = false) {
  const confirmMsg = deleteFile
    ? `确定要从本地磁盘永久删除视频文件 "${v.file_name}" 吗？此操作无法撤销！`
    : `确定要将 "${v.file_name}" 从媒体库中移除吗？（磁盘文件将被保留）`;
  if (!window.confirm(confirmMsg)) return;

  try {
    await invoke('delete_video', { id: v.id, deleteFile });
    if (selectedVideo.value?.id === v.id) {
      selectedVideo.value = null;
    }
    await fetchVideos(currentPage.value);
  } catch (err: unknown) {
    alert(`移除失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds === undefined || isNaN(seconds)) return '--:--';
  const totalSecs = Math.floor(seconds);
  const hours = Math.floor(totalSecs / 3600);
  const minutes = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hours > 0) {
    return `${hours.toString().padStart(2, '0')}:${minutes
      .toString()
      .padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

function getResolutionBadge(width: number | null, height: number | null): string {
  if (!width || !height) return '';
  if (width >= 3840 || height >= 2160) return '4K UHD';
  if (width >= 2560 || height >= 1440) return '2K QHD';
  if (width >= 1920 || height >= 1080) return '1080P';
  if (width >= 1280 || height >= 720) return '720P';
  return `${width}x${height}`;
}

function formatDate(timestampMs: number): string {
  if (!timestampMs) return '-';
  const d = new Date(timestampMs);
  return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

onMounted(() => {
  fetchVideos(1);
});
</script>

<template>
  <div class="video-library">
    <!-- Top Action & Scanner Bar -->
    <section class="scanner-card card">
      <div class="scanner-row">
        <div class="scanner-input-wrap">
          <label class="scanner-label" for="scan-input">本地视频导入与目录递归扫描：</label>
          <div class="input-with-actions">
            <div class="input-icon-box">
              <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
              </svg>
            </div>
            <input
              id="scan-input"
              v-model="scanPath"
              class="scan-text-input"
              placeholder="输入待扫描的本地目录绝对路径，如 D:\Videos..."
              @keyup.enter="triggerScan"
            />
            <button class="btn btn-primary" :disabled="isScanning" @click="triggerScan">
              <svg v-if="!isScanning" class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
              </svg>
              <span v-else class="spinner"></span>
              {{ isScanning ? '正在递归扫描入库...' : '开始扫描入库' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Scan Feedback Result Banner -->
      <div v-if="scanError" class="scan-alert scan-alert-err">
        {{ scanError }}
      </div>
      <div v-if="scanSummary" class="scan-alert scan-alert-ok">
        <div class="summary-top">
          <strong>扫描完成！</strong> 目录: {{ scanSummary.scanned_dir }} (耗时: {{ scanSummary.duration_ms }} ms)
        </div>
        <div class="summary-stats">
          <span>文件遍历: <strong>{{ scanSummary.total_files_examined }}</strong></span>
          <span>发现视频: <strong>{{ scanSummary.video_files_found }}</strong></span>
          <span class="stat-highlight">新增入库: <strong>+{{ scanSummary.new_imported }}</strong></span>
          <span>更新比对: <strong>{{ scanSummary.updated }}</strong></span>
          <span>无变动跳过: <strong>{{ scanSummary.skipped }}</strong></span>
          <span v-if="scanSummary.failed > 0" class="stat-warn">失败: <strong>{{ scanSummary.failed }}</strong></span>
        </div>
      </div>
    </section>

    <!-- Toolbar: Search, Filter, Sort, View Modes -->
    <section class="toolbar-wrap">
      <div class="toolbar-left">
        <div class="search-box">
          <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          </svg>
          <input
            v-model="searchText"
            class="search-input"
            placeholder="搜索视频标题、文件名或全文..."
            @input="fetchVideos(1)"
          />
          <button v-if="searchText" class="clear-btn" @click="searchText = ''; fetchVideos(1)">✕</button>
        </div>

        <select v-model="selectedCodec" class="select-control" @change="fetchVideos(1)">
          <option value="">全部编码格式</option>
          <option v-for="c in availableCodecs" :key="c" :value="c">{{ c }}</option>
          <option value="H264">H264 / AVC</option>
          <option value="HEVC">HEVC / H265</option>
          <option value="VP9">VP9</option>
          <option value="AV1">AV1</option>
        </select>

        <select v-model="selectedSort" class="select-control" @change="fetchVideos(1)">
          <option value="created_desc">入库时间 (最新优先)</option>
          <option value="created_asc">入库时间 (最早优先)</option>
          <option value="duration_desc">视频时长 (由长到短)</option>
          <option value="duration_asc">视频时长 (由短到长)</option>
          <option value="size_desc">文件体积 (由大到小)</option>
          <option value="size_asc">文件体积 (由小到大)</option>
          <option value="name_asc">文件名称 (A - Z)</option>
        </select>
      </div>

      <div class="toolbar-right">
        <span class="video-counter">共 <strong>{{ totalVideos }}</strong> 部视频</span>
        <div class="view-toggle">
          <button
            class="toggle-btn"
            :class="{ active: viewMode === 'grid' }"
            title="网格视图"
            @click="viewMode = 'grid'"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="7" height="7"></rect>
              <rect x="14" y="3" width="7" height="7"></rect>
              <rect x="14" y="14" width="7" height="7"></rect>
              <rect x="3" y="14" width="7" height="7"></rect>
            </svg>
          </button>
          <button
            class="toggle-btn"
            :class="{ active: viewMode === 'list' }"
            title="列表视图"
            @click="viewMode = 'list'"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="8" y1="6" x2="21" y2="6"></line>
              <line x1="8" y1="12" x2="21" y2="12"></line>
              <line x1="8" y1="18" x2="21" y2="18"></line>
              <line x1="3" y1="6" x2="3.01" y2="6"></line>
              <line x1="3" y1="12" x2="3.01" y2="12"></line>
              <line x1="3" y1="18" x2="3.01" y2="18"></line>
            </svg>
          </button>
        </div>
      </div>
    </section>

    <!-- Empty State -->
    <div v-if="!isLoading && videos.length === 0" class="empty-state card">
      <div class="empty-icon-circle">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect>
          <line x1="7" y1="2" x2="7" y2="22"></line>
          <line x1="17" y1="2" x2="17" y2="22"></line>
          <line x1="2" y1="12" x2="22" y2="12"></line>
          <line x1="2" y1="7" x2="7" y2="7"></line>
          <line x1="2" y1="17" x2="7" y2="17"></line>
          <line x1="17" y1="17" x2="22" y2="17"></line>
          <line x1="17" y1="7" x2="22" y2="7"></line>
        </svg>
      </div>
      <h3 class="empty-title">当前本地视频库为空</h3>
      <p class="empty-desc">
        在上方输入本地视频存放目录（例如 D:\Videos 或 D:\AI_serach_image），点击“开始扫描入库”自动提取元数据与缩略图。
      </p>
      <button class="btn btn-primary" @click="triggerScan">
        立即扫描当前目录 ({{ scanPath }})
      </button>
    </div>

    <!-- Grid View -->
    <div v-else-if="viewMode === 'grid'" class="videos-grid">
      <div
        v-for="video in videos"
        :key="video.id"
        class="card video-card"
        @click="selectedVideo = video"
      >
        <div class="thumb-container">
          <img
            v-if="video.thumbnail_data"
            :src="video.thumbnail_data"
            class="thumb-img"
            :alt="video.file_name"
            loading="lazy"
          />
          <div v-else class="thumb-placeholder">
            <svg class="video-placeholder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
          </div>

          <!-- Badges on Thumbnail -->
          <span v-if="video.duration" class="thumb-badge badge-duration">
            {{ formatDuration(video.duration) }}
          </span>
          <span v-if="getResolutionBadge(video.width, video.height)" class="thumb-badge badge-reso">
            {{ getResolutionBadge(video.width, video.height) }}
          </span>
        </div>

        <div class="video-body">
          <h4 class="video-title" :title="video.file_name">{{ video.file_name }}</h4>
          <div class="video-meta-row">
            <span class="meta-tag" v-if="video.codec">{{ video.codec }}</span>
            <span class="meta-size">{{ formatFileSize(video.file_size) }}</span>
          </div>
          <div class="video-bottom-row">
            <span class="video-date">{{ formatDate(video.created_at) }}</span>
            <button
              class="card-action-btn"
              title="移除"
              @click.stop="removeVideo(video, false)"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- List View -->
    <div v-else class="videos-list card">
      <div class="list-header">
        <span class="col-thumb">缩略图</span>
        <span class="col-name">视频名称 / 路径</span>
        <span class="col-duration">时长</span>
        <span class="col-reso">分辨率</span>
        <span class="col-codec">编码</span>
        <span class="col-size">体积</span>
        <span class="col-actions">操作</span>
      </div>
      <div
        v-for="video in videos"
        :key="video.id"
        class="list-row"
        @click="selectedVideo = video"
      >
        <div class="col-thumb">
          <img
            v-if="video.thumbnail_data"
            :src="video.thumbnail_data"
            class="list-thumb-img"
            :alt="video.file_name"
          />
          <div v-else class="list-thumb-placeholder">▶</div>
        </div>
        <div class="col-name">
          <div class="list-video-title" :title="video.file_name">{{ video.file_name }}</div>
          <div class="list-video-path" :title="video.file_path">{{ video.file_path }}</div>
        </div>
        <div class="col-duration">{{ formatDuration(video.duration) }}</div>
        <div class="col-reso">{{ getResolutionBadge(video.width, video.height) || '-' }}</div>
        <div class="col-codec">
          <span class="meta-tag">{{ video.codec || '-' }}</span>
        </div>
        <div class="col-size">{{ formatFileSize(video.file_size) }}</div>
        <div class="col-actions" @click.stop>
          <button class="btn btn-sm btn-secondary" title="移除" @click="removeVideo(video, false)">
            移除
          </button>
        </div>
      </div>
    </div>

    <!-- Pagination -->
    <section v-if="totalPages > 1" class="pagination-wrap">
      <button
        class="btn btn-secondary btn-sm"
        :disabled="currentPage <= 1"
        @click="fetchVideos(currentPage - 1)"
      >
        上一页
      </button>
      <span class="page-indicator">第 {{ currentPage }} / {{ totalPages }} 页</span>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="currentPage >= totalPages"
        @click="fetchVideos(currentPage + 1)"
      >
        下一页
      </button>
    </section>

    <!-- Detail Modal -->
    <div v-if="selectedVideo" class="modal-backdrop" @click="selectedVideo = null">
      <div class="modal-card card" @click.stop>
        <div class="modal-header">
          <h3 class="modal-title">{{ selectedVideo.file_name }}</h3>
          <button class="modal-close-btn" @click="selectedVideo = null">✕</button>
        </div>

        <div class="modal-body">
          <div v-if="selectedVideo.thumbnail_data" class="modal-thumb-box">
            <img :src="selectedVideo.thumbnail_data" class="modal-thumb-img" :alt="selectedVideo.file_name" />
          </div>

          <div class="modal-info-grid">
            <div class="modal-info-item">
              <span class="info-label">时长:</span>
              <span class="info-val font-mono">{{ formatDuration(selectedVideo.duration) }} ({{ selectedVideo.duration?.toFixed(2) }}s)</span>
            </div>
            <div class="modal-info-item">
              <span class="info-label">分辨率:</span>
              <span class="info-val font-mono">{{ selectedVideo.width }} × {{ selectedVideo.height }} ({{ getResolutionBadge(selectedVideo.width, selectedVideo.height) }})</span>
            </div>
            <div class="modal-info-item">
              <span class="info-label">视频编码:</span>
              <span class="info-val font-mono">{{ selectedVideo.codec || '未知' }}</span>
            </div>
            <div class="modal-info-item">
              <span class="info-label">文件体积:</span>
              <span class="info-val font-mono">{{ formatFileSize(selectedVideo.file_size) }} ({{ selectedVideo.file_size }} bytes)</span>
            </div>
            <div class="modal-info-item span-2">
              <span class="info-label">文件路径:</span>
              <span class="info-val font-mono path-break">{{ selectedVideo.file_path }}</span>
            </div>
            <div class="modal-info-item span-2">
              <span class="info-label">内容哈希 (SHA256):</span>
              <span class="info-val font-mono path-break">{{ selectedVideo.file_hash }}</span>
            </div>
            <div class="modal-info-item">
              <span class="info-label">入库时间:</span>
              <span class="info-val">{{ formatDate(selectedVideo.created_at) }}</span>
            </div>
            <div class="modal-info-item">
              <span class="info-label">更新时间:</span>
              <span class="info-val">{{ formatDate(selectedVideo.updated_at) }}</span>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="selectedVideo = null">关闭</button>
          <button class="btn btn-danger" @click="removeVideo(selectedVideo, false)">从媒体库移除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-library {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.scanner-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.scanner-label {
  display: block;
  font-size: 0.88rem;
  font-weight: 600;
  margin-bottom: 0.5rem;
}

.input-with-actions {
  display: flex;
  gap: 0.75rem;
  align-items: center;
}

.input-icon-box {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background-color: var(--btn-sec-bg);
  border-radius: 8px;
  color: var(--text-muted);
}

.input-icon-box .icon {
  width: 20px;
  height: 20px;
}

.scan-text-input {
  flex: 1;
  padding: 0.65rem 0.85rem;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background-color: var(--input-bg);
  color: var(--text-color);
  font-size: 0.95rem;
  outline: none;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.scan-text-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.scan-alert {
  padding: 0.85rem 1.15rem;
  border-radius: 8px;
  font-size: 0.88rem;
}

.scan-alert-ok {
  background-color: #f0fdf4;
  color: #166534;
  border: 1px solid #bbf7d0;
}

.scan-alert-err {
  background-color: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
}

.summary-top {
  margin-bottom: 0.4rem;
}

.summary-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 1.25rem;
  font-size: 0.84rem;
}

.stat-highlight {
  color: #15803d;
  font-weight: 700;
}

.stat-warn {
  color: #b91c1c;
}

.toolbar-wrap {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}

.toolbar-left {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
  width: 280px;
}

.search-icon {
  position: absolute;
  left: 10px;
  width: 16px;
  height: 16px;
  color: var(--text-muted);
}

.search-input {
  width: 100%;
  padding: 0.55rem 2rem 0.55rem 2.2rem;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background-color: var(--input-bg);
  color: var(--text-color);
  font-size: 0.88rem;
  outline: none;
}

.clear-btn {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.8rem;
}

.select-control {
  padding: 0.55rem 0.85rem;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background-color: var(--input-bg);
  color: var(--text-color);
  font-size: 0.88rem;
  outline: none;
  cursor: pointer;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.video-counter {
  font-size: 0.85rem;
  color: var(--text-muted);
}

.view-toggle {
  display: flex;
  background-color: var(--btn-sec-bg);
  border-radius: 8px;
  padding: 2px;
}

.toggle-btn {
  background: none;
  border: none;
  padding: 6px 10px;
  border-radius: 6px;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
}

.toggle-btn.active {
  background-color: var(--card-bg);
  color: var(--text-color);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}

.toggle-btn svg {
  width: 16px;
  height: 16px;
}

/* Empty state */
.empty-state {
  text-align: center;
  padding: 4rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.empty-icon-circle {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background-color: var(--btn-sec-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  margin-bottom: 1.25rem;
}

.empty-icon-circle svg {
  width: 36px;
  height: 36px;
}

.empty-title {
  margin: 0 0 0.5rem;
  font-size: 1.25rem;
}

.empty-desc {
  max-width: 500px;
  color: var(--text-muted);
  font-size: 0.9rem;
  line-height: 1.5;
  margin: 0 0 1.5rem;
}

/* Grid View */
.videos-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 1.25rem;
}

.video-card {
  padding: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.video-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.12);
}

.thumb-container {
  position: relative;
  width: 100%;
  padding-top: 56.25%; /* 16:9 ratio */
  background-color: #0f172a;
  overflow: hidden;
}

.thumb-img {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-placeholder {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1e293b, #0f172a);
}

.video-placeholder-icon {
  width: 36px;
  height: 36px;
  color: #64748b;
}

.thumb-badge {
  position: absolute;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.72rem;
  font-weight: 600;
  color: #ffffff;
  background-color: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
}

.badge-duration {
  bottom: 8px;
  right: 8px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.badge-reso {
  top: 8px;
  left: 8px;
  background-color: rgba(37, 99, 235, 0.85);
}

.video-body {
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  flex: 1;
}

.video-title {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 600;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.video-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.78rem;
  color: var(--text-muted);
  margin-top: auto;
}

.meta-tag {
  background-color: var(--btn-sec-bg);
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 600;
  font-size: 0.72rem;
}

.video-bottom-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.75rem;
  color: var(--text-muted);
  padding-top: 0.25rem;
  border-top: 1px solid var(--border-color);
}

.card-action-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 3px;
  border-radius: 4px;
}

.card-action-btn:hover {
  color: #ef4444;
}

.card-action-btn svg {
  width: 14px;
  height: 14px;
}

/* List View */
.videos-list {
  padding: 0;
  overflow: hidden;
}

.list-header {
  display: grid;
  grid-template-columns: 80px 1fr 100px 100px 80px 90px 70px;
  padding: 0.75rem 1rem;
  background-color: var(--btn-sec-bg);
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border-color);
}

.list-row {
  display: grid;
  grid-template-columns: 80px 1fr 100px 100px 80px 90px 70px;
  padding: 0.65rem 1rem;
  align-items: center;
  font-size: 0.85rem;
  border-bottom: 1px solid var(--border-color);
  cursor: pointer;
  transition: background-color 0.15s;
}

.list-row:hover {
  background-color: var(--btn-sec-bg);
}

.list-thumb-img {
  width: 64px;
  height: 36px;
  object-fit: cover;
  border-radius: 4px;
}

.list-thumb-placeholder {
  width: 64px;
  height: 36px;
  background-color: #1e293b;
  color: #64748b;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
}

.list-video-title {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.list-video-path {
  font-size: 0.75rem;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* Pagination */
.pagination-wrap {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  margin-top: 1rem;
}

.page-indicator {
  font-size: 0.88rem;
  color: var(--text-muted);
}

/* Modal */
.modal-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1.5rem;
}

.modal-card {
  width: 100%;
  max-width: 680px;
  max-height: 90vh;
  overflow-y: auto;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.modal-title {
  margin: 0;
  font-size: 1.15rem;
}

.modal-close-btn {
  background: none;
  border: none;
  font-size: 1.2rem;
  cursor: pointer;
  color: var(--text-muted);
}

.modal-thumb-box {
  width: 100%;
  max-height: 280px;
  background-color: #0f172a;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
}

.modal-thumb-img {
  max-width: 100%;
  max-height: 280px;
  object-fit: contain;
}

.modal-info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.85rem;
  font-size: 0.85rem;
}

.modal-info-item {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.span-2 {
  grid-column: span 2;
}

.info-label {
  font-size: 0.78rem;
  color: var(--text-muted);
  font-weight: 600;
}

.info-val {
  color: var(--text-color);
}

.path-break {
  word-break: break-all;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 0.5rem;
}

.btn-danger {
  background-color: #ef4444;
  color: white;
}

.btn-danger:hover {
  background-color: #dc2626;
}

.btn-sm {
  padding: 0.35rem 0.75rem;
  font-size: 0.8rem;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
