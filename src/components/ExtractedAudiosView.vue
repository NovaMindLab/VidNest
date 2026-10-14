<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type { ExtractedAudio, ExtractedAudioFilter } from '../types';
import { t } from '../i18n';

const emit = defineEmits<{
  (e: 'navigateLibrary'): void;
}>();

const audios = ref<ExtractedAudio[]>([]);
const isLoading = ref<boolean>(false);
const searchText = ref<string>('');
const selectedFormat = ref<string>('all');
const selectedSort = ref<string>('created_desc');

// Active Playing State
const currentPlayingAudio = ref<ExtractedAudio | null>(null);
const audioElement = ref<HTMLAudioElement | null>(null);
const isAudioPlaying = ref<boolean>(false);
const audioCurrentTime = ref<number>(0);
const audioDuration = ref<number>(0);
const audioVolume = ref<number>(1);

// Delete Modal State
const showDeleteModal = ref<boolean>(false);
const audioToDelete = ref<ExtractedAudio | null>(null);
const alsoDeleteDiskFile = ref<boolean>(false);
const isDeleting = ref<boolean>(false);

// Toast
const toastMessage = ref<string>('');
let toastTimer: number | null = null;

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2500);
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

function formatDate(epochMs: number): string {
  if (!epochMs) return '-';
  const d = new Date(epochMs);
  const year = d.getFullYear();
  const month = (d.getMonth() + 1).toString().padStart(2, '0');
  const day = d.getDate().toString().padStart(2, '0');
  const hours = d.getHours().toString().padStart(2, '0');
  const mins = d.getMinutes().toString().padStart(2, '0');
  return `${year}-${month}-${day} ${hours}:${mins}`;
}

// Compute statistics
const totalSize = computed(() => {
  return audios.value.reduce((acc, cur) => acc + (cur.fileSize || 0), 0);
});

const totalDuration = computed(() => {
  return audios.value.reduce((acc, cur) => acc + (cur.duration || 0), 0);
});

async function fetchAudios() {
  isLoading.value = true;
  try {
    const filter: ExtractedAudioFilter = {
      search: searchText.value.trim() || undefined,
      format: selectedFormat.value !== 'all' ? selectedFormat.value : undefined,
      sortBy: selectedSort.value,
    };
    const list = await invoke<ExtractedAudio[]>('list_extracted_audios', { filter });
    audios.value = list;
  } catch (err: unknown) {
    console.error('Failed to list extracted audios:', err);
    showToast(t('audioLibrary.queryFailed', { err: err instanceof Error ? err.message : String(err) }));
  } finally {
    isLoading.value = false;
  }
}

let searchTimer: number | null = null;
function onSearchInput() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => {
    fetchAudios();
  }, 250);
}

function clearSearch() {
  searchText.value = '';
  fetchAudios();
}

function selectFormatTab(fmt: string) {
  selectedFormat.value = fmt;
  fetchAudios();
}

function onSortChange() {
  fetchAudios();
}

// Playback Logic
function togglePlay(item: ExtractedAudio) {
  if (currentPlayingAudio.value?.id === item.id) {
    if (audioElement.value) {
      if (isAudioPlaying.value) {
        audioElement.value.pause();
      } else {
        audioElement.value.play();
      }
    }
  } else {
    currentPlayingAudio.value = item;
    // Set source and play
    if (audioElement.value) {
      audioElement.value.src = convertFileSrc(item.filePath);
      audioElement.value.play().catch((e) => console.warn('Audio play error:', e));
    }
  }
}

function onAudioTimeUpdate() {
  if (audioElement.value) {
    audioCurrentTime.value = audioElement.value.currentTime;
    audioDuration.value = audioElement.value.duration || 0;
  }
}

function onAudioPlay() {
  isAudioPlaying.value = true;
}

function onAudioPause() {
  isAudioPlaying.value = false;
}

function onAudioEnded() {
  isAudioPlaying.value = false;
}

