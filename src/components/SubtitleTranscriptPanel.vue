<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import type { SubtitleSegment, AiTranscriptionConfig } from '../types';

const props = withDefaults(
  defineProps<{
    videoId: number;
    videoTitle?: string;
    currentTime: number;
    isPlaying?: boolean;
    compact?: boolean;
  }>(),
  {
    videoTitle: '',
    isPlaying: false,
    compact: false,
  }
);

const emit = defineEmits<{
  (e: 'seek', timeSeconds: number): void;
  (e: 'close'): void;
  (e: 'subtitlesUpdated', segments: SubtitleSegment[]): void;
}>();

const subtitles = ref<SubtitleSegment[]>([]);
const isLoading = ref<boolean>(false);
const isTranscribing = ref<boolean>(false);
const transcribeMessage = ref<string>('');
const transcribeElapsed = ref<number>(0);
let transcribeTimer: number | null = null;

const searchQuery = ref<string>('');
const autoScroll = ref<boolean>(true);
const toastMessage = ref<string>('');
let toastTimeout: number | null = null;

const showConfigModal = ref<boolean>(false);
const aiConfig = ref<AiTranscriptionConfig>({
  engine: 'api',
  api_url: 'https://api.openai.com/v1',
  api_key: '',
  model: 'whisper-1',
  language: 'auto',
  prompt: '',
  local_model_path: '',
});

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimeout) clearTimeout(toastTimeout);
  toastTimeout = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2500);
}

function formatTime(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  const mm = m.toString().padStart(2, '0');
  const ss = s.toString().padStart(2, '0');
  return `${mm}:${ss}`;
}

async function loadSubtitles() {
  if (!props.videoId) return;
  isLoading.value = true;
  try {
    const list = await invoke<SubtitleSegment[]>('get_video_subtitles', { videoId: props.videoId });
    subtitles.value = list || [];
    emit('subtitlesUpdated', subtitles.value);
  } catch (err) {
    console.warn('Failed to load subtitles:', err);
    subtitles.value = [];
  } finally {
    isLoading.value = false;
  }
}

async function loadAiConfig() {
  try {
    const cfg = await invoke<AiTranscriptionConfig>('get_ai_config');
    if (cfg) {
      aiConfig.value = cfg;
    }
  } catch (err) {
    console.warn('Failed to get ai config:', err);
  }
}

// Compute active segment matching current video playback time
const activeSegmentIndex = computed(() => {
  const currentMs = Math.round(props.currentTime * 1000);
  return subtitles.value.findIndex(
    (seg) => currentMs >= seg.start_time_ms && currentMs <= seg.end_time_ms
  );
});

// Filtered list based on transcript search query
const filteredSubtitles = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return subtitles.value;
  return subtitles.value.filter((seg) => seg.content.toLowerCase().includes(q));
});

