<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import VideoPlayerModal from './VideoPlayerModal.vue';
import VideoAiDrawer from './VideoAiDrawer.vue';
import AudioExtractionModal from './AudioExtractionModal.vue';
import CategorySidebar, { type CategorySelection } from './CategorySidebar.vue';
import type { Video, VideoFilter, VideoListResponse, ScanSummary, ScanProgress, CategoryStats, Tag, SubtitleSnippetMatch, VideoAiStatus } from '../types';
import { t } from '../i18n';

const emit = defineEmits<{
  (e: 'navigateDedup'): void;
  (e: 'navigateAudios'): void;
  (e: 'navigateClipper', video?: Video): void;
}>();

// Subtitle snippet dialogue search matches
const subtitleSnippets = ref<Map<number, SubtitleSnippetMatch[]>>(new Map());

// AI Intelligence drawer state & batch status
const showAiDrawer = ref<boolean>(false);
const aiDrawerVideo = ref<Video | null>(null);
const aiStatusMap = ref<Map<number, VideoAiStatus>>(new Map());
const playerInitialSeek = ref<number>(0);

// Classification & Organization state
const categoryStats = ref<CategoryStats | null>(null);
const isCategorySidebarCollapsed = ref<boolean>(localStorage.getItem('vidnest_category_collapsed') === 'true');
const activeCategory = ref<CategorySelection>({ type: 'all', value: 'all', label: t('library.allVideos') });

// Multi-dimensional filters
const selectedQuality = ref<string>('');
const selectedDurationRange = ref<string>('');
const selectedWatchStatus = ref<string>('');

// Batch selection
const selectedVideoIds = ref<Set<number>>(new Set());

// Tag modal state
const showTagModal = ref<boolean>(false);
const videoForTagging = ref<Video | null>(null);

// Hover scrub state (storyboard preview simulation)
const hoverVideoId = ref<number | null>(null);
const hoverScrubPercent = ref<number>(0);

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

const showAudioExtractModal = ref<boolean>(false);
const videoForAudioExtract = ref<Video | null>(null);

function openAudioExtract(video: Video) {
  videoForAudioExtract.value = video;
  showAudioExtractModal.value = true;
}

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

async function openVideo(video: Video, startSec: number = 0) {
  try {
    showToast(t('library.startingPlayback', { name: video.file_name }));
    await invoke('open_video_player_window', { video });
  } catch (err) {
    console.warn('Failed to open native player window, falling back to modal:', err);
    activePlayerVideo.value = video;
    playerInitialSeek.value = startSec;
    isPlayerVisible.value = true;
  }
}

function openAiDrawer(video: Video) {
  aiDrawerVideo.value = video;
  showAiDrawer.value = true;
}

function handleAiDrawerPlay(video: Video, startSec: number = 0) {
  showAiDrawer.value = false;
  openVideo(video, startSec);
}

async function handleAiTagsUpdated() {
  await fetchCategoryStats();
  await fetchVideos(false);
}

async function fetchAiStatusForVideos(videoList?: Video[]) {
  const list = videoList || videos.value;
  if (!list || list.length === 0) return;
  const ids = list.map((v) => v.id);
  try {
    const res = await invoke<Record<string, VideoAiStatus>>('get_videos_ai_status', { videoIds: ids });
    const newMap = new Map(aiStatusMap.value);
    for (const [key, val] of Object.entries(res)) {
      newMap.set(Number(key), val);
    }
    aiStatusMap.value = newMap;
  } catch (err) {
    console.warn('Failed to load videos AI status:', err);
  }
}

