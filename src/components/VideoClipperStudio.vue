<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { downloadDir } from '@tauri-apps/api/path';
import type {
  Video,
  VideoClipSegment,
  ClipVideoRequest,
  ClipVideoResponse,
  RecommendSegmentsResponse,
  SegmentRecommendation,
  VideoListResponse,
} from '../types';

const props = defineProps<{
  initialVideo?: Video | null;
}>();

const emit = defineEmits<{
  (e: 'navigateLibrary'): void;
  (e: 'navigateClips'): void;
  (e: 'playVideo', video: Video): void;
}>();

// Palette of distinctive colors for segment blocks
const SEGMENT_COLORS = [
  '#6366f1', // Indigo
  '#10b981', // Emerald
  '#f59e0b', // Amber
  '#ec4899', // Pink
  '#06b6d4', // Cyan
  '#8b5cf6', // Purple
  '#14b8a6', // Teal
  '#f97316', // Orange
  '#3b82f6', // Blue
];

// Active video state
const activeVideo = ref<Video | null>(props.initialVideo || null);
const showPickerModal = ref<boolean>(false);
const libraryVideos = ref<Video[]>([]);
const pickerSearch = ref<string>('');
const isLibraryLoading = ref<boolean>(false);

// Video player state
const videoRef = ref<HTMLVideoElement | null>(null);
const isPlaying = ref<boolean>(false);
const currentTime = ref<number>(0);
const duration = ref<number>(0);
const activePreviewSegmentId = ref<string | null>(null);

// Segments list state
const segments = ref<VideoClipSegment[]>([]);
const selectedSegmentId = ref<string | null>(null);

// AI Recommendation & Climax Highlight state
const isAnalyzingAi = ref<boolean>(false);
const showAiModal = ref<boolean>(false);
const aiRecommendations = ref<SegmentRecommendation[]>([]);
const aiHighlights = ref<SegmentRecommendation[]>([]);
const aiModalTab = ref<'highlights' | 'chapters'>('highlights');
const aiSource = ref<string>('');
const aiMessage = ref<string>('');

const currentAiDisplayList = computed(() => {
  if (aiModalTab.value === 'highlights') {
    return aiHighlights.value.length > 0 ? aiHighlights.value : aiRecommendations.value;
  }
  return aiRecommendations.value;
});

// Export settings state
const exportMode = ref<'separate' | 'merge' | 'both'>('separate');
const fastCopy = ref<boolean>(true);
const saveDest = ref<'source' | 'downloads' | 'custom'>('source');
const customOutputDir = ref<string>('');

// Execution / Progress / Result state
const isClipping = ref<boolean>(false);
const clipElapsed = ref<number>(0);
let clipTimer: number | null = null;
const clipResult = ref<ClipVideoResponse | null>(null);
const showResultModal = ref<boolean>(false);
const errorMessage = ref<string>('');

// Toast notification
const toastMessage = ref<string>('');
let toastTimer: number | null = null;

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2600);
}

// Convert video file path to streamable asset URL
const videoSrc = computed(() => {
  if (!activeVideo.value?.file_path) return '';
  return convertFileSrc(activeVideo.value.file_path);
});

// Format ms or seconds to HH:MM:SS.mmm
function formatTime(sec: number, includeMs = false): string {
  if (isNaN(sec) || sec < 0) sec = 0;
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  const ms = Math.floor((sec % 1) * 1000);

  const mm = m.toString().padStart(2, '0');
  const ss = s.toString().padStart(2, '0');

  if (includeMs) {
    const msStr = ms.toString().padStart(3, '0');
    if (h > 0) {
      return `${h.toString().padStart(2, '0')}:${mm}:${ss}.${msStr}`;
    }
    return `${mm}:${ss}.${msStr}`;
  }

  if (h > 0) {
    return `${h.toString().padStart(2, '0')}:${mm}:${ss}`;
  }
  return `${mm}:${ss}`;
}

function msToTimeStr(ms: number): string {
  return formatTime(ms / 1000.0, true);
}

