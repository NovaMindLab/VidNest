<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import VideoPlayerModal from './VideoPlayerModal.vue';
import type { Video, VideoFilter, VideoListResponse, ScanSummary, ScanProgress } from '../types';

const emit = defineEmits<{
  (e: 'navigateDedup'): void;
}>();

const videos = ref<Video[]>([]);
const totalVideos = ref<number>(0);
const currentPage = ref<number>(1);
const batchSize = ref<number>(36);
const hasMore = ref<boolean>(true);

const isLoading = ref<boolean>(false);
const isLoadingMore = ref<boolean>(false);
const isScanning = ref<boolean>(false);
const isCancelling = ref<boolean>(false);
const scanPath = ref<string>('');
const minSizeMb = ref<number>(10);
const scanSummary = ref<ScanSummary | null>(null);
const scanProgress = ref<ScanProgress | null>(null);
const scanError = ref<string>('');
const showScanner = ref<boolean>(false);

const isPlayerVisible = ref<boolean>(false);
const activePlayerVideo = ref<Video | null>(null);

let unlistenScanProgress: UnlistenFn | null = null;
let unlistenVideoEnriched: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;

const galleryPaneRef = ref<HTMLElement | null>(null);
const scrollTop = ref<number>(0);
const viewportHeight = ref<number>(800);
const galleryWidth = ref<number>(900);

const searchText = ref<string>('');
const selectedCodec = ref<string>('');
const selectedSort = ref<string>('created_desc');
const viewMode = ref<'grid' | 'list'>('grid');

const toastMessage = ref<string>('');
let toastTimer: number | null = null;

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2500);
}

const showDeleteConfirmModal = ref<boolean>(false);
const videoToDelete = ref<Video | null>(null);
const alsoDeleteDiskFile = ref<boolean>(false);
const isDeletingVideo = ref<boolean>(false);

let searchTimer: number | null = null;

function onSearchInput() {
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  searchTimer = window.setTimeout(() => {
    fetchVideos(true);
  }, 250);
}

function clearSearch() {
  if (searchTimer) {
    clearTimeout(searchTimer);
    searchTimer = null;
  }
  searchText.value = '';
  fetchVideos(true);
}

async function openVideo(video: Video) {
  try {
    showToast(`▶ 正在启动播放: ${video.file_name}`);
    await invoke('open_video_player_window', { video });
  } catch (err) {
    console.warn('Failed to open native player window, falling back to modal:', err);
    activePlayerVideo.value = video;
    isPlayerVisible.value = true;
  }
}

