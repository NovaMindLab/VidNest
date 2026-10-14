<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import SubtitleTranscriptPanel from './SubtitleTranscriptPanel.vue';
import type { Video, SubtitleSegment, VideoCompatibilityStatus, TranscodeResult } from '../types';
import { t } from '../i18n';

const video = ref<Video | null>(null);
const compatibleFilePath = ref<string | null>(null);
const isConverting = ref(false);
const convertProgressText = ref('');
const isCompatibleFormat = ref(true);
const videoRef = ref<HTMLVideoElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);

const isPlaying = ref(false);
const currentTime = ref(0);
const duration = ref(0);
const isScrubbing = ref(false);
const scrubTime = ref(0);
const volume = ref(1);
const isMuted = ref(false);
const playbackRate = ref(1);
const isLooping = ref(false);
const isFullscreen = ref(false);
const isAlwaysOnTop = ref(false);
const isLoading = ref(true);
const errorMessage = ref('');
const showBigPlayIcon = ref(false);
const copiedPath = ref(false);
const showSpeedMenu = ref(false);

const speedOptions = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
let unlistenPlayVideo: UnlistenFn | null = null;

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

async function fetchSubtitles(vidId: number) {
  if (!vidId) {
    videoSubtitles.value = [];
    return;
  }
  try {
    const list = await invoke<SubtitleSegment[]>('get_video_subtitles', {
      videoId: vidId,
    });
    videoSubtitles.value = list || [];
  } catch (err) {
    console.warn('Failed to fetch subtitles in standalone player:', err);
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

const videoSrc = computed(() => {
  if (compatibleFilePath.value) {
    return convertFileSrc(compatibleFilePath.value);
  }
  if (!video.value?.file_path) return '';
  return convertFileSrc(video.value.file_path);
});

async function checkCompatibility(v: Video) {
  compatibleFilePath.value = null;
  isConverting.value = false;
  if (!v.file_path) return;

  try {
    const status = await invoke<VideoCompatibilityStatus>('check_video_compatibility', {
      filePath: v.file_path,
      fileHash: v.file_hash || null,
    });
    if (status.has_cached_transcode && status.cached_path) {
      compatibleFilePath.value = status.cached_path;
      isCompatibleFormat.value = true;
      errorMessage.value = '';
    } else if (!status.is_compatible) {
      isCompatibleFormat.value = false;
      errorMessage.value = status.reason || t('player.needOptimization');
      isLoading.value = false;
    } else {
      isCompatibleFormat.value = true;
    }
  } catch (err) {
    console.warn('check_video_compatibility error in standalone window:', err);
  }
}

async function startCompatibilityConversion() {
  if (!video.value?.file_path || isConverting.value) return;
  isConverting.value = true;
  convertProgressText.value = t('player.optimizingCompatibility');
  errorMessage.value = '';

  try {
    const res = await invoke<TranscodeResult>('convert_video_to_compatible', {
      filePath: video.value.file_path,
      fileHash: video.value.file_hash || null,
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
    errorMessage.value = `${t('player.playbackError')}: ${err?.message || err}`;
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
  }, 450);
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
  let desc = t('player.cannotDecodeDirectly');
  if (mediaError) {
    switch (mediaError.code) {
      case 1:
      case 2:
      case 3:
      case 4:
      default:
        desc = mediaError.message || t('player.cannotDecodeDirectly');
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

async function toggleAlwaysOnTop() {
  try {
    const nextState = !isAlwaysOnTop.value;
    await invoke('set_player_always_on_top', { alwaysOnTop: nextState });
    isAlwaysOnTop.value = nextState;
  } catch (err) {
    console.error('Failed to set always on top:', err);
  }
}

async function openExternal() {
  if (!video.value?.file_path) return;
  try {
    await openPath(video.value.file_path);
  } catch (err) {
    console.error('Failed to open externally:', err);
  }
}

async function revealFile() {
  if (!video.value?.file_path) return;
  try {
    await revealItemInDir(video.value.file_path);
  } catch (err) {
    console.error('Failed to reveal file:', err);
  }
}

async function copyPath() {
  if (!video.value?.file_path) return;
  try {
    await navigator.clipboard.writeText(video.value.file_path);
    copiedPath.value = true;
    setTimeout(() => {
      copiedPath.value = false;
    }, 1800);
  } catch (err) {
    console.error('Failed to copy path:', err);
  }
}

function handleKeydown(e: KeyboardEvent) {
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
    case 'KeyF':
      e.preventDefault();
      toggleFullscreen();
      break;
    case 'KeyT':
      e.preventDefault();
      toggleAlwaysOnTop();
      break;
    case 'KeyC':
      e.preventDefault();
      showCc.value = !showCc.value;
      break;
    case 'Escape':
      if (document.fullscreenElement) {
        e.preventDefault();
        document.exitFullscreen();
      }
      break;
  }
}

function loadVideo(v: Video) {
  video.value = v;
  isLoading.value = true;
  errorMessage.value = '';
  currentTime.value = 0;
  fetchSubtitles(v.id);
  checkCompatibility(v);
  setTimeout(() => {
    if (videoRef.value && isCompatibleFormat.value) {
      videoRef.value.play().catch(() => {});
      isPlaying.value = true;
    }
  }, 150);
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown);
  document.addEventListener('fullscreenchange', () => {
    isFullscreen.value = !!document.fullscreenElement;
  });

  // 1. Fetch initial video payload from Tauri state
  try {
    const initVideo = await invoke<Video | null>('get_player_init_video');
    if (initVideo) {
      loadVideo(initVideo);
    }
  } catch (err) {
    console.error('Failed to fetch initial video payload:', err);
  }

  // 2. Listen to switch video events from main window
  try {
    unlistenPlayVideo = await listen<Video>('play-video', (event) => {
      if (event.payload) {
        loadVideo(event.payload);
      }
    });
  } catch (err) {
    console.error('Failed to listen to play-video event:', err);
  }
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
  if (unlistenPlayVideo) {
    unlistenPlayVideo();
  }
});
</script>

<template>
  <div ref="containerRef" class="standalone-player-root">
    <!-- Top Header Bar -->
    <header class="player-topbar">
      <div class="topbar-left">
        <span class="player-icon">🎬</span>
        <h2 class="player-title" :title="video?.file_name || t('player.standaloneWindow')">
          {{ video?.file_name || t('player.waitVideo') }}
        </h2>
        <span v-if="video?.width && video?.height" class="badge-pill badge-res">
          {{ video.width >= 3840 ? '4K UHD' : video.width >= 1920 ? '1080P' : `${video.width}x${video.height}` }}
        </span>
        <span v-if="video?.codec" class="badge-pill badge-codec">
          {{ video.codec.toUpperCase() }}
        </span>
        <span v-if="videoSubtitles.length > 0" class="badge-pill badge-sub-ready" :title="t('player.ccReady')">
          CC · {{ videoSubtitles.length }}
        </span>
      </div>

      <div class="topbar-actions">
        <!-- Always on top Pin button -->
        <button
          class="act-btn"
          :class="{ 'act-btn-active': isAlwaysOnTop }"
          :title="isAlwaysOnTop ? t('player.pinned') : t('player.pinTitle')"
          @click="toggleAlwaysOnTop"
        >
          <span class="act-ic">📌</span>
          <span>{{ isAlwaysOnTop ? t('player.pinned') : t('player.pinWindow') }}</span>
        </button>

        <!-- System external player button -->
        <button class="act-btn" :title="t('player.systemPlayerTitle')" @click="openExternal">
          <span class="act-ic">🚀</span>
          <span>{{ t('player.systemPlayer') }}</span>
        </button>

        <!-- Locate file in directory -->
        <button class="act-btn" :title="t('player.locateFileTitle')" @click="revealFile">
          <span class="act-ic">📁</span>
          <span>{{ t('player.locateFile') }}</span>
        </button>

        <!-- Copy path -->
        <button class="act-btn" :title="t('player.copyPathTitle')" @click="copyPath">
          <span class="act-ic">📋</span>
          <span>{{ copiedPath ? t('player.copied') : t('player.copyPath') }}</span>
        </button>
      </div>
    </header>

    <!-- Body Layout with optional Transcript Drawer -->
    <div class="standalone-body-layout">
      <div class="standalone-stage-main">
        <!-- Main Video Screen -->
        <main class="player-viewport" @click="togglePlay" @dblclick="toggleFullscreen">
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

          <!-- Loading overlay -->
          <div v-if="isLoading && !errorMessage" class="player-center-overlay">
            <div class="spinner-player"></div>
            <span class="loading-label">{{ t('player.streaming') }}</span>
          </div>

          <!-- Big Play Indicator Badge -->
          <transition name="pop">
            <div v-if="showBigPlayIcon" class="big-indicator-badge">
              <span v-if="isPlaying">▶</span>
              <span v-else>❚❚</span>
            </div>
          </transition>

          <!-- Unsupported Codec / Compatibility / Error Banner -->
          <div v-if="errorMessage" class="player-error-overlay" @click.stop>
            <div class="error-box">
              <span class="err-icon">{{ isConverting ? '⏳' : (isCompatibleFormat ? '⚠️' : '⚡') }}</span>
              <h3 class="err-title">{{ isConverting ? t('player.optimizingCompatibility') : (isCompatibleFormat ? t('player.cannotDecodeDirectly') : t('player.needOptimization')) }}</h3>
              <p class="err-text">{{ errorMessage }}</p>

              <!-- In-progress state -->
              <div v-if="isConverting" class="converting-state-bar">
                <div class="spinner-player mini"></div>
                <span class="converting-desc">{{ convertProgressText }}</span>
              </div>

              <!-- Actions -->
              <div v-else class="err-actions">
                <button
                  v-if="!isCompatibleFormat"
                  class="btn-err-action highlight-transcode"
                  @click.stop="startCompatibilityConversion"
                >
                  <span class="btn-ic">⚡</span> {{ t('player.btnOptimize') }}
                </button>
                <button class="btn-err-action primary" @click="openExternal">
                  <span class="btn-ic">🚀</span> {{ t('player.btnSystemPlayer') }}
                </button>
                <button class="btn-err-action secondary" @click="revealFile">
                  <span class="btn-ic">📁</span> {{ t('player.btnLocate') }}
                </button>
              </div>
            </div>
          </div>
        </main>

        <!-- Player Controls Bar -->
        <footer class="player-bottom-bar" @click.stop>
          <!-- Timeline Scrubber -->
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

          <!-- Controls Main Row -->
          <div class="controls-row">
            <!-- Left Group -->
            <div class="ctrl-group ctrl-left">
              <button
                class="ctrl-btn play-pause-btn"
                :title="isPlaying ? t('player.pause') : t('player.play')"
                @click="togglePlay"
              >
                <span v-if="isPlaying">❚❚</span>
                <span v-else>▶</span>
              </button>

              <button class="ctrl-btn" :title="t('player.back5s')" @click="seekRelative(-5)">
                ↺ 5s
              </button>
              <button class="ctrl-btn" :title="t('player.forward5s')" @click="seekRelative(5)">
                5s ↻
              </button>

              <div class="time-readout">
                <span class="t-cur">{{ formatTime(isScrubbing ? scrubTime : currentTime) }}</span>
                <span class="t-sep">/</span>
                <span class="t-dur">{{ formatTime(duration) }}</span>
              </div>
            </div>

            <!-- Right Group -->
            <div class="ctrl-group ctrl-right">
              <!-- CC Subtitles Toggle -->
              <button
                class="ctrl-btn"
                :class="{ 'ctrl-active': showCc }"
                :title="showCc ? t('player.hideCc') : t('player.showCc')"
                @click="showCc = !showCc"
              >
                <span class="cc-badge-text">CC</span>
              </button>

              <!-- Transcript Drawer Toggle -->
              <button
                class="ctrl-btn transcript-toggle-btn"
                :class="{ 'ctrl-active': showTranscript }"
                :title="showTranscript ? t('player.collapseTranscript') : t('player.transcriptTitle')"
                @click="showTranscript = !showTranscript"
              >
                <span>📝 {{ t('player.transcript') }}</span>
                <span v-if="videoSubtitles.length > 0" class="sub-dot-indicator"></span>
              </button>

              <!-- Speed Menu -->
              <div class="speed-menu-wrap">
                <button class="ctrl-btn speed-btn" :title="t('player.speed')" @click="showSpeedMenu = !showSpeedMenu">
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
                <button class="ctrl-btn vol-btn" :title="isMuted ? t('player.unmute') : t('player.mute')" @click="toggleMute">
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

              <!-- Loop toggle -->
              <button
                class="ctrl-btn"
                :class="{ 'ctrl-active': isLooping }"
                :title="t('player.loop')"
                @click="toggleLoop"
              >
                🔁
              </button>

              <!-- Fullscreen toggle -->
              <button class="ctrl-btn" :title="isFullscreen ? t('player.exitFullscreen') : t('player.fullscreen')" @click="toggleFullscreen">
                <span v-if="isFullscreen">⛶</span>
                <span v-else>⛶</span>
              </button>
            </div>
          </div>
        </footer>

        <!-- Bottom Specs Strip -->
        <div class="specs-strip">
          <div class="specs-left">
            <span class="s-item">{{ t('player.specFormat') }}: <b>{{ video?.codec?.toUpperCase() || 'MP4' }}</b></span>
            <span class="s-sep">•</span>
            <span class="s-item">{{ t('player.specResolution') }}: <b>{{ video?.width && video?.height ? `${video.width}×${video.height}` : t('system.unknown') }}</b></span>
            <span class="s-sep">•</span>
            <span class="s-item">{{ t('player.specSize') }}: <b>{{ formatBytes(video?.file_size || 0) }}</b></span>
            <span class="s-sep">•</span>
            <span class="s-item">{{ t('player.specDuration') }}: <b>{{ formatTime(video?.duration || duration) }}</b></span>
          </div>
          <div class="specs-right">
            <span class="s-path" :title="video?.file_path">{{ video?.file_path || '' }}</span>
          </div>
        </div>
      </div>

      <!-- Side Transcript Drawer -->
      <div v-if="showTranscript" class="standalone-drawer-side">
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
</template>

<style scoped>
.standalone-player-root {
  width: 100vw;
  height: 100vh;
  margin: 0;
  padding: 0;
  background: #03060a;
  color: #f8fafc;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  user-select: none;
}

.standalone-body-layout {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.standalone-stage-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
}

.standalone-drawer-side {
  width: 380px;
  min-width: 340px;
  max-width: 440px;
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

/* Header Topbar */
.player-topbar {
  height: 48px;
  min-height: 48px;
  background: #0c1322;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 1rem;
  gap: 1rem;
  z-index: 10;
}

.topbar-left {
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
  font-size: 0.95rem;
  font-weight: 600;
  color: #f8fafc;
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

.topbar-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.act-btn {
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

.act-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #ffffff;
  border-color: rgba(255, 255, 255, 0.25);
}

.act-btn-active {
  background: rgba(59, 130, 246, 0.25) !important;
  color: #60a5fa !important;
  border-color: rgba(59, 130, 246, 0.45) !important;
}

.act-ic {
  font-size: 0.85rem;
}

/* Main Viewport */
.player-viewport {
  flex: 1;
  position: relative;
  background: #000000;
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
  background: rgba(0, 0, 0, 0.5);
  gap: 0.75rem;
  pointer-events: none;
}

.spinner-player {
  width: 46px;
  height: 46px;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.loading-label {
  font-size: 0.85rem;
  color: #94a3b8;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.big-indicator-badge {
  position: absolute;
  width: 76px;
  height: 76px;
  border-radius: 50%;
  background: rgba(59, 130, 246, 0.85);
  color: #ffffff;
  font-size: 2.2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 10px 36px rgba(0, 0, 0, 0.6);
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
  z-index: 20;
}

.error-box {
  background: #18121d;
  border: 1px solid rgba(239, 68, 68, 0.35);
  border-radius: 12px;
  padding: 2rem 2.5rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 500px;
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.7);
}

.err-icon {
  font-size: 2.6rem;
  margin-bottom: 0.4rem;
}

.err-title {
  margin: 0 0 0.5rem;
  font-size: 1.05rem;
  font-weight: 600;
  color: #fca5a5;
}

.err-text {
  margin: 0 0 1.5rem;
  font-size: 0.85rem;
  color: #cbd5e1;
  line-height: 1.55;
}

.err-actions {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  width: 100%;
}

.btn-err-action {
  border: none;
  padding: 9px 16px;
  border-radius: 7px;
  font-size: 0.84rem;
  cursor: pointer;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  transition: all 0.15s ease;
}

.btn-err-action.primary {
  background: #3b82f6;
  color: #ffffff;
}

.btn-err-action.primary:hover {
  background: #2563eb;
}

.btn-err-action.secondary {
  background: rgba(255, 255, 255, 0.08);
  color: #cbd5e1;
  border: 1px solid rgba(255, 255, 255, 0.15);
}

.btn-err-action.highlight-transcode {
  background: linear-gradient(135deg, #f59e0b 0%, #ea580c 100%);
  color: #ffffff;
  box-shadow: 0 4px 14px rgba(245, 158, 11, 0.35);
}

.btn-err-action.highlight-transcode:hover {
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

/* Bottom Controls */
.player-bottom-bar {
  background: #0c1322;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.65rem 1.25rem 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
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
  transform: scale(1.25);
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
  gap: 0.45rem;
}

.ctrl-btn {
  background: transparent;
  border: none;
  color: #cbd5e1;
  padding: 5px 9px;
  border-radius: 5px;
  font-size: 0.84rem;
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
  width: 34px;
  height: 34px;
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
  gap: 0.35rem;
  font-size: 0.8rem;
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
  font-size: 0.8rem;
  font-weight: 600;
  padding: 3px 8px;
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
  width: 70px;
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

/* Specs Footer Strip */
.specs-strip {
  height: 32px;
  min-height: 32px;
  background: #070c16;
  border-top: 1px solid rgba(255, 255, 255, 0.05);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 1rem;
  font-size: 0.72rem;
  color: #64748b;
}

.specs-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.s-item b {
  color: #cbd5e1;
  font-weight: 500;
}

.s-sep {
  color: #334155;
}

.specs-right {
  max-width: 45%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.s-path {
  color: #475569;
}
</style>