function timeStrToMs(str: string): number {
  const parts = str.trim().split(':');
  if (parts.length === 2) {
    const [m, s] = parts;
    return Math.round((parseFloat(m) * 60 + parseFloat(s)) * 1000);
  } else if (parts.length === 3) {
    const [h, m, s] = parts;
    return Math.round((parseFloat(h) * 3600 + parseFloat(m) * 60 + parseFloat(s)) * 1000);
  }
  const val = parseFloat(str);
  return isNaN(val) ? 0 : Math.round(val * 1000);
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

// Color helper for segment index
function getSegmentColor(index: number): string {
  return SEGMENT_COLORS[index % SEGMENT_COLORS.length];
}

// -------------------------------------------------------------
// Video Playback Controls
// -------------------------------------------------------------

function togglePlay() {
  if (!videoRef.value) return;
  if (videoRef.value.paused) {
    videoRef.value.play();
  } else {
    videoRef.value.pause();
  }
}

function seekRelative(deltaSec: number) {
  if (!videoRef.value) return;
  const target = Math.min(Math.max(0, videoRef.value.currentTime + deltaSec), duration.value);
  videoRef.value.currentTime = target;
}

function seekToSec(sec: number) {
  if (!videoRef.value) return;
  videoRef.value.currentTime = Math.min(Math.max(0, sec), duration.value);
}

function onTimeUpdate() {
  if (!videoRef.value) return;
  currentTime.value = videoRef.value.currentTime;

  // Segment isolated preview check
  if (activePreviewSegmentId.value) {
    const activeSeg = segments.value.find((s) => s.id === activePreviewSegmentId.value);
    if (activeSeg) {
      const endSec = activeSeg.endMs / 1000.0;
      if (currentTime.value >= endSec) {
        videoRef.value.pause();
        videoRef.value.currentTime = activeSeg.startMs / 1000.0;
        activePreviewSegmentId.value = null;
        showToast('片段试看结束');
      }
    }
  }
}

function onLoadedMetadata() {
  if (!videoRef.value) return;
  duration.value = videoRef.value.duration || (activeVideo.value?.duration || 0);

  // If segments are empty, create an initial default segment [0, duration]
  if (segments.value.length === 0 && duration.value > 0) {
    const endMs = Math.round(Math.min(duration.value, 30) * 1000);
    const initialSeg: VideoClipSegment = {
      id: `seg_${Date.now()}_1`,
      title: '精彩片段 01',
      startMs: 0,
      endMs: endMs,
    };
    segments.value = [initialSeg];
    selectedSegmentId.value = initialSeg.id;
  }
}

function playSegment(seg: VideoClipSegment) {
  if (!videoRef.value) return;
  activePreviewSegmentId.value = seg.id;
  selectedSegmentId.value = seg.id;
  videoRef.value.currentTime = seg.startMs / 1000.0;
  videoRef.value.play();
  showToast(`试看片段: ${seg.title}`);
}

// -------------------------------------------------------------
// Timeline & Segment Manipulation
// -------------------------------------------------------------

const activeSegment = computed(() => {
  return segments.value.find((s) => s.id === selectedSegmentId.value) || null;
});

function handleTimelineClick(e: MouseEvent) {
  const target = e.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  const clickX = e.clientX - rect.left;
  const ratio = Math.max(0, Math.min(1, clickX / rect.width));
  const targetTime = ratio * duration.value;
  seekToSec(targetTime);
}

function setInPoint() {
  const curMs = Math.round(currentTime.value * 1000);
  if (activeSegment.value) {
    if (curMs >= activeSegment.value.endMs) {
      activeSegment.value.endMs = Math.min(Math.round(duration.value * 1000), curMs + 5000);
    }
    activeSegment.value.startMs = curMs;
    showToast(`设置起点: ${formatTime(currentTime.value, true)}`);
  } else {
    // Create new segment starting here
    addNewSegment(curMs);
  }
}

function setOutPoint() {
  const curMs = Math.round(currentTime.value * 1000);
  if (activeSegment.value) {
    if (curMs <= activeSegment.value.startMs) {
      showToast('终点时间不能早于或等于起点时间');
      return;
    }
    activeSegment.value.endMs = curMs;
    showToast(`设置终点: ${formatTime(currentTime.value, true)}`);
  }
}

function splitAtPlayhead() {
  const curMs = Math.round(currentTime.value * 1000);
  // Find which segment contains current playhead
  const targetIndex = segments.value.findIndex(
    (s) => curMs > s.startMs && curMs < s.endMs
  );

  if (targetIndex >= 0) {
    const original = segments.value[targetIndex];
    const oldEnd = original.endMs;
    original.endMs = curMs;

    const newSeg: VideoClipSegment = {
      id: `seg_${Date.now()}_${segments.value.length + 1}`,
      title: `${original.title} (后段)`,
      startMs: curMs,
      endMs: oldEnd,
    };

    segments.value.splice(targetIndex + 1, 0, newSeg);
    selectedSegmentId.value = newSeg.id;
    showToast(`已在 ${formatTime(currentTime.value, true)} 截断为两个片段`);
  } else {
    // Add a new segment from playhead
    addNewSegment(curMs);
  }
}

function addNewSegment(startMs?: number) {
  const start = startMs !== undefined ? startMs : Math.round(currentTime.value * 1000);
  const end = Math.min(Math.round(duration.value * 1000), start + 15000);

  const newSeg: VideoClipSegment = {
    id: `seg_${Date.now()}_${segments.value.length + 1}`,
    title: `精彩片段 ${String(segments.value.length + 1).padStart(2, '0')}`,
    startMs: start,
    endMs: end > start ? end : start + 3000,
  };

  segments.value.push(newSeg);
  selectedSegmentId.value = newSeg.id;
  showToast(`已添加新片段 #${segments.value.length}`);
}

function removeSegment(id: string) {
  const idx = segments.value.findIndex((s) => s.id === id);
  if (idx >= 0) {
    segments.value.splice(idx, 1);
    if (selectedSegmentId.value === id) {
      selectedSegmentId.value = segments.value[0]?.id || null;
    }
    showToast('已删除片段');
  }
}

function updateSegmentStart(seg: VideoClipSegment, val: string) {
  const ms = timeStrToMs(val);
  if (ms >= 0 && ms < seg.endMs) {
    seg.startMs = ms;
  }
}

function updateSegmentEnd(seg: VideoClipSegment, val: string) {
  const ms = timeStrToMs(val);
  if (ms > seg.startMs) {
    seg.endMs = Math.min(Math.round(duration.value * 1000), ms);
  }
}

// -------------------------------------------------------------
// AI Smart Recommendation Logic
// -------------------------------------------------------------

async function requestAiRecommendation(initialTab?: 'highlights' | 'chapters' | Event | unknown) {
  if (!activeVideo.value) return;
  isAnalyzingAi.value = true;
  aiMessage.value = '';
  const tab = initialTab === 'highlights' || initialTab === 'chapters' ? initialTab : undefined;
  if (tab) {
    aiModalTab.value = tab;
  }

  try {
    const res = await invoke<RecommendSegmentsResponse>('recommend_video_segments', {
      videoId: activeVideo.value.id,
      videoPath: activeVideo.value.file_path,
      mode: 'all',
    });

    aiRecommendations.value = res.segments || [];
    aiHighlights.value = res.highlights || [];
    aiSource.value = res.source;
    aiMessage.value = res.message;
    if (tab) {
      aiModalTab.value = tab;
    } else if (aiHighlights.value.length > 0) {
      aiModalTab.value = 'highlights';
    }
    showAiModal.value = true;
  } catch (err: unknown) {
    showToast('AI 推荐分段获取失败: ' + String(err));
  } finally {
    isAnalyzingAi.value = false;
  }
}

function applyAllAiRecommendations() {
  const targetList = currentAiDisplayList.value;
  if (targetList.length === 0) return;
  segments.value = targetList.map((rec, i) => ({
    id: `ai_seg_${Date.now()}_${i + 1}`,
    title: rec.title,
    startMs: rec.startMs,
    endMs: rec.endMs,
  }));
  selectedSegmentId.value = segments.value[0]?.id || null;
  showAiModal.value = false;
  showToast(`已成功采纳 ${segments.value.length} 个 ${aiModalTab.value === 'highlights' ? '核心高能高潮' : 'AI 智能'} 分段！`);
}

function applyOnlyHighlightsAndMerge() {
  if (aiHighlights.value.length === 0) return;
  segments.value = aiHighlights.value.map((rec, i) => ({
    id: `climax_seg_${Date.now()}_${i + 1}`,
    title: rec.title,
    startMs: rec.startMs,
    endMs: rec.endMs,
  }));
  selectedSegmentId.value = segments.value[0]?.id || null;
  exportMode.value = 'merge';
  showAiModal.value = false;
  showToast(`🔥 已采纳 ${aiHighlights.value.length} 个高潮片段，已自动切换为「拼接成片」！`);
}

function adoptSingleAiRecommendation(rec: SegmentRecommendation) {
  const newSeg: VideoClipSegment = {
    id: `ai_single_${Date.now()}`,
    title: rec.title,
    startMs: rec.startMs,
    endMs: rec.endMs,
  };
  segments.value.push(newSeg);
  selectedSegmentId.value = newSeg.id;
  showToast(`已采纳: ${rec.title}`);
}

function previewRecommendation(rec: SegmentRecommendation) {
  seekToSec(rec.startMs / 1000.0);
  if (videoRef.value && videoRef.value.paused) {
    videoRef.value.play().catch(() => {});
  }
}

const aiSourceLabel = computed(() => {
  switch (aiSource.value) {
    case 'ai_climax':
      return '🔥 多模态声学爆发与语义核心高潮';
    case 'ai_topics':
      return '🧠 AI 深度语义主题章节 (精准要点)';
    case 'subtitles':
      return '🎙️ AI 语音语义停顿聚类 (Whisper 识别)';
    case 'silence_detect':
      return '🔇 音频静音波形间歇分割 (自适应停顿)';
    default:
      return '⏱️ 智能黄金时长节奏切片';
  }
});

// -------------------------------------------------------------
// Video Selection Modal & Switching
// -------------------------------------------------------------

async function openVideoPicker() {
  showPickerModal.value = true;
  pickerSearch.value = '';
  if (libraryVideos.value.length === 0) {
    isLibraryLoading.value = true;
    try {
      const resp = await invoke<VideoListResponse>('list_videos', {
        filter: { page_size: 100 },
      });
      libraryVideos.value = resp.items || [];
    } catch (err) {
      console.warn('Failed to load library videos for picker:', err);
    } finally {
      isLibraryLoading.value = false;
    }
  }
}

function selectVideo(vid: Video) {
  activeVideo.value = vid;
  segments.value = [];
  selectedSegmentId.value = null;
  currentTime.value = 0;
  showPickerModal.value = false;
  showToast(`已载入视频: ${vid.file_name}`);
}

const filteredPickerVideos = computed(() => {
  const q = pickerSearch.value.trim().toLowerCase();
  if (!q) return libraryVideos.value;
  return libraryVideos.value.filter(
    (v) =>
      v.file_name.toLowerCase().includes(q) ||
      (v.tags && v.tags.some((t) => t.name.toLowerCase().includes(q)))
  );
});

// -------------------------------------------------------------
// Export / Execution
// -------------------------------------------------------------

async function startClipping() {
  if (!activeVideo.value?.file_path) {
    showToast('未选择有效视频文件');
    return;
  }
  if (segments.value.length === 0) {
    showToast('请至少添加一个裁剪分段');
    return;
  }

  // Validate segment ranges
  for (const seg of segments.value) {
    if (seg.endMs <= seg.startMs) {
      showToast(`片段 "${seg.title}" 终点时间必须大于起点时间`);
      selectedSegmentId.value = seg.id;
      return;
    }
  }

  isClipping.value = true;
  errorMessage.value = '';
  clipElapsed.value = 0;
  if (clipTimer) clearInterval(clipTimer);
  clipTimer = window.setInterval(() => {
    clipElapsed.value += 1;
  }, 1000);

  try {
    let resolvedOutputDir: string | undefined = undefined;
    if (saveDest.value === 'downloads') {
      resolvedOutputDir = await downloadDir();
    } else if (saveDest.value === 'custom') {
      resolvedOutputDir = customOutputDir.value.trim() || undefined;
    }

    const request: ClipVideoRequest = {
      videoId: activeVideo.value.id,
      videoPath: activeVideo.value.file_path,
      segments: segments.value,
      outputDir: resolvedOutputDir,
      exportMode: exportMode.value,
      fastCopy: fastCopy.value,
    };

    const res = await invoke<ClipVideoResponse>('clip_video_segments', { request });
    clipResult.value = res;
    showResultModal.value = true;
    showToast('🎉 视频截断与导出顺利完成！');
  } catch (err: unknown) {
    errorMessage.value = String(err);
    showToast('裁剪导出失败: ' + String(err));
  } finally {
    isClipping.value = false;
    if (clipTimer) clearInterval(clipTimer);
  }
}

async function revealFile(path: string) {
  try {
    await invoke('reveal_in_explorer', { path });
    showToast('已在系统文件管理器中定位');
  } catch (err) {
    showToast('定位文件失败: ' + String(err));
  }
}

// Keyboard shortcuts (Space to toggle play)
function handleKeydown(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
    return;
  }
  if (e.code === 'Space') {
    e.preventDefault();
    togglePlay();
  } else if (e.code === 'ArrowLeft') {
    e.preventDefault();
    seekRelative(-5);
  } else if (e.code === 'ArrowRight') {
    e.preventDefault();
    seekRelative(5);
  } else if (e.key === 'i' || e.key === 'I') {
    setInPoint();
  } else if (e.key === 'o' || e.key === 'O') {
    setOutPoint();
  } else if (e.key === 's' || e.key === 'S') {
    splitAtPlayhead();
  }
}