async function revealVideo(filePath: string) {
  try {
    await invoke('reveal_in_explorer', { path: filePath });
    showToast('📂 已在系统文件管理器中定位');
  } catch (err) {
    try {
      await revealItemInDir(filePath);
      showToast('📂 已在系统文件管理器中定位');
    } catch (e) {
      console.error('Failed to reveal video:', e);
      showToast(`定位失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
}

const availableCodecs = computed(() => {
  const set = new Set<string>();
  videos.value.forEach((v) => {
    if (v.codec) set.add(v.codec.toUpperCase());
  });
  return Array.from(set);
});

// Date grouping logic (按天时间分割)
function getDateKeyAndLabel(timestampMs: number): { key: string; label: string; dateStr: string } {
  if (!timestampMs || timestampMs <= 0) {
    return { key: 'unknown', label: '更早 / 未知日期', dateStr: '未知日期' };
  }
  const date = new Date(timestampMs);
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  const key = `${y}-${m}-${d}`;

  const now = new Date();
  const todayKey = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;

  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  const yesterdayKey = `${yesterday.getFullYear()}-${String(yesterday.getMonth() + 1).padStart(2, '0')}-${String(yesterday.getDate()).padStart(2, '0')}`;

  const weekDays = ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六'];
  const weekDayStr = weekDays[date.getDay()];

  let label = '';
  if (key === todayKey) {
    label = `今天 · ${date.getMonth() + 1}月${date.getDate()}日 (${weekDayStr})`;
  } else if (key === yesterdayKey) {
    label = `昨天 · ${date.getMonth() + 1}月${date.getDate()}日 (${weekDayStr})`;
  } else if (date.getFullYear() === now.getFullYear()) {
    label = `${date.getMonth() + 1}月${date.getDate()}日 · ${weekDayStr}`;
  } else {
    label = `${y}年${date.getMonth() + 1}月${date.getDate()}日 · ${weekDayStr}`;
  }

  return { key, label, dateStr: `${y}-${m}-${d}` };
}

export interface DateGroup {
  key: string;
  label: string;
  videos: Video[];
  totalSize: number;
}

const dateGroups = computed<DateGroup[]>(() => {
  const groups: DateGroup[] = [];
  const map = new Map<string, DateGroup>();

  for (const video of videos.value) {
    const { key, label } = getDateKeyAndLabel(video.created_at || video.updated_at || 0);
    let group = map.get(key);
    if (!group) {
      group = {
        key,
        label,
        videos: [],
        totalSize: 0,
      };
      map.set(key, group);
      groups.push(group);
    }
    group.videos.push(video);
    group.totalSize += video.file_size || 0;
  }

  return groups;
});

const columnsCount = computed(() => {
  if (viewMode.value === 'list') return 1;
  const w = galleryWidth.value;
  if (w <= 480) return 1;
  if (w <= 740) return 2;
  if (w <= 1040) return 3;
  if (w <= 1380) return 4;
  if (w <= 1720) return 5;
  return 6;
});

const currentGridRowHeight = computed(() => {
  const w = galleryWidth.value || 900;
  const cols = columnsCount.value || 3;
  const paddingX = 40; // gallery-pane padding: 1.25rem * 2
  const gapX = 17.6; // virtual-grid-row gap: 1.1rem
  const availableWidth = Math.max(260, w - paddingX - (cols - 1) * gapX);
  const cardWidth = availableWidth / cols;
  const thumbHeight = Math.round(cardWidth * (9 / 16));
  // 底部信息栏（标题 2 行高度 + 格式大小 + 快捷操作按钮条 + 内边距 + 间距等）
  // 预留 140px，外加行下外边距 16px，彻底杜绝底部操作菜单被遮挡/截断
  const cardInfoHeight = 140;
  const rowMargin = 16;
  return thumbHeight + cardInfoHeight + rowMargin;
});

export type VirtualRow =
  | { type: 'header'; key: string; group: DateGroup; height: number }
  | { type: 'grid-row'; key: string; groupKey: string; videos: Video[]; height: number }
  | { type: 'list-row'; key: string; video: Video; height: number };

export interface VirtualRowPosition {
  top: number;
  height: number;
  bottom: number;
  row: VirtualRow;
}

const flatVirtualRows = computed<VirtualRow[]>(() => {
  const rows: VirtualRow[] = [];
  const isGrid = viewMode.value === 'grid';
  const cols = columnsCount.value;

  for (const group of dateGroups.value) {
    // 1. Date Header Row
    rows.push({
      type: 'header',
      key: `hdr-${group.key}`,
      group,
      height: 48,
    });

    // 2. Video items
    if (isGrid) {
      const vids = group.videos;
      const rHeight = currentGridRowHeight.value;
      for (let i = 0; i < vids.length; i += cols) {
        const chunk = vids.slice(i, i + cols);
        rows.push({
          type: 'grid-row',
          key: `grow-${group.key}-${i}`,
          groupKey: group.key,
          videos: chunk,
          height: rHeight,
        });
      }
    } else {
      for (const v of group.videos) {
        rows.push({
          type: 'list-row',
          key: `lrow-${v.id}`,
          video: v,
          height: 56,
        });
      }
    }
  }

  return rows;
});

const rowPositions = computed<VirtualRowPosition[]>(() => {
  let top = 0;
  const list: VirtualRowPosition[] = [];
  for (const r of flatVirtualRows.value) {
    list.push({
      top,
      height: r.height,
      bottom: top + r.height,
      row: r,
    });
    top += r.height;
  }
  return list;
});

const totalVirtualHeight = computed(() => {
  const positions = rowPositions.value;
  if (positions.length === 0) return 0;
  return positions[positions.length - 1].bottom;
});

const visibleRows = computed<VirtualRowPosition[]>(() => {
  const positions = rowPositions.value;
  if (positions.length === 0) return [];

  const st = scrollTop.value;
  const vh = viewportHeight.value;
  const overscanPixels = 400;

  const minTop = Math.max(0, st - overscanPixels);
  const maxBottom = st + vh + overscanPixels;

  let low = 0;
  let high = positions.length - 1;
  let startIdx = 0;
  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    if (positions[mid].bottom >= minTop) {
      startIdx = mid;
      high = mid - 1;
    } else {
      low = mid + 1;
    }
  }

  low = startIdx;
  high = positions.length - 1;
  let endIdx = positions.length - 1;
  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    if (positions[mid].top <= maxBottom) {
      endIdx = mid;
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }

  return positions.slice(startIdx, endIdx + 1);
});

async function fetchVideos(reset = true) {
  if (reset) {
    isLoading.value = true;
    currentPage.value = 1;
  } else {
    isLoadingMore.value = true;
  }

  try {
    const filter: VideoFilter = {
      search: searchText.value.trim() || undefined,
      codec: selectedCodec.value || undefined,
      sort_by: selectedSort.value || undefined,
      page: currentPage.value,
      page_size: batchSize.value,
    };
    const res = await invoke<VideoListResponse>('list_videos', { filter });
    if (reset) {
      videos.value = res.items;
      totalVideos.value = res.total;
    } else {
      const existingIds = new Set(videos.value.map((v) => v.id));
      const newItems = res.items.filter((v) => !existingIds.has(v.id));
      videos.value.push(...newItems);
      totalVideos.value = res.total;
    }

    hasMore.value = videos.value.length < res.total;
  } catch (err: unknown) {
    console.error('Failed fetching videos:', err);
  } finally {
    isLoading.value = false;
    isLoadingMore.value = false;
  }
}

async function loadNextBatch() {
  if (isLoadingMore.value || isLoading.value || !hasMore.value) return;
  currentPage.value += 1;
  await fetchVideos(false);
}

function handleGalleryScroll(e: Event) {
  const target = e.target as HTMLElement;
  if (!target) return;
  scrollTop.value = target.scrollTop;

  // Infinite scroll trigger: when within 350px of bottom, auto-fetch next batch
  if (target.scrollTop + target.clientHeight >= target.scrollHeight - 350) {
    if (!isLoadingMore.value && !isLoading.value && hasMore.value) {
      loadNextBatch();
    }
  }
}

async function triggerFullDiskScan() {
  isScanning.value = true;
  isCancelling.value = false;
  scanError.value = '';
  scanSummary.value = null;
  scanProgress.value = {
    phase: 'discovering',
    message: '正在极速发现全盘视频...',
    total_found: 0,
    processed: 0,
  };

  try {
    const res = await invoke<ScanSummary>('start_full_disk_scan', {
      minSizeMb: minSizeMb.value,
    });
    scanSummary.value = res;
    await fetchVideos(true);
  } catch (err: unknown) {
    scanError.value = `全盘扫描失败: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    isScanning.value = false;
    isCancelling.value = false;
  }
}

async function cancelScan() {
  if (!isScanning.value) return;
  isCancelling.value = true;
  try {
    await invoke<boolean>('cancel_full_disk_scan');
  } catch (err) {
    console.error('Cancel scan error:', err);
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
  scanProgress.value = null;
  try {
    const res = await invoke<ScanSummary>('scan_video_directory', {
      path: scanPath.value.trim(),
    });
    scanSummary.value = res;
    await fetchVideos(true);
  } catch (err: unknown) {
    scanError.value = `扫描失败: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    isScanning.value = false;
  }
}

function promptRemoveVideo(video: Video) {
  videoToDelete.value = video;
  alsoDeleteDiskFile.value = false;
  showDeleteConfirmModal.value = true;
}

async function confirmRemoveVideo() {
  if (!videoToDelete.value) return;
  const v = videoToDelete.value;
  const deleteFile = alsoDeleteDiskFile.value;
  isDeletingVideo.value = true;
  try {
    await invoke('delete_video', {
      id: v.id,
      delete_file: deleteFile,
      deleteFile: deleteFile,
    });
    videos.value = videos.value.filter((item) => item.id !== v.id);
    totalVideos.value = Math.max(0, totalVideos.value - 1);
    showToast(deleteFile ? `已永久删除文件: ${v.file_name}` : `已从媒体库移除: ${v.file_name}`);
    showDeleteConfirmModal.value = false;
    videoToDelete.value = null;
  } catch (err: unknown) {
    showToast(`移除失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    isDeletingVideo.value = false;
  }
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast('📋 文件路径已复制到剪贴板');
  } catch {
    const el = document.createElement('textarea');
    el.value = text;
    document.body.appendChild(el);
    el.select();
    document.execCommand('copy');
    document.body.removeChild(el);
    showToast('📋 文件路径已复制到剪贴板');
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


onMounted(async () => {
  fetchVideos(true);

  if (galleryPaneRef.value) {
    galleryWidth.value = galleryPaneRef.value.clientWidth;
    viewportHeight.value = galleryPaneRef.value.clientHeight;
    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        galleryWidth.value = entry.contentRect.width;
        viewportHeight.value = entry.contentRect.height;
      }
    });
    resizeObserver.observe(galleryPaneRef.value);
  }

  try {
    unlistenScanProgress = await listen<ScanProgress>('scan:progress', (event) => {
      scanProgress.value = event.payload;
      if (event.payload.phase === 'indexing' && event.payload.processed % 50 === 0) {
        fetchVideos(false);
      }
      if (event.payload.phase === 'completed') {
        fetchVideos(true);
      }
    });

    unlistenVideoEnriched = await listen<number>('video:enriched', () => {
      fetchVideos(false);
    });
  } catch (err) {
    console.error('Failed to attach scan listeners:', err);
  }
});

onUnmounted(() => {
  if (toastTimer) clearTimeout(toastTimer);
  if (searchTimer) clearTimeout(searchTimer);
  if (resizeObserver) resizeObserver.disconnect();
  if (unlistenScanProgress) unlistenScanProgress();
  if (unlistenVideoEnriched) unlistenVideoEnriched();
});
</script>

<template>
  <div class="video-library-container">
    <!-- Top Compact Header & Toolbar -->
    <header class="library-toolbar">
      <div class="toolbar-left-group">
        <!-- Search -->
        <div class="search-input-wrap">
          <svg class="search-ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          </svg>
          <input
            v-model="searchText"
            class="search-input-field"
            placeholder="搜索视频标题、文件名或路径..."
            @input="onSearchInput"
          />
          <button
            v-if="searchText"
            class="search-clear-btn"
            @click="clearSearch"
          >✕</button>
        </div>

        <!-- Codec Pills -->
        <div class="codec-pills-row">
          <button
            class="pill-filter-btn"
            :class="{ active: selectedCodec === '' }"
            @click="selectedCodec = ''; fetchVideos(true)"
          >
            全部
          </button>
          <button
            v-for="c in (availableCodecs.length > 0 ? availableCodecs : ['H264', 'HEVC', 'AV1'])"
            :key="c"
            class="pill-filter-btn font-mono"
            :class="{ active: selectedCodec === c }"
            @click="selectedCodec = selectedCodec === c ? '' : c; fetchVideos(true)"
          >
            {{ c }}
          </button>
        </div>

        <!-- Sort Select -->
        <select v-model="selectedSort" class="select-dropdown" @change="fetchVideos(true)">
          <option value="created_desc">入库时间 (最新优先)</option>
          <option value="created_asc">入库时间 (最早优先)</option>
          <option value="duration_desc">视频时长 (长到短)</option>
          <option value="duration_asc">视频时长 (短到长)</option>
          <option value="size_desc">文件体积 (大到小)</option>
          <option value="size_asc">文件体积 (小到大)</option>
          <option value="name_asc">文件名称 (A - Z)</option>
        </select>
      </div>

      <div class="toolbar-right-group">
        <!-- Scan / Import Folder Toggle Button -->
        <button
          class="btn-toggle-scan"
          :class="{ active: showScanner }"
          @click="showScanner = !showScanner"
        >
          <svg class="btn-sm-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
          </svg>
          <span>{{ showScanner ? '收起扫描' : '扫描导入' }}</span>
        </button>

        <!-- Jump to Deduplication View Button -->
        <button
          class="btn-toggle-scan"
          title="查找相同或重复视频并清理"
          @click="emit('navigateDedup')"
        >
          <svg class="btn-sm-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
          <span>查重清理</span>
        </button>

        <!-- View mode -->
        <div class="view-switch-btns">
          <button
            class="view-btn"
            :class="{ active: viewMode === 'grid' }"
            title="网格平铺"
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
            class="view-btn"
            :class="{ active: viewMode === 'list' }"
            title="紧凑列表"
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

        <span class="video-counter-tag">
          共 <strong>{{ totalVideos }}</strong> 部
        </span>
      </div>
    </header>

    <!-- Collapsible Scan Directory Panel -->
    <transition name="drawer">
      <div v-if="showScanner" class="scanner-drawer">
        <div class="scanner-content-box">
          <!-- 1. Full-Disk Smart Scan Card -->
          <div class="full-disk-card">
            <div class="full-disk-left">
              <div class="full-disk-heading">
                <span class="badge-smart">⚡️ 系统级极速</span>
                <span class="full-disk-title">全盘低开销智能扫描 (Win & Mac)</span>
              </div>
              <p class="full-disk-desc">
                优先调用操作系统底层索引（macOS Spotlight / Windows 索引）与智能剪枝，秒级发现所有视频。零高强度磁盘磨损，自动排除系统目录、代码与碎片。
              </p>
              <div class="full-disk-filter-row">
                <label class="filter-opt-label">
                  <span>过滤体积门槛：忽略小于</span>
                  <input
                    type="number"
                    v-model.number="minSizeMb"
                    min="1"
                    max="2000"
                    class="min-size-input font-mono"
                    :disabled="isScanning"
                  />
                  <span>MB 的文件 (默认 10MB)</span>
                </label>
              </div>
            </div>

            <div class="full-disk-actions">
              <button
                v-if="!isScanning"
                type="button"
                class="btn btn-primary btn-run-full-scan"
                @click="triggerFullDiskScan"
              >
                <span>⚡️ 启动全盘极速扫描</span>
              </button>
              <button
                v-else
                type="button"
                class="btn btn-danger btn-cancel-scan"
                :disabled="isCancelling"
                @click="cancelScan"
              >
                <span class="spinner-sm"></span>
                <span>{{ isCancelling ? '正在中止...' : '⏹ 停止扫描' }}</span>
              </button>
            </div>
          </div>

          <!-- 2. Real-Time Scan Progress Strip -->
          <div v-if="isScanning && scanProgress" class="scan-progress-strip">
            <div class="progress-info-row">
              <div class="progress-msg">
                <span class="pulse-dot"></span>
                <strong>{{ scanProgress.message }}</strong>
              </div>
              <span v-if="scanProgress.total_found > 0" class="progress-counts font-mono">
                {{ scanProgress.processed }} / {{ scanProgress.total_found }}
              </span>
            </div>
            <div class="progress-bar-track">
              <div
                class="progress-bar-fill"
                :style="{
                  width:
                    scanProgress.total_found > 0
                      ? `${Math.min(100, Math.max(5, (scanProgress.processed / scanProgress.total_found) * 100))}%`
                      : '30%',
                }"
                :class="{ indeterminate: scanProgress.phase === 'discovering' }"
              ></div>
            </div>
          </div>

          <!-- Divider -->
          <div class="drawer-divider">
            <span>或者指定单个特定文件夹扫描</span>
          </div>

          <!-- 3. Custom Single Folder Line -->
          <div class="scanner-input-line">
            <svg class="folder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
            <input
              v-model="scanPath"
              class="scan-dir-input font-mono"
              placeholder="输入待扫描的文件夹绝对路径，如 /Users/hou/Movies 或 D:\Videos..."
              :disabled="isScanning"
              @keyup.enter="triggerScan"
            />
            <button class="btn btn-secondary" :disabled="isScanning" @click="triggerScan">
              <span v-if="isScanning" class="spinner-sm"></span>
              <span>{{ isScanning ? '扫描中...' : '扫描指定目录' }}</span>
            </button>
          </div>

          <div v-if="scanError" class="scan-feedback-alert fb-err">
            {{ scanError }}
          </div>

          <div v-if="scanSummary" class="scan-feedback-alert fb-ok">
            <div class="summary-line">
              {{ scanSummary.scanned_dir }} · 耗时 <strong>{{ scanSummary.duration_ms }} ms</strong>
            </div>
            <div class="summary-tags-row">
              <span>发现视频: <strong>{{ scanSummary.video_files_found }}</strong></span>
              <span class="hl-green">新增导入: <strong>+{{ scanSummary.new_imported }}</strong></span>
              <span>更新: <strong>{{ scanSummary.updated }}</strong></span>
              <span>跳过已存在: <strong>{{ scanSummary.skipped }}</strong></span>
              <span v-if="scanSummary.failed > 0" class="hl-red">失败: <strong>{{ scanSummary.failed }}</strong></span>
            </div>
          </div>
        </div>
      </div>
    </transition>

    <!-- MASTER-DETAIL SPLIT WORKSPACE (左右分栏核心) -->
    <div class="master-detail-workspace">
      <!-- LEFT PANE: Video Gallery (Master) -->
      <section ref="galleryPaneRef" class="gallery-pane" @scroll="handleGalleryScroll">
        <!-- Empty State -->
        <div v-if="!isLoading && videos.length === 0" class="empty-gallery-state">
          <div class="empty-icon-circle">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect>
              <line x1="7" y1="2" x2="7" y2="22"></line>
              <line x1="17" y1="2" x2="17" y2="22"></line>
              <line x1="2" y1="12" x2="22" y2="12"></line>
            </svg>
          </div>
          <h3 class="empty-h3">媒体库暂无匹配视频</h3>
          <p class="empty-p">点击右上角“扫描导入”指定本地目录，快速提取视频元数据并入库管理。</p>
          <button class="btn btn-secondary" @click="showScanner = true">
            展开扫描目录面板
          </button>
        </div>

        <!-- Initial Loading State Skeleton -->
        <div v-else-if="isLoading && videos.length === 0" class="gallery-loading-skeleton">
          <div class="spinner-lg"></div>
          <span>正在极速载入媒体库...</span>
        </div>

        <!-- Virtual Scroll Container with Date Grouping -->
        <div v-else class="virtual-scroll-viewport" :style="{ height: totalVirtualHeight + 'px' }">
          <!-- List Mode Header (Sticky atop list view) -->
          <div v-if="viewMode === 'list'" class="list-head-sticky-row">
            <span class="lh-thumb">缩略图</span>
            <span class="lh-title">文件名与路径</span>
            <span class="lh-dur">时长</span>
            <span class="lh-reso">分辨率</span>
            <span class="lh-codec">编码</span>
            <span class="lh-size">大小</span>
            <span class="lh-actions">操作</span>
          </div>

          <!-- Virtual Rows (Headers + Grid/List Rows) -->
          <div
            v-for="item in visibleRows"
            :key="item.row.key"
            class="virtual-row-wrap"
            :style="{
              top: item.top + 'px',
              height: item.height + 'px',
            }"
          >
            <!-- 1. Date Header Row (按天时间分割) -->
            <div v-if="item.row.type === 'header'" class="date-group-header">
              <div class="dgh-left">
                <svg class="dgh-calendar-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
                  <line x1="16" y1="2" x2="16" y2="6"></line>
                  <line x1="8" y1="2" x2="8" y2="6"></line>
                  <line x1="3" y1="10" x2="21" y2="10"></line>
                </svg>
                <span class="dgh-title">{{ item.row.group.label }}</span>
                <span class="dgh-badge">{{ item.row.group.videos.length }} 部视频</span>
              </div>
              <div class="dgh-right">
                <span class="dgh-size font-mono">共 {{ formatFileSize(item.row.group.totalSize) }}</span>
              </div>
            </div>

            <!-- 2. Grid Row -->
            <div
              v-else-if="item.row.type === 'grid-row'"
              class="virtual-grid-row"
              :style="{
                gridTemplateColumns: `repeat(${columnsCount}, minmax(0, 1fr))`,
              }"
            >
              <div
                v-for="video in item.row.videos"
                :key="video.id"
                class="video-grid-card"
                @dblclick="openVideo(video)"
              >
                <!-- 16:9 Thumbnail -->
                <div class="grid-thumb-box" @click="openVideo(video)" title="点击播放视频">
                  <img
                    v-if="video.thumbnail_data"
                    :src="video.thumbnail_data"
                    class="grid-thumb-img"
                    :alt="video.file_name"
                    loading="lazy"
                  />
                  <div v-else class="grid-thumb-placeholder">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                      <polygon points="5 3 19 12 5 21 5 3"></polygon>
                    </svg>
                  </div>

                  <div class="grid-thumb-hover-overlay">
                    <span class="play-icon-badge">▶</span>
                  </div>

                  <!-- Overlaid badges -->
                  <span v-if="video.duration" class="pill-badge-corner pill-duration font-mono">
                    {{ formatDuration(video.duration) }}
                  </span>
                  <span v-if="getResolutionBadge(video.width, video.height)" class="pill-badge-corner pill-reso">
                    {{ getResolutionBadge(video.width, video.height) }}
                  </span>
                </div>

                <!-- Card Info -->
                <div class="grid-card-info">
                  <span class="grid-card-title" :title="video.file_name" @click="openVideo(video)">
                    {{ video.file_name }}
                  </span>
                  <div class="grid-card-sub">
                    <span v-if="video.codec" class="codec-pill font-mono">{{ video.codec }}</span>
                    <span class="size-text font-mono">{{ formatFileSize(video.file_size) }}</span>
                  </div>

                  <!-- Quick Actions Row -->
                  <div class="card-quick-actions">
                    <button class="card-act-btn" title="播放" @click.stop="openVideo(video)">
                      ▶ 播放
                    </button>
                    <button class="card-act-btn" title="在文件夹中定位" @click.stop="revealVideo(video.file_path)">
                      📁 定位
                    </button>
                    <button class="card-act-btn" title="复制路径" @click.stop="copyToClipboard(video.file_path)">
                      📋 复制
                    </button>
                    <button class="card-act-btn btn-act-del" title="从媒体库移除" @click.stop="promptRemoveVideo(video)">
                      ✕ 移除
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <!-- 3. List Item Row -->
            <div
              v-else-if="item.row.type === 'list-row'"
              class="list-item-row"
              @dblclick="openVideo(item.row.video)"
            >
              <div class="li-thumb" @click="openVideo(item.row.video)" title="点击播放">
                <img
                  v-if="item.row.video.thumbnail_data"
                  :src="item.row.video.thumbnail_data"
                  class="li-thumb-img"
                  :alt="item.row.video.file_name"
                />
                <div v-else class="li-thumb-placeholder">▶</div>
              </div>
              <div class="li-title-col">
                <span class="li-name" :title="item.row.video.file_name" @click="openVideo(item.row.video)">
                  {{ item.row.video.file_name }}
                </span>
                <span class="li-path font-mono" :title="item.row.video.file_path">
                  {{ item.row.video.file_path }}
                </span>
              </div>
              <div class="li-dur font-mono">{{ formatDuration(item.row.video.duration) }}</div>
              <div class="li-reso">{{ getResolutionBadge(item.row.video.width, item.row.video.height) || '-' }}</div>
              <div class="li-codec font-mono">{{ item.row.video.codec || '-' }}</div>
              <div class="li-size font-mono">{{ formatFileSize(item.row.video.file_size) }}</div>
              <div class="li-actions">
                <button class="li-btn" title="播放" @click.stop="openVideo(item.row.video)">▶</button>
                <button class="li-btn" title="定位文件" @click.stop="revealVideo(item.row.video.file_path)">📁</button>
                <button class="li-btn" title="复制路径" @click.stop="copyToClipboard(item.row.video.file_path)">📋</button>
                <button class="li-btn btn-act-del" title="从媒体库移除" @click.stop="promptRemoveVideo(item.row.video)">✕</button>
              </div>
            </div>
          </div>
        </div>

        <!-- Infinite Scroll Bottom Status (无需翻页，滚动触底加载) -->
        <div v-if="videos.length > 0" class="infinite-scroll-footer">
          <div v-if="isLoadingMore" class="loading-more-pill">
            <span class="spinner-sm"></span>
            <span>正在加载更多视频...</span>
          </div>
          <div v-else-if="!hasMore" class="all-loaded-pill">
            <span>✓ 已加载全部 {{ totalVideos }} 部视频</span>
          </div>
        </div>
      </section>
    </div>

    <!-- Video Player Modal Component -->
    <VideoPlayerModal
      :video="activePlayerVideo"
      :visible="isPlayerVisible"
      @close="isPlayerVisible = false"
    />

    <!-- Confirm Delete Modal (彻底替换被系统拦截的 window.confirm) -->
    <div v-if="showDeleteConfirmModal" class="confirm-modal-overlay" @click.self="showDeleteConfirmModal = false">
      <div class="confirm-modal-card">
        <div class="confirm-modal-header">
          <div class="confirm-icon-box">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18m-2 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              <line x1="10" y1="11" x2="10" y2="17"></line>
              <line x1="14" y1="11" x2="14" y2="17"></line>
            </svg>
          </div>
          <div class="confirm-header-text">
            <h3 class="confirm-title">从媒体库移除视频</h3>
            <p class="confirm-subtitle">请确认对该视频的操作</p>
          </div>
          <button class="confirm-close-btn" @click="showDeleteConfirmModal = false">✕</button>
        </div>

        <div class="confirm-modal-body">
          <div class="video-info-preview">
            <span class="preview-name" :title="videoToDelete?.file_name">{{ videoToDelete?.file_name }}</span>
            <span class="preview-path font-mono" :title="videoToDelete?.file_path">{{ videoToDelete?.file_path }}</span>
          </div>

          <label class="confirm-checkbox-row">
            <input type="checkbox" v-model="alsoDeleteDiskFile" />
            <span class="checkbox-label-text">
              同时从本地磁盘彻底删除原视频文件
              <strong class="text-danger">（不可撤销）</strong>
            </span>
          </label>
        </div>

        <div class="confirm-modal-footer">
          <button
            class="btn btn-secondary"
            :disabled="isDeletingVideo"
            @click="showDeleteConfirmModal = false"
          >
            取消
          </button>
          <button
            class="btn btn-danger"
            :disabled="isDeletingVideo"
            @click="confirmRemoveVideo"
          >
            {{ isDeletingVideo ? '正在处理...' : (alsoDeleteDiskFile ? '彻底删除原文件' : '确认仅从媒体库移除') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Floating Toast Notification -->
    <div v-if="toastMessage" class="toast-popup">
      {{ toastMessage }}
    </div>
  </div>
</template>

<style scoped>
.video-library-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  overflow: hidden;
  background-color: var(--bg-app);
}

/* Toolbar */
.library-toolbar {
  height: 50px;
  min-height: 50px;
  padding: 0 1.25rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-header);
  gap: 1rem;
}