function onScrub(e: Event) {
  const target = e.target as HTMLInputElement;
  const time = parseFloat(target.value);
  if (audioElement.value && !isNaN(time)) {
    audioElement.value.currentTime = time;
  }
}

function onVolumeChange(e: Event) {
  const target = e.target as HTMLInputElement;
  const vol = parseFloat(target.value);
  audioVolume.value = vol;
  if (audioElement.value) {
    audioElement.value.volume = vol;
  }
}

function closePlayer() {
  if (audioElement.value) {
    audioElement.value.pause();
  }
  currentPlayingAudio.value = null;
  isAudioPlaying.value = false;
}

async function revealFile(filePath: string) {
  try {
    await invoke('reveal_in_explorer', { path: filePath });
    showToast(t('library.locatedInFileManager'));
  } catch (err: unknown) {
    showToast(t('audioExtract.locateFailed', { err: err instanceof Error ? err.message : String(err) }));
  }
}

async function copyPath(filePath: string) {
  try {
    await navigator.clipboard.writeText(filePath);
    showToast(t('audioLibrary.copySuccess'));
  } catch (e) {
    showToast(t('audioLibrary.copyFailed'));
  }
}

function promptDelete(item: ExtractedAudio) {
  audioToDelete.value = item;
  alsoDeleteDiskFile.value = false;
  showDeleteModal.value = true;
}

async function confirmDelete() {
  if (!audioToDelete.value) return;
  isDeleting.value = true;
  try {
    const id = audioToDelete.value.id;
    await invoke('delete_extracted_audio', {
      id,
      deleteDiskFile: alsoDeleteDiskFile.value,
    });
    if (currentPlayingAudio.value?.id === id) {
      closePlayer();
    }
    showDeleteModal.value = false;
    audioToDelete.value = null;
    showToast(t('audioLibrary.deleteSuccess'));
    fetchAudios();
  } catch (err: unknown) {
    showToast(t('audioLibrary.deleteFailed', { err: err instanceof Error ? err.message : String(err) }));
  } finally {
    isDeleting.value = false;
  }
}

onMounted(() => {
  fetchAudios();
});
</script>