async function revealVideo(filePath: string) {
  try {
    await invoke('reveal_in_explorer', { path: filePath });
    showToast(t('library.locatedInFileManager'));
  } catch (err) {
    try {
      await revealItemInDir(filePath);
      showToast(t('library.locatedInFileManager'));
    } catch (e) {
      console.error('Failed to reveal video:', e);
      showToast(t('library.locateFailed', { err: e instanceof Error ? e.message : String(e) }));
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
    return { key: 'unknown', label: t('library.olderOrUnknown'), dateStr: t('library.olderOrUnknown') };
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

  const dateFormatted = date.toLocaleDateString(undefined, {
    year: date.getFullYear() === now.getFullYear() ? undefined : 'numeric',
    month: 'short',
    day: 'numeric',
    weekday: 'short',
  });

  let label = dateFormatted;
  if (key === todayKey) {
    label = `${t('library.today')} · ${dateFormatted}`;
  } else if (key === yesterdayKey) {
    label = `${t('library.yesterday')} · ${dateFormatted}`;
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

async function fetchCategoryStats() {
  try {
    const stats = await invoke<CategoryStats>('get_category_stats');
    categoryStats.value = stats;
  } catch (err) {
    console.warn('Failed to fetch category stats:', err);
  }
}

function toggleCategorySidebar() {
  isCategorySidebarCollapsed.value = !isCategorySidebarCollapsed.value;
  localStorage.setItem('vidnest_category_collapsed', String(isCategorySidebarCollapsed.value));
}

function handleCategorySelect(selection: CategorySelection) {
  activeCategory.value = selection;
  fetchVideos(true);
}

function resetAllFilters() {
  activeCategory.value = { type: 'all', value: 'all', label: t('library.allVideos') };
  selectedCodec.value = '';
  selectedQuality.value = '';
  selectedDurationRange.value = '';
  selectedWatchStatus.value = '';
  searchText.value = '';
  fetchVideos(true);
}

async function toggleStar(video: Video) {
  try {
    const newStarred = await invoke<boolean>('toggle_video_star', { videoId: video.id });
    video.is_starred = newStarred;
    showToast(newStarred ? t('library.addedToFavorites') : t('library.removedFromFavorites'));
    fetchCategoryStats();
  } catch (err) {
    showToast(t('library.favoriteFailed', { err: String(err) }));
  }
}

async function cycleWatchStatus(video: Video) {
  const nextStatus = video.watch_status === 'unwatched' ? 'in_progress' : video.watch_status === 'in_progress' ? 'watched' : 'unwatched';
  const progress = nextStatus === 'watched' ? 1.0 : (nextStatus === 'unwatched' ? 0.0 : 0.5);
  try {
    await invoke('update_video_watch_progress', {
      videoId: video.id,
      status: nextStatus,
      progress,
    });
    video.watch_status = nextStatus as any;
    video.playback_progress = progress;
    showToast(t('library.markedAs', { status: formatWatchStatus(nextStatus) }));
    fetchCategoryStats();
  } catch (err) {
    showToast(t('library.updateStatusFailed', { err: String(err) }));
  }
}

function formatWatchStatus(status: string): string {
  if (status === 'watched') return t('library.watched');
  if (status === 'in_progress') return t('library.inProgress');
  return t('library.unwatched');
}

async function cycleColorTag(video: Video) {
  const colors = [null, 'red', 'yellow', 'green', 'blue'];
  const currentIndex = colors.indexOf(video.color_tag as any);
  const nextColor = colors[(currentIndex + 1) % colors.length];
  try {
    await invoke('set_video_color_tag', {
      videoId: video.id,
      color: nextColor,
    });
    video.color_tag = nextColor;
    showToast(nextColor ? t('library.colorMarked') : t('library.colorCleared'));
    fetchCategoryStats();
  } catch (err) {
    showToast(t('library.colorMarkFailed', { err: String(err) }));
  }
}

function openTagModal(video: Video) {
  videoForTagging.value = video;
  showTagModal.value = true;
}

async function toggleTagOnVideo(tag: Tag) {
  if (!videoForTagging.value) return;
  const video = videoForTagging.value;
  const hasTag = video.tags?.some((t) => t.id === tag.id);
  try {
    if (hasTag) {
      await invoke('remove_tag_from_video', { videoId: video.id, tagId: tag.id });
      video.tags = video.tags.filter((t) => t.id !== tag.id);
    } else {
      await invoke('add_tag_to_video', { videoId: video.id, tagId: tag.id });
      if (!video.tags) video.tags = [];
      video.tags.push(tag);
    }
    fetchCategoryStats();
  } catch (err) {
    showToast(t('library.tagUpdateFailed', { err: String(err) }));
  }
}

function toggleSelectVideo(id: number) {
  if (selectedVideoIds.value.has(id)) {
    selectedVideoIds.value.delete(id);
  } else {
    selectedVideoIds.value.add(id);
  }
}

function clearSelection() {
  selectedVideoIds.value.clear();
}

async function batchMarkWatched() {
  if (selectedVideoIds.value.size === 0) return;
  const ids = Array.from(selectedVideoIds.value);
  try {
    await invoke('batch_update_videos_status', {
      videoIds: ids,
      watchStatus: 'watched',
    });
    for (const v of videos.value) {
      if (selectedVideoIds.value.has(v.id)) {
        v.watch_status = 'watched';
        v.playback_progress = 1.0;
      }
    }
    showToast(t('library.batchMarkedWatched', { count: ids.length }));
    clearSelection();
    fetchCategoryStats();
  } catch (err) {
    showToast(t('library.batchActionFailed', { err: String(err) }));
  }
}

async function batchStarVideos() {
  if (selectedVideoIds.value.size === 0) return;
  const ids = Array.from(selectedVideoIds.value);
  try {
    await invoke('batch_update_videos_status', {
      videoIds: ids,
      isStarred: true,
    });
    for (const v of videos.value) {
      if (selectedVideoIds.value.has(v.id)) {
        v.is_starred = true;
      }
    }
    showToast(t('library.batchStarred', { count: ids.length }));
    clearSelection();
    fetchCategoryStats();
  } catch (err) {
    showToast(t('library.batchStarFailed', { err: String(err) }));
  }
}

function handleThumbMouseMove(video: Video, event: MouseEvent) {
  const target = event.currentTarget as HTMLElement;
  if (!target) return;
  const rect = target.getBoundingClientRect();
  const offsetX = event.clientX - rect.left;
  const percent = Math.max(0, Math.min(1, offsetX / rect.width));
  hoverVideoId.value = video.id;
  hoverScrubPercent.value = percent;
}

function handleThumbMouseLeave() {
  hoverVideoId.value = null;
  hoverScrubPercent.value = 0;
}

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
      quality: selectedQuality.value || undefined,
      duration_range: selectedDurationRange.value || undefined,
      watch_status: selectedWatchStatus.value || undefined,
    };

    if (activeCategory.value.type === 'smart') {
      if (activeCategory.value.value === 'starred') {
        filter.is_starred = true;
      } else if (['unwatched', 'in_progress', 'watched'].includes(activeCategory.value.value)) {
        filter.watch_status = activeCategory.value.value;
      }
    } else if (activeCategory.value.type === 'format') {
      filter.format_category = activeCategory.value.value;
    } else if (activeCategory.value.type === 'color') {
      filter.color_tag = activeCategory.value.value;
    } else if (activeCategory.value.type === 'tag') {
      filter.tag_id = Number(activeCategory.value.value);
    } else if (activeCategory.value.type === 'folder') {
      filter.source_folder = activeCategory.value.value;
    } else if (activeCategory.value.type === 'collection') {
      filter.collection_id = Number(activeCategory.value.value);
    }

    const res = await invoke<VideoListResponse>('list_videos', { filter });
    if (reset) {
      videos.value = res.items;
      totalVideos.value = res.total;

      const q = searchText.value.trim();
      if (q) {
        try {
          const matches = await invoke<SubtitleSnippetMatch[]>('search_subtitles_snippets', {
            query: q,
            limit: 50,
          });
          const map = new Map<number, SubtitleSnippetMatch[]>();
          matches.forEach((m) => {
            const arr = map.get(m.video_id) || [];
            arr.push(m);
            map.set(m.video_id, arr);
          });
          subtitleSnippets.value = map;
        } catch (e) {
          console.warn('Subtitle snippet search failed:', e);
          subtitleSnippets.value = new Map();
        }
      } else {
        subtitleSnippets.value = new Map();
      }
    } else {
      const existingIds = new Set(videos.value.map((v) => v.id));
      const newItems = res.items.filter((v) => !existingIds.has(v.id));
      videos.value.push(...newItems);
      totalVideos.value = res.total;
    }

    hasMore.value = videos.value.length < res.total;
    fetchAiStatusForVideos(videos.value);
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
    message: t('library.btnScanning'),
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
    scanError.value = `${t('common.failed')}: ${err instanceof Error ? err.message : String(err)}`;
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
    scanError.value = t('library.invalidPathError');
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
    scanError.value = `${t('common.failed')}: ${err instanceof Error ? err.message : String(err)}`;
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
    showToast(deleteFile ? t('library.fileDeletedPermanently', { name: v.file_name }) : t('library.fileRemovedFromLibrary', { name: v.file_name }));
    showDeleteConfirmModal.value = false;
    videoToDelete.value = null;
  } catch (err: unknown) {
    showToast(t('library.removeFailed', { err: err instanceof Error ? err.message : String(err) }));
  } finally {
    isDeletingVideo.value = false;
  }
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast(t('library.pathCopiedToClipboard'));
  } catch {
    const el = document.createElement('textarea');
    el.value = text;
    document.body.appendChild(el);
    el.select();
    document.execCommand('copy');
    document.body.removeChild(el);
    showToast(t('library.pathCopiedToClipboard'));
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
  fetchCategoryStats();
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
        fetchCategoryStats();
      }
    });

    unlistenVideoEnriched = await listen<number>('video:enriched', () => {
      fetchVideos(false);
      fetchCategoryStats();
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
  <div class="video-library-split-layout">
    <!-- Category Sidebar (Collapsible) -->
    <CategorySidebar
      :stats="categoryStats"
      :active-selection="activeCategory"
      :is-collapsed="isCategorySidebarCollapsed"
      @select="handleCategorySelect"
      @toggle-collapse="toggleCategorySidebar"
      @refresh-stats="fetchCategoryStats"
    />

    <!-- Main Video Library Pane -->
    <div class="video-library-main-pane">
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
                :placeholder="t('library.searchInputPlaceholder')"
                @input="onSearchInput"
              />
              <button
                v-if="searchText"
                class="search-clear-btn"
                :title="t('library.clearSearch')"
                @click="clearSearch"
              >✕</button>
            </div>

            <div class="toolbar-divider"></div>

            <!-- Action Buttons Group -->
            <div class="toolbar-actions-group">
              <!-- Scan / Import Folder Toggle Button -->
              <button
                class="btn-toolbar-action"
                :class="{ active: showScanner }"
                :title="t('library.scannerTooltip')"
                @click="showScanner = !showScanner"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                </svg>
                <span>{{ showScanner ? t('library.collapseScanner') : t('library.openScanner') }}</span>
              </button>

              <!-- Jump to Deduplication View Button -->
              <button
                class="btn-toolbar-action"
                :title="t('library.dedupTooltip')"
                @click="emit('navigateDedup')"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
                <span>{{ t('library.dedupBtn') }}</span>
              </button>

              <!-- Audio Library Button -->
              <button
                class="btn-toolbar-action"
                :title="t('library.audiosTooltip')"
                @click="emit('navigateAudios')"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M9 18V5l12-2v13"></path>
                  <circle cx="6" cy="18" r="3"></circle>
                  <circle cx="18" cy="16" r="3"></circle>
                </svg>
                <span>{{ t('library.audiosBtn') }}</span>
              </button>
            </div>
          </div>

          <div class="toolbar-right-group">
            <!-- Sort dropdown -->
            <select
              v-model="selectedSort"
              class="sort-select"
              :title="t('library.sortRecent')"
              @change="fetchVideos(true)"
            >
              <option value="created_desc">{{ t('library.sortCreatedDesc') }}</option>
              <option value="created_asc">{{ t('library.sortCreatedAsc') }}</option>
              <option value="duration_desc">{{ t('library.sortDurationDesc') }}</option>
              <option value="duration_asc">{{ t('library.sortDurationAsc') }}</option>
              <option value="size_desc">{{ t('library.sortSizeDesc') }}</option>
              <option value="size_asc">{{ t('library.sortSizeAsc') }}</option>
              <option value="name_asc">{{ t('library.sortNameAsc') }}</option>
            </select>

            <div class="toolbar-divider"></div>

            <!-- View mode switcher -->
            <div class="view-switch-btns">
              <button
                class="view-btn"
                :class="{ active: viewMode === 'grid' }"
                :title="t('library.gridPave')"
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
                :title="t('library.compactList')"
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
              {{ t('library.totalCount', { count: totalVideos }) }}
            </span>
          </div>
        </header>

        <!-- Category Breadcrumb & Filter Reset Bar -->
        <div class="category-breadcrumb-bar">
          <div class="breadcrumb-left">
            <div class="active-category-pill">
              <span class="breadcrumb-prefix-label">{{ t('library.currentView') }}</span>
              <span class="active-category-title">{{ activeCategory.label }}</span>
            </div>

            <div class="toolbar-divider compact"></div>

            <!-- Multi-dimensional Filter Dropdowns -->
            <div class="filter-dropdowns-row">
              <select
                v-model="selectedQuality"
                class="filter-select font-mono"
                :class="{ 'is-filtered': selectedQuality }"
                :title="t('library.filterQualityTooltip')"
                @change="fetchVideos(true)"
              >
                <option value="">{{ t('library.filterQualityAll') }}</option>
                <option value="4k">{{ t('library.filterQuality4k') }}</option>
                <option value="1080p">{{ t('library.filterQuality1080p') }}</option>
                <option value="720p">{{ t('library.filterQuality720p') }}</option>
                <option value="sd">{{ t('library.filterQualitySd') }}</option>
              </select>

              <select
                v-model="selectedDurationRange"
                class="filter-select"
                :class="{ 'is-filtered': selectedDurationRange }"
                :title="t('library.filterDurationTooltip')"
                @change="fetchVideos(true)"
              >
                <option value="">{{ t('library.filterDurationAll') }}</option>
                <option value="under_3m">{{ t('library.filterDurationUnder3') }}</option>
                <option value="3_to_30m">{{ t('library.filterDuration3to30') }}</option>
                <option value="over_30m">{{ t('library.filterDurationOver30') }}</option>
              </select>

              <select
                v-model="selectedWatchStatus"
                class="filter-select"
                :class="{ 'is-filtered': selectedWatchStatus }"
                :title="t('library.filterStatusTooltip')"
                @change="fetchVideos(true)"
              >
                <option value="">{{ t('library.filterStatusAll') }}</option>
                <option value="unwatched">{{ t('library.filterStatusUnwatched') }}</option>
                <option value="in_progress">{{ t('library.filterStatusInProgress') }}</option>
                <option value="watched">{{ t('library.filterStatusWatched') }}</option>
              </select>

              <select
                v-model="selectedCodec"
                class="filter-select font-mono"
                :class="{ 'is-filtered': selectedCodec }"
                :title="t('library.filterCodecTooltip')"
                @change="fetchVideos(true)"
              >
                <option value="">{{ t('library.filterCodecAll') }}</option>
                <option v-for="c in (availableCodecs.length > 0 ? availableCodecs : ['H264', 'HEVC', 'AV1'])" :key="c" :value="c">
                  {{ c }}
                </option>
              </select>
            </div>
          </div>

          <div class="breadcrumb-right">
            <span
              v-if="selectedQuality || selectedDurationRange || selectedWatchStatus || selectedCodec"
              class="filter-badge-hint"
            >
              {{ t('library.filterAppliedHint') }}
            </span>

            <button
              v-if="selectedQuality || selectedDurationRange || selectedWatchStatus || selectedCodec || activeCategory.type !== 'all' || searchText"
              class="btn-reset-filters"
              :title="t('library.resetFiltersTooltip')"
              @click="resetAllFilters"
            >
              {{ t('library.resetFilters') }} ✕
            </button>
          </div>
        </div>

    <!-- Collapsible Scan Directory Panel -->
    <transition name="drawer">
      <div v-if="showScanner" class="scanner-drawer">
        <div class="scanner-content-box">
          <!-- 1. Full-Disk Smart Scan Card -->
          <div class="full-disk-card">
            <div class="full-disk-left">
              <div class="full-disk-heading">
                <span class="badge-smart">{{ t('library.systemFast') }}</span>
                <span class="full-disk-title">{{ t('library.fullDiskSmartScan') }}</span>
              </div>
              <p class="full-disk-desc">
                {{ t('library.fullDiskDesc') }}
              </p>
              <div class="full-disk-filter-row">
                <label class="filter-opt-label">
                  <span>{{ t('library.minSizeThreshold') }}</span>
                  <input
                    type="number"
                    v-model.number="minSizeMb"
                    min="1"
                    max="2000"
                    class="min-size-input font-mono"
                    :disabled="isScanning"
                  />
                  <span>{{ t('library.minSizeThresholdSuffix') }}</span>
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
                <span>{{ t('library.btnStartFullScan') }}</span>
              </button>
              <button
                v-else
                type="button"
                class="btn btn-danger btn-cancel-scan"
                :disabled="isCancelling"
                @click="cancelScan"
              >
                <span class="spinner-sm"></span>
                <span>{{ isCancelling ? t('library.btnStopping') : t('library.btnStopScan') }}</span>
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
            <span>{{ t('library.orSpecificFolder') }}</span>
          </div>

          <!-- 3. Custom Single Folder Line -->
          <div class="scanner-input-line">
            <svg class="folder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
            <input
              v-model="scanPath"
              class="scan-dir-input font-mono"
              :placeholder="t('library.scanFolderPlaceholder')"
              :disabled="isScanning"
              @keyup.enter="triggerScan"
            />
            <button class="btn btn-secondary" :disabled="isScanning" @click="triggerScan">
              <span v-if="isScanning" class="spinner-sm"></span>
              <span>{{ isScanning ? t('library.btnScanning') : t('library.btnScanSpecific') }}</span>
            </button>
          </div>

          <div v-if="scanError" class="scan-feedback-alert fb-err">
            {{ scanError }}
          </div>

          <div v-if="scanSummary" class="scan-feedback-alert fb-ok">
            <div class="summary-line">
              {{ scanSummary.scanned_dir }} · {{ t('library.scannedCost') }} <strong>{{ scanSummary.duration_ms }} ms</strong>
            </div>
            <div class="summary-tags-row">
              <span>{{ t('library.foundVideos') }}: <strong>{{ scanSummary.video_files_found }}</strong></span>
              <span class="hl-green">{{ t('library.newImported') }}: <strong>+{{ scanSummary.new_imported }}</strong></span>
              <span>{{ t('library.updated') }}: <strong>{{ scanSummary.updated }}</strong></span>
              <span>{{ t('library.skippedExist') }}: <strong>{{ scanSummary.skipped }}</strong></span>
              <span v-if="scanSummary.failed > 0" class="hl-red">{{ t('library.failedCount') }}: <strong>{{ scanSummary.failed }}</strong></span>
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
          <h3 class="empty-h3">{{ t('library.emptyTitle') }}</h3>
          <p class="empty-p">{{ t('library.emptyDesc') }}</p>
          <button class="btn btn-secondary" @click="showScanner = true">
            {{ t('library.openScannerBtn') }}
          </button>
        </div>

        <!-- Initial Loading State Skeleton -->
        <div v-else-if="isLoading && videos.length === 0" class="gallery-loading-skeleton">
          <div class="spinner-lg"></div>
          <span>{{ t('library.loadingVideos') }}</span>
        </div>

        <!-- Virtual Scroll Container with Date Grouping -->
        <div v-else class="virtual-scroll-viewport" :style="{ height: totalVirtualHeight + 'px' }">
          <!-- List Mode Header (Sticky atop list view) -->
          <div v-if="viewMode === 'list'" class="list-head-sticky-row">
            <span class="lh-check"></span>
            <span class="lh-star"></span>
            <span class="lh-thumb">{{ t('library.colThumbnail') }}</span>
            <span class="lh-title">{{ t('library.colTitlePath') }}</span>
            <span class="lh-dur">{{ t('library.colDuration') }}</span>
            <span class="lh-reso">{{ t('library.colResolution') }}</span>
            <span class="lh-codec">{{ t('library.colCodec') }}</span>
            <span class="lh-size">{{ t('library.colSize') }}</span>
            <span class="lh-actions">{{ t('library.colActions') }}</span>
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
                <span class="dgh-badge">{{ t('library.videosCount', { count: item.row.group.videos.length }) }}</span>
              </div>
              <div class="dgh-right">
                <span class="dgh-size font-mono">{{ t('library.totalSize', { size: formatFileSize(item.row.group.totalSize) }) }}</span>
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
                :class="{ 'is-selected': selectedVideoIds.has(video.id) }"
                @dblclick="openVideo(video)"
              >
                <!-- Multi-select Checkbox (top-left) -->
                <div
                  class="card-select-checkbox"
                  :class="{ checked: selectedVideoIds.has(video.id) }"
                  :title="t('library.multiSelect')"
                  @click.stop="toggleSelectVideo(video.id)"
                >
                  <span v-if="selectedVideoIds.has(video.id)">✓</span>
                </div>

                <!-- Card Top Badges (Star, Color, Watch Status) -->
                <div class="card-top-badges">
                  <button
                    class="badge-star-btn"
                    :class="{ starred: video.is_starred }"
                    :title="video.is_starred ? t('library.starred') : t('library.unstarred')"
                    @click.stop="toggleStar(video)"
                  >
                    ★
                  </button>
                  <button
                    v-if="video.color_tag"
                    class="badge-color-dot"
                    :class="video.color_tag"
                    :title="t('library.colorTagTooltip')"
                    @click.stop="cycleColorTag(video)"
                  ></button>
                  <button
                    class="badge-watch-status"
                    :class="video.watch_status"
                    :title="t('library.watchStatusTooltip')"
                    @click.stop="cycleWatchStatus(video)"
                  >
                    {{ formatWatchStatus(video.watch_status) }}
                  </button>
                </div>

                <!-- 16:9 Thumbnail -->
                <div
                  class="grid-thumb-box"
                  @click="openVideo(video)"
                  @mousemove="handleThumbMouseMove(video, $event)"
                  @mouseleave="handleThumbMouseLeave"
                  :title="t('library.clickToPlayPreview')"
                >
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

                  <!-- Hover Scrub Visual Cursor & Timestamp -->
                  <div
                    v-if="hoverVideoId === video.id && video.duration"
                    class="hover-scrub-cursor"
                    :style="{ left: (hoverScrubPercent * 100) + '%' }"
                  >
                    <span class="hover-scrub-time font-mono">
                      {{ formatDuration(video.duration * hoverScrubPercent) }}
                    </span>
                  </div>

                  <!-- Playback Progress Bar (at bottom of thumb) -->
                  <div
                    v-if="video.playback_progress > 0"
                    class="playback-progress-track"
                    :title="t('library.playedPercent', { percent: Math.round(video.playback_progress * 100) })"
                  >
                    <div
                      class="playback-progress-fill"
                      :style="{ width: (video.playback_progress * 100) + '%' }"
                    ></div>
                  </div>

                  <!-- Overlaid badges -->
                  <span v-if="video.duration" class="pill-badge-corner pill-duration font-mono">
                    {{ formatDuration(video.duration) }}
                  </span>
                  <span v-if="getResolutionBadge(video.width, video.height)" class="pill-badge-corner pill-reso">
                    {{ getResolutionBadge(video.width, video.height) }}
                  </span>
                  <span
                    v-if="aiStatusMap.get(video.id)?.has_analysis"
                    class="pill-badge-corner pill-ai-analysis font-mono"
                    :title="t('library.aiSummaryReady')"
                    @click.stop="openAiDrawer(video)"
                  >
                    {{ t('library.aiBadgeSummary') }}
                  </span>
                  <span
                    v-else-if="aiStatusMap.get(video.id)?.has_subtitles"
                    class="pill-badge-corner pill-ai-subtitles font-mono"
                    :title="t('library.aiSubtitlesReady', { count: aiStatusMap.get(video.id)?.subtitle_count ?? 0 })"
                    @click.stop="openAiDrawer(video)"
                  >
                    {{ t('library.aiBadgeLines', { count: aiStatusMap.get(video.id)?.subtitle_count ?? 0 }) }}
                  </span>
                </div>

                <!-- Card Info -->
                <div class="grid-card-info">
                  <span class="grid-card-title" :title="video.file_name" @click="openVideo(video)">
                    {{ video.file_name }}
                  </span>

                  <!-- Tags List / Add Tag Button -->
                  <div class="card-tags-row">
                    <span
                      v-for="tagItem in video.tags || []"
                      :key="tagItem.id"
                      class="card-tag-pill"
                      :title="t('library.filterByTag', { name: tagItem.name })"
                      @click.stop="handleCategorySelect({ type: 'tag', value: String(tagItem.id), label: `#${tagItem.name}` })"
                    >
                      #{{ tagItem.name }}
                    </span>
                    <button class="add-tag-pill-btn" :title="t('library.addTagTooltip')" @click.stop="openTagModal(video)">
                      {{ t('library.addTagBtn') }}
                    </button>
                  </div>

                  <!-- Subtitle Dialogue Match Badge -->
                  <div
                    v-if="subtitleSnippets.get(video.id)?.length"
                    class="sub-snippet-match-bar"
                    :title="t('library.clickToSeekDialogue')"
                    @click.stop="openVideo(video, subtitleSnippets.get(video.id)![0].start_time_ms / 1000)"
                  >
                    <span class="sub-match-ic">{{ t('library.hitDialogue') }}</span>
                    <span class="sub-match-text">"{{ subtitleSnippets.get(video.id)![0].snippet }}"</span>
                    <span class="sub-match-time font-mono">
                      ▶ {{ formatDuration(subtitleSnippets.get(video.id)![0].start_time_ms / 1000) }}
                    </span>
                  </div>

                  <div class="grid-card-sub">
                    <span v-if="video.file_name.toLowerCase().endsWith('.avi')" class="format-pill avi-pill font-mono" :title="t('library.aviLegacy')">AVI</span>
                    <span v-if="video.codec" class="codec-pill font-mono">{{ video.codec }}</span>
                    <span class="size-text font-mono">{{ formatFileSize(video.file_size) }}</span>
                  </div>

                  <!-- Quick Actions Row -->
                  <div class="card-quick-actions">
                    <button class="card-act-btn" :title="t('library.playTooltip')" @click.stop="openVideo(video)">
                      {{ t('library.playBtn') }}
                    </button>
                    <button
                      class="card-act-btn btn-act-ai"
                      :class="{ 'has-ai-active': aiStatusMap.get(video.id)?.has_subtitles || aiStatusMap.get(video.id)?.has_analysis }"
                      :title="aiStatusMap.get(video.id)?.has_analysis ? t('library.aiSummaryTooltip') : t('library.aiDrawerTooltip')"
                      @click.stop="openAiDrawer(video)"
                    >
                      <span v-if="aiStatusMap.get(video.id)?.has_analysis">{{ t('library.aiSummaryBtn') }}</span>
                      <span v-else-if="aiStatusMap.get(video.id)?.has_subtitles">{{ t('library.aiSubtitlesBtn') }}</span>
                      <span v-else>{{ t('library.aiDrawerBtn') }}</span>
                    </button>
                    <button class="card-act-btn btn-act-tag" :title="t('library.manageTagsTooltip')" @click.stop="openTagModal(video)">
                      {{ t('library.tagBtn') }}
                    </button>
                    <button class="card-act-btn btn-act-audio" :title="t('library.extractAudio')" @click.stop="openAudioExtract(video)">
                      {{ t('library.audioExtractBtn') }}
                    </button>
                    <button class="card-act-btn btn-act-clip" :title="'视频阶段与智能裁剪'" @click.stop="emit('navigateClipper', video)">
                      ✂️ 裁剪
                    </button>
                    <button class="card-act-btn" :title="t('library.locateFolderTooltip')" @click.stop="revealVideo(video.file_path)">
                      {{ t('library.locateBtn') }}
                    </button>
                    <button class="card-act-btn" :title="t('library.copyPathTooltip')" @click.stop="copyToClipboard(video.file_path)">
                      {{ t('library.copyBtn') }}
                    </button>
                    <button class="card-act-btn btn-act-del" :title="t('library.removeTooltip')" @click.stop="promptRemoveVideo(video)">
                      {{ t('library.removeBtn') }}
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <!-- 3. List Item Row -->
            <div
              v-else-if="item.row.type === 'list-row'"
              class="list-item-row"
              :class="{ 'is-selected': selectedVideoIds.has(item.row.video.id) }"
              @dblclick="openVideo(item.row.video)"
            >
              <div
                class="li-select-cell"
                @click.stop="toggleSelectVideo(item.row.video.id)"
              >
                <div class="card-select-checkbox compact" :class="{ checked: selectedVideoIds.has(item.row.video.id) }">
                  <span v-if="selectedVideoIds.has(item.row.video.id)">✓</span>
                </div>
              </div>
              <button
                class="li-star-btn"
                :class="{ starred: item.row.video.is_starred }"
                :title="item.row.video.is_starred ? t('library.starred') : t('library.unstarred')"
                @click.stop="toggleStar(item.row.video)"
              >
                ★
              </button>
              <div class="li-thumb" @click="openVideo(item.row.video)" :title="t('library.clickToPlayPreview')">
                <img
                  v-if="item.row.video.thumbnail_data"
                  :src="item.row.video.thumbnail_data"
                  class="li-thumb-img"
                  :alt="item.row.video.file_name"
                />
                <div v-else class="li-thumb-placeholder">▶</div>
                <div
                  v-if="item.row.video.playback_progress > 0"
                  class="playback-progress-track"
                >
                  <div
                    class="playback-progress-fill"
                    :style="{ width: (item.row.video.playback_progress * 100) + '%' }"
                  ></div>
                </div>
              </div>
              <div class="li-title-col">
                <div class="li-name-row">
                  <span
                    v-if="item.row.video.color_tag"
                    class="badge-color-dot small"
                    :class="item.row.video.color_tag"
                    :title="t('library.colorTagTooltip')"
                    @click.stop="cycleColorTag(item.row.video)"
                  ></span>
                  <span class="li-name" :title="item.row.video.file_name" @click="openVideo(item.row.video)">
                    {{ item.row.video.file_name }}
                  </span>
                  <span
                    class="badge-watch-status compact"
                    :class="item.row.video.watch_status"
                    :title="t('library.watchStatusTooltip')"
                    @click.stop="cycleWatchStatus(item.row.video)"
                  >
                    {{ formatWatchStatus(item.row.video.watch_status) }}
                  </span>
                </div>
                <div class="li-tags-sub">
                  <span
                    v-if="aiStatusMap.get(item.row.video.id)?.has_analysis"
                    class="li-ai-badge has-analysis font-mono"
                    :title="t('library.aiSummaryReady')"
                    @click.stop="openAiDrawer(item.row.video)"
                  >
                    {{ t('library.aiBadgeSummary') }}
                  </span>
                  <span
                    v-else-if="aiStatusMap.get(item.row.video.id)?.has_subtitles"
                    class="li-ai-badge has-subtitles font-mono"
                    :title="t('library.aiSubtitlesReady', { count: aiStatusMap.get(item.row.video.id)?.subtitle_count ?? 0 })"
                    @click.stop="openAiDrawer(item.row.video)"
                  >
                    {{ t('library.aiBadgeLines', { count: aiStatusMap.get(item.row.video.id)?.subtitle_count ?? 0 }) }}
                  </span>
                  <span
                    v-for="tagItem in item.row.video.tags || []"
                    :key="tagItem.id"
                    class="card-tag-pill compact"
                    :title="t('library.filterByTag', { name: tagItem.name })"
                    @click.stop="handleCategorySelect({ type: 'tag', value: String(tagItem.id), label: `#${tagItem.name}` })"
                  >
                    #{{ tagItem.name }}
                  </span>
                  <span
                    v-if="subtitleSnippets.get(item.row.video.id)?.length"
                    class="li-sub-match-pill"
                    :title="t('library.clickToSeekDialogue')"
                    @click.stop="openVideo(item.row.video, subtitleSnippets.get(item.row.video.id)![0].start_time_ms / 1000)"
                  >
                    {{ t('library.hitDialogue') }}"{{ subtitleSnippets.get(item.row.video.id)![0].snippet }}"
                    <span class="li-time-chip font-mono">▶ {{ formatDuration(subtitleSnippets.get(item.row.video.id)![0].start_time_ms / 1000) }}</span>
                  </span>
                  <span class="li-path font-mono" :title="item.row.video.file_path">
                    {{ item.row.video.file_path }}
                  </span>
                </div>
              </div>
              <div class="li-dur font-mono">{{ formatDuration(item.row.video.duration) }}</div>
              <div class="li-reso">{{ getResolutionBadge(item.row.video.width, item.row.video.height) || '-' }}</div>
              <div class="li-codec font-mono">{{ item.row.video.codec || '-' }}</div>
              <div class="li-size font-mono">{{ formatFileSize(item.row.video.file_size) }}</div>
              <div class="li-actions">
                <button class="li-btn" :title="t('library.playTooltip')" @click.stop="openVideo(item.row.video)">▶</button>
                <button
                  class="li-btn btn-act-ai"
                  :class="{ 'has-ai-active': aiStatusMap.get(item.row.video.id)?.has_subtitles || aiStatusMap.get(item.row.video.id)?.has_analysis }"
                  :title="aiStatusMap.get(item.row.video.id)?.has_analysis ? t('library.aiSummaryTooltip') : t('library.aiDrawerTooltip')"
                  @click.stop="openAiDrawer(item.row.video)"
                >
                  ✨
                </button>
                <button class="li-btn btn-act-tag" :title="t('library.manageTagsTooltip')" @click.stop="openTagModal(item.row.video)">🏷️</button>
                <button class="li-btn btn-act-audio" :title="t('library.extractAudio')" @click.stop="openAudioExtract(item.row.video)">🎵</button>
                <button class="li-btn btn-act-clip" :title="'视频阶段与智能裁剪'" @click.stop="emit('navigateClipper', item.row.video)">✂️</button>
                <button class="li-btn" :title="t('library.locateFolderTooltip')" @click.stop="revealVideo(item.row.video.file_path)">📁</button>
                <button class="li-btn" :title="t('library.copyPathTooltip')" @click.stop="copyToClipboard(item.row.video.file_path)">📋</button>
                <button class="li-btn btn-act-del" :title="t('library.removeTooltip')" @click.stop="promptRemoveVideo(item.row.video)">✕</button>
              </div>
            </div>
          </div>
        </div>

        <!-- Infinite Scroll Bottom Status (无需翻页，滚动触底加载) -->
        <div v-if="videos.length > 0" class="infinite-scroll-footer">
          <div v-if="isLoadingMore" class="loading-more-pill">
            <span class="spinner-sm"></span>
            <span>{{ t('library.loadingMore') }}</span>
          </div>
          <div v-else-if="!hasMore" class="all-loaded-pill">
            <span>{{ t('library.allLoaded', { count: totalVideos }) }}</span>
          </div>
        </div>
      </section>
    </div> <!-- .master-detail-workspace -->
      </div> <!-- .video-library-container -->
    </div> <!-- .video-library-main-pane -->

    <!-- Batch Selection Floating Toolbar -->
    <transition name="fade-slide">
      <div v-if="selectedVideoIds.size > 0" class="batch-floating-bar">
        <div class="batch-bar-left">
          <span class="batch-count-badge">{{ t('library.batchSelectedCount', { count: selectedVideoIds.size }) }}</span>
        </div>
        <div class="batch-bar-actions">
          <button class="btn-batch-act btn-batch-star" @click="batchStarVideos">
            {{ t('library.batchStar') }}
          </button>
          <button class="btn-batch-act btn-batch-watched" @click="batchMarkWatched">
            {{ t('library.batchMarkWatched') }}
          </button>
          <button class="btn-batch-act btn-batch-clear" @click="clearSelection">
            {{ t('library.batchDeselect') }}
          </button>
        </div>
      </div>
    </transition>

    <!-- Tag Assignment Modal -->
    <div v-if="showTagModal && videoForTagging" class="tag-modal-overlay" @click.self="showTagModal = false">
      <div class="tag-modal-card">
        <div class="tag-modal-header">
          <div class="tag-modal-header-info">
            <h3 class="tag-modal-title">{{ t('library.manageTagsTitle') }}</h3>
            <p class="tag-modal-subtitle" :title="videoForTagging.file_name">
              {{ videoForTagging.file_name }}
            </p>
          </div>
          <button class="tag-modal-close-btn" @click="showTagModal = false">✕</button>
        </div>

        <div class="tag-modal-body">
          <div class="tag-checklist-label">{{ t('library.manageTagsSubtitle') }}</div>
          <div v-if="categoryStats?.tags && categoryStats.tags.length > 0" class="tag-chips-wrap">
            <button
              v-for="tag in categoryStats.tags"
              :key="tag.id"
              class="tag-select-chip"
              :class="{ active: videoForTagging.tags?.some(t => t.id === tag.id) }"
              @click="toggleTagOnVideo(tag)"
            >
              <span class="chip-dot" :style="{ backgroundColor: tag.color || '#3b82f6' }"></span>
              <span>#{{ tag.name }}</span>
              <span class="chip-check" v-if="videoForTagging.tags?.some(t => t.id === tag.id)">✓</span>
            </button>
          </div>
          <div v-else class="tag-empty-hint">
            {{ t('library.noCustomTags') }}
          </div>
        </div>

        <div class="tag-modal-footer">
          <button class="btn btn-primary" @click="showTagModal = false">{{ t('library.btnDone') }}</button>
        </div>
      </div>
    </div>

    <!-- Video Player Modal Component -->
    <VideoPlayerModal
      :video="activePlayerVideo"
      :visible="isPlayerVisible"
      :initial-seek-time="playerInitialSeek"
      @close="isPlayerVisible = false"
    />

    <!-- AI Video Intelligence Drawer Component -->
    <VideoAiDrawer
      :video="aiDrawerVideo"
      :visible="showAiDrawer"
      @close="showAiDrawer = false"
      @play="handleAiDrawerPlay"
      @tags-updated="handleAiTagsUpdated"
      @analysis-updated="fetchAiStatusForVideos"
    />

    <!-- Audio Extraction Modal Component -->
    <AudioExtractionModal
      :video="videoForAudioExtract"
      :visible="showAudioExtractModal"
      @close="showAudioExtractModal = false"
      @view-in-audio-library="emit('navigateAudios')"
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
            <h3 class="confirm-title">{{ t('library.removeModalTitle') }}</h3>
            <p class="confirm-subtitle">{{ t('library.removeModalSubtitle') }}</p>
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
              {{ t('library.alsoDeleteFile') }}
              <strong class="text-danger">{{ t('library.irreversible') }}</strong>
            </span>
          </label>
        </div>

        <div class="confirm-modal-footer">
          <button
            class="btn btn-secondary"
            :disabled="isDeletingVideo"
            @click="showDeleteConfirmModal = false"
          >
            {{ t('common.cancel') }}
          </button>
          <button
            class="btn btn-danger"
            :disabled="isDeletingVideo"
            @click="confirmRemoveVideo"
          >
            {{ isDeletingVideo ? t('library.processing') : (alsoDeleteDiskFile ? t('library.btnPermanentlyDelete') : t('library.btnRemoveFromLibraryOnly')) }}
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
.video-library-split-layout {
  display: flex;
  width: 100%;
  height: 100%;
  overflow: hidden;
  position: relative;
  background-color: var(--bg-app);
}

.video-library-main-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  position: relative;
}

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
  height: 48px;
  min-height: 48px;
  padding: 0 1.25rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-header);
  gap: 0.75rem;
}

