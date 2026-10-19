<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import VideoPlayerModal from './VideoPlayerModal.vue';
import type {
  ClippedVideo,
  MasterVideoClipsGroup,
  ClippedVideoFilter,
  Video,
  ExtractedAudio,
  ExtractedAudioFilter,
  ReplaceVideoAudioRequest,
  ReplaceVideoAudioResponse,
} from '../types';

const emit = defineEmits<{
  (e: 'navigateLibrary'): void;
  (e: 'navigateClipper', video?: Video): void;
}>();

const groups = ref<MasterVideoClipsGroup[]>([]);
const isLoading = ref<boolean>(false);
const searchQuery = ref<string>('');
const isMergedOnly = ref<boolean>(false);
const sortBy = ref<string>('created_desc');
const collapsedGroupKeys = ref<Set<string>>(new Set());

// Delete Modal State
const showDeleteModal = ref<boolean>(false);
const deleteTargetType = ref<'single' | 'batch'>('single');
const singleClipToDelete = ref<ClippedVideo | null>(null);
const masterGroupToDelete = ref<MasterVideoClipsGroup | null>(null);
const alsoDeleteDiskFile = ref<boolean>(false);
const isDeleting = ref<boolean>(false);

// Video Player Modal State
const isPlayerVisible = ref<boolean>(false);
const activePlayerVideo = ref<Video | null>(null);

// Toast Notification
const toastMessage = ref<string>('');
let toastTimer: number | null = null;

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2600);
}

// Helpers
function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function formatDuration(sec: number | null | undefined): string {
  if (!sec || isNaN(sec) || sec < 0) return '00:00';
  const total = Math.floor(sec);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) {
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

function msToTimeStr(ms: number): string {
  if (isNaN(ms) || ms < 0) ms = 0;
  const sec = ms / 1000.0;
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  const mm = m.toString().padStart(2, '0');
  const ss = s.toString().padStart(2, '0');
  if (h > 0) {
    return `${h.toString().padStart(2, '0')}:${mm}:${ss}`;
  }
  return `${mm}:${ss}`;
}

function formatDate(epochMs: number): string {
  if (!epochMs) return '-';
  const d = new Date(epochMs);
  return `${d.getFullYear()}-${(d.getMonth() + 1).toString().padStart(2, '0')}-${d.getDate().toString().padStart(2, '0')} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`;
}

// Data Fetching
async function fetchClippedGroups() {
  isLoading.value = true;
  try {
    const filter: ClippedVideoFilter = {
      search: searchQuery.value.trim() || undefined,
      isMergedOnly: isMergedOnly.value || undefined,
      sortBy: sortBy.value,
    };
    const res = await invoke<MasterVideoClipsGroup[]>('list_clipped_video_groups', { filter });
    groups.value = res || [];
  } catch (err: unknown) {
    showToast('获取裁剪视频列表失败: ' + String(err));
  } finally {
    isLoading.value = false;
  }
}

// Stats
const totalMasterVideosCount = computed(() => groups.value.length);
const totalClipsCount = computed(() =>
  groups.value.reduce((acc, g) => acc + g.totalClips, 0)
);
const grandTotalDuration = computed(() =>
  groups.value.reduce((acc, g) => acc + g.totalDuration, 0)
);
const grandTotalSize = computed(() =>
  groups.value.reduce((acc, g) => acc + g.totalSize, 0)
);

// Toggle Group Collapse
function toggleGroupCollapse(key: string) {
  if (collapsedGroupKeys.value.has(key)) {
    collapsedGroupKeys.value.delete(key);
  } else {
    collapsedGroupKeys.value.add(key);
  }
}

function expandAllGroups() {
  collapsedGroupKeys.value.clear();
}

function collapseAllGroups() {
  for (const g of groups.value) {
    collapsedGroupKeys.value.add(g.sourceVideoPath);
  }
}

// Action: Copy Path
async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    showToast('已复制文件路径至剪贴板');
  } catch {
    showToast('复制路径失败');
  }
}

// Action: Reveal in Folder
async function revealPath(path: string) {
  try {
    await revealItemInDir(path);
  } catch (err: unknown) {
    showToast('打开所在目录失败: ' + String(err));
  }
}

// Action: Play Clip in Modal
function playClip(clip: ClippedVideo) {
  activePlayerVideo.value = {
    id: -clip.id,
    file_path: clip.filePath,
    file_name: clip.fileName,
    file_hash: '',
    file_size: clip.fileSize,
    duration: clip.duration || 0,
    width: null,
    height: null,
    codec: null,
    created_at: clip.createdAt,
    updated_at: clip.createdAt,
    status: 'ready',
    thumbnail_path: null,
    thumbnail_data: null,
    is_starred: false,
    watch_status: 'watched',
    playback_progress: 0,
    tags: [],
  };
  isPlayerVisible.value = true;
}

// Action: Re-clip in Studio
function reClipMasterVideo(group: MasterVideoClipsGroup) {
  const masterVideoObj: Video = {
    id: group.videoId || Date.now(),
    file_path: group.sourceVideoPath,
    file_name: group.sourceVideoTitle,
    file_hash: '',
    file_size: 0,
    duration: 0,
    width: null,
    height: null,
    codec: null,
    created_at: group.latestCreatedAt,
    updated_at: group.latestCreatedAt,
    status: 'ready',
    thumbnail_path: null,
    thumbnail_data: null,
    is_starred: false,
    watch_status: 'unwatched',
    playback_progress: 0,
    tags: [],
  };
  emit('navigateClipper', masterVideoObj);
}

// Deletion Handlers
function promptDeleteSingleClip(clip: ClippedVideo) {
  deleteTargetType.value = 'single';
  singleClipToDelete.value = clip;
  masterGroupToDelete.value = null;
  alsoDeleteDiskFile.value = false;
  showDeleteModal.value = true;
}

function promptDeleteMasterGroup(group: MasterVideoClipsGroup) {
  deleteTargetType.value = 'batch';
  masterGroupToDelete.value = group;
  singleClipToDelete.value = null;
  alsoDeleteDiskFile.value = false;
  showDeleteModal.value = true;
}

async function confirmDelete() {
  isDeleting.value = true;
  try {
    if (deleteTargetType.value === 'single' && singleClipToDelete.value) {
      await invoke('delete_clipped_video', {
        id: singleClipToDelete.value.id,
        deleteDiskFile: alsoDeleteDiskFile.value,
      });
      showToast(
        `切片已删除${alsoDeleteDiskFile.value ? '（已同步清除磁盘文件）' : ''}`
      );
    } else if (deleteTargetType.value === 'batch' && masterGroupToDelete.value) {
      const count = await invoke<number>('batch_delete_master_clips', {
        sourceVideoPath: masterGroupToDelete.value.sourceVideoPath,
        deleteDiskFile: alsoDeleteDiskFile.value,
      });
      showToast(
        `已清理该主视频的 ${count} 个分段切片${alsoDeleteDiskFile.value ? '（已清除本地文件）' : ''}`
      );
    }
    showDeleteModal.value = false;
    await fetchClippedGroups();
  } catch (err: unknown) {
    showToast('删除失败: ' + String(err));
  } finally {
    isDeleting.value = false;
  }
}