<template>
  <div class="audio-library-container">
    <!-- Hidden HTML5 Audio Element -->
    <audio
      ref="audioElement"
      @timeupdate="onAudioTimeUpdate"
      @play="onAudioPlay"
      @pause="onAudioPause"
      @ended="onAudioEnded"
    ></audio>

    <!-- Top Statistics Summary Cards -->
    <section class="stats-overview-grid">
      <div class="stat-card">
        <div class="stat-icon-wrap icon-cyan">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 18V5l12-2v13"></path>
            <circle cx="6" cy="18" r="3"></circle>
            <circle cx="18" cy="16" r="3"></circle>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">{{ t('audioLibrary.statsTotal') }}</div>
          <div class="stat-value font-mono">{{ audios.length }} <span class="stat-unit">{{ t('audioLibrary.unitTracks') }}</span></div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon-wrap icon-purple">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
            <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path>
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">{{ t('audioLibrary.statsSize') }}</div>
          <div class="stat-value font-mono">{{ formatBytes(totalSize) }}</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon-wrap icon-emerald">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">{{ t('audioLibrary.statsDuration') }}</div>
          <div class="stat-value font-mono">{{ formatDuration(totalDuration) }}</div>
        </div>
      </div>
    </section>

    <!-- Controls Bar: Search, Format Filter & Sorter -->
    <section class="audio-control-bar">
      <!-- Search Input -->
      <div class="search-input-wrap">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"></circle>
          <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
        <input
          type="text"
          v-model="searchText"
          @input="onSearchInput"
          :placeholder="t('audioLibrary.searchPlaceholder')"
          class="audio-search-input"
        />
        <button v-if="searchText" class="clear-btn" @click="clearSearch" :title="t('audioLibrary.clearSearch')">✕</button>
      </div>

      <!-- Format Filter Pills -->
      <div class="format-pills">
        <button
          class="pill-btn"
          :class="{ active: selectedFormat === 'all' }"
          @click="selectFormatTab('all')"
        >
          {{ t('audioLibrary.allFormats') }}
        </button>
        <button
          class="pill-btn"
          :class="{ active: selectedFormat === 'mp3' }"
          @click="selectFormatTab('mp3')"
        >
          MP3
        </button>
        <button
          class="pill-btn"
          :class="{ active: selectedFormat === 'm4a' }"
          @click="selectFormatTab('m4a')"
        >
          M4A
        </button>
        <button
          class="pill-btn"
          :class="{ active: selectedFormat === 'wav' }"
          @click="selectFormatTab('wav')"
        >
          WAV
        </button>
        <button
          class="pill-btn"
          :class="{ active: selectedFormat === 'flac' }"
          @click="selectFormatTab('flac')"
        >
          FLAC
        </button>
      </div>

      <!-- Sorter -->
      <div class="sorter-wrap">
        <select v-model="selectedSort" @change="onSortChange" class="sort-select">
          <option value="created_desc">{{ t('audioLibrary.sortRecent') }}</option>
          <option value="duration_desc">{{ t('audioLibrary.sortDuration') }}</option>
          <option value="size_desc">{{ t('audioLibrary.sortSize') }}</option>
          <option value="name_asc">{{ t('audioLibrary.sortName') }}</option>
        </select>
        <button class="btn-refresh" @click="fetchAudios" :disabled="isLoading" :title="t('audioLibrary.refresh')">
          <svg class="icon-refresh" :class="{ 'spin-anim': isLoading }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
          </svg>
        </button>
      </div>
    </section>

    <!-- Main Content Area: Audio Items Table or Empty State -->
    <section class="audio-list-wrapper">
      <div v-if="audios.length === 0 && !isLoading" class="empty-audio-state">
        <div class="empty-icon-circle">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M9 18V5l12-2v13"></path>
            <circle cx="6" cy="18" r="3"></circle>
            <circle cx="18" cy="16" r="3"></circle>
          </svg>
        </div>
        <h3 class="empty-title">{{ t('audioLibrary.emptyTitle') }}</h3>
        <p class="empty-desc">{{ t('audioLibrary.emptyDesc') }}</p>
        <button class="btn-goto-library" @click="emit('navigateLibrary')">
          🚀 {{ t('audioLibrary.goToLibrary') }}
        </button>
      </div>

      <!-- Audio Table -->
      <div v-else class="audio-table-container">
        <table class="audio-table">
          <thead>
            <tr>
              <th class="th-play">#</th>
              <th class="th-name">{{ t('audioLibrary.colName') }}</th>
              <th class="th-fmt">{{ t('audioLibrary.colFormat') }}</th>
              <th class="th-dur font-mono">{{ t('audioLibrary.colDuration') }}</th>
              <th class="th-size font-mono">{{ t('audioLibrary.colSize') }}</th>
              <th class="th-date font-mono">{{ t('audioLibrary.colDate') }}</th>
              <th class="th-act">{{ t('audioLibrary.colActions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="item in audios"
              :key="item.id"
              class="audio-row"
              :class="{ 'is-playing-row': currentPlayingAudio?.id === item.id }"
              @dblclick="togglePlay(item)"
            >
              <!-- Play / Status Column -->
              <td class="td-play">
                <button
                  class="row-play-btn"
                  :class="{ active: currentPlayingAudio?.id === item.id && isAudioPlaying }"
                  @click="togglePlay(item)"
                  :title="currentPlayingAudio?.id === item.id && isAudioPlaying ? t('audioLibrary.pause') : t('audioLibrary.play')"
                >
                  <span v-if="currentPlayingAudio?.id === item.id && isAudioPlaying">⏸</span>
                  <span v-else>▶</span>
                </button>
              </td>

              <!-- Audio Name and Source Reference -->
              <td class="td-name">
                <div class="audio-title-wrap">
                  <span class="audio-title" :title="item.fileName">{{ item.fileName }}</span>
                  <div v-if="item.sourceVideoTitle" class="source-ref-line" :title="item.sourceVideoPath || ''">
                    <span class="source-tag">🎬 {{ t('audioLibrary.sourceFrom') }}</span>
                    <span class="source-name">{{ item.sourceVideoTitle }}</span>
                  </div>
                  <div class="path-sub font-mono" :title="item.filePath">{{ item.filePath }}</div>
                </div>
              </td>

              <!-- Format & Bitrate -->
              <td class="td-fmt">
                <div class="fmt-badge-group">
                  <span class="fmt-pill font-mono" :class="'fmt-' + item.format.toLowerCase()">
                    {{ item.format.toUpperCase() }}
                  </span>
                  <span v-if="item.qualityBitrate" class="bitrate-sub font-mono">
                    {{ item.qualityBitrate }}
                  </span>
                </div>
              </td>

              <!-- Duration -->
              <td class="td-dur font-mono">
                {{ formatDuration(item.duration) }}
              </td>

              <!-- Size -->
              <td class="td-size font-mono">
                {{ formatBytes(item.fileSize) }}
              </td>

              <!-- Date -->
              <td class="td-date font-mono">
                {{ formatDate(item.createdAt) }}
              </td>

              <!-- Quick Action Buttons -->
              <td class="td-act">
                <div class="row-act-btns">
                  <button class="act-btn" @click.stop="revealFile(item.filePath)" :title="t('audioLibrary.openFolder')">
                    📁
                  </button>
                  <button class="act-btn" @click.stop="copyPath(item.filePath)" :title="t('audioLibrary.copyPath')">
                    📋
                  </button>
                  <button class="act-btn act-btn-del" @click.stop="promptDelete(item)" :title="t('audioLibrary.deleteConfirmTitle')">
                    🗑️
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- Bottom Audio Player Dock Bar (Shown when an audio is actively loaded) -->
    <div v-if="currentPlayingAudio" class="audio-player-dock">
      <div class="dock-left">
        <button class="dock-play-btn" @click="togglePlay(currentPlayingAudio)">
          <span v-if="isAudioPlaying">⏸</span>
          <span v-else>▶</span>
        </button>
        <div class="dock-meta">
          <div class="dock-title" :title="currentPlayingAudio.fileName">{{ currentPlayingAudio.fileName }}</div>
          <div class="dock-sub font-mono">
            {{ currentPlayingAudio.format.toUpperCase() }} · {{ formatBytes(currentPlayingAudio.fileSize) }}
          </div>
        </div>
      </div>

      <!-- Progress Track -->
      <div class="dock-center">
        <span class="dock-time font-mono">{{ formatDuration(audioCurrentTime) }}</span>
        <input
          type="range"
          min="0"
          :max="audioDuration || 100"
          step="0.1"
          :value="audioCurrentTime"
          @input="onScrub"
          class="dock-scrubber"
        />
        <span class="dock-time font-mono">{{ formatDuration(audioDuration) }}</span>
      </div>

      <!-- Volume & Close -->
      <div class="dock-right">
        <span class="vol-icon">🔊</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          :value="audioVolume"
          @input="onVolumeChange"
          class="dock-vol-slider"
          :title="t('audioLibrary.volume')"
        />
        <button class="dock-close-btn" @click="closePlayer" :title="t('audioLibrary.closePlayer')">✕</button>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div v-if="showDeleteModal" class="confirm-modal-overlay" @click.self="showDeleteModal = false">
      <div class="confirm-modal-card">
        <div class="confirm-modal-header">
          <div class="confirm-icon-box">⚠️</div>
          <div class="confirm-header-text">
            <h3 class="confirm-title">{{ t('audioLibrary.deleteConfirmTitle') }}</h3>
            <p class="confirm-subtitle">{{ t('audioLibrary.deleteConfirmDesc', { name: audioToDelete?.fileName || '' }) }}</p>
          </div>
          <button class="confirm-close-btn" @click="showDeleteModal = false">✕</button>
        </div>

        <div class="confirm-modal-body">
          <div class="preview-box">
            <div class="preview-name font-mono">{{ audioToDelete?.fileName }}</div>
            <div class="preview-path font-mono text-muted">{{ audioToDelete?.filePath }}</div>
          </div>

          <label class="confirm-check-row">
            <input type="checkbox" v-model="alsoDeleteDiskFile" />
            <span>{{ t('audioLibrary.deleteDiskLabel') }}</span>
          </label>
        </div>

        <div class="confirm-modal-footer">
          <button class="btn btn-secondary" :disabled="isDeleting" @click="showDeleteModal = false">
            {{ t('audioLibrary.cancel') }}
          </button>
          <button class="btn btn-danger" :disabled="isDeleting" @click="confirmDelete">
            {{ isDeleting ? t('audioLibrary.deleting') : t('audioLibrary.confirmDelete') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Toast Notification -->
    <div v-if="toastMessage" class="toast-popup">
      {{ toastMessage }}
    </div>
  </div>
</template>

<style scoped>
.audio-library-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  overflow: hidden;
  background-color: var(--bg-app);
  position: relative;
}

/* Statistics Grid */
.stats-overview-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  padding: 1.25rem 1.5rem 0.75rem 1.5rem;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  padding: 1rem 1.25rem;
}

.stat-icon-wrap {
  width: 2.75rem;
  height: 2.75rem;
  border-radius: 0.65rem;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.stat-icon-wrap svg {
  width: 1.4rem;
  height: 1.4rem;
}

.icon-cyan {
  background: rgba(6, 182, 212, 0.12);
  color: #06b6d4;
  border: 1px solid rgba(6, 182, 212, 0.3);
}

.icon-purple {
  background: rgba(168, 85, 247, 0.12);
  color: #a855f7;
  border: 1px solid rgba(168, 85, 247, 0.3);
}

.icon-emerald {
  background: rgba(16, 185, 129, 0.12);
  color: #10b981;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.stat-content {
  display: flex;
  flex-direction: column;
}

.stat-label {
  font-size: 0.78rem;
  color: var(--text-muted);
}

.stat-value {
  font-size: 1.35rem;
  font-weight: 700;
  color: var(--text-primary);
  margin-top: 0.2rem;
}

.stat-unit {
  font-size: 0.8rem;
  font-weight: normal;
  color: var(--text-muted);
}

/* Control Bar */
.audio-control-bar {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem 1.5rem;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  margin-top: 0.5rem;
}

.search-input-wrap {
  position: relative;
  flex: 1;
  max-width: 380px;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: 0.75rem;
  width: 1rem;
  height: 1rem;
  color: var(--text-muted);
  pointer-events: none;
}

.audio-search-input {
  width: 100%;
  padding: 0.5rem 2rem 0.5rem 2.2rem;
  border-radius: 0.5rem;
  background: var(--bg-input, rgba(0, 0, 0, 0.25));
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 0.84rem;
  outline: none;
  transition: all 0.15s;
}

.audio-search-input:focus {
  border-color: #06b6d4;
  box-shadow: 0 0 0 2px rgba(6, 182, 212, 0.2);
}

.clear-btn {
  position: absolute;
  right: 0.6rem;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.8rem;
}

.format-pills {
  display: flex;
  gap: 0.4rem;
}

.pill-btn {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color);
  color: var(--text-muted);
  font-size: 0.76rem;
  font-weight: 500;
  padding: 0.4rem 0.8rem;
  border-radius: 9999px;
  cursor: pointer;
  transition: all 0.15s;
}

.pill-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.pill-btn.active {
  background: #06b6d4;
  border-color: #06b6d4;
  color: #fff;
  font-weight: 600;
}

.sorter-wrap {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.sort-select {
  background: var(--bg-input, rgba(0, 0, 0, 0.25));
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 0.8rem;
  padding: 0.45rem 0.75rem;
  border-radius: 0.5rem;
  outline: none;
}

.btn-refresh {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color);
  color: var(--text-muted);
  padding: 0.45rem;
  border-radius: 0.5rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-refresh:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.icon-refresh {
  width: 1rem;
  height: 1rem;
}

.spin-anim {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Audio List Area */
.audio-list-wrapper {
  flex: 1;
  overflow-y: auto;
  padding: 1rem 1.5rem 5rem 1.5rem;
}

/* Empty State */
.empty-audio-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 2rem;
  text-align: center;
}

.empty-icon-circle {
  width: 4rem;
  height: 4rem;
  border-radius: 50%;
  background: rgba(6, 182, 212, 0.1);
  border: 1px solid rgba(6, 182, 212, 0.25);
  color: #06b6d4;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
}

.empty-icon-circle svg {
  width: 2rem;
  height: 2rem;
}

.empty-title {
  font-size: 1.15rem;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.empty-desc {
  font-size: 0.85rem;
  color: var(--text-muted);
  margin: 0.5rem 0 1.5rem 0;
  max-width: 480px;
}

.btn-goto-library {
  background: linear-gradient(135deg, #06b6d4, #3b82f6);
  color: #fff;
  border: none;
  padding: 0.6rem 1.25rem;
  border-radius: 0.5rem;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(6, 182, 212, 0.3);
  transition: all 0.15s;
}

.btn-goto-library:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(6, 182, 212, 0.4);
}

/* Table Style */
.audio-table-container {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  overflow: hidden;
}

.audio-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
}

.audio-table th {
  padding: 0.75rem 1rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border-color);
  background: rgba(255, 255, 255, 0.02);
}

.audio-table td {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  vertical-align: middle;
}

.audio-row {
  transition: background 0.15s ease;
}

.audio-row:hover {
  background: rgba(255, 255, 255, 0.03);
}

.audio-row.is-playing-row {
  background: rgba(6, 182, 212, 0.06);
}

.th-play { width: 50px; text-align: center; }
.td-play { text-align: center; }

.row-play-btn {
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 0.8rem;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.row-play-btn:hover, .row-play-btn.active {
  background: #06b6d4;
  border-color: #06b6d4;
  color: #fff;
  box-shadow: 0 0 10px rgba(6, 182, 212, 0.4);
}

.audio-title-wrap {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  max-width: 420px;
}

.audio-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.source-ref-line {
  font-size: 0.72rem;
  color: #94a3b8;
  display: flex;
  align-items: center;
  gap: 0.3rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.source-tag {
  color: #06b6d4;
  font-weight: 500;
}

.path-sub {
  font-size: 0.68rem;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Badges */
.fmt-badge-group {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.fmt-pill {
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.15rem 0.45rem;
  border-radius: 0.3rem;
  background: rgba(255, 255, 255, 0.08);
}

.fmt-mp3 { color: #38bdf8; background: rgba(56, 189, 248, 0.15); border: 1px solid rgba(56, 189, 248, 0.3); }
.fmt-m4a { color: #a78bfa; background: rgba(167, 139, 250, 0.15); border: 1px solid rgba(167, 139, 250, 0.3); }
.fmt-wav { color: #34d399; background: rgba(52, 211, 153, 0.15); border: 1px solid rgba(52, 211, 153, 0.3); }
.fmt-flac { color: #fbbf24; background: rgba(251, 191, 36, 0.15); border: 1px solid rgba(251, 191, 36, 0.3); }

.bitrate-sub {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.td-dur, .td-size, .td-date {
  font-size: 0.78rem;
  color: var(--text-muted);
}

/* Action Buttons */
.row-act-btns {
  display: flex;
  gap: 0.35rem;
}

.act-btn {
  background: var(--bg-input, rgba(255, 255, 255, 0.05));
  border: 1px solid var(--border-color);
  color: var(--text-muted);
  padding: 0.35rem 0.55rem;
  border-radius: 0.4rem;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.15s;
}

.act-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.act-btn-del:hover {
  background: rgba(239, 68, 68, 0.15);
  border-color: rgba(239, 68, 68, 0.4);
  color: #ef4444;
}

/* Floating Audio Player Dock */
.audio-player-dock {
  position: absolute;
  bottom: 1rem;
  left: 1.5rem;
  right: 1.5rem;
  background: rgba(15, 23, 42, 0.95);
  backdrop-filter: blur(12px);
  border: 1px solid #06b6d4;
  border-radius: 1rem;
  padding: 0.75rem 1.25rem;
  box-shadow: 0 15px 30px rgba(0, 0, 0, 0.6), 0 0 15px rgba(6, 182, 212, 0.2);
  display: flex;
  align-items: center;
  gap: 1.25rem;
  z-index: 100;
  animation: slideUpDock 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes slideUpDock {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}

.dock-left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 220px;
  max-width: 280px;
}

.dock-play-btn {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: 50%;
  background: linear-gradient(135deg, #06b6d4, #3b82f6);
  border: none;
  color: #fff;
  font-size: 1rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 0 10px rgba(6, 182, 212, 0.4);
}

.dock-meta {
  overflow: hidden;
}

.dock-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dock-sub {
  font-size: 0.72rem;
  color: #94a3b8;
}

.dock-center {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.dock-time {
  font-size: 0.75rem;
  color: #cbd5e1;
  width: 40px;
  text-align: center;
}

.dock-scrubber {
  flex: 1;
  accent-color: #06b6d4;
  cursor: pointer;
}

.dock-right {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.vol-icon {
  font-size: 0.85rem;
}

.dock-vol-slider {
  width: 70px;
  accent-color: #06b6d4;
  cursor: pointer;
}

.dock-close-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  font-size: 1rem;
  cursor: pointer;
  padding: 0.3rem;
  border-radius: 0.3rem;
}

.dock-close-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
}

/* Delete Modal */
.confirm-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
}

.confirm-modal-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 0.85rem;
  width: 100%;
  max-width: 480px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
  overflow: hidden;
}

.confirm-modal-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1.15rem 1.25rem;
  border-bottom: 1px solid var(--border-color);
}

.confirm-icon-box {
  font-size: 1.35rem;
}

.confirm-header-text {
  flex: 1;
}

.confirm-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--text-primary);
}

.confirm-subtitle {
  margin: 0.2rem 0 0 0;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.confirm-close-btn {
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 1rem;
}

.confirm-modal-body {
  padding: 1.15rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.preview-box {
  background: rgba(0, 0, 0, 0.25);
  border: 1px solid var(--border-color);
  padding: 0.75rem;
  border-radius: 0.5rem;
}

.preview-name {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--text-primary);
}

.preview-path {
  font-size: 0.72rem;
  margin-top: 0.25rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.confirm-check-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8rem;
  color: #fca5a5;
  cursor: pointer;
}

.confirm-check-row input {
  accent-color: #ef4444;
}

.confirm-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.6rem;
  padding: 0.9rem 1.25rem;
  border-top: 1px solid var(--border-color);
  background: rgba(255, 255, 255, 0.02);
}

.btn {
  padding: 0.5rem 1rem;
  border-radius: 0.45rem;
  font-size: 0.82rem;
  font-weight: 500;
  cursor: pointer;
  border: none;
  transition: all 0.15s;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.12);
}

.btn-danger {
  background: #ef4444;
  color: #fff;
}

.btn-danger:hover {
  background: #dc2626;
}

/* Toast */
.toast-popup {
  position: absolute;
  top: 1.5rem;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid #06b6d4;
  color: #f1f5f9;
  padding: 0.5rem 1.2rem;
  border-radius: 9999px;
  font-size: 0.82rem;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
  z-index: 9999;
  pointer-events: none;
  animation: fadeIn 0.2s ease-out;
}
</style>
