<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { downloadDir, audioDir } from '@tauri-apps/api/path';
import type { Video, AudioExtractionOptions, AudioExtractionResult } from '../types';
import { t } from '../i18n';

const props = defineProps<{
  visible: boolean;
  video: Video | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

type AudioFormat = 'mp3' | 'm4a' | 'wav' | 'flac';
type DestOption = 'source' | 'downloads' | 'music';

const selectedFormat = ref<AudioFormat>('mp3');
const qualityBitrate = ref<string>('320k');
const copyStream = ref<boolean>(false);
const saveDest = ref<DestOption>('source');
const customName = ref<string>('');

const isExtracting = ref<boolean>(false);
const extractionResult = ref<AudioExtractionResult | null>(null);
const errorMessage = ref<string>('');
const toastMessage = ref<string>('');
let toastTimer: number | null = null;

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2200);
}

// Strip extension from video file name for default stem
function getBaseStem(fileName?: string): string {
  if (!fileName) return '';
  const idx = fileName.lastIndexOf('.');
  if (idx <= 0) return fileName;
  return fileName.slice(0, idx);
}

watch(
  () => props.video,
  (newVid) => {
    if (newVid) {
      customName.value = getBaseStem(newVid.file_name);
      extractionResult.value = null;
      errorMessage.value = '';
      isExtracting.value = false;
      copyStream.value = false;
      selectedFormat.value = 'mp3';
      qualityBitrate.value = '320k';
    }
  },
  { immediate: true }
);

function selectFormat(fmt: AudioFormat) {
  selectedFormat.value = fmt;
  if (fmt === 'wav' || fmt === 'flac') {
    copyStream.value = false;
  }
}

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
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

const audioPreviewUrl = computed(() => {
  if (!extractionResult.value?.outputPath) return '';
  return convertFileSrc(extractionResult.value.outputPath);
});

async function executeExtract() {
  if (!props.video?.file_path) return;

  isExtracting.value = true;
  errorMessage.value = '';
  extractionResult.value = null;

  try {
    let resolvedOutputDir: string | undefined = undefined;
    if (saveDest.value === 'downloads') {
      resolvedOutputDir = await downloadDir();
    } else if (saveDest.value === 'music') {
      resolvedOutputDir = await audioDir();
    }

    const options: AudioExtractionOptions = {
      videoPath: props.video.file_path,
      format: selectedFormat.value,
      qualityBitrate: (selectedFormat.value === 'mp3' || selectedFormat.value === 'm4a') && !copyStream.value
        ? qualityBitrate.value
        : undefined,
      copyStream: copyStream.value,
      outputDir: resolvedOutputDir,
      customFilename: customName.value.trim() || undefined,
    };

    const res = await invoke<AudioExtractionResult>('extract_audio_from_video', { options });
    extractionResult.value = res;
    showToast(t('audioExtract.successTitle'));
  } catch (err: unknown) {
    errorMessage.value = err instanceof Error ? err.message : String(err);
  } finally {
    isExtracting.value = false;
  }
}

