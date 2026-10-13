<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import SubtitleTranscriptPanel from './SubtitleTranscriptPanel.vue';
import type { Video, SubtitleSegment, VideoCompatibilityStatus, TranscodeResult } from '../types';

const props = defineProps<{
  video: Video | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const videoRef = ref<HTMLVideoElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);

const isPlaying = ref(false);
const currentTime = ref(0);
const duration = ref(0);
const isScrubbing = ref(false);
const scrubTime = ref(0);
const modalOffset = ref({ x: 0, y: 0 });
const isDraggingModal = ref(false);
let dragOrigin = { clientX: 0, clientY: 0, startX: 0, startY: 0 };

const volume = ref(1);
const isMuted = ref(false);
const playbackRate = ref(1);
const isLooping = ref(false);
const isFullscreen = ref(false);
const isLoading = ref(true);
const errorMessage = ref('');
const showBigPlayIcon = ref(false);
const copiedPath = ref(false);

const speedOptions = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
const showSpeedMenu = ref(false);

// Subtitle & Interactive Transcript state
const showTranscript = ref(false);
const showCc = ref(true);
const videoSubtitles = ref<SubtitleSegment[]>([]);

const activeSubtitleText = computed(() => {
  const currentMs = Math.round(currentTime.value * 1000);
  const seg = videoSubtitles.value.find(
    (s) => currentMs >= s.start_time_ms && currentMs <= s.end_time_ms
  );
  return seg ? seg.content : '';
});

async function fetchSubtitles() {
  if (!props.video?.id) {
    videoSubtitles.value = [];
    return;
  }
  try {
    const list = await invoke<SubtitleSegment[]>('get_video_subtitles', {
      videoId: props.video.id,
    });
    videoSubtitles.value = list || [];
  } catch (err) {
    console.warn('Failed to fetch subtitles in modal:', err);
    videoSubtitles.value = [];
  }
}

function onSubtitlesUpdated(segs: SubtitleSegment[]) {
  videoSubtitles.value = segs;
}

function onTranscriptSeek(sec: number) {
  if (videoRef.value) {
    videoRef.value.currentTime = sec;
    currentTime.value = sec;
    videoRef.value.play().catch(() => {});
    isPlaying.value = true;
  }
}

const compatibleFilePath = ref<string | null>(null);
const isConverting = ref(false);
const convertProgressText = ref('');
const isCompatibleFormat = ref(true);

const videoSrc = computed(() => {
  if (compatibleFilePath.value) {
    return convertFileSrc(compatibleFilePath.value);
  }
  if (!props.video?.file_path) return '';
  return convertFileSrc(props.video.file_path);
});

async function checkCompatibility() {
  compatibleFilePath.value = null;
  isConverting.value = false;
  if (!props.video?.file_path) return;

  try {
    const status = await invoke<VideoCompatibilityStatus>('check_video_compatibility', {
      filePath: props.video.file_path,
      fileHash: props.video.file_hash || null,
    });
    if (status.has_cached_transcode && status.cached_path) {
      compatibleFilePath.value = status.cached_path;
      isCompatibleFormat.value = true;
      errorMessage.value = '';
    } else if (!status.is_compatible) {
      isCompatibleFormat.value = false;
      errorMessage.value = status.reason || '当前视频为传统封装格式（如 AVI），需一键优化以在内嵌播放器中流畅播放。';
      isLoading.value = false;
    } else {
      isCompatibleFormat.value = true;
    }
  } catch (err) {
    console.warn('check_video_compatibility error:', err);
  }
}

async function startCompatibilityConversion() {
  if (!props.video?.file_path || isConverting.value) return;
  isConverting.value = true;
  convertProgressText.value = '正在进行极速兼容优化 (检测编码与硬件加速)...';
  errorMessage.value = '';

  try {
    const res = await invoke<TranscodeResult>('convert_video_to_compatible', {
      filePath: props.video.file_path,
      fileHash: props.video.file_hash || null,
    });
    compatibleFilePath.value = res.compatible_path;
    isCompatibleFormat.value = true;
    errorMessage.value = '';
    if (videoRef.value) {
      videoRef.value.load();
      videoRef.value.play().catch(() => {});
      isPlaying.value = true;
    }
  } catch (err: any) {
    console.error('convert_video_to_compatible error:', err);
    errorMessage.value = `转换兼容格式失败: ${err?.message || err}`;
  } finally {
    isConverting.value = false;
  }
}