watch(
  () => props.initialVideo,
  (newVid) => {
    if (newVid) {
      activeVideo.value = newVid;
      segments.value = [];
      selectedSegmentId.value = null;
    }
  }
);

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
  if (!activeVideo.value) {
    openVideoPicker();
  }
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
  if (clipTimer) clearInterval(clipTimer);
});
</script>

<template>
  <div class="clipper-studio-container">
    <!-- Studio Header & Video Selector -->
    <header class="studio-header-card">
      <div class="studio-header-left">
        <!-- Navigation Pill Group -->
        <div class="nav-pill-group">
          <button
            class="btn-nav-pill"
            title="返回媒体库列表"
            @click="emit('navigateLibrary')"
          >
            <span class="back-arrow">←</span>
            <span>返回媒体库</span>
          </button>
          <button
            class="btn-nav-pill secondary"
            title="查看已裁剪的视频与分段管理列表"
            @click="emit('navigateClips')"
          >
            <span>📁 裁剪管理</span>
          </button>
        </div>

        <div class="header-v-divider"></div>

        <!-- Video Identity & Metadata -->
        <div class="active-video-identity">
          <div class="video-thumb-preview" title="点击从媒体库更换视频" @click="openVideoPicker">
            <img
              v-if="activeVideo?.thumbnail_data"
              :src="activeVideo.thumbnail_data"
              class="thumb-img"
              alt="thumbnail"
            />
            <div v-else class="thumb-placeholder">🎬</div>
            <span class="change-overlay-badge">换片</span>
          </div>

          <div class="video-meta-texts">
            <div class="title-row">
              <span class="video-title" :title="activeVideo?.file_name">
                {{ activeVideo?.file_name || '未选择视频，请点击选择' }}
              </span>
              <button class="btn-switch-video" title="更换当前裁剪视频" @click="openVideoPicker">
                🔄 更换
              </button>
            </div>

            <div v-if="activeVideo" class="meta-pills-row">
              <span class="meta-pill" title="视频总时长">
                ⏱️ {{ formatTime(duration || activeVideo.duration || 0) }}
              </span>
              <span v-if="activeVideo.width && activeVideo.height" class="meta-pill" title="分辨率">
                📐 {{ activeVideo.width }}×{{ activeVideo.height }}
              </span>
              <span v-if="activeVideo.codec" class="meta-pill" title="编码格式">
                🎞️ {{ activeVideo.codec.toUpperCase() }}
              </span>
              <span class="meta-pill" title="文件大小">
                💾 {{ formatBytes(activeVideo.file_size) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Top Action Bar -->
      <div class="studio-top-actions">
        <button
          class="btn-act climax-btn"
          :disabled="!activeVideo || isAnalyzingAi"
          :title="'结合声学能量爆发与 Whisper 语义台词，精准截取全片高潮与高能精彩亮点'"
          @click="requestAiRecommendation('highlights')"
        >
          <span class="climax-fire-icon">🔥</span>
          <span>{{ isAnalyzingAi ? '正在提取...' : '提取高潮亮点' }}</span>
        </button>

        <button
          class="btn-act ai-btn"
          :disabled="!activeVideo || isAnalyzingAi"
          :title="'根据 AI 深度语义要点、语音停顿或静音检测自动推荐全片结构化阶段'"
          @click="requestAiRecommendation('chapters')"
        >
          <span class="ai-sparkle-icon">✨</span>
          <span>{{ isAnalyzingAi ? 'AI 分析中...' : 'AI 智能分段' }}</span>
        </button>

        <div class="header-v-divider"></div>

        <button
          class="btn-act export-cta-btn"
          :disabled="!activeVideo || segments.length === 0 || isClipping"
          @click="startClipping"
        >
          <span>🚀 截断导出 ({{ segments.length }})</span>
        </button>
      </div>
    </header>

    <!-- Main Workspace Grid -->
    <main class="studio-workspace-grid">
      <!-- Left Column: Video Player & Visual Scrubber -->
      <section class="player-panel-card">
        <div class="video-viewport">
          <video
            ref="videoRef"
            :src="videoSrc"
            class="native-video-el"
            @timeupdate="onTimeUpdate"
            @loadedmetadata="onLoadedMetadata"
            @play="isPlaying = true"
            @pause="isPlaying = false"
            @click="togglePlay"
          ></video>

          <!-- Floating Time Badge -->
          <div class="floating-time-hud">
            <span class="time-current font-mono">{{ formatTime(currentTime, true) }}</span>
            <span class="time-divider">/</span>
            <span class="time-total font-mono">{{ formatTime(duration, true) }}</span>
          </div>
        </div>

        <!-- Video Player Controls Toolbar -->
        <div class="player-controls-strip">
          <div class="left-transport-buttons">
            <button class="transport-btn" title="后退 5 秒 (←)" @click="seekRelative(-5)">
              ⏪ -5s
            </button>
            <button
              class="transport-btn play-btn"
              :class="{ playing: isPlaying }"
              title="播放 / 暂停 (空格键)"
              @click="togglePlay"
            >
              {{ isPlaying ? '⏸️ 暂停' : '▶️ 播放' }}
            </button>
            <button class="transport-btn" title="前进 5 秒 (→)" @click="seekRelative(5)">
              +5s ⏩
            </button>
          </div>

          <div class="right-quick-markers">
            <button
              class="marker-shortcut-btn in-btn"
              title="将当前播放点设为阶段起点 (快捷键: I)"
              @click="setInPoint"
            >
              [ 设起点 (I)
            </button>
            <button
              class="marker-shortcut-btn split-btn"
              title="在当前播放进度处将片段一分为二 (快捷键: S)"
              @click="splitAtPlayhead"
            >
              ✂️ 在此截断 (S)
            </button>
            <button
              class="marker-shortcut-btn out-btn"
              title="将当前播放点设为阶段终点 (快捷键: O)"
              @click="setOutPoint"
            >
              设终点 (O) ]
            </button>
          </div>
        </div>

        <!-- Visual Multi-Segment Timeline Scrubber -->
        <div class="visual-timeline-section">
          <div class="timeline-labels-row">
            <div class="timeline-title-left">
              <span class="timeline-icon">🎬</span>
              <span class="timeline-title-text">时间轴分段轨道</span>
              <span class="timeline-hint">（点击轨道跳转播放；彩条为截断阶段）</span>
            </div>
            <div class="timeline-counter">
              当前规划阶段: <span class="badge-seg-count">{{ segments.length }}</span> 段
            </div>
          </div>

          <div class="timeline-track-container" @click="handleTimelineClick">
            <!-- Background base track -->
            <div class="timeline-base-rail"></div>

            <!-- Segment Blocks -->
            <div
              v-for="(seg, idx) in segments"
              :key="seg.id"
              class="timeline-segment-block"
              :class="{ active: selectedSegmentId === seg.id }"
              :style="{
                left: `${duration > 0 ? (seg.startMs / 1000.0 / duration) * 100 : 0}%`,
                width: `${duration > 0 ? ((seg.endMs - seg.startMs) / 1000.0 / duration) * 100 : 0}%`,
                backgroundColor: getSegmentColor(idx),
              }"
              :title="`${seg.title} (${msToTimeStr(seg.startMs)} ~ ${msToTimeStr(seg.endMs)})`"
              @click.stop="selectedSegmentId = seg.id"
            >
              <span class="block-title-text">{{ seg.title }}</span>
            </div>

            <!-- Current Playhead Scrubber Handle -->
            <div
              class="timeline-playhead-line"
              :style="{
                left: `${duration > 0 ? (currentTime / duration) * 100 : 0}%`,
              }"
            >
              <div class="playhead-thumb"></div>
            </div>
          </div>

          <div class="timeline-ticks-row font-mono">
            <span>00:00.000</span>
            <span>{{ formatTime(duration, true) }}</span>
          </div>
        </div>
      </section>

      <!-- Right Column: Segments List & Export Settings -->
      <section class="segments-editor-panel">
        <div class="panel-card-header">
          <div class="card-title-group">
            <span class="header-icon">📑</span>
            <h4>阶段切片列表</h4>
            <span class="badge-count">{{ segments.length }}</span>
          </div>

          <div class="card-header-actions">
            <button class="btn-xs-primary" title="新建一段截取范围" @click="addNewSegment()">
              ➕ 添加片段
            </button>
            <button
              v-if="segments.length > 0"
              class="btn-xs-danger"
              title="清空全部阶段"
              @click="segments = []"
            >
              🗑️ 全部清空
            </button>
          </div>
        </div>

        <!-- Segments Items Scrollable Container -->
        <div class="segments-list-scroll">
          <div v-if="segments.length === 0" class="empty-segments-state">
            <span class="empty-icon">✂️</span>
            <h5>暂无划分的视频阶段</h5>
            <p>可点击右上角“添加片段”手动划定，或点击“提取高潮”/“AI 智能分段”一键自动划分</p>
            <div class="empty-action-btns">
              <button class="btn-empty-act climax" @click="requestAiRecommendation('highlights')">
                🔥 提取高潮亮点
              </button>
              <button class="btn-empty-act ai" @click="requestAiRecommendation('chapters')">
                ✨ AI 智能分段
              </button>
              <button class="btn-empty-act manual" @click="addNewSegment()">
                ➕ 手动添加
              </button>
            </div>
          </div>

          <div
            v-for="(seg, idx) in segments"
            :key="seg.id"
            class="segment-item-card"
            :class="{ active: selectedSegmentId === seg.id }"
            :style="{ '--seg-accent': getSegmentColor(idx) }"
            @click="selectedSegmentId = seg.id"
          >
            <div class="card-accent-bar" :style="{ backgroundColor: getSegmentColor(idx) }"></div>

            <div class="item-content-body">
              <div class="item-title-row">
                <div class="title-left">
                  <span class="item-index-pill" :style="{ backgroundColor: getSegmentColor(idx) }">
                    #{{ idx + 1 }}
                  </span>
                  <input
                    v-model="seg.title"
                    type="text"
                    class="segment-title-input"
                    placeholder="片段名称 (如: 精彩亮点)"
                  />
                </div>
                <div class="title-right">
                  <span class="duration-tag font-mono">
                    {{ formatTime((seg.endMs - seg.startMs) / 1000.0) }}
                  </span>
                  <button
                    class="btn-item-act play"
                    title="在左侧试看预览该片段"
                    @click.stop="playSegment(seg)"
                  >
                    ▶
                  </button>
                  <button
                    class="btn-item-act delete"
                    title="删除该片段"
                    @click.stop="removeSegment(seg.id)"
                  >
                    ✕
                  </button>
                </div>
              </div>

              <div class="item-timecode-grid">
                <!-- Start Time Picker -->
                <div class="time-picker-box">
                  <span class="lbl">起点</span>
                  <input
                    :value="msToTimeStr(seg.startMs)"
                    type="text"
                    class="time-input font-mono"
                    @change="updateSegmentStart(seg, ($event.target as HTMLInputElement).value)"
                  />
                  <button
                    class="btn-stamp-now"
                    title="将当前播放进度设为起点"
                    @click.stop="seg.startMs = Math.round(currentTime * 1000)"
                  >
                    🎯 当前
                  </button>
                </div>

                <span class="time-arrow">→</span>

                <!-- End Time Picker -->
                <div class="time-picker-box">
                  <span class="lbl">终点</span>
                  <input
                    :value="msToTimeStr(seg.endMs)"
                    type="text"
                    class="time-input font-mono"
                    @change="updateSegmentEnd(seg, ($event.target as HTMLInputElement).value)"
                  />
                  <button
                    class="btn-stamp-now"
                    title="将当前播放进度设为终点"
                    @click.stop="seg.endMs = Math.round(currentTime * 1000)"
                  >
                    🎯 当前
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Compact Export Configuration Dock -->
        <div class="export-settings-card">
          <div class="export-header-row">
            <span class="export-title">⚙️ 截取与导出模式</span>
            <label class="toggle-control-label">
              <input v-model="fastCopy" type="checkbox" />
              <span>⚡ 极速流拷贝 (无损秒出)</span>
            </label>
          </div>

          <div class="export-mode-segmented">
            <button
              class="segmented-btn"
              :class="{ active: exportMode === 'separate' }"
              @click="exportMode = 'separate'"
            >
              <span>📂 独立切片</span>
            </button>
            <button
              class="segmented-btn"
              :class="{ active: exportMode === 'merge' }"
              @click="exportMode = 'merge'"
            >
              <span>🔗 顺序合并</span>
            </button>
            <button
              class="segmented-btn"
              :class="{ active: exportMode === 'both' }"
              @click="exportMode = 'both'"
            >
              <span>📦 全部产出</span>
            </button>
          </div>
        </div>
      </section>
    </main>

    <!-- AI Smart Recommendation Modal -->
    <div
      v-if="showAiModal"
      class="modal-backdrop-layer"
      @click.self="showAiModal = false"
    >
      <div class="ai-recommend-dialog">
        <div class="modal-dialog-header">
          <div class="dialog-title">
            <span class="sparkle-ic">{{ aiModalTab === 'highlights' ? '🔥' : '✨' }}</span>
            <h4>{{ aiModalTab === 'highlights' ? 'AI 精准高潮高能片段提取' : 'AI 智能结构化阶段建议' }}</h4>
          </div>
          <button class="btn-close-dialog" @click="showAiModal = false">✕</button>
        </div>

        <!-- Mode Switch Tabs -->
        <div class="ai-tabs-nav-bar">
          <button
            class="ai-tab-btn"
            :class="{ active: aiModalTab === 'highlights' }"
            @click="aiModalTab = 'highlights'"
          >
            🔥 精选高能高潮 ({{ aiHighlights.length }})
          </button>
          <button
            class="ai-tab-btn"
            :class="{ active: aiModalTab === 'chapters' }"
            @click="aiModalTab = 'chapters'"
          >
            📑 完整结构化阶段 ({{ aiRecommendations.length }})
          </button>
        </div>

        <div class="modal-dialog-body">
          <div class="ai-strategy-chip" :class="{ 'climax-style': aiModalTab === 'highlights' }">
            <div class="strategy-header">
              <span class="strategy-badge">{{ aiSourceLabel }}</span>
              <span v-if="aiModalTab === 'highlights'" class="climax-hint-badge">
                ⚡ 基于声学能量 RMS 爆发峰值 + Whisper 情绪台词自然对齐
              </span>
            </div>
            <p class="strategy-message">{{ aiMessage }}</p>
          </div>

          <div v-if="currentAiDisplayList.length === 0" class="empty-recs-tip">
            暂无当前标签分段，可尝试切换标签页查看完整阶段。
          </div>

          <div v-else class="recommendations-list">
            <div
              v-for="(rec, idx) in currentAiDisplayList"
              :key="rec.id"
              class="recommendation-item"
              :class="{ 'is-climax-item': rec.isHighlight }"
            >
              <div class="rec-left">
                <span class="rec-idx">#{{ idx + 1 }}</span>
                <div class="rec-info">
                  <div class="rec-title-row">
                    <h5 class="rec-title">{{ rec.title }}</h5>
                    <span v-if="rec.isHighlight" class="highlight-fire-badge">
                      🔥 高潮高能
                    </span>
                    <span v-if="rec.score !== undefined && rec.score !== null" class="score-pill">
                      ⚡ 能量分: {{ (rec.score * 100).toFixed(0) }}
                    </span>
                  </div>
                  <div class="rec-meta">
                    <span class="time-range">
                      {{ msToTimeStr(rec.startMs) }} ~ {{ msToTimeStr(rec.endMs) }}
                    </span>
                    <span class="dur-badge">
                      {{ formatTime((rec.endMs - rec.startMs) / 1000.0) }}
                    </span>
                    <span class="reason-tag">{{ rec.reason }}</span>
                  </div>
                </div>
              </div>

              <div class="rec-actions">
                <button
                  class="btn-preview-rec"
                  title="跳转播放此片段"
                  @click="previewRecommendation(rec)"
                >
                  ▶ 预览
                </button>
                <button
                  class="btn-adopt-rec"
                  :title="'将此片段添加至编辑列表'"
                  @click="adoptSingleAiRecommendation(rec)"
                >
                  + 采纳
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-dialog-footer">
          <button class="btn-cancel" @click="showAiModal = false">
            取消
          </button>
          <button
            v-if="aiHighlights.length > 0"
            class="btn-climax-merge-quick"
            title="清空当前列表，仅载入全部高潮片段，并将导出模式自动切换为拼接合并"
            @click="applyOnlyHighlightsAndMerge"
          >
            🔥 仅采纳高潮并拼接成片 ({{ aiHighlights.length }})
          </button>
          <button class="btn-apply-all ai-glow" @click="applyAllAiRecommendations">
            ✨ 一键采纳当前Tab全部分段
          </button>
        </div>
      </div>
    </div>

    <!-- Video Picker Modal -->
    <div
      v-if="showPickerModal"
      class="modal-backdrop-layer"
      @click.self="showPickerModal = false"
    >
      <div class="video-picker-dialog">
        <div class="modal-dialog-header">
          <div class="dialog-title">
            <span>🎬</span>
            <h4>从媒体库选择视频</h4>
          </div>
          <button class="btn-close-dialog" @click="showPickerModal = false">✕</button>
        </div>

        <div class="picker-search-bar">
          <input
            v-model="pickerSearch"
            type="text"
            class="picker-input"
            placeholder="搜索视频标题、文件名或标签..."
          />
        </div>

        <div class="picker-videos-grid">
          <div v-if="isLibraryLoading" class="picker-loading">
            <div class="loading-spinner"></div>
            <span>正在加载媒体库...</span>
          </div>

          <div
            v-for="v in filteredPickerVideos"
            :key="v.id"
            class="picker-video-card"
            :class="{ current: activeVideo?.id === v.id }"
            @click="selectVideo(v)"
          >
            <div class="card-thumb">
              <img v-if="v.thumbnail_data" :src="v.thumbnail_data" alt="thumb" />
              <div v-else class="thumb-fallback">🎬</div>
              <span class="card-dur">{{ formatTime(v.duration || 0) }}</span>
            </div>
            <div class="card-info">
              <span class="card-name" :title="v.file_name">{{ v.file_name }}</span>
              <span class="card-sub">{{ formatBytes(v.file_size) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Exporting Progress Overlay -->
    <div v-if="isClipping" class="modal-backdrop-layer progress-mode">
      <div class="progress-card">
        <div class="ai-spinner"></div>
        <h4>正在执行视频截断与导出...</h4>
        <p>FFmpeg 正在全力处理您所指定的阶段切片，请稍候</p>
        <span class="elapsed-badge">已耗时: {{ clipElapsed }} 秒</span>
      </div>
    </div>

    <!-- Export Success Result Modal -->
    <div
      v-if="showResultModal && clipResult"
      class="modal-backdrop-layer"
      @click.self="showResultModal = false"
    >
      <div class="result-dialog">
        <div class="modal-dialog-header">
          <div class="dialog-title success">
            <span>🎉</span>
            <h4>裁剪导出已完成</h4>
          </div>
          <button class="btn-close-dialog" @click="showResultModal = false">✕</button>
        </div>

        <div class="modal-dialog-body">
          <p class="result-summary-text">
            已成功生成 <strong>{{ clipResult.clips.length }}</strong> 个切片片段
            <span v-if="clipResult.mergedPath">，以及 1 个合并成片</span>。
          </p>

          <!-- Merged File Card -->
          <div v-if="clipResult.mergedPath" class="merged-result-card">
            <div class="card-header-badge">🔗 拼接合并视频</div>
            <span class="file-path-text">{{ clipResult.mergedPath }}</span>
            <div class="file-action-buttons">
              <button class="btn-res-act" @click="revealFile(clipResult.mergedPath!)">
                📁 打开所在目录
              </button>
            </div>
          </div>

          <!-- Individual Clips List -->
          <div class="clips-result-list">
            <div
              v-for="(item, i) in clipResult.clips"
              :key="item.id"
              class="clip-result-row"
            >
              <div class="row-left">
                <span class="row-num">#{{ i + 1 }}</span>
                <div class="row-details">
                  <span class="row-title">{{ item.title }}</span>
                  <span class="row-meta">
                    {{ formatTime(item.durationSec) }} · {{ formatBytes(item.fileSize) }}
                  </span>
                </div>
              </div>
              <button class="btn-res-act" @click="revealFile(item.outputPath)">
                📁 定位
              </button>
            </div>
          </div>
        </div>

        <div class="modal-dialog-footer">
          <button class="btn-cancel" @click="showResultModal = false; emit('navigateClips')">
            📁 查看切片管理
          </button>
          <button class="btn-apply-all" @click="showResultModal = false">
            完成
          </button>
        </div>
      </div>
    </div>

    <!-- Toast Notification -->
    <transition name="fade">
      <div v-if="toastMessage" class="toast-chip-floating">
        {{ toastMessage }}
      </div>
    </transition>
  </div>
</template>

<style scoped>
.clipper-studio-container {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  padding: 0.85rem 1.1rem;
  height: 100%;
  box-sizing: border-box;
  background: #090d16;
  color: #f1f5f9;
  overflow-y: auto;
}

/* Studio Header */
.studio-header-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.6rem 0.95rem;
  background: #111827;
  border: 1px solid #1f293d;
  border-radius: 10px;
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.4);
  flex-shrink: 0;
}