.toolbar-left-group {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex: 1;
  min-width: 0;
}

.search-input-wrap {
  position: relative;
  width: 220px;
  min-width: 170px;
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.search-ico {
  position: absolute;
  left: 9px;
  width: 14px;
  height: 14px;
  color: var(--text-muted);
  pointer-events: none;
}

.search-input-field {
  width: 100%;
  height: 30px;
  padding: 0 1.8rem 0 2rem;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.8rem;
  outline: none;
  transition: all 0.15s ease;
}

.search-input-field:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.12);
}

.search-clear-btn {
  position: absolute;
  right: 6px;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(148, 163, 184, 0.2);
  border: none;
  border-radius: 50%;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.65rem;
  transition: all 0.15s ease;
}

.search-clear-btn:hover {
  background: rgba(148, 163, 184, 0.4);
  color: var(--text-primary);
}

.toolbar-divider {
  width: 1px;
  height: 18px;
  background: var(--border-subtle);
  margin: 0 2px;
  flex-shrink: 0;
}

.toolbar-divider.compact {
  height: 14px;
}

.toolbar-actions-group {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.btn-toolbar-action {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 0.65rem;
  height: 30px;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 0.78rem;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
  user-select: none;
}

.btn-toolbar-action:hover {
  background: var(--btn-hover);
  border-color: rgba(59, 130, 246, 0.3);
}

.btn-toolbar-action.active {
  background: #3b82f6;
  color: #ffffff;
  border-color: #3b82f6;
}

.btn-toolbar-action svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.toolbar-right-group {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex-shrink: 0;
}

.sort-select {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  padding: 0 1.6rem 0 0.65rem;
  height: 30px;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background-color: var(--bg-card);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  color: var(--text-primary);
  font-size: 0.78rem;
  font-weight: 500;
  outline: none;
  cursor: pointer;
  transition: all 0.15s ease;
}

.sort-select:hover {
  border-color: rgba(59, 130, 246, 0.4);
}

.sort-select:focus {
  border-color: #3b82f6;
}

.view-switch-btns {
  display: flex;
  background: var(--bg-input);
  border-radius: 6px;
  padding: 2px;
  border: 1px solid var(--border-subtle);
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
  justify-content: center;
  transition: all 0.15s ease;
}

.view-btn.active {
  background: var(--bg-card);
  color: #3b82f6;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.view-btn svg {
  width: 14px;
  height: 14px;
}

.video-counter-tag {
  font-size: 0.78rem;
  color: var(--text-secondary);
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  padding: 3px 8px;
  border-radius: 6px;
  white-space: nowrap;
}

.video-counter-tag strong {
  color: #3b82f6;
  font-weight: 700;
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
  grid-template-columns: 28px 28px 56px 1fr 80px 80px 75px 75px 160px;
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

.pill-ai-analysis {
  bottom: 6px;
  left: 6px;
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.95), rgba(99, 102, 241, 0.95));
  box-shadow: 0 2px 8px rgba(168, 85, 247, 0.4);
  cursor: pointer;
  border: 1px solid rgba(255, 255, 255, 0.3);
  transition: all 0.15s;
}