function formatTime(seconds: number): string {
  if (isNaN(seconds) || seconds < 0) return '00:00';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const mm = m.toString().padStart(2, '0');
  const ss = s.toString().padStart(2, '0');
  if (h > 0) {
    return `${h}:${mm}:${ss}`;
  }
  return `${mm}:${ss}`;
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

function togglePlay() {
  const el = videoRef.value;
  if (!el) return;
  if (el.paused) {
    el.play().catch((err) => {
      console.warn('Autoplay prevented or playback error:', err);
    });
    isPlaying.value = true;
  } else {
    el.pause();
    isPlaying.value = false;
  }
  triggerPlayIndicator();
}

function triggerPlayIndicator() {
  showBigPlayIcon.value = true;
  setTimeout(() => {
    showBigPlayIcon.value = false;
  }, 500);
}

function handleTimeUpdate() {
  if (!videoRef.value || isScrubbing.value) return;
  currentTime.value = videoRef.value.currentTime;
}

function handleLoadedMetadata() {
  if (!videoRef.value) return;
  duration.value = videoRef.value.duration || 0;
  isLoading.value = false;
  errorMessage.value = '';
}

function handleWaiting() {
  isLoading.value = true;
}

function handleCanPlay() {
  isLoading.value = false;
}

function handleError(e: Event) {
  isLoading.value = false;
  const target = e.target as HTMLVideoElement;
  const mediaError = target?.error;
  let desc = '无法解析此视频格式';
  if (mediaError) {
    switch (mediaError.code) {
      case 1:
        desc = '视频流读取被终止';
        break;
      case 2:
        desc = '本地媒体流读取失败（可能权限受限或路径变动）';
        break;
      case 3:
        desc = '视频解码失败，可能存在损坏帧或索引丢失';
        isCompatibleFormat.value = false;
        break;
      case 4:
        desc = '内嵌渲染器不支持此格式或音视频编码（如 AVI、MKV、FLV、AC3 环绕声）。可一键极速转换为兼容 MP4。';
        isCompatibleFormat.value = false;
        break;
      default:
        desc = mediaError.message || '未知媒体解码异常';
        isCompatibleFormat.value = false;
    }
  }
  errorMessage.value = desc;
}

function onSeekStart(e: Event) {
  isScrubbing.value = true;
  const target = e.target as HTMLInputElement;
  scrubTime.value = parseFloat(target.value);
}

function onSeekInput(e: Event) {
  const target = e.target as HTMLInputElement;
  scrubTime.value = parseFloat(target.value);
}

function onSeekChange(e: Event) {
  const target = e.target as HTMLInputElement;
  const val = parseFloat(target.value);
  if (videoRef.value && !isNaN(val)) {
    videoRef.value.currentTime = val;
    currentTime.value = val;
  }
  isScrubbing.value = false;
}

function seekRelative(delta: number) {
  if (!videoRef.value) return;
  const newTime = Math.max(0, Math.min(duration.value, videoRef.value.currentTime + delta));
  videoRef.value.currentTime = newTime;
  currentTime.value = newTime;
}

function toggleMute() {
  if (!videoRef.value) return;
  isMuted.value = !isMuted.value;
  videoRef.value.muted = isMuted.value;
}

function onVolumeInput(e: Event) {
  const target = e.target as HTMLInputElement;
  const val = parseFloat(target.value);
  volume.value = val;
  if (videoRef.value) {
    videoRef.value.volume = val;
    isMuted.value = val === 0;
    videoRef.value.muted = isMuted.value;
  }
}

function adjustVolume(delta: number) {
  const newVol = Math.max(0, Math.min(1, volume.value + delta));
  volume.value = newVol;
  if (videoRef.value) {
    videoRef.value.volume = newVol;
    isMuted.value = newVol === 0;
    videoRef.value.muted = isMuted.value;
  }
}

function setPlaybackRate(rate: number) {
  playbackRate.value = rate;
  if (videoRef.value) {
    videoRef.value.playbackRate = rate;
  }
  showSpeedMenu.value = false;
}

function toggleLoop() {
  isLooping.value = !isLooping.value;
  if (videoRef.value) {
    videoRef.value.loop = isLooping.value;
  }
}

async function togglePiP() {
  if (!videoRef.value) return;
  try {
    if (document.pictureInPictureElement) {
      await document.exitPictureInPicture();
    } else if (document.pictureInPictureEnabled) {
      await videoRef.value.requestPictureInPicture();
    }
  } catch (err) {
    console.warn('PiP error:', err);
  }
}

async function toggleFullscreen() {
  if (!containerRef.value) return;
  try {
    if (!document.fullscreenElement) {
      await containerRef.value.requestFullscreen();
      isFullscreen.value = true;
    } else {
      await document.exitFullscreen();
      isFullscreen.value = false;
    }
  } catch (err) {
    console.warn('Fullscreen toggle failed:', err);
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (!props.visible) return;
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

  switch (e.code) {
    case 'Space':
      e.preventDefault();
      togglePlay();
      break;
    case 'ArrowLeft':
      e.preventDefault();
      seekRelative(-5);
      break;
    case 'ArrowRight':
      e.preventDefault();
      seekRelative(5);
      break;
    case 'ArrowUp':
      e.preventDefault();
      adjustVolume(0.1);
      break;
    case 'ArrowDown':
      e.preventDefault();
      adjustVolume(-0.1);
      break;
    case 'KeyM':
      e.preventDefault();
      toggleMute();
      break;
    case 'KeyC':
      e.preventDefault();
      showCc.value = !showCc.value;
      break;
    case 'KeyT':
      e.preventDefault();
      showTranscript.value = !showTranscript.value;
      break;
    case 'KeyF':
      e.preventDefault();
      toggleFullscreen();
      break;
    case 'Escape':
      e.preventDefault();
      if (document.fullscreenElement) {
        document.exitFullscreen();
      } else {
        close();
      }
      break;
  }
}

function close() {
  if (videoRef.value) {
    videoRef.value.pause();
  }
  emit('close');
}

async function openExternal() {
  if (!props.video?.file_path) return;
  try {
    await openPath(props.video.file_path);
  } catch (err) {
    console.error('Failed to open externally:', err);
  }
}

async function revealFile() {
  if (!props.video?.file_path) return;
  try {
    await revealItemInDir(props.video.file_path);
  } catch (err) {
    console.error('Failed to reveal file:', err);
  }
}

async function copyPath() {
  if (!props.video?.file_path) return;
  try {
    await navigator.clipboard.writeText(props.video.file_path);
    copiedPath.value = true;
    setTimeout(() => {
      copiedPath.value = false;
    }, 1800);
  } catch (err) {
    console.error('Failed to copy path:', err);
  }
}

async function openInStandaloneWindow() {
  if (!props.video) return;
  try {
    await invoke('open_video_player_window', { video: props.video });
    close();
  } catch (err) {
    console.error('Failed to open standalone window:', err);
  }
}

function startHeaderDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest('button') || target.closest('input') || target.closest('a')) {
    return;
  }
  if (isFullscreen.value) return;

  isDraggingModal.value = true;
  dragOrigin = {
    clientX: e.clientX,
    clientY: e.clientY,
    startX: modalOffset.value.x,
    startY: modalOffset.value.y,
  };
  window.addEventListener('mousemove', onHeaderDragMove);
  window.addEventListener('mouseup', onHeaderDragEnd);
}