// Auto-scroll to active segment
watch(
  () => activeSegmentIndex.value,
  async (idx) => {
    if (!autoScroll.value || idx < 0 || searchQuery.value.trim()) return;
    await nextTick();
    const el = document.getElementById(`sub-seg-${idx}`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }
);

watch(
  () => props.videoId,
  () => {
    loadSubtitles();
  }
);

function handleSeek(seg: SubtitleSegment) {
  const sec = seg.start_time_ms / 1000;
  emit('seek', Math.max(0, sec));
}

async function triggerTranscription() {
  if (!props.videoId) return;
  isTranscribing.value = true;
  transcribeMessage.value = t('ai.transcribingWhisper');
  transcribeElapsed.value = 0;
  if (transcribeTimer) clearInterval(transcribeTimer);
  transcribeTimer = window.setInterval(() => {
    transcribeElapsed.value += 1;
  }, 1000);

  try {
    const segs = await invoke<SubtitleSegment[]>('transcribe_video', {
      videoId: props.videoId,
      config: aiConfig.value,
    });
    subtitles.value = segs || [];
    emit('subtitlesUpdated', subtitles.value);
    showToast(t('subtitles.transcribeSuccess', { count: subtitles.value.length }));
    showConfigModal.value = false;
  } catch (err) {
    console.error('Transcription error:', err);
    showToast(t('subtitles.transcribeFailed', { err: err instanceof Error ? err.message : String(err) }));
  } finally {
    isTranscribing.value = false;
    if (transcribeTimer) {
      clearInterval(transcribeTimer);
      transcribeTimer = null;
    }
  }
}

async function handleDeleteSubtitles() {
  if (!confirm(t('subtitles.deleteConfirm'))) return;
  try {
    await invoke('delete_video_subtitles', { videoId: props.videoId });
    subtitles.value = [];
    emit('subtitlesUpdated', []);
    showToast(t('subtitles.cleared'));
  } catch (err) {
    showToast(t('subtitles.clearFailed', { err: String(err) }));
  }
}

function exportSubtitles(format: 'srt' | 'vtt') {
  if (subtitles.value.length === 0) {
    showToast(t('subtitles.emptyPrompt'));
    return;
  }

  let text = '';
  if (format === 'srt') {
    text = subtitles.value
      .map((seg, idx) => {
        const start = formatSrtTimestamp(seg.start_time_ms);
        const end = formatSrtTimestamp(seg.end_time_ms);
        return `${idx + 1}\n${start} --> ${end}\n${seg.content}\n`;
      })
      .join('\n');
  } else {
    text =
      'WEBVTT\n\n' +
      subtitles.value
        .map((seg, idx) => {
          const start = formatVttTimestamp(seg.start_time_ms);
          const end = formatVttTimestamp(seg.end_time_ms);
          return `${idx + 1}\n${start} --> ${end}\n${seg.content}\n`;
        })
        .join('\n');
  }

  const blob = new Blob([text], { type: 'text/plain;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  const safeName = (props.videoTitle || `video_${props.videoId}`).replace(/[/\\?%*:|"<>]/g, '_');
  a.href = url;
  a.download = `${safeName}.${format}`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  showToast(t('subtitles.exportSuccess', { format: format.toUpperCase() }));
}

function formatSrtTimestamp(ms: number): string {
  const h = Math.floor(ms / 3600000);
  const m = Math.floor((ms % 3600000) / 60000);
  const s = Math.floor((ms % 60000) / 1000);
  const rem = ms % 1000;
  return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')},${rem.toString().padStart(3, '0')}`;
}

function formatVttTimestamp(ms: number): string {
  const h = Math.floor(ms / 3600000);
  const m = Math.floor((ms % 3600000) / 60000);
  const s = Math.floor((ms % 60000) / 1000);
  const rem = ms % 1000;
  return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}.${rem.toString().padStart(3, '0')}`;
}

onMounted(() => {
  loadSubtitles();
  loadAiConfig();
});
</script>

<template>
  <div class="transcript-panel" :class="{ 'compact-mode': compact }">
    <!-- Panel Header -->
    <div class="panel-header">
      <div class="title-group">
        <span class="header-icon">🎙️</span>
        <div class="header-texts">
          <h4 class="header-title">{{ t('subtitles.title') }}</h4>
          <span class="sub-counter">
            {{ subtitles.length > 0 ? t('subtitles.lineCount', { count: subtitles.length }) : t('subtitles.noDialogue') }}
          </span>
        </div>
      </div>

      <div class="header-actions">
        <!-- Quick AI Transcribe Trigger -->
        <button
          v-if="!isTranscribing"
          class="btn-act-sub ai-glow"
          :title="subtitles.length > 0 ? t('subtitles.reExtract') : t('subtitles.quickAiTranscribe')"
          @click="showConfigModal = true"
        >
          <span class="btn-ic">✨</span>
          <span>{{ subtitles.length > 0 ? t('subtitles.reExtract') : t('subtitles.aiTranscribe') }}</span>
        </button>

        <!-- Export Dropdown -->
        <div v-if="subtitles.length > 0" class="export-group">
          <button class="btn-act-sub" :title="t('subtitles.exportSrt')" @click="exportSubtitles('srt')">
            SRT
          </button>
          <button class="btn-act-sub" :title="t('subtitles.exportVtt')" @click="exportSubtitles('vtt')">
            VTT
          </button>
          <button class="btn-act-sub danger" :title="t('subtitles.clearSubtitles')" @click="handleDeleteSubtitles">
            🗑️
          </button>
        </div>

        <button class="btn-close-sub" :title="t('subtitles.closeDrawer')" @click="emit('close')">
          ✕
        </button>
      </div>
    </div>

    <!-- Toolbar: Search & Auto-Scroll -->
    <div class="panel-toolbar">
      <div class="search-input-wrap">
        <span class="search-ic">🔍</span>
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('subtitles.searchPlaceholder')"
          class="search-sub-input"
        />
        <button
          v-if="searchQuery"
          class="clear-q-btn"
          :title="t('common.cancel')"
          @click="searchQuery = ''"
        >
          ✕
        </button>
      </div>

      <button
        class="btn-toggle-scroll"
        :class="{ active: autoScroll }"
        :title="autoScroll ? t('subtitles.followScrollOn') : t('subtitles.followScrollOff')"
        @click="autoScroll = !autoScroll"
      >
        <span class="scroll-dot" :class="{ pulsing: autoScroll && isPlaying }"></span>
        <span>{{ t('subtitles.followScroll') }}</span>
      </button>
    </div>

    <!-- Toast Banner -->
    <transition name="fade">
      <div v-if="toastMessage" class="toast-chip">
        {{ toastMessage }}
      </div>
    </transition>

    <!-- Transcribing Progress Card -->
    <div v-if="isTranscribing" class="transcribing-overlay-card">
      <div class="spinner-ai"></div>
      <div class="ai-status-text">
        <h5>{{ t('subtitles.transcribingOverlay') }}</h5>
        <p>{{ transcribeMessage }}</p>
        <span class="timer-badge">{{ t('subtitles.timeElapsed', { sec: transcribeElapsed }) }}</span>
      </div>
    </div>

    <!-- Subtitle Segments List -->
    <div class="segments-container" :class="{ 'is-loading': isLoading }">
      <div v-if="isLoading" class="loading-state">
        <div class="mini-spinner"></div>
        <span>{{ t('subtitles.readingTimeline') }}</span>
      </div>

      <div
        v-else-if="subtitles.length === 0 && !isTranscribing"
        class="empty-transcript-state"
      >
        <div class="empty-ai-icon">🎙️</div>
        <h4>{{ t('subtitles.emptyPrompt') }}</h4>
        <p>{{ t('subtitles.emptyPromptDesc') }}</p>
        <button class="btn-primary-ai" @click="showConfigModal = true">
          <span class="ic">✨</span> {{ t('subtitles.startExtractBtn') }}
        </button>
      </div>

      <div
        v-else-if="filteredSubtitles.length === 0"
        class="no-search-results"
      >
        <span>{{ t('subtitles.noSearchResults', { query: searchQuery }) }}</span>
      </div>

      <div v-else class="segments-list">
        <div
          v-for="(seg, idx) in filteredSubtitles"
          :id="`sub-seg-${idx}`"
          :key="seg.id || idx"
          class="segment-item"
          :class="{
            active: activeSegmentIndex === idx,
            'search-matched': !!searchQuery.trim(),
          }"
          @click="handleSeek(seg)"
        >
          <div class="seg-time-col">
            <span class="seg-badge-time">
              {{ formatTime(seg.start_time_ms) }}
            </span>
          </div>
          <div class="seg-content-col">
            <p class="seg-text">{{ seg.content }}</p>
          </div>
          <button class="btn-play-line" :title="t('subtitles.playFromHere')">
            ▶
          </button>
        </div>
      </div>
    </div>

    <!-- AI Transcribe Settings / Launch Modal -->
    <div v-if="showConfigModal" class="modal-layer-backdrop" @click.self="showConfigModal = false">
      <div class="ai-modal-box">
        <div class="ai-modal-header">
          <div class="ai-modal-title">
            <span>✨</span>
            <h4>{{ t('subtitles.modalTitle') }}</h4>
          </div>
          <button class="close-x" @click="showConfigModal = false">✕</button>
        </div>

        <div class="ai-modal-body">
          <p class="ai-tip">
            {{ t('subtitles.modalDesc') }}
          </p>

          <div class="form-group">
            <label class="form-label">{{ t('subtitles.enginePlan') }}</label>
            <div class="engine-pills">
              <button
                class="engine-pill"
                :class="{ active: aiConfig.engine === 'api' }"
                @click="aiConfig.engine = 'api'"
              >
                {{ t('subtitles.engineApiDesc') }}
              </button>
              <button
                class="engine-pill"
                :class="{ active: aiConfig.engine === 'local' }"
                @click="aiConfig.engine = 'local'"
              >
                {{ t('subtitles.engineLocalDesc') }}
              </button>
            </div>
          </div>

          <div v-if="aiConfig.engine === 'api'" class="api-fields">
            <div class="form-group">
              <label class="form-label">{{ t('subtitles.apiBaseUrl') }}</label>
              <input
                v-model="aiConfig.api_url"
                type="text"
                class="ai-input"
                placeholder="https://api.openai.com/v1"
              />
            </div>

            <div class="form-group">
              <label class="form-label">{{ t('subtitles.apiKey') }}</label>
              <input
                v-model="aiConfig.api_key"
                type="password"
                class="ai-input"
                placeholder="sk-..."
              />
            </div>

            <div class="form-group">
              <label class="form-label">{{ t('subtitles.modelLabel') }}</label>
              <input
                v-model="aiConfig.model"
                type="text"
                class="ai-input"
                placeholder="whisper-1 / whisper-large-v3 / base"
              />
            </div>
          </div>

          <div v-else class="local-fields">
            <div class="form-group">
              <label class="form-label">{{ t('subtitles.localModelPath') }}</label>
              <input
                v-model="aiConfig.local_model_path"
                type="text"
                class="ai-input"
                placeholder="ggml-base.bin"
              />
            </div>
          </div>

          <div class="form-row-grid">
            <div class="form-group">
              <label class="form-label">{{ t('subtitles.targetLanguage') }}</label>
              <select v-model="aiConfig.language" class="ai-select">
                <option value="auto">🌐 {{ t('ai.langAuto') }}</option>
                <option value="zh">🇨🇳 {{ t('ai.langZh') }}</option>
                <option value="en">🇺🇸 {{ t('ai.langEn') }}</option>
                <option value="ja">🇯🇵 {{ t('ai.langJa') }}</option>
                <option value="ko">🇰🇷 한국어 (Korean)</option>
                <option value="es">🇪🇸 Español (Spanish)</option>
                <option value="fr">🇫🇷 Français (French)</option>
                <option value="de">🇩🇪 Deutsch (German)</option>
              </select>
            </div>

            <div class="form-group">
              <label class="form-label">{{ t('subtitles.promptGuide') }}</label>
              <input
                v-model="aiConfig.prompt"
                type="text"
                class="ai-input"
                :placeholder="t('subtitles.promptPlaceholder')"
              />
            </div>
          </div>
        </div>

        <div class="ai-modal-footer">
          <button class="btn-cancel" @click="showConfigModal = false">{{ t('common.cancel') }}</button>
          <button class="btn-confirm-ai" @click="triggerTranscription">
            <span>{{ t('subtitles.launchBtn') }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.transcript-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  background: #14171f;
  border-left: 1px solid rgba(255, 255, 255, 0.08);
  color: #e6edf3;
  user-select: none;
  position: relative;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.85rem 1rem;
  background: rgba(255, 255, 255, 0.02);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.title-group {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.header-icon {
  font-size: 1.25rem;
}

.header-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: #fff;
}

.sub-counter {
  font-size: 0.72rem;
  color: #8b949e;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.btn-act-sub {
  padding: 0.25rem 0.6rem;
  font-size: 0.75rem;
  font-weight: 500;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.05);
  color: #c9d1d9;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  transition: all 0.2s ease;
}

.btn-act-sub:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
  border-color: rgba(255, 255, 255, 0.2);
}

.btn-act-sub.ai-glow {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(168, 85, 247, 0.2));
  border-color: rgba(139, 92, 246, 0.4);
  color: #c4b5fd;
}

.btn-act-sub.ai-glow:hover {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.35), rgba(168, 85, 247, 0.35));
  border-color: rgba(139, 92, 246, 0.7);
  color: #fff;
  box-shadow: 0 0 12px rgba(139, 92, 246, 0.4);
}

.btn-act-sub.danger:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.4);
  color: #f87171;
}

.export-group {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.btn-close-sub {
  background: none;
  border: none;
  color: #8b949e;
  font-size: 0.9rem;
  padding: 0.25rem 0.4rem;
  cursor: pointer;
  border-radius: 4px;
}

.btn-close-sub:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

/* Toolbar */
.panel-toolbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.85rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  background: rgba(0, 0, 0, 0.15);
}

.search-input-wrap {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 0.35rem;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  padding: 0.2rem 0.5rem;
}

.search-ic {
  font-size: 0.75rem;
  opacity: 0.6;
}

.search-sub-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  font-size: 0.78rem;
  color: #e6edf3;
}

.search-sub-input::placeholder {
  color: #6e7681;
}

.clear-q-btn {
  background: none;
  border: none;
  color: #8b949e;
  font-size: 0.75rem;
  cursor: pointer;
  padding: 0;
}

.btn-toggle-scroll {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.72rem;
  padding: 0.25rem 0.55rem;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: #8b949e;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-toggle-scroll.active {
  background: rgba(59, 130, 246, 0.15);
  border-color: rgba(59, 130, 246, 0.4);
  color: #60a5fa;
}

.scroll-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #6e7681;
}

.btn-toggle-scroll.active .scroll-dot {
  background: #3b82f6;
}

.scroll-dot.pulsing {
  box-shadow: 0 0 6px #3b82f6;
  animation: pulse-dot 1.2s infinite ease-in-out;
}

@keyframes pulse-dot {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.4); opacity: 0.6; }
}

/* Toast */
.toast-chip {
  position: absolute;
  top: 4.5rem;
  left: 50%;
  transform: translateX(-50%);
  background: #1f2937;
  border: 1px solid #374151;
  color: #f3f4f6;
  padding: 0.4rem 0.85rem;
  border-radius: 20px;
  font-size: 0.75rem;
  z-index: 20;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  white-space: nowrap;
}

/* Transcribing overlay banner */
.transcribing-overlay-card {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  padding: 0.85rem 1rem;
  margin: 0.6rem;
  background: linear-gradient(135deg, rgba(79, 70, 229, 0.15), rgba(147, 51, 234, 0.15));
  border: 1px solid rgba(139, 92, 246, 0.4);
  border-radius: 8px;
  animation: pulse-card 2s infinite ease-in-out;
}

@keyframes pulse-card {
  0%, 100% { border-color: rgba(139, 92, 246, 0.3); }
  50% { border-color: rgba(139, 92, 246, 0.7); box-shadow: 0 0 16px rgba(139, 92, 246, 0.2); }
}

.spinner-ai {
  width: 24px;
  height: 24px;
  border: 3px solid rgba(168, 85, 247, 0.2);
  border-top-color: #a855f7;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.ai-status-text h5 {
  margin: 0 0 0.15rem 0;
  font-size: 0.82rem;
  color: #e9d5ff;
}

.ai-status-text p {
  margin: 0;
  font-size: 0.72rem;
  color: #c4b5fd;
}

.timer-badge {
  display: inline-block;
  font-size: 0.68rem;
  color: #a78bfa;
  margin-top: 0.2rem;
}

/* Segments Container */
.segments-container {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem 0.6rem;
  scroll-behavior: smooth;
}

.loading-state, .no-search-results {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  height: 120px;
  font-size: 0.8rem;
  color: #8b949e;
}

.mini-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.empty-transcript-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 2.5rem 1.5rem;
  color: #8b949e;
}

.empty-ai-icon {
  font-size: 2.5rem;
  margin-bottom: 0.75rem;
  opacity: 0.8;
}

.empty-transcript-state h4 {
  margin: 0 0 0.4rem 0;
  font-size: 0.95rem;
  color: #e6edf3;
}

.empty-transcript-state p {
  margin: 0 0 1.25rem 0;
  font-size: 0.78rem;
  line-height: 1.4;
  color: #8b949e;
  max-width: 280px;
}

.btn-primary-ai {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.55rem 1.1rem;
  background: linear-gradient(135deg, #6366f1, #a855f7);
  border: none;
  border-radius: 8px;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
  transition: all 0.2s ease;
}

.btn-primary-ai:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(168, 85, 247, 0.4);
}

/* Segments list items */
.segments-list {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.segment-item {
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
  padding: 0.5rem 0.6rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
}

.segment-item:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.1);
}

.segment-item.active {
  background: linear-gradient(90deg, rgba(99, 102, 241, 0.22), rgba(168, 85, 247, 0.1));
  border-color: rgba(99, 102, 241, 0.5);
  box-shadow: inset 3px 0 0 #818cf8;
}

.segment-item.active .seg-badge-time {
  background: rgba(99, 102, 241, 0.4);
  color: #fff;
  font-weight: 600;
}

.segment-item.active .seg-text {
  color: #ffffff;
  font-weight: 500;
}

.seg-time-col {
  flex-shrink: 0;
  padding-top: 0.1rem;
}

.seg-badge-time {
  display: inline-block;
  padding: 0.12rem 0.35rem;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
  color: #8b949e;
  font-size: 0.68rem;
  font-family: ui-monospace, SFMono-Regular, monospace;
}

.seg-content-col {
  flex: 1;
}

.seg-text {
  margin: 0;
  font-size: 0.82rem;
  line-height: 1.45;
  color: #c9d1d9;
  word-break: break-word;
}

.btn-play-line {
  opacity: 0;
  background: none;
  border: none;
  color: #818cf8;
  font-size: 0.72rem;
  cursor: pointer;
  padding: 0.2rem;
  transition: opacity 0.15s ease;
}

.segment-item:hover .btn-play-line {
  opacity: 0.8;
}

/* Modal Backdrop & Dialog */
.modal-layer-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(6px);
  z-index: 1200;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.ai-modal-box {
  width: 100%;
  max-width: 540px;
  background: #161b22;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  box-shadow: 0 16px 36px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: popIn 0.2s ease-out;
}

@keyframes popIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}

.ai-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.ai-modal-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.ai-modal-title h4 {
  margin: 0;
  font-size: 1.05rem;
  color: #fff;
}

.close-x {
  background: none;
  border: none;
  color: #8b949e;
  font-size: 1rem;
  cursor: pointer;
}

.close-x:hover {
  color: #fff;
}

.ai-modal-body {
  padding: 1.25rem;
  max-height: 70vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.ai-tip {
  margin: 0;
  font-size: 0.78rem;
  line-height: 1.45;
  color: #8b949e;
  background: rgba(255, 255, 255, 0.03);
  padding: 0.65rem 0.85rem;
  border-radius: 6px;
  border-left: 3px solid #818cf8;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.form-label {
  font-size: 0.78rem;
  font-weight: 500;
  color: #c9d1d9;
}

.engine-pills {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.engine-pill {
  text-align: left;
  padding: 0.6rem 0.85rem;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.03);
  color: #c9d1d9;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.engine-pill:hover {
  border-color: rgba(255, 255, 255, 0.25);
  background: rgba(255, 255, 255, 0.06);
}

.engine-pill.active {
  background: rgba(99, 102, 241, 0.18);
  border-color: #6366f1;
  color: #e0e7ff;
  font-weight: 500;
}

.ai-input, .ai-select {
  padding: 0.55rem 0.75rem;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: #0d1117;
  color: #e6edf3;
  font-size: 0.82rem;
  outline: none;
}

.ai-input:focus, .ai-select:focus {
  border-color: #6366f1;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.form-row-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.ai-modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.6rem;
  padding: 0.85rem 1.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.15);
}

.btn-cancel {
  padding: 0.45rem 0.9rem;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #c9d1d9;
  font-size: 0.8rem;
  cursor: pointer;
}

.btn-confirm-ai {
  padding: 0.5rem 1.1rem;
  background: linear-gradient(135deg, #6366f1, #a855f7);
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}

.btn-confirm-ai:hover {
  filter: brightness(1.1);
}
</style>