async function revealFile() {
  if (!extractionResult.value?.outputPath) return;
  try {
    await invoke('reveal_in_explorer', { path: extractionResult.value.outputPath });
    showToast('📂 已在系统文件管理器中定位');
  } catch (err: unknown) {
    showToast(`定位失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function copyPath() {
  if (!extractionResult.value?.outputPath) return;
  try {
    await navigator.clipboard.writeText(extractionResult.value.outputPath);
    showToast(t('audioExtract.copied'));
  } catch (e) {
    showToast('复制路径失败');
  }
}

function handleClose() {
  emit('close');
}
</script>

<template>
  <div v-if="visible" class="audio-modal-overlay" @click.self="handleClose">
    <div class="audio-modal-container">
      <!-- Modal Header -->
      <div class="audio-modal-header">
        <div class="header-icon-box">
          <svg class="h-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 18V5l12-2v13"></path>
            <circle cx="6" cy="18" r="3"></circle>
            <circle cx="18" cy="16" r="3"></circle>
          </svg>
        </div>
        <div class="header-title-box">
          <h3 class="header-title">{{ t('audioExtract.title') }}</h3>
          <p class="header-subtitle">{{ t('audioExtract.subtitle') }}</p>
        </div>
        <button class="modal-close-btn" @click="handleClose" title="关闭">✕</button>
      </div>

      <!-- Source Video Preview Banner -->
      <div v-if="video" class="source-video-card">
        <div class="source-thumb-wrap">
          <img
            v-if="video.thumbnail_data"
            :src="video.thumbnail_data"
            :alt="video.file_name"
            class="source-thumb-img"
          />
          <div v-else class="source-thumb-fallback">🎬</div>
          <span v-if="video.duration" class="source-thumb-dur font-mono">
            {{ formatDuration(video.duration) }}
          </span>
        </div>
        <div class="source-details">
          <div class="source-name" :title="video.file_name">{{ video.file_name }}</div>
          <div class="source-meta-row">
            <span v-if="video.codec" class="meta-pill font-mono">{{ video.codec }}</span>
            <span class="meta-pill font-mono">{{ formatBytes(video.file_size) }}</span>
            <span v-if="video.width && video.height" class="meta-pill font-mono">
              {{ video.width }}×{{ video.height }}
            </span>
          </div>
          <div class="source-path-row font-mono" :title="video.file_path">
            {{ video.file_path }}
          </div>
        </div>
      </div>

      <!-- Configuration Body (Shown when NOT extracted yet) -->
      <div v-if="!extractionResult" class="audio-modal-body">
        <!-- 1. Format Selection Grid -->
        <div class="form-section">
          <label class="section-label">{{ t('audioExtract.formatTitle') }}</label>
          <div class="format-grid">
            <!-- MP3 -->
            <div
              class="format-card"
              :class="{ active: selectedFormat === 'mp3' }"
              @click="selectFormat('mp3')"
            >
              <div class="format-card-top">
                <span class="format-ext font-mono">MP3</span>
                <span v-if="selectedFormat === 'mp3'" class="active-dot">●</span>
              </div>
              <div class="format-desc">{{ t('audioExtract.formatMp3Desc') }}</div>
            </div>

            <!-- M4A -->
            <div
              class="format-card"
              :class="{ active: selectedFormat === 'm4a' }"
              @click="selectFormat('m4a')"
            >
              <div class="format-card-top">
                <span class="format-ext font-mono">M4A / AAC</span>
                <span v-if="selectedFormat === 'm4a'" class="active-dot">●</span>
              </div>
              <div class="format-desc">{{ t('audioExtract.formatM4aDesc') }}</div>
            </div>

            <!-- WAV -->
            <div
              class="format-card"
              :class="{ active: selectedFormat === 'wav' }"
              @click="selectFormat('wav')"
            >
              <div class="format-card-top">
                <span class="format-ext font-mono">WAV (PCM)</span>
                <span v-if="selectedFormat === 'wav'" class="active-dot">●</span>
              </div>
              <div class="format-desc">{{ t('audioExtract.formatWavDesc') }}</div>
            </div>

            <!-- FLAC -->
            <div
              class="format-card"
              :class="{ active: selectedFormat === 'flac' }"
              @click="selectFormat('flac')"
            >
              <div class="format-card-top">
                <span class="format-ext font-mono">FLAC</span>
                <span v-if="selectedFormat === 'flac'" class="active-dot">●</span>
              </div>
              <div class="format-desc">{{ t('audioExtract.formatFlacDesc') }}</div>
            </div>
          </div>
        </div>

        <!-- 2. Stream Copy Option (available for M4A/AAC) -->
        <div v-if="selectedFormat === 'm4a'" class="form-section">
          <label class="stream-copy-card" :class="{ 'is-active': copyStream }">
            <input type="checkbox" v-model="copyStream" class="stream-checkbox" />
            <div class="stream-info">
              <div class="stream-title">{{ t('audioExtract.copyStream') }}</div>
              <div class="stream-subtitle">{{ t('audioExtract.copyStreamDesc') }}</div>
            </div>
          </label>
        </div>

        <!-- 3. Bitrate Selection (For MP3 or M4A when stream copy is off) -->
        <div
          v-if="!copyStream && (selectedFormat === 'mp3' || selectedFormat === 'm4a')"
          class="form-section"
        >
          <label class="section-label">{{ t('audioExtract.bitrateTitle') }}</label>
          <div class="bitrate-pills">
            <button
              class="bitrate-btn"
              :class="{ active: qualityBitrate === '320k' }"
              @click="qualityBitrate = '320k'"
            >
              {{ t('audioExtract.bitrate320') }}
            </button>
            <button
              class="bitrate-btn"
              :class="{ active: qualityBitrate === '256k' }"
              @click="qualityBitrate = '256k'"
            >
              {{ t('audioExtract.bitrate256') }}
            </button>
            <button
              class="bitrate-btn"
              :class="{ active: qualityBitrate === '192k' }"
              @click="qualityBitrate = '192k'"
            >
              {{ t('audioExtract.bitrate192') }}
            </button>
            <button
              class="bitrate-btn"
              :class="{ active: qualityBitrate === '128k' }"
              @click="qualityBitrate = '128k'"
            >
              {{ t('audioExtract.bitrate128') }}
            </button>
          </div>
        </div>

        <!-- 4. Save Location Choice -->
        <div class="form-section">
          <label class="section-label">{{ t('audioExtract.saveDirTitle') }}</label>
          <div class="save-dir-group">
            <label class="save-dir-item" :class="{ selected: saveDest === 'source' }">
              <input type="radio" value="source" v-model="saveDest" />
              <span>{{ t('audioExtract.saveDirSame') }}</span>
            </label>
            <label class="save-dir-item" :class="{ selected: saveDest === 'downloads' }">
              <input type="radio" value="downloads" v-model="saveDest" />
              <span>{{ t('audioExtract.saveDirDownloads') }}</span>
            </label>
            <label class="save-dir-item" :class="{ selected: saveDest === 'music' }">
              <input type="radio" value="music" v-model="saveDest" />
              <span>{{ t('audioExtract.saveDirMusic') }}</span>
            </label>
          </div>
        </div>

        <!-- 5. Output Filename -->
        <div class="form-section">
          <label class="section-label">{{ t('audioExtract.filenameTitle') }}</label>
          <div class="filename-input-wrap">
            <input
              type="text"
              v-model="customName"
              :placeholder="t('audioExtract.filenamePlaceholder')"
              class="filename-input font-mono"
            />
            <span class="filename-ext-tag font-mono">.{{ selectedFormat }}</span>
          </div>
        </div>

        <!-- Error Message Alert -->
        <div v-if="errorMessage" class="error-alert-banner">
          <span class="err-icon">⚠️</span>
          <span class="err-text">{{ errorMessage }}</span>
        </div>
      </div>

      <!-- Success Result Screen -->
      <div v-else class="audio-modal-body success-screen">
        <div class="success-header-badge">
          <div class="success-circle-icon">✓</div>
          <h4 class="success-title">{{ t('audioExtract.successTitle') }}</h4>
          <p class="success-msg">
            {{ extractionResult.fileName }}
            <span class="font-mono text-cyan">({{ formatBytes(extractionResult.fileSize) }})</span>
          </p>
        </div>

        <!-- Instant Audio Player Widget -->
        <div class="audio-player-box">
          <div class="audio-player-header">
            <span class="player-label">🎵 {{ t('audioExtract.playAudio') }}</span>
            <span v-if="extractionResult.duration" class="font-mono text-muted text-xs">
              {{ formatDuration(extractionResult.duration) }}
            </span>
          </div>
          <audio :src="audioPreviewUrl" controls autoplay class="native-audio-player"></audio>
        </div>

        <!-- Result File Info -->
        <div class="result-path-box">
          <span class="path-label">文件路径:</span>
          <span class="path-val font-mono" :title="extractionResult.outputPath">
            {{ extractionResult.outputPath }}
          </span>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="audio-modal-footer">
        <template v-if="!extractionResult">
          <button class="btn btn-secondary" :disabled="isExtracting" @click="handleClose">
            取消
          </button>
          <button
            class="btn btn-primary btn-extract"
            :disabled="isExtracting"
            @click="executeExtract"
          >
            <span v-if="isExtracting" class="spinner-sm"></span>
            <span v-else>⚡</span>
            <span>{{ isExtracting ? t('audioExtract.extracting') : t('audioExtract.startExtract') }}</span>
          </button>
        </template>

        <template v-else>
          <button class="btn btn-secondary" @click="revealFile">
            📂 {{ t('audioExtract.openFolder') }}
          </button>
          <button class="btn btn-secondary" @click="copyPath">
            📋 {{ t('audioExtract.copyPath') }}
          </button>
          <button class="btn btn-primary" @click="handleClose">
            {{ t('audioExtract.close') }}
          </button>
        </template>
      </div>
    </div>

    <!-- Toast Notification -->
    <div v-if="toastMessage" class="modal-toast-popup">
      {{ toastMessage }}
    </div>
  </div>
</template>

<style scoped>
.audio-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background-color: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px);
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

.audio-modal-container {
  background: var(--bg-card, #14171f);
  border: 1px solid var(--border-color, #2d3348);
  border-radius: 1rem;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.05);
  width: 100%;
  max-width: 600px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: slideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(16px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

/* Header */
.audio-modal-header {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--border-color, #2d3348);
  background: rgba(255, 255, 255, 0.02);
}

.header-icon-box {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: 0.75rem;
  background: linear-gradient(135deg, rgba(6, 182, 212, 0.2), rgba(59, 130, 246, 0.2));
  border: 1px solid rgba(6, 182, 212, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #38bdf8;
  flex-shrink: 0;
}

.h-icon {
  width: 1.25rem;
  height: 1.25rem;
}

.header-title-box {
  flex: 1;
  min-width: 0;
}

.header-title {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.header-subtitle {
  margin: 0.2rem 0 0 0;
  font-size: 0.8rem;
  color: var(--text-muted, #94a3b8);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.modal-close-btn {
  background: transparent;
  border: none;
  color: var(--text-muted, #94a3b8);
  font-size: 1.1rem;
  cursor: pointer;
  padding: 0.4rem;
  border-radius: 0.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.modal-close-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

/* Source Video Preview */
.source-video-card {
  display: flex;
  gap: 1rem;
  padding: 1rem 1.5rem;
  background: rgba(0, 0, 0, 0.25);
  border-bottom: 1px solid var(--border-color, #2d3348);
}

.source-thumb-wrap {
  position: relative;
  width: 100px;
  height: 60px;
  border-radius: 0.5rem;
  overflow: hidden;
  background: #0f172a;
  flex-shrink: 0;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.source-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.source-thumb-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
}

.source-thumb-dur {
  position: absolute;
  bottom: 0.25rem;
  right: 0.25rem;
  background: rgba(0, 0, 0, 0.75);
  color: #fff;
  font-size: 0.65rem;
  padding: 0.1rem 0.3rem;
  border-radius: 0.25rem;
}

.source-details {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.source-name {
  font-size: 0.9rem;
  font-weight: 500;
  color: var(--text-primary, #f1f5f9);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.source-meta-row {
  display: flex;
  gap: 0.4rem;
  margin-top: 0.35rem;
}

.meta-pill {
  font-size: 0.7rem;
  background: rgba(255, 255, 255, 0.06);
  padding: 0.1rem 0.4rem;
  border-radius: 0.25rem;
  color: var(--text-muted, #94a3b8);
}

.source-path-row {
  font-size: 0.72rem;
  color: #64748b;
  margin-top: 0.3rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Modal Body */
.audio-modal-body {
  padding: 1.25rem 1.5rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1.15rem;
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.section-label {
  font-size: 0.82rem;
  font-weight: 500;
  color: var(--text-muted, #cbd5e1);
}

/* Format Grid */
.format-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.6rem;
}

.format-card {
  padding: 0.75rem 0.85rem;
  border-radius: 0.65rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border-color, #2d3348);
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
}

.format-card:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.2);
}

.format-card.active {
  background: rgba(6, 182, 212, 0.08);
  border-color: #06b6d4;
  box-shadow: 0 0 12px rgba(6, 182, 212, 0.15);
}

.format-card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.format-ext {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary, #f1f5f9);
}

.active-dot {
  color: #06b6d4;
  font-size: 0.85rem;
}

.format-desc {
  font-size: 0.73rem;
  color: var(--text-muted, #94a3b8);
  line-height: 1.3;
}

/* Stream Copy Card */
.stream-copy-card {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.75rem 0.9rem;
  background: rgba(234, 179, 8, 0.06);
  border: 1px solid rgba(234, 179, 8, 0.25);
  border-radius: 0.65rem;
  cursor: pointer;
  transition: all 0.15s;
}

.stream-copy-card.is-active {
  background: rgba(234, 179, 8, 0.12);
  border-color: #eab308;
  box-shadow: 0 0 12px rgba(234, 179, 8, 0.15);
}

.stream-checkbox {
  margin-top: 0.2rem;
  accent-color: #eab308;
  cursor: pointer;
  transform: scale(1.1);
}

.stream-info {
  flex: 1;
}

.stream-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: #fde047;
}

.stream-subtitle {
  font-size: 0.72rem;
  color: #cbd5e1;
  margin-top: 0.15rem;
  line-height: 1.35;
}

/* Bitrate Pills */
.bitrate-pills {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.5rem;
}

.bitrate-btn {
  padding: 0.5rem 0.75rem;
  border-radius: 0.5rem;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color, #2d3348);
  color: var(--text-muted, #cbd5e1);
  font-size: 0.76rem;
  cursor: pointer;
  transition: all 0.15s;
  text-align: center;
}

.bitrate-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.bitrate-btn.active {
  background: rgba(59, 130, 246, 0.15);
  border-color: #3b82f6;
  color: #60a5fa;
  font-weight: 500;
}

/* Save Dir Group */
.save-dir-group {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.save-dir-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.8rem;
  border-radius: 0.5rem;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--border-color, #2d3348);
  font-size: 0.82rem;
  color: var(--text-muted, #cbd5e1);
  cursor: pointer;
  transition: all 0.15s;
}

.save-dir-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.save-dir-item.selected {
  border-color: #06b6d4;
  color: #fff;
  background: rgba(6, 182, 212, 0.06);
}

.save-dir-item input[type='radio'] {
  accent-color: #06b6d4;
  cursor: pointer;
}

/* Filename Input */
.filename-input-wrap {
  display: flex;
  align-items: center;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid var(--border-color, #2d3348);
  border-radius: 0.5rem;
  overflow: hidden;
  transition: border-color 0.15s;
}

.filename-input-wrap:focus-within {
  border-color: #06b6d4;
}

.filename-input {
  flex: 1;
  background: transparent;
  border: none;
  padding: 0.55rem 0.8rem;
  color: #fff;
  font-size: 0.85rem;
  outline: none;
}

.filename-ext-tag {
  padding: 0.55rem 0.8rem;
  background: rgba(255, 255, 255, 0.06);
  border-left: 1px solid var(--border-color, #2d3348);
  color: #94a3b8;
  font-size: 0.8rem;
}

/* Error Banner */
.error-alert-banner {
  display: flex;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  background: rgba(239, 68, 68, 0.12);
  border: 1px solid rgba(239, 68, 68, 0.35);
  color: #fca5a5;
  font-size: 0.8rem;
  align-items: flex-start;
}

.err-icon {
  font-size: 1rem;
}

.err-text {
  flex: 1;
  line-height: 1.4;
}

/* Success Screen */
.success-screen {
  align-items: center;
  text-align: center;
  padding: 2rem 1.5rem;
}

.success-header-badge {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.success-circle-icon {
  width: 3.5rem;
  height: 3.5rem;
  border-radius: 50%;
  background: linear-gradient(135deg, #10b981, #059669);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.8rem;
  font-weight: bold;
  box-shadow: 0 0 20px rgba(16, 185, 129, 0.4);
  margin-bottom: 0.75rem;
}

.success-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: #fff;
}

.success-msg {
  margin: 0.35rem 0 1rem 0;
  font-size: 0.88rem;
  color: #94a3b8;
}

.text-cyan {
  color: #38bdf8;
}

/* Instant Audio Player Widget */
.audio-player-box {
  width: 100%;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid var(--border-color, #2d3348);
  border-radius: 0.75rem;
  padding: 0.85rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  text-align: left;
}

.audio-player-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.player-label {
  font-size: 0.85rem;
  font-weight: 500;
  color: #e2e8f0;
}

.native-audio-player {
  width: 100%;
  height: 36px;
  border-radius: 0.5rem;
  outline: none;
}

.result-path-box {
  width: 100%;
  padding: 0.65rem 0.85rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px dashed var(--border-color, #2d3348);
  border-radius: 0.5rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: #94a3b8;
  text-align: left;
}

.path-label {
  flex-shrink: 0;
  color: #64748b;
}

.path-val {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: #cbd5e1;
}

/* Footer */
.audio-modal-footer {
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--border-color, #2d3348);
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  background: rgba(255, 255, 255, 0.02);
}

.btn {
  padding: 0.55rem 1.1rem;
  border-radius: 0.5rem;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  border: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.08);
  color: #cbd5e1;
  border: 1px solid var(--border-color, #2d3348);
}

.btn-secondary:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

.btn-primary {
  background: linear-gradient(135deg, #06b6d4, #3b82f6);
  color: #fff;
  box-shadow: 0 2px 8px rgba(6, 182, 212, 0.25);
}

.btn-primary:hover:not(:disabled) {
  background: linear-gradient(135deg, #0891b2, #2563eb);
  box-shadow: 0 4px 12px rgba(6, 182, 212, 0.35);
}

.btn-extract {
  min-width: 130px;
}

.spinner-sm {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Toast */
.modal-toast-popup {
  position: absolute;
  top: 1.5rem;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid #06b6d4;
  color: #f1f5f9;
  padding: 0.5rem 1rem;
  border-radius: 9999px;
  font-size: 0.82rem;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
  pointer-events: none;
  animation: toastFade 0.2s ease-out;
}

@keyframes toastFade {
  from {
    opacity: 0;
    transform: translate(-50%, -8px);
  }
  to {
    opacity: 1;
    transform: translate(-50%, 0);
  }
}
</style>