// ==================== Audio Replacement / Synthesis State & Logic ====================
const showReplaceAudioModal = ref<boolean>(false);
const targetVideoForAudio = ref<{
  videoId?: number | null;
  title: string;
  filePath: string;
  duration?: number | null;
  isMaster: boolean;
  masterGroupKey?: string;
} | null>(null);

const audioSourceTab = ref<'library' | 'local'>('library');
const extractedAudiosList = ref<ExtractedAudio[]>([]);
const isAudiosLoading = ref<boolean>(false);
const audioSearchQuery = ref<string>('');
const selectedExtractedAudio = ref<ExtractedAudio | null>(null);
const localAudioPath = ref<string>('');

const audioMode = ref<'replace' | 'mix'>('replace');
const bgmVolume = ref<number>(100);
const originalVolume = ref<number>(30);
const loopAudio = ref<boolean>(true);
const customExportTitle = ref<string>('');

const isReplacingAudio = ref<boolean>(false);
const replaceAudioElapsed = ref<number>(0);
let replaceTimer: number | null = null;
const newlyCreatedClipId = ref<number | null>(null);

// Audio Preview
let previewAudioElement: HTMLAudioElement | null = null;
const previewPlayingPath = ref<string>('');

function stopPreviewAudio() {
  if (previewAudioElement) {
    previewAudioElement.pause();
    previewAudioElement.src = '';
    previewAudioElement = null;
  }
  previewPlayingPath.value = '';
}

function togglePreviewAudio(filePath: string) {
  if (previewPlayingPath.value === filePath) {
    stopPreviewAudio();
    return;
  }
  stopPreviewAudio();
  try {
    const audio = new Audio(convertFileSrc(filePath));
    audio.onended = () => {
      previewPlayingPath.value = '';
    };
    audio.onerror = () => {
      previewPlayingPath.value = '';
      showToast('音频预览播放失败');
    };
    audio.play().catch((err) => {
      console.warn('Audio preview play error:', err);
    });
    previewAudioElement = audio;
    previewPlayingPath.value = filePath;
  } catch (e) {
    console.warn('Init preview audio failed:', e);
  }
}

async function fetchExtractedAudios() {
  isAudiosLoading.value = true;
  try {
    const filter: ExtractedAudioFilter = {
      search: audioSearchQuery.value.trim() || undefined,
      sortBy: 'created_desc',
    };
    const list = await invoke<ExtractedAudio[]>('list_extracted_audios', { filter });
    extractedAudiosList.value = list || [];
  } catch (err: unknown) {
    console.error('Failed to list extracted audios:', err);
  } finally {
    isAudiosLoading.value = false;
  }
}

function openReplaceAudioModalForMaster(group: MasterVideoClipsGroup) {
  stopPreviewAudio();
  targetVideoForAudio.value = {
    videoId: group.videoId,
    title: group.sourceVideoTitle,
    filePath: group.sourceVideoPath,
    duration: group.totalDuration,
    isMaster: true,
    masterGroupKey: group.sourceVideoPath,
  };
  customExportTitle.value = `${group.sourceVideoTitle} - 配乐版`;
  selectedExtractedAudio.value = null;
  localAudioPath.value = '';
  audioMode.value = 'replace';
  bgmVolume.value = 100;
  originalVolume.value = 30;
  loopAudio.value = true;
  showReplaceAudioModal.value = true;
  fetchExtractedAudios();
}

function openReplaceAudioModalForClip(clip: ClippedVideo, masterGroupKey: string) {
  stopPreviewAudio();
  targetVideoForAudio.value = {
    videoId: clip.videoId,
    title: clip.title,
    filePath: clip.filePath,
    duration: clip.duration,
    isMaster: false,
    masterGroupKey,
  };
  customExportTitle.value = `${clip.title} - 配乐版`;
  selectedExtractedAudio.value = null;
  localAudioPath.value = '';
  audioMode.value = 'replace';
  bgmVolume.value = 100;
  originalVolume.value = 30;
  loopAudio.value = true;
  showReplaceAudioModal.value = true;
  fetchExtractedAudios();
}

async function pickLocalAudioFile() {
  try {
    const selected = await invoke<string | null>('pick_audio_file');
    if (selected) {
      localAudioPath.value = selected;
      selectedExtractedAudio.value = null;
      audioSourceTab.value = 'local';
    }
  } catch (err: unknown) {
    showToast('打开文件选择器失败: ' + String(err));
  }
}

function selectAudioFromLibrary(audio: ExtractedAudio) {
  selectedExtractedAudio.value = audio;
  localAudioPath.value = audio.filePath;
}

const effectiveAudioPath = computed(() => {
  if (audioSourceTab.value === 'library') {
    return selectedExtractedAudio.value?.filePath || '';
  }
  return localAudioPath.value.trim();
});

const filteredExtractedAudios = computed(() => {
  if (!audioSearchQuery.value.trim()) {
    return extractedAudiosList.value;
  }
  const q = audioSearchQuery.value.trim().toLowerCase();
  return extractedAudiosList.value.filter(
    (a) =>
      a.fileName.toLowerCase().includes(q) ||
      (a.sourceVideoTitle && a.sourceVideoTitle.toLowerCase().includes(q)) ||
      a.format.toLowerCase().includes(q)
  );
});

async function executeReplaceAudio() {
  if (!targetVideoForAudio.value) return;
  const audioPath = effectiveAudioPath.value;
  if (!audioPath) {
    showToast('请先选择或指定一个背景音乐音频文件');
    return;
  }

  stopPreviewAudio();
  isReplacingAudio.value = true;
  replaceAudioElapsed.value = 0;
  if (replaceTimer) clearInterval(replaceTimer);
  replaceTimer = window.setInterval(() => {
    replaceAudioElapsed.value += 1;
  }, 1000);

  try {
    const req: ReplaceVideoAudioRequest = {
      videoId: targetVideoForAudio.value.videoId,
      videoPath: targetVideoForAudio.value.filePath,
      audioPath,
      customTitle: customExportTitle.value.trim() || undefined,
      mode: audioMode.value,
      bgmVolume: bgmVolume.value / 100.0,
      originalVolume: originalVolume.value / 100.0,
      loopAudio: loopAudio.value,
    };

    const res = await invoke<ReplaceVideoAudioResponse>('replace_video_audio', { request: req });

    if (res.success) {
      showToast(res.message || '背景音乐合成成功！已保存至裁剪管理');
      showReplaceAudioModal.value = false;
      if (res.clippedVideo) {
        newlyCreatedClipId.value = res.clippedVideo.id;
      }
      // Ensure master group is uncollapsed so the user sees the new clip
      if (targetVideoForAudio.value.masterGroupKey) {
        collapsedGroupKeys.value.delete(targetVideoForAudio.value.masterGroupKey);
      }
      await fetchClippedGroups();
    } else {
      showToast('合成失败: ' + res.message);
    }
  } catch (err: unknown) {
    showToast('合成背景音乐失败: ' + String(err));
  } finally {
    isReplacingAudio.value = false;
    if (replaceTimer) {
      clearInterval(replaceTimer);
      replaceTimer = null;
    }
  }
}