function onHeaderDragMove(e: MouseEvent) {
  if (!isDraggingModal.value) return;
  const dx = e.clientX - dragOrigin.clientX;
  const dy = e.clientY - dragOrigin.clientY;
  modalOffset.value = {
    x: dragOrigin.startX + dx,
    y: dragOrigin.startY + dy,
  };
}

function onHeaderDragEnd() {
  if (isDraggingModal.value) {
    isDraggingModal.value = false;
    window.removeEventListener('mousemove', onHeaderDragMove);
    window.removeEventListener('mouseup', onHeaderDragEnd);
  }
}

function resetModalPosition() {
  modalOffset.value = { x: 0, y: 0 };
}

watch(
  () => props.visible,
  (val) => {
    if (val) {
      isLoading.value = true;
      errorMessage.value = '';
      currentTime.value = 0;
      resetModalPosition();
      fetchSubtitles();
      checkCompatibility();
      setTimeout(() => {
        if (videoRef.value && isCompatibleFormat.value) {
          videoRef.value.play().catch(() => {});
          isPlaying.value = true;
        }
      }, 150);
    } else {
      if (videoRef.value) {
        videoRef.value.pause();
        isPlaying.value = false;
      }
    }
  }
);

watch(
  () => props.video?.id,
  () => {
    if (props.visible) {
      fetchSubtitles();
      checkCompatibility();
    }
  }
);

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
  document.addEventListener('fullscreenchange', () => {
    isFullscreen.value = !!document.fullscreenElement;
  });
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
  window.removeEventListener('mousemove', onHeaderDragMove);
  window.removeEventListener('mouseup', onHeaderDragEnd);
});
</script>