.studio-header-left {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  min-width: 0;
  flex: 1;
}

.nav-pill-group {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  background: rgba(255, 255, 255, 0.04);
  padding: 3px;
  border-radius: 7px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.btn-nav-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.35rem 0.65rem;
  font-size: 0.78rem;
  font-weight: 600;
  color: #cbd5e1;
  background: transparent;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.btn-nav-pill:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.btn-nav-pill.secondary {
  color: #fb923c;
  background: rgba(251, 146, 60, 0.1);
}

.btn-nav-pill.secondary:hover {
  background: rgba(251, 146, 60, 0.2);
  color: #fff;
}

.btn-nav-pill .back-arrow {
  font-size: 0.95rem;
  line-height: 1;
}

.header-v-divider {
  width: 1px;
  height: 24px;
  background: rgba(255, 255, 255, 0.12);
  flex-shrink: 0;
}

.active-video-identity {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
  flex: 1;
  overflow: hidden;
}

.video-thumb-preview {
  position: relative;
  width: 48px;
  height: 32px;
  border-radius: 5px;
  overflow: hidden;
  background: #000;
  cursor: pointer;
  flex-shrink: 0;
  border: 1px solid rgba(255, 255, 255, 0.2);
  transition: transform 0.15s ease, border-color 0.15s ease;
}

.video-thumb-preview:hover {
  transform: scale(1.05);
  border-color: #3b82f6;
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  font-size: 1rem;
}

.change-overlay-badge {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: rgba(0, 0, 0, 0.75);
  color: #cbd5e1;
  font-size: 0.6rem;
  text-align: center;
  line-height: 1.2;
  padding: 1px 0;
}

.video-meta-texts {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
  flex: 1;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.video-title {
  font-size: 0.88rem;
  font-weight: 700;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 280px;
}

.btn-switch-video {
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #94a3b8;
  font-size: 0.7rem;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: all 0.15s ease;
}

.btn-switch-video:hover {
  background: rgba(255, 255, 255, 0.18);
  color: #fff;
}

.meta-pills-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-wrap: nowrap;
  overflow: hidden;
}

.meta-pill {
  font-size: 0.68rem;
  color: #94a3b8;
  background: rgba(255, 255, 255, 0.05);
  padding: 1px 6px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  white-space: nowrap;
  flex-shrink: 0;
}

.studio-top-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
}