onMounted(() => {
  fetchClippedGroups();
});

onUnmounted(() => {
  stopPreviewAudio();
  if (replaceTimer) {
    clearInterval(replaceTimer);
    replaceTimer = null;
  }
});
</script>

<template>
  <div class="clipped-manager-container">
    <!-- Top Control Bar -->
    <header class="manager-header-card">
      <div class="header-left-col">
        <div class="header-nav-btns">
          <button
            class="btn-nav-back"
            title="返回媒体库列表"
            @click="emit('navigateLibrary')"
          >
            <span class="back-arrow">←</span>
            <span>返回媒体库</span>
          </button>
          <button
            class="btn-to-clipper"
            title="进入视频阶段与智能裁剪工坊"
            @click="emit('navigateClipper')"
          >
            <span>✂️ 视频裁剪工坊</span>
          </button>
        </div>

        <div class="header-title-box">
          <h2 class="manager-title">✂️ 视频裁剪与分段管理</h2>
          <p class="manager-subtitle">
            按主视频归集分段，一处集中管理全部分段切片、高潮提取与拼接成片
          </p>
        </div>
      </div>

      <!-- Search & Filters -->
      <div class="header-right-col">
        <div class="search-input-wrap">
          <span class="search-icon">🔍</span>
          <input
            v-model="searchQuery"
            type="text"
            class="search-input"
            placeholder="搜索主视频名称、切片标题或路径..."
            @input="fetchClippedGroups"
          />
          <button
            v-if="searchQuery"
            class="btn-clear-search"
            @click="searchQuery = ''; fetchClippedGroups();"
          >
            ✕
          </button>
        </div>

        <div class="filter-controls-row">
          <label class="checkbox-filter-label" title="仅查看由多个分段拼接而成的完整合集成片">
            <input
              v-model="isMergedOnly"
              type="checkbox"
              @change="fetchClippedGroups"
            />
            <span>🔗 仅看合集拼接成片</span>
          </label>

          <select
            v-model="sortBy"
            class="select-sort"
            @change="fetchClippedGroups"
          >
            <option value="created_desc">⏱️ 按创建时间 (最新优先)</option>
            <option value="created_asc">⏱️ 按创建时间 (最早优先)</option>
            <option value="duration_desc">⏳ 按时长从长到短</option>
            <option value="title_asc">🔤 按名称字母排序</option>
          </select>

          <button
            class="btn-refresh"
            :disabled="isLoading"
            title="刷新数据"
            @click="fetchClippedGroups"
          >
            <span>🔄 刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Stats Summary Ribbon -->
    <section class="stats-ribbon-card">
      <div class="stat-item">
        <span class="stat-label">🎬 主视频数</span>
        <span class="stat-value highlight-blue">{{ totalMasterVideosCount }}</span>
      </div>
      <div class="stat-divider"></div>
      <div class="stat-item">
        <span class="stat-label">✂️ 分段切片总数</span>
        <span class="stat-value highlight-orange">{{ totalClipsCount }}</span>
      </div>
      <div class="stat-divider"></div>
      <div class="stat-item">
        <span class="stat-label">⏳ 切片总时长</span>
        <span class="stat-value">{{ formatDuration(grandTotalDuration) }}</span>
      </div>
      <div class="stat-divider"></div>
      <div class="stat-item">
        <span class="stat-label">💾 占用总空间</span>
        <span class="stat-value">{{ formatBytes(grandTotalSize) }}</span>
      </div>

      <div class="ribbon-actions">
        <button
          v-if="collapsedGroupKeys.size > 0"
          class="btn-text-action"
          @click="expandAllGroups"
        >
          ➕ 展开全部
        </button>
        <button
          v-else-if="groups.length > 0"
          class="btn-text-action"
          @click="collapseAllGroups"
        >
          ➖ 折叠全部
        </button>
      </div>
    </section>

    <!-- Main Content: Master Videos Groups List -->
    <main class="groups-scroll-area">
      <!-- Loading State -->
      <div v-if="isLoading" class="manager-loading-state">
        <div class="spinner-wave"></div>
        <p>正在载入裁剪视频与分段数据...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="groups.length === 0" class="manager-empty-state">
        <div class="empty-icon-art">✂️</div>
        <h3>暂无已裁剪的视频记录</h3>
        <p>
          在媒体库中选择任意视频，点击工具栏中的「✂️ 裁剪」划分阶段或提取高潮片段，导出的切片将自动汇总于此。
        </p>
        <div class="empty-cta-btns">
          <button class="btn-cta primary" @click="emit('navigateLibrary')">
            🎬 前往媒体库选择视频
          </button>
          <button class="btn-cta ghost" @click="emit('navigateClipper')">
            ✂️ 打开裁剪工坊
          </button>
        </div>
      </div>

      <!-- Groups List (1 Master Video -> Multiple Clipped Segments) -->
      <div v-else class="groups-list">
        <div
          v-for="group in groups"
          :key="group.sourceVideoPath"
          class="master-group-card"
        >
          <!-- 1. Master Video Header -->
          <div
            class="master-header-row"
            @click="toggleGroupCollapse(group.sourceVideoPath)"
          >
            <div class="master-left-info">
              <span
                class="collapse-toggle-arrow"
                :class="{ collapsed: collapsedGroupKeys.has(group.sourceVideoPath) }"
              >
                ▼
              </span>

              <div class="master-icon-badge">🎬</div>

              <div class="master-texts">
                <div class="master-title-line">
                  <h4 class="master-video-title" :title="group.sourceVideoTitle">
                    {{ group.sourceVideoTitle }}
                  </h4>
                  <span class="badge-clips-count">
                    ✂️ {{ group.totalClips }} 个分段
                  </span>
                </div>
                <div class="master-meta-line">
                  <span class="meta-path font-mono" :title="group.sourceVideoPath">
                    {{ group.sourceVideoPath }}
                  </span>
                  <span class="meta-dot">·</span>
                  <span class="meta-summary">
                    总计 {{ formatDuration(group.totalDuration) }} · {{ formatBytes(group.totalSize) }}
                  </span>
                  <span class="meta-dot">·</span>
                  <span class="meta-date">
                    最新剪辑: {{ formatDate(group.latestCreatedAt) }}
                  </span>
                </div>
              </div>
            </div>

            <!-- Master Action Buttons -->
            <div class="master-right-actions" @click.stop>
              <button
                class="btn-m-act music-btn"
                title="为此主视频替换或合成外部背景音乐"
                @click="openReplaceAudioModalForMaster(group)"
              >
                <span>🎵 换背景音乐</span>
              </button>
              <button
                class="btn-m-act reclip-btn"
                title="重新载入此主视频进入裁剪工坊继续编辑"
                @click="reClipMasterVideo(group)"
              >
                <span>✂️ 继续剪辑</span>
              </button>
              <button
                class="btn-m-act locate-btn"
                title="定位原主视频所在文件夹"
                @click="revealPath(group.sourceVideoPath)"
              >
                <span>📁 原片目录</span>
              </button>
              <button
                class="btn-m-act copy-btn"
                title="复制原视频绝对路径"
                @click="copyPath(group.sourceVideoPath)"
              >
                <span>📋 路径</span>
              </button>
              <button
                class="btn-m-act delete-group-btn"
                title="批量清理该主视频下的全部裁剪切片"
                @click="promptDeleteMasterGroup(group)"
              >
                <span>🗑️ 批量清理</span>
              </button>
            </div>
          </div>

          <!-- 2. Children: Segment Clips List (Collapsible) -->
          <div
            v-if="!collapsedGroupKeys.has(group.sourceVideoPath)"
            class="children-clips-wrapper"
          >
            <div class="clips-grid">
              <div
                v-for="(clip, idx) in group.clips"
                :key="clip.id"
                class="clip-item-card"
                :class="{ 'is-merged-card': clip.isMerged, 'is-new-created': newlyCreatedClipId === clip.id }"
              >
                <!-- Clip Top Header: Index & Title -->
                <div class="clip-card-header">
                  <div class="clip-title-group">
                    <span class="clip-num-badge">#{{ idx + 1 }}</span>
                    <h5 class="clip-card-title" :title="clip.title">
                      {{ clip.title }}
                    </h5>
                  </div>

                  <div class="clip-badge-tags">
                    <span v-if="clip.isMerged" class="tag-merged">
                      🔗 拼接合集
                    </span>
                    <span
                      v-else-if="clip.title.includes('配乐') || clip.title.includes('BGM') || clip.title.includes('换背景音乐') || clip.title.includes('伴奏')"
                      class="tag-music"
                    >
                      🎵 音乐配乐
                    </span>
                    <span
                      v-else-if="clip.title.includes('高潮') || clip.title.includes('亮点') || clip.title.includes('精彩')"
                      class="tag-highlight"
                    >
                      🔥 高能亮点
                    </span>
                  </div>
                </div>

                <!-- Clip Meta Details -->
                <div class="clip-meta-grid">
                  <div class="meta-cell">
                    <span class="cell-label">原片时间区间</span>
                    <span class="cell-value time font-mono">
                      {{ msToTimeStr(clip.startMs) }} ~ {{ msToTimeStr(clip.endMs) }}
                    </span>
                  </div>
                  <div class="meta-cell">
                    <span class="cell-label">片段时长</span>
                    <span class="cell-value font-mono">
                      {{ formatDuration(clip.duration) }}
                    </span>
                  </div>
                  <div class="meta-cell">
                    <span class="cell-label">文件体积</span>
                    <span class="cell-value font-mono">
                      {{ formatBytes(clip.fileSize) }}
                    </span>
                  </div>
                  <div class="meta-cell">
                    <span class="cell-label">生成时间</span>
                    <span class="cell-value text-muted">
                      {{ formatDate(clip.createdAt) }}
                    </span>
                  </div>
                </div>

                <div class="clip-file-path-row font-mono" :title="clip.filePath">
                  {{ clip.fileName }}
                </div>

                <!-- Clip Action Buttons -->
                <div class="clip-actions-row">
                  <button
                    class="btn-clip-act play-btn"
                    title="在内置播放器中试听预览"
                    @click="playClip(clip)"
                  >
                    <span>▶ 播放</span>
                  </button>
                  <button
                    class="btn-clip-act music-btn"
                    title="为此切片替换或合成外部背景音乐"
                    @click="openReplaceAudioModalForClip(clip, group.sourceVideoPath)"
                  >
                    <span>🎵 换音乐</span>
                  </button>
                  <button
                    class="btn-clip-act dir-btn"
                    title="在系统文件管理器中定位该切片文件"
                    @click="revealPath(clip.filePath)"
                  >
                    <span>📁 定位</span>
                  </button>
                  <button
                    class="btn-clip-act copy-btn"
                    title="复制切片绝对路径"
                    @click="copyPath(clip.filePath)"
                  >
                    <span>📋 复制</span>
                  </button>
                  <button
                    class="btn-clip-act del-btn"
                    title="删除此分段切片记录"
                    @click="promptDeleteSingleClip(clip)"
                  >
                    <span>🗑️ 删除</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- Delete Confirmation Modal -->
    <div
      v-if="showDeleteModal"
      class="modal-backdrop-layer"
      @click.self="showDeleteModal = false"
    >
      <div class="delete-dialog-card">
        <div class="dialog-header">
          <span class="warn-icon">⚠️</span>
          <h4>
            {{ deleteTargetType === 'single' ? '删除切片确认' : '批量清理主视频切片确认' }}
          </h4>
        </div>

        <div class="dialog-body">
          <p v-if="deleteTargetType === 'single' && singleClipToDelete">
            确定要从管理列表中移除片段
            <strong>"{{ singleClipToDelete.title }}"</strong> 吗？
          </p>
          <p v-else-if="deleteTargetType === 'batch' && masterGroupToDelete">
            确定要清理主视频
            <strong>"{{ masterGroupToDelete.sourceVideoTitle }}"</strong>
            名下的全部 <strong>{{ masterGroupToDelete.totalClips }}</strong> 个分段切片吗？
          </p>

          <label class="disk-delete-checkbox-label">
            <input v-model="alsoDeleteDiskFile" type="checkbox" />
            <span class="checkbox-text">
              <strong>同时永久从本地磁盘中删除实际视频文件</strong>
              <small class="warn-subtext">（注意：此操作不可逆，文件将彻底移除）</small>
            </span>
          </label>
        </div>

        <div class="dialog-footer">
          <button
            class="btn-dialog-cancel"
            :disabled="isDeleting"
            @click="showDeleteModal = false"
          >
            取消
          </button>
          <button
            class="btn-dialog-danger"
            :disabled="isDeleting"
            @click="confirmDelete"
          >
            {{ isDeleting ? '正在删除...' : '确认删除' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Replace / Mix Background Music Modal -->
    <div
      v-if="showReplaceAudioModal"
      class="modal-backdrop-layer"
      @click.self="showReplaceAudioModal = false"
    >
      <div class="replace-audio-dialog-card">
        <div class="dialog-header music-header">
          <div class="dialog-title-left">
            <span class="music-icon">🎵</span>
            <h4>更换背景音乐 / 音频合成</h4>
          </div>
          <button class="btn-close-dialog" @click="showReplaceAudioModal = false">✕</button>
        </div>

        <div class="dialog-body audio-modal-body">
          <!-- Target Video Summary Card -->
          <div v-if="targetVideoForAudio" class="target-video-box">
            <div class="target-video-badge">
              {{ targetVideoForAudio.isMaster ? '🎬 主视频原片' : '✂️ 裁剪分段切片' }}
            </div>
            <div class="target-video-main">
              <span class="target-video-title font-mono" :title="targetVideoForAudio.filePath">
                {{ targetVideoForAudio.title }}
              </span>
              <span class="target-video-dur font-mono">
                时长: {{ formatDuration(targetVideoForAudio.duration) }}
              </span>
            </div>
          </div>

          <!-- Section 1: Audio Selection Mode Tabs -->
          <div class="modal-section-title">
            <span>1. 选择外部背景音乐 / 音频来源</span>
          </div>

          <div class="audio-source-tabs">
            <button
              class="btn-source-tab"
              :class="{ active: audioSourceTab === 'library' }"
              @click="audioSourceTab = 'library'"
            >
              <span>📻 从已提取音频库选取 ({{ extractedAudiosList.length }})</span>
            </button>
            <button
              class="btn-source-tab"
              :class="{ active: audioSourceTab === 'local' }"
              @click="audioSourceTab = 'local'"
            >
              <span>📂 从本地文件选取 / 浏览</span>
            </button>
          </div>

          <!-- Tab Content: Library Selection -->
          <div v-if="audioSourceTab === 'library'" class="tab-pane-library">
            <div class="audio-search-box">
              <input
                v-model="audioSearchQuery"
                type="text"
                class="audio-search-input"
                placeholder="搜索已提取音频名称、原片名或格式..."
              />
              <button
                class="btn-refresh-audios"
                title="刷新音频列表"
                @click="fetchExtractedAudios"
              >
                🔄 刷新
              </button>
            </div>

            <div class="library-audios-list">
              <div v-if="isAudiosLoading" class="audios-loading-state">
                <div class="spinner-small"></div>
                <span>正在加载音频库...</span>
              </div>
              <div
                v-else-if="filteredExtractedAudios.length === 0"
                class="audios-empty-hint"
              >
                <span>未找到匹配的已提取音频。您可以切换到“从本地文件选取”直接指定本地音频文件。</span>
              </div>
              <div
                v-else
                v-for="audio in filteredExtractedAudios"
                :key="audio.id"
                class="audio-select-item"
                :class="{ selected: selectedExtractedAudio?.id === audio.id }"
                @click="selectAudioFromLibrary(audio)"
              >
                <div class="audio-item-left">
                  <span class="audio-fmt-tag">{{ audio.format.toUpperCase() }}</span>
                  <div class="audio-title-col">
                    <span class="audio-item-name" :title="audio.fileName">{{ audio.fileName }}</span>
                    <span v-if="audio.sourceVideoTitle" class="audio-item-source">
                      原片: {{ audio.sourceVideoTitle }}
                    </span>
                  </div>
                </div>
                <div class="audio-item-right" @click.stop>
                  <button
                    class="btn-audio-preview"
                    :class="{ playing: previewPlayingPath === audio.filePath }"
                    :title="previewPlayingPath === audio.filePath ? '停止试听' : '点击试听预览'"
                    @click="togglePreviewAudio(audio.filePath)"
                  >
                    <span>{{ previewPlayingPath === audio.filePath ? '⏹ 停止' : '▶ 试听' }}</span>
                  </button>
                  <span class="audio-dur font-mono">{{ formatDuration(audio.duration) }}</span>
                  <span class="audio-size">{{ formatBytes(audio.fileSize) }}</span>
                  <span v-if="selectedExtractedAudio?.id === audio.id" class="check-mark">✓ 已选</span>
                  <button
                    v-else
                    class="btn-select-audio"
                    @click="selectAudioFromLibrary(audio)"
                  >
                    选用
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Tab Content: Local File Selection -->
          <div v-else class="tab-pane-local">
            <div class="local-picker-row">
              <input
                v-model="localAudioPath"
                type="text"
                class="local-path-input"
                placeholder="点击右侧按钮选择文件，或直接粘贴音频绝对路径..."
              />
              <button class="btn-browse-file" @click="pickLocalAudioFile">
                <span>📂 浏览本地音频...</span>
              </button>
            </div>
            <div class="local-picker-hint">
              支持主流格式：MP3, M4A, WAV, AAC, FLAC, OGG, WMA, AIFF 等音频文件。
            </div>
          </div>

          <!-- Selected Audio Preview Status -->
          <div v-if="effectiveAudioPath" class="selected-audio-bar">
            <div class="bar-left">
              <span class="bar-label">🎵 当前选定音乐：</span>
              <span class="bar-path font-mono" :title="effectiveAudioPath">
                {{ effectiveAudioPath }}
              </span>
            </div>
            <button
              class="btn-bar-preview"
              :class="{ playing: previewPlayingPath === effectiveAudioPath }"
              @click="togglePreviewAudio(effectiveAudioPath)"
            >
              <span>{{ previewPlayingPath === effectiveAudioPath ? '⏹ 停止试听' : '▶ 试听播放' }}</span>
            </button>
          </div>

          <!-- Section 2: Synthesis Mode & Volume Options -->
          <div class="modal-section-title">
            <span>2. 合成模式与音量配比</span>
          </div>

          <div class="synthesis-mode-row">
            <label
              class="mode-card"
              :class="{ active: audioMode === 'replace' }"
            >
              <input
                v-model="audioMode"
                type="radio"
                value="replace"
                class="radio-hide"
              />
              <div class="mode-card-header">
                <span class="mode-icon">🔇</span>
                <span class="mode-name">完全替换原声 (推荐)</span>
              </div>
              <p class="mode-desc">
                完全去除原视频自带的人声/杂音，仅保留所选背景音乐，视频画面无损极速秒出。
              </p>
            </label>

            <label
              class="mode-card"
              :class="{ active: audioMode === 'mix' }"
            >
              <input
                v-model="audioMode"
                type="radio"
                value="mix"
                class="radio-hide"
              />
              <div class="mode-card-header">
                <span class="mode-icon">🎛️</span>
                <span class="mode-name">混音伴奏模式</span>
              </div>
              <p class="mode-desc">
                保留原视频的人物对白或原声，将所选音乐作为背景伴奏混合叠加。
              </p>
            </label>
          </div>

          <!-- Volume Controls -->
          <div class="volume-controls-grid">
            <div class="volume-slider-box">
              <div class="slider-header">
                <span class="slider-label">🎵 背景音乐音量</span>
                <span class="slider-val font-mono">{{ bgmVolume }}%</span>
              </div>
              <input
                v-model.number="bgmVolume"
                type="range"
                min="0"
                max="200"
                step="5"
                class="volume-slider"
              />
            </div>

            <div v-if="audioMode === 'mix'" class="volume-slider-box">
              <div class="slider-header">
                <span class="slider-label">🗣️ 原视频人声音量</span>
                <span class="slider-val font-mono">{{ originalVolume }}%</span>
              </div>
              <input
                v-model.number="originalVolume"
                type="range"
                min="0"
                max="150"
                step="5"
                class="volume-slider"
              />
            </div>
          </div>

          <!-- Section 3: Extra Settings & Custom Title -->
          <div class="modal-section-title">
            <span>3. 导出设置</span>
          </div>

          <div class="export-settings-row">
            <label class="setting-checkbox-label">
              <input v-model="loopAudio" type="checkbox" />
              <span>音乐短于视频时自动循环播放（确保全片均有配乐）</span>
            </label>
          </div>

          <div class="export-title-row">
            <span class="title-label">新生成视频标题:</span>
            <input
              v-model="customExportTitle"
              type="text"
              class="export-title-input"
              placeholder="请输入导出切片标题..."
            />
          </div>
        </div>

        <div class="dialog-footer">
          <button
            class="btn-dialog-cancel"
            :disabled="isReplacingAudio"
            @click="showReplaceAudioModal = false"
          >
            取消
          </button>
          <button
            class="btn-dialog-primary"
            :disabled="isReplacingAudio || !effectiveAudioPath"
            @click="executeReplaceAudio"
          >
            <span v-if="isReplacingAudio">⏳ 正在合成中...</span>
            <span v-else>🚀 开始合成背景音乐</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Replacing Audio Progress Overlay -->
    <div v-if="isReplacingAudio" class="modal-backdrop-layer progress-mode">
      <div class="progress-card">
        <div class="ai-spinner"></div>
        <h4>正在合成背景音乐...</h4>
        <p>FFmpeg 正在全力合并音轨，采用无损画面流拷贝秒级合成，请稍候</p>
        <span class="elapsed-badge">已耗时: {{ replaceAudioElapsed }} 秒</span>
      </div>
    </div>

    <!-- Video Player Preview Modal -->
    <VideoPlayerModal
      :video="activePlayerVideo"
      :visible="isPlayerVisible"
      @close="isPlayerVisible = false"
    />

    <!-- Toast Notification -->
    <transition name="fade">
      <div v-if="toastMessage" class="toast-chip-floating">
        {{ toastMessage }}
      </div>
    </transition>
  </div>
</template>

<style scoped>
.clipped-manager-container {
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
  padding: 1.25rem;
  height: 100%;
  box-sizing: border-box;
  background: var(--bg-base, #0d1117);
  color: var(--text-color, #e6edf3);
  overflow-y: auto;
}

/* Header Card */
.manager-header-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.5rem;
  padding: 1rem 1.25rem;
  background: var(--bg-card, rgba(22, 27, 34, 0.8));
  border: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
  border-radius: 12px;
  backdrop-filter: blur(16px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
  flex-wrap: wrap;
}

.header-left-col {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  min-width: 0;
}

.header-nav-btns {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.btn-nav-back,
.btn-to-clipper {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.4rem 0.8rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-color, #e6edf3);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-nav-back:hover,
.btn-to-clipper:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #60a5fa;
  transform: translateY(-1px);
}

.manager-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: #fff;
}

.manager-subtitle {
  margin: 0.2rem 0 0 0;
  font-size: 0.78rem;
  color: #94a3b8;
}

.header-right-col {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  align-items: flex-end;
}

.search-input-wrap {
  position: relative;
  width: 320px;
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 0.85rem;
  color: #64748b;
}

.search-input {
  width: 100%;
  padding: 0.45rem 1.8rem 0.45rem 2rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.82rem;
  box-sizing: border-box;
}

.search-input:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.25);
}

.btn-clear-search {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  font-size: 0.75rem;
}

.filter-controls-row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.checkbox-filter-label {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.78rem;
  color: #cbd5e1;
  cursor: pointer;
}

.select-sort {
  padding: 0.4rem 0.65rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.78rem;
  outline: none;
  cursor: pointer;
}

.btn-refresh {
  padding: 0.4rem 0.75rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
  font-size: 0.78rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-refresh:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.16);
}

/* Stats Ribbon */
.stats-ribbon-card {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  padding: 0.75rem 1.25rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 8px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.stat-label {
  font-size: 0.75rem;
  color: #94a3b8;
}

.stat-value {
  font-size: 0.95rem;
  font-weight: 700;
  color: #f1f5f9;
}

.stat-value.highlight-blue {
  color: #60a5fa;
}

.stat-value.highlight-orange {
  color: #fb923c;
}

.stat-divider {
  width: 1px;
  height: 18px;
  background: rgba(255, 255, 255, 0.1);
}

.ribbon-actions {
  margin-left: auto;
}

.btn-text-action {
  background: none;
  border: none;
  color: #38bdf8;
  font-size: 0.78rem;
  cursor: pointer;
  padding: 2px 6px;
}

.btn-text-action:hover {
  text-decoration: underline;
}

/* Groups Area */
.groups-scroll-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.manager-loading-state,
.manager-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3.5rem 1.5rem;
  text-align: center;
  color: #94a3b8;
}

.spinner-wave {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(59, 130, 246, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-icon-art {
  font-size: 3rem;
  margin-bottom: 0.75rem;
}

.empty-cta-btns {
  display: flex;
  gap: 0.75rem;
  margin-top: 1.25rem;
}

.btn-cta {
  padding: 0.55rem 1.2rem;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-cta.primary {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
  color: #fff;
  border: none;
}

.btn-cta.primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 14px rgba(59, 130, 246, 0.4);
}

.btn-cta.ghost {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
}

.btn-cta.ghost:hover {
  background: rgba(255, 255, 255, 0.15);
}

/* Master Group Card */
.master-group-card {
  background: rgba(22, 27, 34, 0.65);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.2s ease;
}

.master-group-card:hover {
  border-color: rgba(255, 255, 255, 0.16);
}

.master-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 1.25rem;
  background: rgba(255, 255, 255, 0.02);
  cursor: pointer;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  user-select: none;
}

.master-header-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.master-left-info {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  min-width: 0;
  flex: 1;
}

.collapse-toggle-arrow {
  font-size: 0.7rem;
  color: #94a3b8;
  transition: transform 0.2s ease;
}

.collapse-toggle-arrow.collapsed {
  transform: rotate(-90deg);
}

.master-icon-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: rgba(59, 130, 246, 0.12);
  border: 1px solid rgba(59, 130, 246, 0.25);
  font-size: 1.15rem;
  flex-shrink: 0;
}

.master-texts {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.master-title-line {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.master-video-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 700;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 420px;
}

.badge-clips-count {
  font-size: 0.72rem;
  font-weight: 600;
  background: rgba(251, 146, 60, 0.15);
  border: 1px solid rgba(251, 146, 60, 0.35);
  color: #fb923c;
  padding: 1px 6px;
  border-radius: 4px;
}

.master-meta-line {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.74rem;
  color: #94a3b8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meta-path {
  color: #64748b;
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.master-right-actions {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-shrink: 0;
}

.btn-m-act {
  padding: 0.32rem 0.65rem;
  border-radius: 5px;
  font-size: 0.74rem;
  font-weight: 600;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  transition: all 0.15s ease;
}

.btn-m-act:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

.btn-m-act.reclip-btn {
  background: rgba(245, 158, 11, 0.12);
  border-color: rgba(245, 158, 11, 0.3);
  color: #fbbf24;
}

.btn-m-act.reclip-btn:hover {
  background: rgba(245, 158, 11, 0.25);
  color: #fff;
}

.btn-m-act.delete-group-btn:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.4);
  color: #f87171;
}

/* Children Clips Grid */
.children-clips-wrapper {
  padding: 0.85rem 1.25rem 1.1rem;
}

.clips-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 0.85rem;
}