<template>
  <div v-if="visible" class="player-modal-backdrop" @click.self="close">
    <div
      ref="containerRef"
      class="player-modal-container"
      :class="{ 'is-dragging': isDraggingModal, 'has-drawer': showTranscript }"
      :style="{
        transform: isFullscreen ? 'none' : `translate3d(${modalOffset.x}px, ${modalOffset.y}px, 0)`
      }"
    >
      <!-- Modal Header -->
      <div
        class="player-modal-header"
        :class="{ 'is-dragging-header': isDraggingModal }"
        title="按住标题栏拖动窗口位置，双击还原居中"
        @mousedown="startHeaderDrag"
        @dblclick="resetModalPosition"
      >
        <div class="modal-title-box">
          <span class="drag-handle-hint" title="按住拖动窗口">⠿</span>
          <span class="player-icon">🎬</span>
          <h3 class="player-title" :title="video?.file_name || '播放视频'">
            {{ video?.file_name || '未知视频' }}
          </h3>
          <span v-if="video?.width && video?.height" class="badge-pill badge-res">
            {{ video.width >= 3840 ? '4K UHD' : video.width >= 1920 ? '1080P' : `${video.width}x${video.height}` }}
          </span>
          <span v-if="video?.codec" class="badge-pill badge-codec">
            {{ video.codec.toUpperCase() }}
          </span>
          <span v-if="videoSubtitles.length > 0" class="badge-pill badge-sub-ready" title="AI 语音字幕已就绪">
            CC · {{ videoSubtitles.length }} 句
          </span>
        </div>

        <div class="modal-header-actions">
          <button
            class="head-act-btn"
            :class="{ active: showTranscript }"
            title="开启/收起 AI 台词同步速览 (快捷键 T)"
            @click="showTranscript = !showTranscript"
          >
            <span>📝</span>
            <span>{{ showTranscript ? '收起台词' : 'AI 台词' }}</span>
          </button>
          <button class="head-act-btn" title="弹出为独立 Webview 桌面窗口 (可跨屏幕拖拽/置顶)" @click="openInStandaloneWindow">
            <svg class="h-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5v-4m0 4h-4m4 0l-5-5" />
            </svg>
            <span>独立窗口</span>
          </button>
          <button class="head-act-btn" title="在系统播放器中打开" @click="openExternal">
            <svg class="h-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
            </svg>
            <span>系统播放器</span>
          </button>
          <button class="head-act-btn" title="画中画模式 (PiP)" @click="togglePiP">
            <svg class="h-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5v-4m0 4h-4m4 0l-5-5" />
            </svg>
          </button>
          <button class="btn-close-modal" title="关闭播放器 (Esc)" @click="close">
            ✕
          </button>
        </div>
      </div>

      <!-- Player Body Layout: Main Stage + Optional Transcript Drawer -->
      <div class="player-body-layout">
        <div class="player-stage-main">
          <!-- Main Video Screen -->
          <div class="player-viewport" @click="togglePlay">
            <video
              ref="videoRef"
              class="video-element"
              :src="videoSrc"
              playsinline
              crossorigin="anonymous"
              preload="auto"
              @timeupdate="handleTimeUpdate"
              @loadedmetadata="handleLoadedMetadata"
              @waiting="handleWaiting"
              @canplay="handleCanPlay"
              @error="handleError"
              @ended="isPlaying = false"
            ></video>

            <!-- Floating On-Screen CC Subtitles -->
            <transition name="fade">
              <div v-if="showCc && activeSubtitleText" class="video-cc-overlay">
                <div class="video-cc-text">
                  {{ activeSubtitleText }}
                </div>
              </div>
            </transition>

            <!-- Loading spinner -->
            <div v-if="isLoading" class="player-center-overlay">
              <div class="spinner-player"></div>
              <span class="loading-label">正在缓冲视频数据...</span>
            </div>

            <!-- Big Play Indicator Anim -->
            <transition name="pop">
              <div v-if="showBigPlayIcon" class="big-indicator-badge">
                <span v-if="isPlaying">▶</span>
                <span v-else>❚❚</span>
              </div>
            </transition>

            <!-- Error / Compatibility Banner -->
            <div v-if="errorMessage" class="player-error-overlay">
              <div class="error-box">
                <span class="err-icon">{{ isConverting ? '⏳' : (isCompatibleFormat ? '⚠️' : '⚡') }}</span>
                <h4 class="err-title">{{ isConverting ? '正在优化视频兼容性' : (isCompatibleFormat ? '视频播放异常' : '需要兼容优化以支持流畅播放') }}</h4>
                <p class="err-text">{{ errorMessage }}</p>

                <!-- In-progress state -->
                <div v-if="isConverting" class="converting-state-bar">
                  <div class="spinner-player mini"></div>
                  <span class="converting-desc">{{ convertProgressText }}</span>
                </div>

                <!-- Action buttons -->
                <div v-else class="err-actions">
                  <button
                    v-if="!isCompatibleFormat"
                    class="btn-err-fallback highlight-transcode"
                    @click.stop="startCompatibilityConversion"
                  >
                    <span class="btn-ic">⚡</span> 极速优化为兼容格式 (保留高清原画)
                  </button>
                  <button class="btn-err-fallback primary" @click.stop="openExternal">
                    <span class="btn-ic">🚀</span> 使用系统播放器打开 (IINA/QuickTime)
                  </button>
                  <button class="btn-err-fallback secondary" @click.stop="revealFile">
                    <span class="btn-ic">📁</span> 定位文件所在目录
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Controls Bar -->
          <div class="player-controls-bar" @click.stop>
            <!-- Scrubber Progress Bar -->
            <div class="scrubber-wrapper">
              <input
                type="range"
                class="scrubber-slider"
                min="0"
                :max="duration || 100"
                step="0.1"
                :value="isScrubbing ? scrubTime : currentTime"
                @pointerdown="onSeekStart"
                @mousedown="onSeekStart"
                @input="onSeekInput"
                @change="onSeekChange"
                @pointerup="onSeekChange"
                @mouseup="onSeekChange"
              />
              <div
                class="scrubber-progress-fill"
                :style="{ width: duration ? `${((isScrubbing ? scrubTime : currentTime) / duration) * 100}%` : '0%' }"
              ></div>
            </div>

            <!-- Buttons Row -->
            <div class="controls-row">
              <!-- Left Controls -->
              <div class="ctrl-group ctrl-left">
                <button class="ctrl-btn play-pause-btn" :title="isPlaying ? '暂停 (Space)' : '播放 (Space)'" @click="togglePlay">
                  <span v-if="isPlaying">❚❚</span>
                  <span v-else>▶</span>
                </button>

                <button class="ctrl-btn" title="后退 5 秒 (←)" @click="seekRelative(-5)">
                  ↺ 5s
                </button>
                <button class="ctrl-btn" title="快进 5 秒 (→)" @click="seekRelative(5)">
                  5s ↻
                </button>

                <div class="time-readout">
                  <span class="t-cur">{{ formatTime(isScrubbing ? scrubTime : currentTime) }}</span>
                  <span class="t-sep">/</span>
                  <span class="t-dur">{{ formatTime(duration) }}</span>
                </div>
              </div>

              <!-- Right Controls -->
              <div class="ctrl-group ctrl-right">
                <!-- CC Subtitles Toggle -->
                <button
                  class="ctrl-btn"
                  :class="{ 'ctrl-active': showCc }"
                  :title="showCc ? '隐藏画面字幕 (CC / C 键)' : '开启画面悬浮字幕 (CC / C 键)'"
                  @click="showCc = !showCc"
                >
                  <span class="cc-badge-text">CC</span>
                </button>

                <!-- Transcript Drawer Toggle -->
                <button
                  class="ctrl-btn transcript-toggle-btn"
                  :class="{ 'ctrl-active': showTranscript }"
                  :title="showTranscript ? '收起 AI 台词抽屉 (T 键)' : '展开 AI 台词同步速览抽屉 (T 键)'"
                  @click="showTranscript = !showTranscript"
                >
                  <span>📝 台词</span>
                  <span v-if="videoSubtitles.length > 0" class="sub-dot-indicator"></span>
                </button>

                <!-- Speed Menu -->
                <div class="speed-menu-wrap">
                  <button class="ctrl-btn speed-btn" title="播放速度" @click="showSpeedMenu = !showSpeedMenu">
                    {{ playbackRate }}x
                  </button>
                  <div v-if="showSpeedMenu" class="speed-dropdown">
                    <button
                      v-for="s in speedOptions"
                      :key="s"
                      class="speed-opt"
                      :class="{ active: playbackRate === s }"
                      @click="setPlaybackRate(s)"
                    >
                      {{ s }}x
                    </button>
                  </div>
                </div>

                <!-- Volume Slider -->
                <div class="volume-box">
                  <button class="ctrl-btn vol-btn" :title="isMuted ? '取消静音 (M)' : '静音 (M)'" @click="toggleMute">
                    <span v-if="isMuted || volume === 0">🔇</span>
                    <span v-else-if="volume < 0.5">🔉</span>
                    <span v-else>🔊</span>
                  </button>
                  <input
                    type="range"
                    class="vol-slider"
                    min="0"
                    max="1"
                    step="0.05"
                    :value="isMuted ? 0 : volume"
                    @input="onVolumeInput"
                  />
                </div>

                <!-- Loop -->
                <button
                  class="ctrl-btn"
                  :class="{ 'ctrl-active': isLooping }"
                  title="循环播放"
                  @click="toggleLoop"
                >
                  🔁
                </button>

                <!-- Fullscreen -->
                <button class="ctrl-btn" :title="isFullscreen ? '退出全屏 (F)' : '全屏 (F)'" @click="toggleFullscreen">
                  <span v-if="isFullscreen">⛶</span>
                  <span v-else>⛶</span>
                </button>
              </div>
            </div>
          </div>

          <!-- Specs & Details Footer -->
          <div class="player-footer-strip">
            <div class="meta-tag-list">
              <div class="meta-unit">
                <span class="m-k">格式:</span>
                <span class="m-v">{{ video?.codec?.toUpperCase() || 'MP4' }}</span>
              </div>
              <div class="meta-unit">
                <span class="m-k">分辨率:</span>
                <span class="m-v">{{ video?.width && video?.height ? `${video.width} × ${video.height}` : '未知' }}</span>
              </div>
              <div class="meta-unit">
                <span class="m-k">大小:</span>
                <span class="m-v">{{ formatBytes(video?.file_size || 0) }}</span>
              </div>
              <div class="meta-unit">
                <span class="m-k">时长:</span>
                <span class="m-v">{{ formatTime(video?.duration || duration) }}</span>
              </div>
            </div>

            <div class="footer-actions">
              <button class="btn-foot-act" title="复制完整文件路径" @click="copyPath">
                {{ copiedPath ? '✓ 已复制路径' : '📋 复制路径' }}
              </button>
              <button class="btn-foot-act" title="在系统资源管理器中定位" @click="revealFile">
                📁 定位文件
              </button>
            </div>
          </div>
        </div>

        <!-- Side Transcript Drawer -->
        <div v-if="showTranscript" class="player-drawer-side">
          <SubtitleTranscriptPanel
            :video-id="video?.id || 0"
            :video-title="video?.file_name"
            :current-time="currentTime"
            :is-playing="isPlaying"
            @seek="onTranscriptSeek"
            @close="showTranscript = false"
            @subtitles-updated="onSubtitlesUpdated"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.player-modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(14px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.player-modal-container {
  width: 100%;
  max-width: 1100px;
  height: 85vh;
  max-height: 800px;
  background: #090e18;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 14px;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: transform 0.05s ease-out;
  will-change: transform;
}

.player-modal-container.is-dragging {
  transition: none;
}

.player-modal-container.has-drawer {
  max-width: 1420px;
}

.player-body-layout {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.player-stage-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
}

.player-drawer-side {
  width: 380px;
  min-width: 340px;
  max-width: 420px;
  height: 100%;
  display: flex;
  flex-direction: column;
  border-left: 1px solid rgba(255, 255, 255, 0.08);
  animation: slideInRight 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes slideInRight {
  from { transform: translateX(20px); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

.badge-sub-ready {
  background: rgba(99, 102, 241, 0.2);
  color: #a5b4fc;
  border: 1px solid rgba(99, 102, 241, 0.4);
}

.video-cc-overlay {
  position: absolute;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  max-width: 86%;
  pointer-events: none;
  z-index: 15;
  text-align: center;
}

.video-cc-text {
  display: inline-block;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(10px);
  color: #ffffff;
  font-size: 1.15rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  padding: 0.45rem 1.1rem;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.6);
  line-height: 1.45;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.9);
}

.cc-badge-text {
  font-weight: 700;
  font-size: 0.72rem;
  letter-spacing: 0.05em;
}

.transcript-toggle-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
}

.sub-dot-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #818cf8;
  box-shadow: 0 0 6px #818cf8;
}