.pill-ai-analysis:hover {
  filter: brightness(1.2);
  transform: scale(1.05);
}

.pill-ai-subtitles {
  bottom: 6px;
  left: 6px;
  background: linear-gradient(135deg, rgba(79, 70, 229, 0.95), rgba(124, 58, 237, 0.95));
  box-shadow: 0 2px 8px rgba(79, 70, 229, 0.4);
  cursor: pointer;
  border: 1px solid rgba(255, 255, 255, 0.25);
  transition: all 0.15s;
}

.pill-ai-subtitles:hover {
  filter: brightness(1.2);
  transform: scale(1.05);
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

.format-pill.avi-pill {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
  border: 1px solid rgba(245, 158, 11, 0.35);
  padding: 1px 5px;
  border-radius: 4px;
  font-weight: 700;
  font-size: 0.68rem;
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
  grid-template-columns: 28px 28px 56px 1fr 80px 80px 75px 75px 160px;
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

.card-act-btn.btn-act-audio:hover {
  color: #06b6d4;
  border-color: rgba(6, 182, 212, 0.4);
  background: rgba(6, 182, 212, 0.08);
}

.card-act-btn.btn-act-clip:hover {
  color: #f59e0b;
  border-color: rgba(245, 158, 11, 0.4);
  background: rgba(245, 158, 11, 0.08);
}

.card-act-btn.btn-act-ai {
  color: #a78bfa;
}

.card-act-btn.btn-act-ai:hover {
  color: #c084fc;
  border-color: rgba(168, 85, 247, 0.4);
  background: rgba(168, 85, 247, 0.12);
}

.sub-snippet-match-bar {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.25rem 0.5rem;
  margin: 0.25rem 0 0.35rem 0;
  background: linear-gradient(90deg, rgba(99, 102, 241, 0.15), rgba(168, 85, 247, 0.08));
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.72rem;
  transition: all 0.2s ease;
}

.sub-snippet-match-bar:hover {
  background: linear-gradient(90deg, rgba(99, 102, 241, 0.25), rgba(168, 85, 247, 0.18));
  border-color: rgba(139, 92, 246, 0.6);
}

.sub-match-ic {
  font-weight: 600;
  color: #a78bfa;
  white-space: nowrap;
}

.sub-match-text {
  color: #e2e8f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-style: italic;
}

.li-sub-match-pill {
  display: inline-block;
  padding: 0.1rem 0.4rem;
  background: rgba(139, 92, 246, 0.15);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 4px;
  color: #c4b5fd;
  font-size: 0.7rem;
  cursor: pointer;
  white-space: nowrap;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sub-match-time {
  margin-left: auto;
  font-size: 0.68rem;
  color: #c4b5fd;
  background: rgba(139, 92, 246, 0.25);
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
  white-space: nowrap;
}

.li-time-chip {
  margin-left: 0.35rem;
  font-size: 0.68rem;
  color: #a78bfa;
  background: rgba(139, 92, 246, 0.25);
  padding: 0.05rem 0.3rem;
  border-radius: 4px;
}

.li-ai-badge {
  display: inline-block;
  padding: 0.1rem 0.45rem;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.li-ai-badge.has-analysis {
  background: rgba(168, 85, 247, 0.2);
  color: #c084fc;
  border: 1px solid rgba(168, 85, 247, 0.4);
}

.li-ai-badge.has-analysis:hover {
  background: #a855f7;
  color: #fff;
}

.li-ai-badge.has-subtitles {
  background: rgba(99, 102, 241, 0.2);
  color: #818cf8;
  border: 1px solid rgba(99, 102, 241, 0.4);
}

.li-ai-badge.has-subtitles:hover {
  background: #6366f1;
  color: #fff;
}

.card-act-btn.btn-act-ai.has-ai-active,
.li-btn.btn-act-ai.has-ai-active {
  background: rgba(168, 85, 247, 0.25);
  color: #d8b4fe;
  border-color: rgba(168, 85, 247, 0.5);
  font-weight: 600;
}

.card-act-btn.btn-act-ai.has-ai-active:hover,
.li-btn.btn-act-ai.has-ai-active:hover {
  background: #a855f7;
  color: #fff;
}

.li-btn.btn-act-ai {
  color: #a78bfa;
}

.li-btn.btn-act-ai:hover {
  color: #c084fc;
  border-color: rgba(168, 85, 247, 0.4);
  background: rgba(168, 85, 247, 0.12);
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

.li-btn.btn-act-audio:hover {
  color: #06b6d4;
  border-color: rgba(6, 182, 212, 0.4);
  background: rgba(6, 182, 212, 0.08);
}

.li-btn.btn-act-clip:hover {
  color: #f59e0b;
  border-color: rgba(245, 158, 11, 0.4);
  background: rgba(245, 158, 11, 0.08);
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
/* Category Breadcrumb & Filter Row */
.category-breadcrumb-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 1.25rem;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-subtle);
  height: 38px;
  min-height: 38px;
  gap: 12px;
}

.breadcrumb-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
}

.active-category-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  font-size: 0.76rem;
  white-space: nowrap;
  flex-shrink: 0;
}

.breadcrumb-prefix-label {
  color: var(--text-muted);
  font-size: 0.72rem;
}

.active-category-title {
  font-weight: 600;
  color: var(--text-primary);
}

.filter-dropdowns-row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: nowrap;
  overflow-x: auto;
}

.filter-select {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  padding: 0 1.6rem 0 0.6rem;
  height: 26px;
  border-radius: 6px;
  border: 1px solid var(--border-subtle);
  background-color: var(--bg-input);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 7px center;
  color: var(--text-secondary);
  font-size: 0.76rem;
  font-weight: 500;
  outline: none;
  cursor: pointer;
  transition: all 0.15s ease;
  line-height: 24px;
  white-space: nowrap;
}

.filter-select:hover {
  border-color: rgba(59, 130, 246, 0.4);
  color: var(--text-primary);
}

.filter-select:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.12);
}