.clip-item-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 0.75rem;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 0.85rem;
  transition: all 0.18s ease;
}

.clip-item-card:hover {
  background: rgba(0, 0, 0, 0.5);
  border-color: rgba(59, 130, 246, 0.35);
  transform: translateY(-2px);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
}

.clip-item-card.is-merged-card {
  border-color: rgba(16, 185, 129, 0.3);
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.05), rgba(0, 0, 0, 0.35));
}

.clip-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.5rem;
}

.clip-title-group {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  flex: 1;
}

.clip-num-badge {
  font-size: 0.72rem;
  font-weight: 700;
  color: #818cf8;
  background: rgba(129, 140, 248, 0.15);
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}

.clip-card-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.clip-badge-tags {
  display: flex;
  gap: 0.3rem;
  flex-shrink: 0;
}

.tag-merged {
  font-size: 0.65rem;
  font-weight: 700;
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.35);
  padding: 1px 5px;
  border-radius: 4px;
}

.tag-highlight {
  font-size: 0.65rem;
  font-weight: 700;
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
  border: 1px solid rgba(239, 68, 68, 0.35);
  padding: 1px 5px;
  border-radius: 4px;
}

.clip-meta-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
  background: rgba(255, 255, 255, 0.02);
  padding: 0.5rem;
  border-radius: 6px;
}