.toolbar-left-group {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex: 1;
}

.search-input-wrap {
  position: relative;
  width: 260px;
  display: flex;
  align-items: center;
}

.search-ico {
  position: absolute;
  left: 9px;
  width: 15px;
  height: 15px;
  color: var(--text-muted);
}

.search-input-field {
  width: 100%;
  padding: 0.4rem 1.8rem 0.4rem 2rem;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.82rem;
  outline: none;
}

.search-input-field:focus {
  border-color: #3b82f6;
}

.search-clear-btn {
  position: absolute;
  right: 6px;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.75rem;
}

.codec-pills-row {
  display: flex;
  background: var(--bg-input);
  padding: 2px;
  border-radius: 7px;
  gap: 2px;
}

.pill-filter-btn {
  background: none;
  border: none;
  padding: 0.25rem 0.6rem;
  border-radius: 5px;
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.pill-filter-btn.active {
  background: var(--bg-card);
  color: #3b82f6;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.select-dropdown {
  padding: 0.4rem 0.65rem;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.8rem;
  outline: none;
  cursor: pointer;
}

.toolbar-right-group {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.btn-toggle-scan {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.75rem;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-toggle-scan.active {
  background: #3b82f6;
  color: white;
  border-color: #3b82f6;
}

.view-switch-btns {
  display: flex;
  background: var(--bg-input);
  border-radius: 6px;
  padding: 2px;
}

.view-btn {
  background: none;
  border: none;
  padding: 4px 6px;
  border-radius: 5px;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
}

.view-btn.active {
  background: var(--bg-card);
  color: var(--text-primary);
}

.view-btn svg {
  width: 14px;
  height: 14px;
}

.video-counter-tag {
  font-size: 0.8rem;
  color: var(--text-muted);
}

/* Collapsible Scanner Drawer */
.scanner-drawer {
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-subtle);
  padding: 1rem 1.5rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.scanner-content-box {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-width: 900px;
}

/* Full-Disk Smart Scan Card */
.full-disk-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.5rem;
  padding: 1rem 1.25rem;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.08) 0%, rgba(99, 102, 241, 0.04) 100%);
  border: 1px solid rgba(59, 130, 246, 0.25);
  border-radius: 10px;
}