.btn-act {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.42rem 0.8rem;
  border-radius: 7px;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid transparent;
  white-space: nowrap;
}

.btn-act:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none !important;
}

.climax-btn {
  background: rgba(249, 115, 22, 0.14);
  border-color: rgba(249, 115, 22, 0.35);
  color: #fdba74;
}

.climax-btn:hover:not(:disabled) {
  background: rgba(249, 115, 22, 0.25);
  color: #fff;
  border-color: #f97316;
  box-shadow: 0 0 12px rgba(249, 115, 22, 0.35);
}

.climax-fire-icon {
  display: inline-block;
  font-size: 0.95rem;
}

.ai-btn {
  background: rgba(139, 92, 246, 0.14);
  border-color: rgba(139, 92, 246, 0.35);
  color: #c4b5fd;
}

.ai-btn:hover:not(:disabled) {
  background: rgba(139, 92, 246, 0.25);
  color: #fff;
  border-color: #8b5cf6;
  box-shadow: 0 0 12px rgba(139, 92, 246, 0.35);
}

.ai-sparkle-icon {
  display: inline-block;
  font-size: 0.95rem;
}

.export-cta-btn {
  background: linear-gradient(135deg, #10b981, #059669);
  border: none;
  color: #fff;
  padding: 0.42rem 0.95rem;
  box-shadow: 0 2px 10px rgba(16, 185, 129, 0.35);
}

.export-cta-btn:hover:not(:disabled) {
  opacity: 0.95;
  transform: translateY(-1px);
  box-shadow: 0 4px 16px rgba(16, 185, 129, 0.5);
}

/* Workspace Grid */
.studio-workspace-grid {
  display: grid;
  grid-template-columns: 1.35fr 1fr;
  gap: 0.85rem;
  flex: 1;
  min-height: 0;
}

/* Player Panel Card */
.player-panel-card {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  background: #111827;
  border: 1px solid #1f293d;
  border-radius: 10px;
  padding: 0.75rem;
  min-height: 0;
}

.video-viewport {
  position: relative;
  width: 100%;
  aspect-ratio: 16 / 9;
  background: #000;
  border-radius: 7px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: inset 0 0 16px rgba(0, 0, 0, 0.85);
  cursor: pointer;
}

.native-video-el {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.floating-time-hud {
  position: absolute;
  top: 8px;
  right: 8px;
  background: rgba(0, 0, 0, 0.75);
  padding: 2px 8px;
  border-radius: 5px;
  font-size: 0.75rem;
  color: #38bdf8;
  border: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(6px);
}

.time-divider {
  margin: 0 4px;
  color: #64748b;
}

/* Transport Toolbar */
.player-controls-strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  background: #0c121e;
  padding: 0.4rem 0.6rem;
  border-radius: 7px;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.left-transport-buttons,
.right-quick-markers {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.transport-btn,
.marker-shortcut-btn {
  padding: 0.35rem 0.6rem;
  border-radius: 5px;
  font-size: 0.76rem;
  font-weight: 500;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cbd5e1;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.transport-btn:hover,
.marker-shortcut-btn:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

.transport-btn.play-btn {
  background: rgba(59, 130, 246, 0.2);
  border-color: rgba(59, 130, 246, 0.4);
  color: #93c5fd;
  font-weight: 600;
}

.transport-btn.play-btn:hover {
  background: rgba(59, 130, 246, 0.35);
  color: #fff;
}

.marker-shortcut-btn.in-btn {
  color: #34d399;
  background: rgba(16, 185, 129, 0.12);
  border-color: rgba(16, 185, 129, 0.25);
}

.marker-shortcut-btn.in-btn:hover {
  background: rgba(16, 185, 129, 0.24);
  color: #fff;
}

.marker-shortcut-btn.split-btn {
  color: #fbbf24;
  background: rgba(245, 158, 11, 0.15);
  border-color: rgba(245, 158, 11, 0.35);
  font-weight: 600;
}

.marker-shortcut-btn.split-btn:hover {
  background: rgba(245, 158, 11, 0.3);
  color: #fff;
}

.marker-shortcut-btn.out-btn {
  color: #f472b6;
  background: rgba(236, 72, 153, 0.12);
  border-color: rgba(236, 72, 153, 0.25);
}

.marker-shortcut-btn.out-btn:hover {
  background: rgba(236, 72, 153, 0.24);
  color: #fff;
}

/* Visual Timeline */
.visual-timeline-section {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  margin-top: 0.15rem;
}

.timeline-labels-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.72rem;
}

.timeline-title-left {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  color: #cbd5e1;
  font-weight: 600;
}

.timeline-icon {
  font-size: 0.85rem;
}

.timeline-hint {
  font-weight: normal;
  color: #64748b;
  font-size: 0.68rem;
}

.timeline-counter {
  font-size: 0.7rem;
  color: #94a3b8;
}

.badge-seg-count {
  color: #38bdf8;
  font-weight: 700;
}

.timeline-track-container {
  position: relative;
  height: 38px;
  background: #080c14;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  cursor: pointer;
  overflow: hidden;
}

.timeline-base-rail {
  width: 100%;
  height: 100%;
  background: repeating-linear-gradient(
    90deg,
    transparent,
    transparent 19px,
    rgba(255, 255, 255, 0.04) 20px
  );
}

.timeline-segment-block {
  position: absolute;
  top: 3px;
  bottom: 3px;
  border-radius: 4px;
  opacity: 0.85;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  padding: 0 6px;
  overflow: hidden;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.25);
}

.timeline-segment-block:hover {
  opacity: 1;
  filter: brightness(1.15);
}

.timeline-segment-block.active {
  opacity: 1;
  box-shadow: 0 0 0 2px #fff, inset 0 0 0 1px rgba(0, 0, 0, 0.6);
  z-index: 2;
}

.block-title-text {
  font-size: 0.68rem;
  font-weight: 700;
  color: #fff;
  white-space: nowrap;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9);
}

.timeline-playhead-line {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: #ef4444;
  pointer-events: none;
  z-index: 5;
}

.playhead-thumb {
  position: absolute;
  top: 0;
  left: -5px;
  width: 12px;
  height: 10px;
  background: #ef4444;
  clip-path: polygon(0% 0%, 100% 0%, 50% 100%);
}

.timeline-ticks-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.68rem;
  color: #64748b;
  padding: 0 2px;
}