.meta-cell {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.cell-label {
  font-size: 0.65rem;
  color: #64748b;
}

.cell-value {
  font-size: 0.74rem;
  color: #e2e8f0;
  font-weight: 500;
}

.cell-value.time {
  color: #38bdf8;
}

.cell-value.text-muted {
  font-size: 0.68rem;
  color: #94a3b8;
}

.clip-file-path-row {
  font-size: 0.68rem;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.clip-actions-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 0.55rem;
}

.btn-clip-act {
  flex: 1;
  padding: 0.32rem 0;
  font-size: 0.72rem;
  font-weight: 600;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cbd5e1;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-clip-act:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.btn-clip-act.play-btn {
  background: rgba(59, 130, 246, 0.18);
  border-color: rgba(59, 130, 246, 0.35);
  color: #93c5fd;
}

.btn-clip-act.play-btn:hover {
  background: rgba(59, 130, 246, 0.35);
  color: #fff;
}

.btn-clip-act.del-btn:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.35);
  color: #f87171;
}

/* Delete Dialog */
.modal-backdrop-layer {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.delete-dialog-card {
  width: 440px;
  background: #161b22;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6);
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem 1.25rem;
  background: rgba(239, 68, 68, 0.1);
  border-bottom: 1px solid rgba(239, 68, 68, 0.2);
}

.warn-icon {
  font-size: 1.25rem;
}

.dialog-header h4 {
  margin: 0;
  font-size: 1rem;
  color: #fca5a5;
}

.dialog-body {
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  font-size: 0.85rem;
  color: #cbd5e1;
}

.dialog-body p {
  margin: 0;
  line-height: 1.5;
}

.disk-delete-checkbox-label {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.75rem;
  background: rgba(239, 68, 68, 0.08);
  border: 1px solid rgba(239, 68, 68, 0.2);
  border-radius: 6px;
  cursor: pointer;
}

.checkbox-text {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  font-size: 0.78rem;
  color: #fca5a5;
}

.warn-subtext {
  font-size: 0.7rem;
  color: #f87171;
}

.dialog-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0.85rem 1.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.2);
}