/* Modal Header */
.player-modal-header {
  height: 52px;
  min-height: 52px;
  padding: 0 1.25rem;
  background: #0d1526;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  cursor: grab;
  user-select: none;
}

.player-modal-header.is-dragging-header {
  cursor: grabbing;
}

.drag-handle-hint {
  font-size: 1.15rem;
  color: #64748b;
  cursor: grab;
  margin-right: 0.2rem;
  user-select: none;
}

.player-modal-header.is-dragging-header .drag-handle-hint {
  cursor: grabbing;
  color: #94a3b8;
}

.modal-title-box {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  min-width: 0;
  flex: 1;
}

.player-icon {
  font-size: 1.15rem;
}

.player-title {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 600;
  color: #f1f5f9;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge-pill {
  font-size: 0.7rem;
  font-weight: 700;
  padding: 2px 7px;
  border-radius: 999px;
  white-space: nowrap;
}

.badge-res {
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
  border: 1px solid rgba(59, 130, 246, 0.35);
}

.badge-codec {
  background: rgba(16, 185, 129, 0.18);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.modal-header-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.head-act-btn {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  border-radius: 6px;
  padding: 4px 9px;
  font-size: 0.75rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.35rem;
  transition: all 0.15s ease;
}

.head-act-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #ffffff;
  border-color: rgba(255, 255, 255, 0.25);
}