/* Right Column: Segments Editor Panel */
.segments-editor-panel {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  background: #111827;
  border: 1px solid #1f293d;
  border-radius: 10px;
  padding: 0.85rem;
  min-height: 0;
}

.panel-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-title-group {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.card-title-group h4 {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 700;
  color: #fff;
}

.badge-count {
  font-size: 0.7rem;
  font-weight: 700;
  background: rgba(59, 130, 246, 0.18);
  color: #60a5fa;
  padding: 1px 6px;
  border-radius: 10px;
}

.card-header-actions {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.btn-xs-primary {
  padding: 0.25rem 0.65rem;
  border-radius: 5px;
  font-size: 0.74rem;
  font-weight: 600;
  background: #2563eb;
  border: none;
  color: #fff;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-xs-primary:hover {
  background: #3b82f6;
}

.btn-xs-danger {
  padding: 0.25rem 0.55rem;
  border-radius: 5px;
  font-size: 0.72rem;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: #fca5a5;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-xs-danger:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #fff;
}

/* Segments List Scroll */
.segments-list-scroll {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  flex: 1;
  overflow-y: auto;
  min-height: 160px;
  padding-right: 2px;
}

.empty-segments-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2.2rem 1rem;
  text-align: center;
  color: #64748b;
}

.empty-icon {
  font-size: 2rem;
}

.empty-segments-state h5 {
  margin: 0;
  color: #cbd5e1;
  font-size: 0.9rem;
}

.empty-segments-state p {
  margin: 0;
  font-size: 0.74rem;
  color: #64748b;
  max-width: 260px;
  line-height: 1.4;
}

.empty-action-btns {
  display: flex;
  gap: 0.45rem;
  margin-top: 0.4rem;
}

.btn-empty-act {
  padding: 0.35rem 0.65rem;
  border-radius: 5px;
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s ease;
}

.btn-empty-act.climax {
  background: rgba(249, 115, 22, 0.15);
  color: #fdba74;
  border-color: rgba(249, 115, 22, 0.3);
}

.btn-empty-act.ai {
  background: rgba(139, 92, 246, 0.15);
  color: #c4b5fd;
  border-color: rgba(139, 92, 246, 0.3);
}

.btn-empty-act.manual {
  background: rgba(255, 255, 255, 0.08);
  color: #e2e8f0;
  border-color: rgba(255, 255, 255, 0.15);
}

/* Segment Card Item */
.segment-item-card {
  position: relative;
  display: flex;
  align-items: stretch;
  background: #0d131f;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 7px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.15s ease;
}

.segment-item-card:hover {
  background: #121a2b;
  border-color: rgba(255, 255, 255, 0.16);
}

.segment-item-card.active {
  background: #141e33;
  border-color: var(--seg-accent, #3b82f6);
  box-shadow: 0 0 10px rgba(59, 130, 246, 0.2);
}

.card-accent-bar {
  width: 4px;
  flex-shrink: 0;
}

.item-content-body {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.5rem 0.65rem;
  flex: 1;
  min-width: 0;
}

.item-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.title-left {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex: 1;
  min-width: 0;
}

.item-index-pill {
  font-size: 0.68rem;
  font-weight: 700;
  color: #fff;
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}

.segment-title-input {
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 600;
  padding: 1px 4px;
  outline: none;
  flex: 1;
  min-width: 0;
  transition: border-color 0.15s ease;
}

.segment-title-input:focus {
  background: rgba(0, 0, 0, 0.3);
  border-color: #3b82f6;
}

.title-right {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-shrink: 0;
}

.duration-tag {
  font-size: 0.72rem;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.1);
  padding: 1px 5px;
  border-radius: 3px;
  font-weight: 600;
}

.btn-item-act {
  width: 22px;
  height: 22px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.05);
  color: #cbd5e1;
  font-size: 0.7rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.btn-item-act:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.btn-item-act.delete:hover {
  background: rgba(239, 68, 68, 0.25);
  border-color: rgba(239, 68, 68, 0.5);
  color: #fca5a5;
}

.item-timecode-grid {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.time-arrow {
  font-size: 0.75rem;
  color: #64748b;
}

.time-picker-box {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  background: #080c14;
  padding: 2px 5px;
  border-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  flex: 1;
}

.time-picker-box .lbl {
  font-size: 0.68rem;
  color: #64748b;
  white-space: nowrap;
}

.time-input {
  flex: 1;
  width: 72px;
  background: transparent;
  border: none;
  color: #e2e8f0;
  font-size: 0.75rem;
  outline: none;
}

.btn-stamp-now {
  font-size: 0.65rem;
  background: rgba(255, 255, 255, 0.08);
  border: none;
  border-radius: 3px;
  color: #94a3b8;
  padding: 1px 4px;
  cursor: pointer;
  white-space: nowrap;
}

.btn-stamp-now:hover {
  background: rgba(59, 130, 246, 0.3);
  color: #93c5fd;
}

/* Compact Export Settings Dock */
.export-settings-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: 0.6rem 0.75rem;
  background: #0c121e;
  border-radius: 7px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.export-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.export-title {
  font-size: 0.78rem;
  font-weight: 700;
  color: #cbd5e1;
}

.toggle-control-label {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.72rem;
  color: #34d399;
  cursor: pointer;
}

.export-mode-segmented {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  background: rgba(0, 0, 0, 0.35);
  padding: 2px;
  border-radius: 6px;
  gap: 2px;
}

.segmented-btn {
  padding: 0.35rem 0.5rem;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #94a3b8;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.3rem;
}

.segmented-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.06);
}