.btn-dialog-cancel {
  padding: 0.45rem 1rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.82rem;
  cursor: pointer;
}

.btn-dialog-danger {
  padding: 0.45rem 1.15rem;
  border-radius: 6px;
  background: #dc2626;
  border: none;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-dialog-danger:hover:not(:disabled) {
  background: #ef4444;
}

/* Toast */
.toast-chip-floating {
  position: fixed;
  bottom: 24px;
  right: 24px;
  padding: 0.6rem 1.1rem;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(59, 130, 246, 0.4);
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.45);
  border-radius: 8px;
  color: #fff;
  font-size: 0.85rem;
  z-index: 2000;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

/* Music Replace / Synthesis Styles */
.btn-m-act.music-btn {
  background: rgba(168, 85, 247, 0.15);
  border-color: rgba(168, 85, 247, 0.35);
  color: #c084fc;
}

.btn-m-act.music-btn:hover {
  background: rgba(168, 85, 247, 0.28);
  color: #fff;
}

.btn-clip-act.music-btn {
  background: rgba(168, 85, 247, 0.12);
  border-color: rgba(168, 85, 247, 0.28);
  color: #c084fc;
}

.btn-clip-act.music-btn:hover {
  background: rgba(168, 85, 247, 0.28);
  color: #fff;
}

.tag-music {
  font-size: 0.65rem;
  font-weight: 700;
  background: rgba(168, 85, 247, 0.18);
  color: #c084fc;
  border: 1px solid rgba(168, 85, 247, 0.35);
  padding: 1px 5px;
  border-radius: 3px;
}

.clip-item-card.is-new-created {
  animation: glowNew 2s ease-in-out infinite alternate;
  border-color: #a855f7;
}

@keyframes glowNew {
  from {
    box-shadow: 0 0 4px rgba(168, 85, 247, 0.3);
  }
  to {
    box-shadow: 0 0 16px rgba(168, 85, 247, 0.7);
  }
}

.replace-audio-dialog-card {
  width: 680px;
  max-width: 95vw;
  max-height: 88vh;
  display: flex;
  flex-direction: column;
  background: #12161f;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.75);
}

.dialog-header.music-header {
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.15), rgba(59, 130, 246, 0.1));
  border-bottom: 1px solid rgba(168, 85, 247, 0.25);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
}