.filter-select.is-filtered {
  border-color: #3b82f6;
  background-color: rgba(59, 130, 246, 0.08);
  color: #3b82f6;
  font-weight: 600;
}

.breadcrumb-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.filter-badge-hint {
  font-size: 0.72rem;
  font-weight: 600;
  color: #3b82f6;
  background: rgba(59, 130, 246, 0.1);
  padding: 2px 7px;
  border-radius: 4px;
  border: 1px solid rgba(59, 130, 246, 0.2);
  white-space: nowrap;
}

.btn-reset-filters {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  font-size: 0.74rem;
  font-weight: 600;
  border-radius: 6px;
  border: 1px solid rgba(239, 68, 68, 0.3);
  background: rgba(239, 68, 68, 0.08);
  color: #ef4444;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.btn-reset-filters:hover {
  background: #ef4444;
  color: #ffffff;
}

/* Card Selection Checkbox */
.card-select-checkbox {
  position: absolute;
  top: 8px;
  left: 8px;
  width: 20px;
  height: 20px;
  border-radius: 4px;
  border: 1.5px solid rgba(255, 255, 255, 0.6);
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 5;
  transition: all 0.15s ease;
  color: #ffffff;
  font-size: 11px;
  font-weight: bold;
}

.video-grid-card:hover .card-select-checkbox,
.card-select-checkbox.checked {
  opacity: 1;
}