.h-icon {
  width: 14px;
  height: 14px;
}

.btn-close-modal {
  background: none;
  border: none;
  color: #94a3b8;
  font-size: 1.15rem;
  cursor: pointer;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.btn-close-modal:hover {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}

/* Viewport */
.player-viewport {
  flex: 1;
  position: relative;
  background: #03060a;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  cursor: pointer;
}

.video-element {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.player-center-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
  gap: 0.75rem;
  pointer-events: none;
}

.spinner-player {
  width: 44px;
  height: 44px;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.loading-label {
  font-size: 0.82rem;
  color: #94a3b8;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.big-indicator-badge {
  position: absolute;
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: rgba(59, 130, 246, 0.85);
  color: #ffffff;
  font-size: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  pointer-events: none;
}

.pop-enter-active, .pop-leave-active {
  transition: transform 0.25s cubic-bezier(0.175, 0.885, 0.32, 1.275), opacity 0.25s ease;
}
.pop-enter-from, .pop-leave-to {
  transform: scale(0.6);
  opacity: 0;
}

.player-error-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  z-index: 10;
}

.error-box {
  background: #18121d;
  border: 1px solid rgba(239, 68, 68, 0.35);
  border-radius: 12px;
  padding: 1.75rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 480px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
}

.err-icon {
  font-size: 2.4rem;
  margin-bottom: 0.35rem;
}

.err-title {
  margin: 0 0 0.5rem;
  font-size: 1rem;
  font-weight: 600;
  color: #fca5a5;
}

.err-text {
  margin: 0 0 1.25rem;
  font-size: 0.83rem;
  color: #cbd5e1;
  line-height: 1.5;
}

.err-actions {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  width: 100%;
}

.btn-err-fallback {
  border: none;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 0.82rem;
  cursor: pointer;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  transition: all 0.15s ease;
}

.btn-err-fallback.primary {
  background: #3b82f6;
  color: #ffffff;
}

.btn-err-fallback.primary:hover {
  background: #2563eb;
}

.btn-err-fallback.secondary {
  background: rgba(255, 255, 255, 0.08);
  color: #cbd5e1;
  border: 1px solid rgba(255, 255, 255, 0.15);
}

.btn-err-fallback.highlight-transcode {
  background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%);
  color: #ffffff;
  box-shadow: 0 4px 14px rgba(245, 158, 11, 0.35);
}