.dialog-title-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.dialog-title-left h4 {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  color: #f3e8ff;
}

.music-icon {
  font-size: 1.3rem;
}

.btn-close-dialog {
  background: none;
  border: none;
  color: #94a3b8;
  font-size: 1.1rem;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.15s ease;
}

.btn-close-dialog:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.audio-modal-body {
  overflow-y: auto;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
}

.target-video-box {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.target-video-badge {
  font-size: 0.72rem;
  font-weight: 600;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.12);
  padding: 2px 6px;
  border-radius: 4px;
  width: fit-content;
}

.target-video-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.target-video-title {
  font-size: 0.85rem;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.target-video-dur {
  font-size: 0.78rem;
  color: #94a3b8;
  flex-shrink: 0;
}

.modal-section-title {
  font-size: 0.82rem;
  font-weight: 700;
  color: #cbd5e1;
  border-left: 3px solid #a855f7;
  padding-left: 0.5rem;
}

.audio-source-tabs {
  display: flex;
  gap: 0.5rem;
}

.btn-source-tab {
  flex: 1;
  padding: 0.55rem 0.8rem;
  font-size: 0.82rem;
  font-weight: 600;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-source-tab:hover {
  background: rgba(255, 255, 255, 0.09);
  color: #fff;
}

.btn-source-tab.active {
  background: rgba(168, 85, 247, 0.2);
  border-color: rgba(168, 85, 247, 0.5);
  color: #e9d5ff;
}

/* Tab Content: Library */
.tab-pane-library {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.audio-search-box {
  display: flex;
  gap: 0.5rem;
}

.audio-search-input {
  flex: 1;
  padding: 0.45rem 0.75rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.82rem;
}

.btn-refresh-audios {
  padding: 0.45rem 0.85rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.78rem;
  cursor: pointer;
}

.library-audios-list {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  padding: 0.35rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.audios-loading-state,
.audios-empty-hint {
  padding: 1.5rem 1rem;
  text-align: center;
  font-size: 0.78rem;
  color: #94a3b8;
}

.spinner-small {
  width: 20px;
  height: 20px;
  border: 2px solid rgba(168, 85, 247, 0.2);
  border-top-color: #a855f7;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin: 0 auto 0.5rem auto;
}

.audio-select-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.45rem 0.65rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.15s ease;
}

.audio-select-item:hover {
  background: rgba(255, 255, 255, 0.08);
}

.audio-select-item.selected {
  background: rgba(168, 85, 247, 0.18);
  border-color: rgba(168, 85, 247, 0.45);
}

.audio-item-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  min-width: 0;
  flex: 1;
}

.audio-fmt-tag {
  font-size: 0.65rem;
  font-weight: 700;
  padding: 1px 4px;
  border-radius: 3px;
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
  border: 1px solid rgba(59, 130, 246, 0.35);
  flex-shrink: 0;
}

.audio-title-col {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.audio-item-name {
  font-size: 0.82rem;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.audio-item-source {
  font-size: 0.7rem;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.audio-item-right {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-shrink: 0;
}

.btn-audio-preview {
  padding: 2px 7px;
  border-radius: 4px;
  font-size: 0.72rem;
  font-weight: 600;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #cbd5e1;
  cursor: pointer;
}

.btn-audio-preview:hover {
  background: rgba(255, 255, 255, 0.18);
  color: #fff;
}

.btn-audio-preview.playing {
  background: rgba(239, 68, 68, 0.25);
  border-color: rgba(239, 68, 68, 0.5);
  color: #fca5a5;
}

.audio-dur {
  font-size: 0.72rem;
  color: #38bdf8;
}

.audio-size {
  font-size: 0.7rem;
  color: #64748b;
}

.check-mark {
  font-size: 0.72rem;
  font-weight: 700;
  color: #34d399;
}

.btn-select-audio {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.72rem;
  background: rgba(168, 85, 247, 0.18);
  border: 1px solid rgba(168, 85, 247, 0.35);
  color: #d8b4fe;
  cursor: pointer;
}

/* Tab Content: Local */
.tab-pane-local {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.local-picker-row {
  display: flex;
  gap: 0.5rem;
}

.local-path-input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.82rem;
}

.btn-browse-file {
  padding: 0.5rem 1rem;
  border-radius: 6px;
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.25), rgba(99, 102, 241, 0.25));
  border: 1px solid rgba(168, 85, 247, 0.4);
  color: #e9d5ff;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}

.btn-browse-file:hover {
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.4), rgba(99, 102, 241, 0.4));
  color: #fff;
}

.local-picker-hint {
  font-size: 0.72rem;
  color: #64748b;
}

/* Selected Audio Bar */
.selected-audio-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.55rem 0.85rem;
  border-radius: 6px;
  background: rgba(168, 85, 247, 0.12);
  border: 1px solid rgba(168, 85, 247, 0.3);
}

.bar-left {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  overflow: hidden;
  flex: 1;
}

.bar-label {
  font-size: 0.76rem;
  font-weight: 600;
  color: #d8b4fe;
  flex-shrink: 0;
}

.bar-path {
  font-size: 0.74rem;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn-bar-preview {
  padding: 3px 10px;
  border-radius: 4px;
  font-size: 0.74rem;
  font-weight: 600;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #fff;
  cursor: pointer;
  white-space: nowrap;
}

.btn-bar-preview:hover {
  background: rgba(255, 255, 255, 0.2);
}

.btn-bar-preview.playing {
  background: rgba(239, 68, 68, 0.3);
  border-color: rgba(239, 68, 68, 0.5);
  color: #fca5a5;
}

/* Synthesis Modes */
.synthesis-mode-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.mode-card {
  padding: 0.75rem 0.9rem;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  transition: all 0.15s ease;
}

.mode-card:hover {
  background: rgba(255, 255, 255, 0.06);
}

.mode-card.active {
  background: rgba(168, 85, 247, 0.14);
  border-color: rgba(168, 85, 247, 0.5);
}

.radio-hide {
  display: none;
}

.mode-card-header {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.mode-icon {
  font-size: 1.1rem;
}

.mode-name {
  font-size: 0.85rem;
  font-weight: 700;
  color: #fff;
}

.mode-desc {
  margin: 0;
  font-size: 0.72rem;
  line-height: 1.4;
  color: #94a3b8;
}

/* Volume Sliders */
.volume-controls-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.volume-slider-box {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  background: rgba(255, 255, 255, 0.03);
  padding: 0.65rem 0.85rem;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.slider-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.slider-label {
  font-size: 0.76rem;
  font-weight: 600;
  color: #cbd5e1;
}

.slider-val {
  font-size: 0.76rem;
  font-weight: 700;
  color: #c084fc;
}

.volume-slider {
  width: 100%;
  accent-color: #a855f7;
  cursor: pointer;
}

/* Extra settings & custom title */
.export-settings-row {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.setting-checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.78rem;
  color: #cbd5e1;
  cursor: pointer;
}

.export-title-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.title-label {
  font-size: 0.8rem;
  color: #94a3b8;
  white-space: nowrap;
}

.export-title-input {
  flex: 1;
  padding: 0.45rem 0.75rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.82rem;
}

.btn-dialog-primary {
  padding: 0.48rem 1.3rem;
  border-radius: 6px;
  background: linear-gradient(135deg, #a855f7, #6366f1);
  border: none;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-dialog-primary:hover:not(:disabled) {
  opacity: 0.95;
  transform: translateY(-1px);
  box-shadow: 0 4px 14px rgba(168, 85, 247, 0.4);
}

.btn-dialog-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Progress Overlay */
.modal-backdrop-layer.progress-mode {
  z-index: 1200;
}

.progress-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.75rem;
  background: #161b22;
  border: 1px solid rgba(168, 85, 247, 0.4);
  border-radius: 14px;
  padding: 2.2rem 2.8rem;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.8);
  max-width: 380px;
}

.progress-card h4 {
  margin: 0;
  font-size: 1.05rem;
  color: #f3e8ff;
}

.progress-card p {
  margin: 0;
  font-size: 0.82rem;
  color: #94a3b8;
  line-height: 1.5;
}

.ai-spinner {
  width: 44px;
  height: 44px;
  border: 3px solid rgba(168, 85, 247, 0.2);
  border-top-color: #a855f7;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.elapsed-badge {
  font-family: monospace;
  font-size: 0.8rem;
  color: #c084fc;
  background: rgba(168, 85, 247, 0.12);
  padding: 3px 8px;
  border-radius: 4px;
}
</style>