.video-grid-card:not(:hover) .card-select-checkbox:not(.checked) {
  opacity: 0;
}

.card-select-checkbox.checked {
  background: #3b82f6;
  border-color: #3b82f6;
}

.card-select-checkbox.compact {
  position: static;
  width: 16px;
  height: 16px;
  font-size: 9px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: #ffffff;
  opacity: 1 !important;
}

.card-select-checkbox.compact.checked {
  background: #3b82f6;
  border-color: #3b82f6;
}

.video-grid-card.is-selected,
.list-item-row.is-selected {
  outline: 2px solid #3b82f6;
  outline-offset: -1px;
  background: rgba(59, 130, 246, 0.05);
}

/* Card Top Badges */
.card-top-badges {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 5px;
  z-index: 5;
}

.badge-star-btn {
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  border: none;
  color: rgba(255, 255, 255, 0.7);
  width: 24px;
  height: 24px;
  border-radius: 50%;
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.badge-star-btn:hover {
  color: #fbbf24;
  transform: scale(1.1);
}

.badge-star-btn.starred {
  background: rgba(245, 158, 11, 0.85);
  color: #ffffff;
  box-shadow: 0 0 8px rgba(245, 158, 11, 0.5);
}

.badge-color-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 1.5px solid rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: transform 0.15s ease;
}