.btn-err-fallback.highlight-transcode:hover {
  background: linear-gradient(135deg, #d97706 0%, #c2410c 100%);
  transform: translateY(-1px);
}

.converting-state-bar {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  padding: 0.8rem 0;
}

.converting-desc {
  font-size: 0.85rem;
  color: #fbbf24;
  font-weight: 500;
}

.spinner-player.mini {
  width: 28px;
  height: 28px;
  border-width: 3px;
}

.btn-ic {
  font-size: 0.95rem;
}

/* Controls Bar */
.player-controls-bar {
  background: #0b1120;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.6rem 1.15rem 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.scrubber-wrapper {
  position: relative;
  width: 100%;
  height: 6px;
  display: flex;
  align-items: center;
  cursor: pointer;
}

.scrubber-slider {
  position: absolute;
  width: 100%;
  height: 6px;
  margin: 0;
  appearance: none;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 3px;
  outline: none;
  cursor: pointer;
  z-index: 2;
}

.scrubber-slider::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #3b82f6;
  border: 2px solid #ffffff;
  cursor: pointer;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
  transition: transform 0.1s ease;
}

.scrubber-slider::-webkit-slider-thumb:hover {
  transform: scale(1.2);
}

.scrubber-progress-fill {
  position: absolute;
  height: 6px;
  background: #3b82f6;
  border-radius: 3px;
  pointer-events: none;
  z-index: 1;
}