.full-disk-left {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.full-disk-heading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.badge-smart {
  padding: 0.15rem 0.5rem;
  background: #3b82f6;
  color: #ffffff;
  font-size: 0.7rem;
  font-weight: 700;
  border-radius: 4px;
  letter-spacing: 0.02em;
}

.full-disk-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
}

.full-disk-desc {
  font-size: 0.8rem;
  color: var(--text-secondary);
  line-height: 1.45;
  margin: 0;
}

.full-disk-filter-row {
  margin-top: 0.2rem;
}

.filter-opt-label {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.min-size-input {
  width: 52px;
  padding: 0.2rem 0.4rem;
  text-align: center;
  border-radius: 4px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.78rem;
}

.btn-run-full-scan {
  white-space: nowrap;
  padding: 0.6rem 1.25rem;
  font-weight: 600;
  font-size: 0.88rem;
  background: linear-gradient(135deg, #2563eb, #3b82f6);
  box-shadow: 0 2px 8px rgba(37, 99, 235, 0.3);
}

.btn-run-full-scan:hover {
  background: linear-gradient(135deg, #1d4ed8, #2563eb);
}

.btn-cancel-scan {
  white-space: nowrap;
  padding: 0.6rem 1.25rem;
  font-weight: 600;
  font-size: 0.88rem;
  background: #ef4444;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

/* Real-Time Scan Progress Strip */
.scan-progress-strip {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.75rem 1rem;
  background: var(--bg-card);
  border: 1px solid #bfdbfe;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.progress-info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.82rem;
}

.progress-msg {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: #1e40af;
}

.pulse-dot {
  width: 8px;
  height: 8px;
  background: #3b82f6;
  border-radius: 50%;
  animation: pulseDot 1.2s infinite;
}

@keyframes pulseDot {
  0% { transform: scale(0.9); opacity: 0.6; }
  50% { transform: scale(1.3); opacity: 1; }
  100% { transform: scale(0.9); opacity: 0.6; }
}

.progress-counts {
  font-size: 0.8rem;
  font-weight: 600;
  color: #2563eb;
}

.progress-bar-track {
  width: 100%;
  height: 6px;
  background: #e2e8f0;
  border-radius: 999px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #3b82f6, #6366f1);
  border-radius: 999px;
  transition: width 0.25s ease-out;
}

.progress-bar-fill.indeterminate {
  width: 40% !important;
  animation: moveIndeterminate 1.5s infinite linear;
}

@keyframes moveIndeterminate {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(350%); }
}

.drawer-divider {
  display: flex;
  align-items: center;
  text-align: center;
  color: var(--text-muted);
  font-size: 0.75rem;
  margin: 0.25rem 0;
}

.drawer-divider::before,
.drawer-divider::after {
  content: '';
  flex: 1;
  border-bottom: 1px dashed var(--border-subtle);
}

.drawer-divider span {
  padding: 0 0.75rem;
}

.scanner-input-line {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.folder-icon {
  width: 20px;
  height: 20px;
  color: #3b82f6;
}

.scan-dir-input {
  flex: 1;
  padding: 0.5rem 0.85rem;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.85rem;
  outline: none;
}

.scan-feedback-alert {
  padding: 0.65rem 0.95rem;
  border-radius: 7px;
  font-size: 0.82rem;
}

.fb-ok {
  background: #f0fdf4;
  color: #166534;
  border: 1px solid #bbf7d0;
}

.fb-err {
  background: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
}

.summary-tags-row {
  display: flex;
  gap: 1rem;
  margin-top: 0.35rem;
  font-size: 0.78rem;
}

.hl-green { color: #15803d; font-weight: 700; }
.hl-red { color: #b91c1c; font-weight: 700; }

/* Master-Detail Split Workspace */
.master-detail-workspace {
  flex: 1;
  display: flex;
  overflow: hidden;
  height: calc(100% - 50px);
}

/* Left Gallery Pane */
.gallery-pane {
  flex: 1;
  overflow-y: auto;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  position: relative;
}

.gallery-loading-skeleton {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 5rem 2rem;
  color: var(--text-muted);
  font-size: 0.9rem;
}

.spinner-lg {
  width: 28px;
  height: 28px;
  border: 3px solid rgba(59, 130, 246, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.virtual-scroll-viewport {
  position: relative;
  width: 100%;
}

.virtual-row-wrap {
  position: absolute;
  left: 0;
  right: 0;
  box-sizing: border-box;
}

/* Date Group Header (按天时间分割) */
.date-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.45rem 0.85rem;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 9px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.dgh-left {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.dgh-calendar-icon {
  width: 17px;
  height: 17px;
  color: #3b82f6;
  flex-shrink: 0;
}

.dgh-title {
  font-weight: 700;
  font-size: 0.88rem;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}

.dgh-badge {
  font-size: 0.72rem;
  padding: 1px 7px;
  border-radius: 999px;
  background: rgba(59, 130, 246, 0.12);
  color: #3b82f6;
  font-weight: 600;
}

.dgh-right {
  display: flex;
  align-items: center;
}

.dgh-size {
  font-size: 0.75rem;
  color: var(--text-muted);
}

/* Virtual Grid Row */
.virtual-grid-row {
  display: grid;
  gap: 1.1rem;
  width: 100%;
  height: calc(100% - 16px);
}

/* Sticky List View Header */
.list-head-sticky-row {
  position: sticky;
  top: 0;
  z-index: 10;
  display: grid;
  grid-template-columns: 56px 1fr 80px 80px 75px 75px 140px;
  padding: 0.6rem 0.85rem;
  background: var(--bg-input);
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  margin-bottom: 0.5rem;
}

/* Infinite Scroll Footer */
.infinite-scroll-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.25rem 0;
  width: 100%;
}

.loading-more-pill {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.4rem 1.1rem;
  border-radius: 999px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  font-size: 0.8rem;
  color: var(--text-muted);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.all-loaded-pill {
  padding: 0.35rem 1rem;
  border-radius: 999px;
  background: var(--bg-input);
  font-size: 0.75rem;
  color: var(--text-muted);
}

.video-grid-card {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 11px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.15s ease-in-out;
  display: flex;
  flex-direction: column;
  height: 100%;
  box-sizing: border-box;
}

.video-grid-card:hover {
  transform: translateY(-2px);
  border-color: rgba(59, 130, 246, 0.4);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.08);
}

.video-grid-card.active {
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.25);
  background: var(--nav-active-bg);
}

.grid-thumb-box {
  position: relative;
  width: 100%;
  padding-top: 56.25%; /* 16:9 */
  background: #090e17;
  overflow: hidden;
  flex-shrink: 0;
}

.grid-thumb-img {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.grid-thumb-placeholder {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #475569;
}

.grid-thumb-placeholder svg {
  width: 32px;
  height: 32px;
}

.pill-badge-corner {
  position: absolute;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 600;
  color: #ffffff;
  backdrop-filter: blur(4px);
}

.pill-duration {
  bottom: 6px;
  right: 6px;
  background: rgba(0, 0, 0, 0.75);
}

.pill-reso {
  top: 6px;
  left: 6px;
  background: rgba(37, 99, 235, 0.85);
}

.grid-card-info {
  padding: 0.65rem 0.75rem 0.75rem;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  flex: 1;
  min-height: 0;
  box-sizing: border-box;
}

.grid-card-title {
  font-size: 0.85rem;
  font-weight: 600;
  line-height: 1.35;
  min-height: 2.7em;
  max-height: 2.7em;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  color: var(--text-primary);
  margin-bottom: 0.2rem;
}

.grid-card-sub {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.72rem;
  color: var(--text-muted);
  margin-bottom: 0.35rem;
}

.codec-pill {
  background: var(--bg-input);
  padding: 1px 5px;
  border-radius: 4px;
  font-weight: 600;
}

/* List View */
.videos-list-wrap {
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  overflow: hidden;
}

.list-head-row {
  display: grid;
  grid-template-columns: 60px 1fr 85px 85px 75px 80px;
  padding: 0.6rem 0.85rem;
  background: var(--bg-input);
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border-subtle);
}

.list-item-row {
  display: grid;
  grid-template-columns: 56px 1fr 80px 80px 75px 75px 140px;
  padding: 0.5rem 0.85rem;
  align-items: center;
  font-size: 0.8rem;
  border-bottom: 1px solid var(--border-subtle);
  cursor: pointer;
  transition: background 0.1s ease;
}

.list-item-row:hover {
  background: var(--btn-hover);
}

.list-item-row.active {
  background: var(--nav-active-bg);
  border-left: 3px solid #3b82f6;
}

.li-thumb {
  width: 50px;
  height: 28px;
  border-radius: 4px;
  overflow: hidden;
  background: #090e17;
}

.li-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.li-thumb-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  font-size: 0.7rem;
}

.li-title-col {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-right: 0.75rem;
}

.li-name {
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text-primary);
}

.li-path {
  font-size: 0.7rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Empty Gallery */
.empty-gallery-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 2rem;
  text-align: center;
}

.empty-icon-circle {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: var(--bg-input);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  margin-bottom: 1rem;
}

.empty-icon-circle svg {
  width: 30px;
  height: 30px;
}

.empty-h3 {
  margin: 0 0 0.4rem;
  font-size: 1.1rem;
  color: var(--text-primary);
}

.empty-p {
  margin: 0 0 1.25rem;
  font-size: 0.85rem;
  color: var(--text-muted);
  max-width: 420px;
}

/* Thumbnail Hover Overlay & Play Badge */
.grid-thumb-box:hover .grid-thumb-hover-overlay {
  opacity: 1;
}

.grid-thumb-hover-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.2s ease;
  backdrop-filter: blur(2px);
  z-index: 2;
}

.play-icon-badge {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  background: rgba(59, 130, 246, 0.9);
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.1rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  transform: scale(0.9);
  transition: transform 0.15s ease;
}

.grid-thumb-box:hover .play-icon-badge {
  transform: scale(1.05);
}

/* Card Quick Action Bar */
.card-quick-actions {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  margin-top: auto;
  padding-top: 0.45rem;
  border-top: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.card-act-btn {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  font-size: 0.72rem;
  padding: 5px 3px;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.card-act-btn:hover {
  background: var(--btn-hover);
  color: #3b82f6;
  border-color: rgba(59, 130, 246, 0.4);
}

.card-act-btn.btn-act-del:hover {
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.4);
  background: rgba(239, 68, 68, 0.08);
}

/* List Table Actions */
.lh-actions {
  text-align: right;
  padding-right: 0.25rem;
}

.li-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.3rem;
}

.li-btn {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  font-size: 0.75rem;
  padding: 3px 7px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.li-btn:hover {
  background: var(--btn-hover);
  color: #3b82f6;
  border-color: rgba(59, 130, 246, 0.4);
}

.li-btn.btn-act-del:hover {
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.4);
  background: rgba(239, 68, 68, 0.08);
}

.spinner-sm {
  width: 13px;
  height: 13px;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}

/* Floating Toast Notification */
.toast-popup {
  position: fixed;
  bottom: 24px;
  right: 24px;
  background: rgba(15, 23, 42, 0.95);
  color: #ffffff;
  padding: 10px 18px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.12);
  z-index: 99999;
  animation: toastFadeIn 0.2s ease-out;
  pointer-events: none;
}

@keyframes toastFadeIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Custom Confirmation Modal */
.confirm-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 99998;
  animation: modalFadeIn 0.15s ease-out;
}

.confirm-modal-card {
  width: 480px;
  max-width: 90vw;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 14px;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.35);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.confirm-modal-header {
  padding: 1.1rem 1.25rem;
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  gap: 0.85rem;
  position: relative;
}

.confirm-icon-box {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: rgba(239, 68, 68, 0.12);
  color: #ef4444;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.confirm-icon-box svg {
  width: 20px;
  height: 20px;
}

.confirm-header-text {
  flex: 1;
}

.confirm-title {
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.confirm-subtitle {
  font-size: 0.78rem;
  color: var(--text-muted);
  margin: 2px 0 0;
}

.confirm-close-btn {
  background: transparent;
  border: none;
  font-size: 1.1rem;
  color: var(--text-muted);
  cursor: pointer;
  padding: 4px;
  line-height: 1;
}

.confirm-close-btn:hover {
  color: var(--text-primary);
}

.confirm-modal-body {
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.video-info-preview {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.preview-name {
  font-size: 0.88rem;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-path {
  font-size: 0.72rem;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.confirm-checkbox-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.82rem;
  color: var(--text-primary);
  cursor: pointer;
  user-select: none;
}

.confirm-checkbox-row input[type="checkbox"] {
  width: 16px;
  height: 16px;
  accent-color: #ef4444;
}

.text-danger {
  color: #ef4444;
}

.confirm-modal-footer {
  padding: 0.85rem 1.25rem;
  border-top: 1px solid var(--border-subtle);
  background: var(--bg-header);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.65rem;
}

.btn-danger {
  background: #ef4444;
  color: #ffffff;
  border: none;
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-weight: 600;
  font-size: 0.85rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-danger:hover:not(:disabled) {
  background: #dc2626;
  box-shadow: 0 2px 8px rgba(239, 68, 68, 0.4);
}

.btn-danger:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@keyframes modalFadeIn {
  from { opacity: 0; transform: scale(0.97); }
  to { opacity: 1; transform: scale(1); }
}
</style>