.badge-color-dot:hover {
  transform: scale(1.2);
}

.badge-color-dot.small {
  width: 8px;
  height: 8px;
  border: none;
  display: inline-block;
  margin-right: 4px;
  vertical-align: middle;
}

.badge-color-dot.red { background: #ef4444; }
.badge-color-dot.yellow { background: #f59e0b; }
.badge-color-dot.green { background: #10b981; }
.badge-color-dot.blue { background: #3b82f6; }

.badge-watch-status {
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  border: none;
  color: #e2e8f0;
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.badge-watch-status:hover {
  background: rgba(0, 0, 0, 0.85);
}

.badge-watch-status.watched {
  color: #34d399;
}

.badge-watch-status.in_progress {
  color: #fbbf24;
}

.badge-watch-status.compact {
  font-size: 9px;
  padding: 1px 5px;
  margin-left: 6px;
}

/* Playback Progress Bar */
.playback-progress-track {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: rgba(0, 0, 0, 0.5);
  z-index: 3;
}

.playback-progress-fill {
  height: 100%;
  background: #3b82f6;
  border-radius: 0 1px 1px 0;
}

/* Hover Scrub Preview Cursor & Time */
.hover-scrub-cursor {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: #ffffff;
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
  pointer-events: none;
  z-index: 4;
}

.hover-scrub-time {
  position: absolute;
  bottom: 8px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.8);
  color: #ffffff;
  font-size: 10px;
  padding: 2px 5px;
  border-radius: 3px;
  white-space: nowrap;
}

/* Tags in Cards */
.card-tags-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}

.card-tag-pill {
  font-size: 10px;
  color: #3b82f6;
  background: rgba(59, 130, 246, 0.08);
  border: 1px solid rgba(59, 130, 246, 0.2);
  padding: 1px 6px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.12s ease;
  white-space: nowrap;
}

.card-tag-pill:hover {
  background: rgba(59, 130, 246, 0.18);
  border-color: #3b82f6;
}

.card-tag-pill.compact {
  font-size: 9px;
  padding: 0 4px;
}

.add-tag-pill-btn {
  font-size: 10px;
  background: none;
  border: 1px dashed var(--border-subtle);
  color: var(--text-muted);
  padding: 1px 5px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.12s ease;
}

.add-tag-pill-btn:hover {
  border-color: #3b82f6;
  color: #3b82f6;
}

/* List Mode Styles */
.li-select-cell {
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.li-star-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.li-star-btn:hover,
.li-star-btn.starred {
  color: #f59e0b;
}

.li-name-row {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow: hidden;
}

.li-tags-sub {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
  overflow: hidden;
}

.btn-act-tag {
  color: #8b5cf6;
}

/* Floating Batch Operation Toolbar */
.batch-floating-bar {
  position: absolute;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25);
  border-radius: 12px;
  padding: 8px 16px;
  display: flex;
  align-items: center;
  gap: 16px;
  z-index: 100;
  backdrop-filter: blur(8px);
}

.batch-count-badge {
  font-size: 0.82rem;
  color: var(--text-primary);
}

.batch-bar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-batch-act {
  padding: 5px 12px;
  border-radius: 6px;
  font-size: 0.78rem;
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-batch-star {
  background: #f59e0b;
  color: #ffffff;
}

.btn-batch-star:hover {
  background: #d97706;
}

.btn-batch-watched {
  background: #10b981;
  color: #ffffff;
}

.btn-batch-watched:hover {
  background: #059669;
}

.btn-batch-clear {
  background: var(--bg-input);
  color: var(--text-secondary);
  border: 1px solid var(--border-subtle);
}

.btn-batch-clear:hover {
  background: var(--btn-hover);
  color: var(--text-primary);
}

/* Tag Modal */
.tag-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: modalFadeIn 0.15s ease;
}

.tag-modal-card {
  width: 90%;
  max-width: 440px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 14px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.35);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.tag-modal-header {
  padding: 1rem 1.25rem;
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.tag-modal-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.tag-modal-subtitle {
  margin: 4px 0 0;
  font-size: 0.75rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 320px;
}

.tag-modal-close-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 1rem;
  cursor: pointer;
  padding: 4px;
}

.tag-modal-body {
  padding: 1.25rem;
  max-height: 360px;
  overflow-y: auto;
}

.tag-checklist-label {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin-bottom: 0.75rem;
}

.tag-chips-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag-select-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.82rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.tag-select-chip:hover {
  border-color: #3b82f6;
  background: var(--btn-hover);
}

.tag-select-chip.active {
  background: rgba(59, 130, 246, 0.12);
  border-color: #3b82f6;
  color: #3b82f6;
  font-weight: 600;
}

.chip-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.chip-check {
  font-size: 11px;
  font-weight: bold;
}

.tag-empty-hint {
  font-size: 0.8rem;
  color: var(--text-muted);
  line-height: 1.5;
  padding: 0.5rem 0;
}

.tag-modal-footer {
  padding: 0.85rem 1.25rem;
  border-top: 1px solid var(--border-subtle);
  background: var(--bg-header);
  display: flex;
  justify-content: flex-end;
}
</style>