.controls-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.ctrl-group {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.ctrl-btn {
  background: transparent;
  border: none;
  color: #cbd5e1;
  padding: 5px 8px;
  border-radius: 5px;
  font-size: 0.82rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.ctrl-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.play-pause-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #3b82f6;
  color: #ffffff;
  font-size: 0.95rem;
}

.play-pause-btn:hover {
  background: #2563eb;
}

.time-readout {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  font-size: 0.78rem;
  font-variant-numeric: tabular-nums;
  margin-left: 0.5rem;
}

.t-cur {
  color: #f8fafc;
  font-weight: 600;
}

.t-sep {
  color: #64748b;
}

.t-dur {
  color: #94a3b8;
}

.speed-menu-wrap {
  position: relative;
}

.speed-btn {
  font-size: 0.78rem;
  font-weight: 600;
  padding: 3px 7px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

.speed-dropdown {
  position: absolute;
  bottom: 120%;
  right: 0;
  background: #0f172a;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  z-index: 10;
}

.speed-opt {
  background: none;
  border: none;
  color: #94a3b8;
  font-size: 0.75rem;
  padding: 4px 12px;
  text-align: left;
  border-radius: 4px;
  cursor: pointer;
}

.speed-opt:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
}

.speed-opt.active {
  color: #3b82f6;
  font-weight: 700;
}

.volume-box {
  display: flex;
  align-items: center;
  gap: 0.2rem;
}

.vol-slider {
  width: 65px;
  height: 4px;
  appearance: none;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}

.vol-slider::-webkit-slider-thumb {
  appearance: none;
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: #ffffff;
  cursor: pointer;
}

.ctrl-active {
  color: #3b82f6 !important;
}

/* Footer Strip */
.player-footer-strip {
  height: 40px;
  min-height: 40px;
  padding: 0 1.25rem;
  background: #090e18;
  border-top: 1px solid rgba(255, 255, 255, 0.05);
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.75rem;
}

.meta-tag-list {
  display: flex;
  align-items: center;
  gap: 1.2rem;
}

.meta-unit {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.m-k {
  color: #64748b;
}

.m-v {
  color: #cbd5e1;
  font-weight: 500;
}

.footer-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.btn-foot-act {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #94a3b8;
  font-size: 0.72rem;
  padding: 2px 7px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-foot-act:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #3b82f6;
  border-color: rgba(59, 130, 246, 0.4);
}
</style>