.segmented-btn.active {
  background: rgba(59, 130, 246, 0.25);
  color: #60a5fa;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
}


/* Modal Backdrop */
.modal-backdrop-layer {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1.5rem;
}

/* Dialog Container */
.ai-recommend-dialog,
.video-picker-dialog,
.result-dialog {
  background: #161b22;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 12px;
  width: 100%;
  max-width: 640px;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.5);
}

.modal-dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.dialog-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.dialog-title h4 {
  margin: 0;
  font-size: 1rem;
  color: #fff;
}

.btn-close-dialog {
  background: transparent;
  border: none;
  color: #94a3b8;
  font-size: 1.1rem;
  cursor: pointer;
}

.btn-close-dialog:hover {
  color: #fff;
}

.modal-dialog-body {
  padding: 1rem 1.25rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.modal-dialog-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0.85rem 1.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.2);
}

.btn-cancel {
  padding: 0.5rem 1rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.85rem;
  cursor: pointer;
}

.btn-apply-all {
  padding: 0.5rem 1.25rem;
  border-radius: 6px;
  background: linear-gradient(135deg, #10b981, #059669);
  border: none;
  color: #fff;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
}

/* AI Recommendations & Climax Tabs */
.ai-tabs-nav-bar {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 1.25rem 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.15);
}

.ai-tab-btn {
  padding: 0.55rem 1rem;
  border: none;
  background: transparent;
  color: #94a3b8;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  position: relative;
  transition: all 0.2s ease;
  border-radius: 6px 6px 0 0;
}

.ai-tab-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.04);
}

.ai-tab-btn.active {
  color: #fb923c;
  background: rgba(251, 146, 60, 0.12);
  border-bottom: 2px solid #f97316;
}

.ai-strategy-chip {
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.12), rgba(59, 130, 246, 0.08));
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 8px;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.ai-strategy-chip.climax-style {
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.15), rgba(245, 158, 11, 0.1));
  border-color: rgba(239, 68, 68, 0.35);
}

.strategy-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.strategy-badge {
  font-size: 0.8rem;
  font-weight: 700;
  color: #c084fc;
}

.ai-strategy-chip.climax-style .strategy-badge {
  color: #fb923c;
}

.climax-hint-badge {
  font-size: 0.7rem;
  color: #fca5a5;
  background: rgba(239, 68, 68, 0.2);
  padding: 2px 6px;
  border-radius: 4px;
}

.strategy-message {
  margin: 0;
  font-size: 0.75rem;
  color: #94a3b8;
  line-height: 1.4;
}

.empty-recs-tip {
  text-align: center;
  padding: 2rem 1rem;
  color: #94a3b8;
  font-size: 0.85rem;
}

.recommendations-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.recommendation-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 0.65rem 0.85rem;
  transition: all 0.15s ease;
}

.recommendation-item.is-climax-item {
  border-color: rgba(249, 115, 22, 0.35);
  background: linear-gradient(135deg, rgba(249, 115, 22, 0.06), rgba(255, 255, 255, 0.02));
}

.recommendation-item:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.18);
}

.rec-left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}

.rec-idx {
  font-size: 0.8rem;
  font-weight: 700;
  color: #818cf8;
}

.recommendation-item.is-climax-item .rec-idx {
  color: #f97316;
}

.rec-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.rec-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.rec-title {
  margin: 0;
  font-size: 0.85rem;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 320px;
}

.highlight-fire-badge {
  font-size: 0.65rem;
  font-weight: 700;
  color: #fff;
  background: linear-gradient(135deg, #ef4444, #f97316);
  padding: 1px 6px;
  border-radius: 4px;
  box-shadow: 0 2px 6px rgba(239, 68, 68, 0.3);
}

.score-pill {
  font-size: 0.68rem;
  font-weight: 600;
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.15);
  padding: 1px 5px;
  border-radius: 4px;
}

.rec-meta {
  display: flex;
  gap: 0.4rem;
  align-items: center;
  flex-wrap: wrap;
}

.time-range {
  font-family: monospace;
  font-size: 0.72rem;
  color: #38bdf8;
}

.dur-badge {
  font-size: 0.68rem;
  background: rgba(255, 255, 255, 0.06);
  padding: 1px 4px;
  border-radius: 3px;
  color: #cbd5e1;
}

.reason-tag {
  font-size: 0.68rem;
  color: #c084fc;
}

.recommendation-item.is-climax-item .reason-tag {
  color: #fb923c;
}

.rec-actions {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-shrink: 0;
}

.btn-preview-rec {
  padding: 0.35rem 0.65rem;
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #cbd5e1;
  font-size: 0.72rem;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-preview-rec:hover {
  background: rgba(255, 255, 255, 0.18);
  color: #fff;
}

.btn-adopt-rec {
  padding: 0.35rem 0.75rem;
  border-radius: 5px;
  background: rgba(99, 102, 241, 0.2);
  border: 1px solid rgba(99, 102, 241, 0.4);
  color: #c7d2fe;
  font-size: 0.75rem;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.2s;
}

.btn-adopt-rec:hover {
  background: rgba(99, 102, 241, 0.35);
  color: #fff;
}

.btn-climax-merge-quick {
  padding: 0.5rem 1.1rem;
  border-radius: 6px;
  background: linear-gradient(135deg, #ef4444, #f97316);
  border: none;
  color: #fff;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 3px 10px rgba(239, 68, 68, 0.3);
  transition: all 0.2s;
}

.btn-climax-merge-quick:hover {
  transform: translateY(-1px);
  box-shadow: 0 5px 14px rgba(239, 68, 68, 0.5);
}

/* Video Picker Modal */
.picker-search-bar {
  padding: 0.75rem 1.25rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.picker-input {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.85rem;
  box-sizing: border-box;
}

.picker-videos-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 0.75rem;
  padding: 1rem 1.25rem;
  overflow-y: auto;
  max-height: 480px;
}

.picker-video-card {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.15s ease;
}

.picker-video-card:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(99, 102, 241, 0.5);
  transform: translateY(-2px);
}

.picker-video-card.current {
  border-color: #10b981;
  background: rgba(16, 185, 129, 0.1);
}

.card-thumb {
  position: relative;
  width: 100%;
  aspect-ratio: 16 / 9;
  background: #000;
}

.card-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.card-dur {
  position: absolute;
  bottom: 4px;
  right: 4px;
  background: rgba(0, 0, 0, 0.75);
  font-size: 0.65rem;
  padding: 1px 4px;
  border-radius: 3px;
  color: #fff;
}

.card-info {
  padding: 0 0.5rem 0.5rem 0.5rem;
  display: flex;
  flex-direction: column;
}

.card-name {
  font-size: 0.75rem;
  font-weight: 600;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-sub {
  font-size: 0.68rem;
  color: #94a3b8;
}

/* Progress Overlay */
.progress-card {
  background: #161b22;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 12px;
  padding: 2rem;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  max-width: 380px;
}

.ai-spinner {
  width: 42px;
  height: 42px;
  border: 3px solid rgba(99, 102, 241, 0.2);
  border-top-color: #6366f1;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.elapsed-badge {
  font-family: monospace;
  font-size: 0.8rem;
  color: #38bdf8;
  background: rgba(56, 189, 248, 0.1);
  padding: 3px 8px;
  border-radius: 4px;
}

/* Result Dialog */
.merged-result-card {
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.15), rgba(5, 150, 105, 0.1));
  border: 1px solid rgba(16, 185, 129, 0.35);
  border-radius: 8px;
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.card-header-badge {
  font-size: 0.8rem;
  font-weight: 700;
  color: #34d399;
}

.file-path-text {
  font-family: monospace;
  font-size: 0.72rem;
  color: #e2e8f0;
  word-break: break-all;
}

.clips-result-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  max-height: 240px;
  overflow-y: auto;
}

.clip-result-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
}

.row-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.row-num {
  font-size: 0.75rem;
  font-weight: 700;
  color: #6366f1;
}

.row-details {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.row-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row-meta {
  font-size: 0.68rem;
  color: #94a3b8;
}

.btn-res-act {
  padding: 3px 8px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #f1f5f9;
  font-size: 0.72rem;
  cursor: pointer;
  flex-shrink: 0;
}

.btn-res-act:hover {
  background: rgba(255, 255, 255, 0.18);
}

/* Floating Toast */
.toast-chip-floating {
  position: fixed;
  bottom: 2rem;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(99, 102, 241, 0.5);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  padding: 0.65rem 1.4rem;
  border-radius: 20px;
  font-size: 0.85rem;
  color: #fff;
  z-index: 2000;
  pointer-events: none;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translate(-50%, 10px);
}
</style>
