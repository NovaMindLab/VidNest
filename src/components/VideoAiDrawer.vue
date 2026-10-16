<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';
import type { Video, SubtitleSegment, AiAnalysis, AiTranscriptionConfig } from '../types';

const props = defineProps<{
  video: Video | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'play', video: Video, startSec?: number): void;
  (e: 'tagsUpdated'): void;
  (e: 'analysisUpdated'): void;
}>();

const activeTab = ref<'insights' | 'transcript' | 'settings'>('insights');
const analysis = ref<AiAnalysis | null>(null);
const subtitles = ref<SubtitleSegment[]>([]);
const isLoading = ref<boolean>(false);
const isAnalyzing = ref<boolean>(false);
const isTranscribing = ref<boolean>(false);
const transcribeElapsed = ref<number>(0);
let transcribeTimer: number | null = null;
const transcribeError = ref<string>('');
const analyzeError = ref<string>('');

const searchQuery = ref<string>('');
const toastMessage = ref<string>('');
let toastTimeout: number | null = null;

const aiConfig = ref<AiTranscriptionConfig>({
  engine: 'local',
  api_url: 'https://api.openai.com/v1',
  api_key: '',
  model: 'base',
  language: 'auto',
  prompt: '',
  local_model_path: '',
  llm_api_url: '',
  llm_api_key: '',
  llm_model: 'gpt-4o-mini',
});
const isSavingConfig = ref<boolean>(false);

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

function formatFileSize(bytes?: number): string {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`;
}

async function loadData() {
  if (!props.video?.id) {
    analysis.value = null;
    subtitles.value = [];
    return;
  }

  isLoading.value = true;
  try {
    const [analysisRes, subRes] = await Promise.all([
      invoke<AiAnalysis | null>('get_video_ai_analysis', { videoId: props.video.id }),
      invoke<SubtitleSegment[]>('get_video_subtitles', { videoId: props.video.id }),
    ]);
    analysis.value = analysisRes;
    subtitles.value = subRes || [];
  } catch (err) {
    console.error('Failed to load video AI data:', err);
  } finally {
    isLoading.value = false;
  }
}

async function loadAiConfig() {
  try {
    const cfg = await invoke<AiTranscriptionConfig>('get_ai_config');
    if (cfg) {
      aiConfig.value = {
        ...cfg,
        llm_api_url: cfg.llm_api_url || '',
        llm_api_key: cfg.llm_api_key || '',
        llm_model: cfg.llm_model || 'gpt-4o-mini',
      };
    }
  } catch (err) {
    console.warn('Failed to load ai config in drawer:', err);
  }
}

async function saveConfig() {
  isSavingConfig.value = true;
  try {
    await invoke('save_ai_config', { config: aiConfig.value });
    showToast(t('ai.configSavedToast'));
  } catch (err) {
    showToast(t('ai.saveConfigFailed', { err: String(err) }));
  } finally {
    isSavingConfig.value = false;
  }
}

function applyPreset(preset: 'openai' | 'deepseek' | 'groq' | 'ollama' | 'local') {
  if (preset === 'openai') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'https://api.openai.com/v1';
    aiConfig.value.model = 'whisper-1';
    aiConfig.value.llm_api_url = 'https://api.openai.com/v1';
    aiConfig.value.llm_model = 'gpt-4o-mini';
  } else if (preset === 'deepseek') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'https://api.deepseek.com/v1';
    aiConfig.value.model = 'whisper-1';
    aiConfig.value.llm_api_url = 'https://api.deepseek.com/v1';
    aiConfig.value.llm_model = 'deepseek-chat';
  } else if (preset === 'groq') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'https://api.groq.com/openai/v1';
    aiConfig.value.model = 'whisper-large-v3';
    aiConfig.value.llm_api_url = 'https://api.groq.com/openai/v1';
    aiConfig.value.llm_model = 'llama-3.3-70b-versatile';
  } else if (preset === 'ollama') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'http://localhost:11434/v1';
    aiConfig.value.model = 'whisper';
    aiConfig.value.llm_api_url = 'http://localhost:11434/v1';
    aiConfig.value.llm_model = 'qwen2.5:7b';
  } else if (preset === 'local') {
    aiConfig.value.engine = 'local';
    aiConfig.value.model = 'base';
    aiConfig.value.llm_api_url = '';
  }
}

async function handleGenerateAnalysis(force: boolean = false) {
  if (!props.video?.id) return;
  isAnalyzing.value = true;
  analyzeError.value = '';
  try {
    const res = await invoke<AiAnalysis>('generate_video_ai_analysis', {
      videoId: props.video.id,
      force,
    });
    analysis.value = res;
    transcribeError.value = '';
    emit('analysisUpdated');
    showToast(t('ai.summaryGeneratedToast'));
  } catch (err) {
    const msg = String(err);
    analyzeError.value = msg;
    showToast(t('ai.summaryGenerateFailed', { err: msg }));
  } finally {
    isAnalyzing.value = false;
  }
}

async function handleTranscribeVideo() {
  if (!props.video?.id) return;
  isTranscribing.value = true;
  transcribeError.value = '';
  transcribeElapsed.value = 0;
  if (transcribeTimer) clearInterval(transcribeTimer);
  transcribeTimer = window.setInterval(() => {
    transcribeElapsed.value += 1;
  }, 1000);

  try {
    const segs = await invoke<SubtitleSegment[]>('transcribe_video', {
      videoId: props.video.id,
      config: aiConfig.value,
    });
    subtitles.value = segs || [];
    showToast(t('ai.transcribeSuccessToast', { count: subtitles.value.length }));

    // Auto trigger analysis upon transcription completion
    try {
      const res = await invoke<AiAnalysis>('generate_video_ai_analysis', {
        videoId: props.video.id,
        force: true,
      });
      analysis.value = res;
      emit('analysisUpdated');
      showToast(t('ai.allGeneratedToast'));
    } catch (aErr) {
      console.warn('Auto analysis error:', aErr);
    }
  } catch (err) {
    const msg = String(err);
    transcribeError.value = msg;
    showToast(t('ai.transcribeFailed', { err: msg }));
  } finally {
    isTranscribing.value = false;
    if (transcribeTimer) {
      clearInterval(transcribeTimer);
      transcribeTimer = null;
    }
  }
}

async function handleApplyTags() {
  if (!props.video?.id || !analysis.value?.tags?.length) return;
  try {
    await invoke('apply_ai_tags_to_video', {
      videoId: props.video.id,
      tags: analysis.value.tags,
    });
    emit('tagsUpdated');
    showToast(t('ai.tagsBoundToast', { count: analysis.value.tags.length }));
  } catch (err) {
    showToast(t('ai.bindTagsFailed', { err: String(err) }));
  }
}

function handlePlayAt(timestampMs: number) {
  if (!props.video) return;
  const sec = Math.max(0, timestampMs / 1000);
  emit('play', props.video, sec);
}

const filteredSubtitles = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return subtitles.value;
  return subtitles.value.filter((seg) => seg.content.toLowerCase().includes(q));
});

function exportSubtitles(format: 'srt' | 'vtt') {
  if (subtitles.value.length === 0) {
    showToast(t('ai.noSubtitlesToExport'));
    return;
  }
  let content = '';
  if (format === 'srt') {
    content = subtitles.value
      .map((seg, idx) => {
        const start = formatTime(seg.start_time_ms) + ',000';
        const end = formatTime(seg.end_time_ms) + ',000';
        return `${idx + 1}\n00:${start} --> 00:${end}\n${seg.content}\n`;
      })
      .join('\n');
  } else {
    content = 'WEBVTT\n\n' + subtitles.value
      .map((seg) => {
        const start = formatTime(seg.start_time_ms) + '.000';
        const end = formatTime(seg.end_time_ms) + '.000';
        return `00:${start} --> 00:${end}\n${seg.content}\n`;
      })
      .join('\n');
  }

  const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  const baseName = props.video?.file_name.replace(/\.[^/.]+$/, '') || 'transcript';
  a.download = `${baseName}.${format}`;
  a.click();
  URL.revokeObjectURL(url);
  showToast(t('ai.exportSuccess', { format: format.toUpperCase() }));
}

watch(
  () => [props.visible, props.video?.id],
  ([visible]) => {
    if (visible && props.video) {
      activeTab.value = 'insights';
      transcribeError.value = '';
      analyzeError.value = '';
      loadData();
      loadAiConfig();
    }
  },
  { immediate: true }
);

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.visible) {
    emit('close');
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
  if (transcribeTimer) clearInterval(transcribeTimer);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="ai-drawer-backdrop" @click="emit('close')">
    <div class="ai-drawer-panel" @click.stop>
      <!-- Drawer Header -->
      <div class="drawer-header">
        <div class="header-main">
          <div class="header-badge">
            <span class="ai-spark-ic">✨</span>
            <span class="ai-badge-text font-mono">VidNest AI Intelligence</span>
          </div>
          <h2 class="drawer-title" :title="video?.file_name">
            {{ video?.file_name || t('ai.hubTitle') }}
          </h2>
          <div class="drawer-meta font-mono" v-if="video">
            <span v-if="video.duration">⏱️ {{ formatTime((video.duration || 0) * 1000) }}</span>
            <span v-if="video.width && video.height">📐 {{ video.width }}x{{ video.height }}</span>
            <span v-if="video.codec">🎞️ {{ video.codec }}</span>
            <span>💾 {{ formatFileSize(video.file_size) }}</span>
          </div>
        </div>

        <div class="header-actions">
          <button
            v-if="video"
            class="hdr-play-btn"
            :title="t('ai.playVideo')"
            @click="emit('play', video, 0)"
          >
            {{ t('ai.playVideo') }}
          </button>
          <button class="hdr-close-btn" :title="t('common.close')" @click="emit('close')">✕</button>
        </div>
      </div>

      <!-- Drawer Tabs -->
      <div class="drawer-nav-tabs">
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'insights' }"
          @click="activeTab = 'insights'"
        >
          {{ t('ai.tabInsights') }}
          <span v-if="analysis" class="tab-indicator-dot"></span>
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'transcript' }"
          @click="activeTab = 'transcript'"
        >
          {{ t('ai.tabTranscript') }} ({{ subtitles.length }})
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'settings' }"
          @click="activeTab = 'settings'"
        >
          {{ t('ai.tabSettings') }}
        </button>
      </div>

      <!-- Toast Notification -->
      <transition name="fade">
        <div v-if="toastMessage" class="drawer-toast">
          {{ toastMessage }}
        </div>
      </transition>

      <!-- Drawer Body Content -->
      <div class="drawer-body">
        <!-- Loading State -->
        <div v-if="isLoading" class="drawer-center-state">
          <span class="spinner-lg"></span>
          <p>{{ t('ai.loadingAssets') }}</p>
        </div>

        <!-- 1. AI Insights & Summary Tab -->
        <div v-else-if="activeTab === 'insights'" class="tab-content insights-container">
          <!-- Transcribing / Analyzing Progress Banner -->
          <div v-if="isTranscribing" class="ai-running-banner transcribing">
            <span class="spinner-md"></span>
            <div class="running-info">
              <h4>{{ t('ai.transcribingWhisper') }} {{ t('ai.transcribeElapsed', { sec: transcribeElapsed }) }}</h4>
              <p>{{ t('ai.transcribeDesc') }}</p>
            </div>
          </div>

          <div v-else-if="isAnalyzing" class="ai-running-banner analyzing">
            <span class="spinner-md"></span>
            <div class="running-info">
              <h4>{{ t('ai.analyzingLlm') }}</h4>
              <p>{{ t('ai.analyzingDesc') }}</p>
            </div>
          </div>

          <!-- Analysis Exists -->
          <template v-if="analysis">
            <!-- Tip Banner when analysis exists but speech not yet transcribed -->
            <div v-if="subtitles.length === 0" class="subtitles-transcribe-tip-banner">
              <div class="tip-left">
                <span class="tip-spark">💡</span>
                <div class="tip-texts">
                  <span class="tip-title">{{ t('ai.metadataAnalysisTipTitle') }}</span>
                  <span class="tip-sub">{{ t('ai.metadataAnalysisTipSub') }}</span>
                </div>
              </div>
              <button
                class="tip-cta-btn"
                :disabled="isTranscribing"
                @click="handleTranscribeVideo"
              >
                <span v-if="isTranscribing" class="btn-spinner mini"></span>
                <span>{{ isTranscribing ? t('ai.transcribing') : t('ai.startTranscribeBtn') }}</span>
              </button>
            </div>

            <!-- Summary Card -->
            <div class="card-glass summary-card">
              <div class="card-sec-header">
                <div class="sec-title-wrap">
                  <span class="sec-ic">📝</span>
                  <span class="sec-title">{{ t('ai.summaryTitle') }}</span>
                </div>
                <div class="sec-actions">
                  <span class="model-badge font-mono">{{ analysis.model_name }}</span>
                  <button
                    class="act-pill-btn"
                    :disabled="isAnalyzing"
                    :title="t('ai.reAnalyzeTitle')"
                    @click="handleGenerateAnalysis(true)"
                  >
                    {{ t('ai.reAnalyze') }}
                  </button>
                </div>
              </div>
              <div class="summary-body">
                <p class="summary-text">{{ analysis.summary }}</p>
              </div>
            </div>

            <!-- Key Topics / Timeline Chapters -->
            <div class="card-glass topics-card">
              <div class="card-sec-header">
                <div class="sec-title-wrap">
                  <span class="sec-ic">✨</span>
                  <span class="sec-title">{{ t('ai.topicsTitle') }}</span>
                  <span class="sec-subtitle">{{ t('ai.topicsSubtitle') }}</span>
                </div>
              </div>

              <div class="topics-timeline">
                <div
                  v-for="(item, idx) in analysis.topics"
                  :key="idx"
                  class="topic-node-item"
                  @click="handlePlayAt(item.timestamp_ms)"
                >
                  <div class="timeline-indicator">
                    <span class="dot"></span>
                    <span class="line" v-if="idx < analysis.topics.length - 1"></span>
                  </div>
                  <div class="topic-content-box">
                    <div class="topic-top-bar">
                      <span class="topic-time-badge font-mono" :title="t('ai.playAt')">
                        ▶ {{ formatTime(item.timestamp_ms) }}
                      </span>
                      <span class="topic-node-title">{{ item.title }}</span>
                    </div>
                    <p class="topic-point-text">{{ item.point }}</p>
                  </div>
                </div>
              </div>
            </div>

            <!-- AI Keywords & Recommended Tags -->
            <div class="card-glass tags-card">
              <div class="card-sec-header">
                <div class="sec-title-wrap">
                  <span class="sec-ic">🏷️</span>
                  <span class="sec-title">{{ t('ai.tagsTitle') }}</span>
                </div>
                <button
                  v-if="analysis.tags && analysis.tags.length"
                  class="apply-tags-btn"
                  :title="t('ai.tagsSubtitle')"
                  @click="handleApplyTags"
                >
                  {{ t('ai.applyTagsBtn') }}
                </button>
              </div>

              <div class="tags-clusters">
                <div class="tag-group" v-if="analysis.tags && analysis.tags.length">
                  <span class="group-label">{{ t('ai.tagsTitle') }}:</span>
                  <div class="pill-list">
                    <span
                      v-for="(tItem, idx) in analysis.tags"
                      :key="'tag-' + idx"
                      class="ai-tag-pill"
                    >
                      #{{ tItem }}
                    </span>
                  </div>
                </div>

                <div class="tag-group" v-if="analysis.keywords && analysis.keywords.length">
                  <span class="group-label">Keywords:</span>
                  <div class="pill-list">
                    <span
                      v-for="(k, idx) in analysis.keywords"
                      :key="'kw-' + idx"
                      class="ai-kw-pill font-mono"
                    >
                      {{ k }}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </template>

          <!-- No Analysis Yet -->
          <div v-else-if="!isTranscribing && !isAnalyzing" class="ai-empty-prompt card-glass">
            <div class="empty-spark-icon">✨</div>
            <h3>{{ subtitles.length > 0 ? t('ai.noAnalysis') : t('ai.noTranscript') }}</h3>
            <p>
              {{ subtitles.length > 0 ? t('ai.noAnalysisDesc') : t('ai.noTranscriptDesc') }}
            </p>

            <!-- Inline Error & Setup Guidance Banner if Transcribe or Analyze Failed -->
            <div v-if="transcribeError" class="transcribe-err-banner">
              <div class="err-header">
                <span class="err-icon">⚠️</span>
                <span class="err-title">{{ t('ai.transcribeNoticeTitle') }}</span>
              </div>
              <p class="err-desc">{{ transcribeError }}</p>
              <div class="err-actions">
                <button class="btn-config-ai" @click="activeTab = 'settings'">
                  ⚙️ {{ t('ai.gotoSettings') }}
                </button>
                <button class="btn-local-fallback" @click="handleGenerateAnalysis(true)">
                  ✨ {{ t('ai.fallbackLocalSummary') }}
                </button>
              </div>
            </div>

            <div class="empty-prompt-actions">
              <!-- Instant local analysis (100% works, zero setup) -->
              <button
                class="cta-primary-btn"
                :disabled="isAnalyzing"
                @click="handleGenerateAnalysis(false)"
              >
                <span v-if="isAnalyzing" class="btn-spinner mini"></span>
                <span>{{ isAnalyzing ? t('ai.analyzing') : t('ai.generateSummaryBtn') }}</span>
              </button>

              <!-- Speech transcription -->
              <button
                class="cta-secondary-btn"
                :disabled="isTranscribing"
                @click="handleTranscribeVideo"
              >
                <span v-if="isTranscribing" class="btn-spinner mini"></span>
                <span>{{ isTranscribing ? t('ai.transcribing') : t('ai.startTranscribeBtn') }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- 2. Subtitle Transcript Tab -->
        <div v-else-if="activeTab === 'transcript'" class="tab-content transcript-container">
          <div class="transcript-toolbar">
            <div class="search-input-box">
              <span class="search-ic">🔍</span>
              <input
                v-model="searchQuery"
                type="text"
                class="ts-search-input"
                :placeholder="t('ai.searchDialoguePlaceholder')"
              />
              <button
                v-if="searchQuery"
                class="clear-input-btn"
                @click="searchQuery = ''"
              >
                ✕
              </button>
            </div>

            <div class="tb-actions">
              <button
                class="tb-act-btn"
                :title="t('ai.exportSrtTitle')"
                :disabled="!subtitles.length"
                @click="exportSubtitles('srt')"
              >
                📥 {{ t('ai.exportSrt') }}
              </button>
              <button
                class="tb-act-btn"
                :title="t('ai.exportVttTitle')"
                :disabled="!subtitles.length"
                @click="exportSubtitles('vtt')"
              >
                📥 {{ t('ai.exportVtt') }}
              </button>
              <button
                class="tb-act-btn btn-regen"
                :title="t('ai.reTranscribeTitle')"
                :disabled="isTranscribing"
                @click="handleTranscribeVideo"
              >
                🎙️ {{ t('ai.reTranscribe') }}
              </button>
            </div>
          </div>

          <div v-if="filteredSubtitles.length > 0" class="subtitles-scroll-list">
            <div
              v-for="seg in filteredSubtitles"
              :key="seg.id || seg.start_time_ms"
              class="sub-item-row"
              @click="handlePlayAt(seg.start_time_ms)"
            >
              <div class="sub-timestamp font-mono">
                <span class="jump-ic">▶</span>
                <span>{{ formatTime(seg.start_time_ms) }}</span>
              </div>
              <div class="sub-text">
                {{ seg.content }}
              </div>
            </div>
          </div>

          <div v-else-if="subtitles.length === 0" class="no-sub-notice">
            <span class="ic">🎙️</span>
            <p>{{ t('ai.noSubNotice') }}</p>

            <!-- Inline Error & Setup Guidance Banner if Transcribe Failed -->
            <div v-if="transcribeError" class="transcribe-err-banner">
              <div class="err-header">
                <span class="err-icon">⚠️</span>
                <span class="err-title">{{ t('ai.transcribeNoticeTitle') }}</span>
              </div>
              <p class="err-desc">{{ transcribeError }}</p>
              <div class="err-actions">
                <button class="btn-config-ai" @click="activeTab = 'settings'">
                  ⚙️ {{ t('ai.gotoSettings') }}
                </button>
                <button class="btn-local-fallback" @click="handleGenerateAnalysis(true)">
                  ✨ {{ t('ai.fallbackLocalSummary') }}
                </button>
              </div>
            </div>

            <div class="no-sub-actions">
              <button class="cta-primary-btn compact" :disabled="isTranscribing" @click="handleTranscribeVideo">
                {{ isTranscribing ? t('ai.transcribing') : t('ai.startTranscribeNow') }}
              </button>
              <button class="cta-secondary-btn compact" @click="activeTab = 'insights'">
                ✨ {{ t('ai.generateSummaryBtn') }}
              </button>
            </div>
          </div>

          <div v-else class="no-sub-notice">
            <p>{{ t('ai.noSearchResults', { query: searchQuery }) }}</p>
          </div>
        </div>

        <!-- 3. AI Settings Tab -->
        <div v-else-if="activeTab === 'settings'" class="tab-content settings-container">
          <div class="card-glass config-card">
            <h3 class="config-sec-title">{{ t('ai.presetTitle') }}</h3>
            <div class="preset-buttons-row">
              <button class="preset-btn" @click="applyPreset('local')">
                {{ t('ai.presetLocal') }}
              </button>
              <button class="preset-btn" @click="applyPreset('ollama')">
                {{ t('ai.presetOllama') }}
              </button>
              <button class="preset-btn" @click="applyPreset('deepseek')">
                {{ t('ai.presetDeepseek') }}
              </button>
              <button class="preset-btn" @click="applyPreset('openai')">
                {{ t('ai.presetOpenai') }}
              </button>
              <button class="preset-btn" @click="applyPreset('groq')">
                {{ t('ai.presetGroq') }}
              </button>
            </div>

            <div class="config-divider"></div>

            <h3 class="config-sec-title">{{ t('ai.whisperSectionTitle') }}</h3>
            <div class="form-grid">
              <div class="form-group">
                <label>{{ t('ai.engineType') }}</label>
                <select v-model="aiConfig.engine" class="form-select">
                  <option value="api">{{ t('ai.engineApi') }}</option>
                  <option value="local">{{ t('ai.engineLocal') }}</option>
                </select>
              </div>

              <div class="form-group">
                <label>{{ t('ai.langMode') }}</label>
                <select v-model="aiConfig.language" class="form-select">
                  <option value="auto">{{ t('ai.langAuto') }}</option>
                  <option value="zh">{{ t('ai.langZh') }}</option>
                  <option value="en">{{ t('ai.langEn') }}</option>
                  <option value="ja">{{ t('ai.langJa') }}</option>
                </select>
              </div>

              <div class="form-group full-width" v-if="aiConfig.engine === 'api'">
                <label>{{ t('ai.apiUrl') }}</label>
                <input
                  v-model="aiConfig.api_url"
                  type="text"
                  class="form-input font-mono"
                  placeholder="https://api.openai.com/v1"
                />
              </div>

              <div class="form-group full-width" v-if="aiConfig.engine === 'api'">
                <label>{{ t('ai.apiKey') }}</label>
                <input
                  v-model="aiConfig.api_key"
                  type="password"
                  class="form-input font-mono"
                  placeholder="sk-... (Ollama / Local: optional)"
                />
              </div>

              <div class="form-group" v-if="aiConfig.engine === 'api'">
                <label>{{ t('ai.whisperModel') }}</label>
                <input
                  v-model="aiConfig.model"
                  type="text"
                  class="form-input font-mono"
                  placeholder="whisper-1 / whisper-large-v3"
                />
              </div>
            </div>

            <div class="config-divider"></div>

            <h3 class="config-sec-title">{{ t('ai.llmSectionTitle') }}</h3>
            <p class="config-hint">
              {{ t('ai.llmHint') }}
            </p>
            <div class="form-grid">
              <div class="form-group full-width">
                <label>{{ t('ai.llmApiUrl') }}</label>
                <input
                  v-model="aiConfig.llm_api_url"
                  type="text"
                  class="form-input font-mono"
                  placeholder="https://api.openai.com/v1, http://localhost:11434/v1"
                />
              </div>

              <div class="form-group full-width">
                <label>{{ t('ai.llmApiKey') }}</label>
                <input
                  v-model="aiConfig.llm_api_key"
                  type="password"
                  class="form-input font-mono"
                  placeholder="sk-..."
                />
              </div>

              <div class="form-group">
                <label>{{ t('ai.llmModel') }}</label>
                <input
                  v-model="aiConfig.llm_model"
                  type="text"
                  class="form-input font-mono"
                  placeholder="gpt-4o-mini / deepseek-chat / qwen2.5:7b"
                />
              </div>
            </div>

            <div class="form-submit-row">
              <button
                class="cta-primary-btn"
                :disabled="isSavingConfig"
                @click="saveConfig"
              >
                {{ isSavingConfig ? t('ai.saving') : t('ai.saveAiConfig') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
  </Teleport>
</template>

<style scoped>
.ai-drawer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 99999;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(8px);
  display: flex;
  justify-content: flex-end;
  animation: backdropFadeIn 0.25s ease-out;
}

@keyframes backdropFadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.ai-drawer-panel {
  width: 580px;
  max-width: 90vw;
  height: 100vh;
  background: #0f172a;
  border-left: 1px solid rgba(255, 255, 255, 0.12);
  display: flex;
  flex-direction: column;
  box-shadow: -10px 0 35px rgba(0, 0, 0, 0.6);
  animation: drawerSlideIn 0.28s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes drawerSlideIn {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}

/* Header */
.drawer-header {
  padding: 1.25rem 1.5rem 1rem;
  background: linear-gradient(180deg, rgba(30, 41, 59, 0.8) 0%, rgba(15, 23, 42, 0.95) 100%);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.header-main {
  flex: 1;
  min-width: 0;
}

.header-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.2rem 0.6rem;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(168, 85, 247, 0.2));
  border: 1px solid rgba(139, 92, 246, 0.35);
  border-radius: 999px;
  margin-bottom: 0.5rem;
}

.ai-spark-ic {
  font-size: 0.85rem;
}

.ai-badge-text {
  font-size: 0.72rem;
  font-weight: 600;
  color: #a5b4fc;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.drawer-title {
  font-size: 1.05rem;
  font-weight: 600;
  color: #f8fafc;
  margin: 0 0 0.35rem 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.drawer-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  font-size: 0.72rem;
  color: #94a3b8;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.hdr-play-btn {
  background: linear-gradient(135deg, #4f46e5, #7c3aed);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 0.4rem 0.8rem;
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}

.hdr-play-btn:hover {
  filter: brightness(1.15);
  transform: translateY(-1px);
}

.hdr-close-btn {
  background: rgba(255, 255, 255, 0.06);
  color: #94a3b8;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  width: 32px;
  height: 32px;
  font-size: 1rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.hdr-close-btn:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
  border-color: rgba(239, 68, 68, 0.4);
}

/* Tabs */
.drawer-nav-tabs {
  display: flex;
  padding: 0 1.25rem;
  background: rgba(15, 23, 42, 0.9);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.tab-btn {
  position: relative;
  background: none;
  border: none;
  padding: 0.75rem 1rem;
  color: #94a3b8;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.4rem;
  transition: all 0.15s;
}

.tab-btn:hover {
  color: #e2e8f0;
}

.tab-btn.active {
  color: #818cf8;
}

.tab-btn.active::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0.75rem;
  right: 0.75rem;
  height: 2px;
  background: #6366f1;
  border-radius: 2px;
}

.tab-indicator-dot {
  width: 6px;
  height: 6px;
  background: #10b981;
  border-radius: 50%;
  display: inline-block;
}

/* Drawer Body */
.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.card-glass {
  background: rgba(30, 41, 59, 0.5);
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 12px;
  padding: 1.15rem;
}

.card-sec-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.85rem;
}

.sec-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.sec-ic {
  font-size: 1rem;
}

.sec-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: #f1f5f9;
}

.sec-subtitle {
  font-size: 0.7rem;
  color: #64748b;
  margin-left: 0.35rem;
}

.sec-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.model-badge {
  background: rgba(99, 102, 241, 0.15);
  color: #a5b4fc;
  border: 1px solid rgba(99, 102, 241, 0.3);
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
  font-size: 0.7rem;
}

.act-pill-btn {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  border-radius: 6px;
  padding: 0.2rem 0.6rem;
  font-size: 0.72rem;
  cursor: pointer;
  transition: all 0.15s;
}

.act-pill-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

/* Summary Card */
.summary-text {
  font-size: 0.85rem;
  line-height: 1.6;
  color: #cbd5e1;
  margin: 0;
  white-space: pre-wrap;
}

/* Topics Timeline */
.topics-timeline {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  margin-top: 0.5rem;
}

.topic-node-item {
  display: flex;
  gap: 0.85rem;
  cursor: pointer;
  padding: 0.5rem 0.65rem;
  border-radius: 8px;
  background: rgba(15, 23, 42, 0.3);
  border: 1px solid transparent;
  transition: all 0.2s;
}

.topic-node-item:hover {
  background: rgba(99, 102, 241, 0.1);
  border-color: rgba(99, 102, 241, 0.3);
  transform: translateX(4px);
}

.timeline-indicator {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 14px;
  padding-top: 0.25rem;
}

.timeline-indicator .dot {
  width: 8px;
  height: 8px;
  background: #6366f1;
  border-radius: 50%;
  box-shadow: 0 0 8px #6366f1;
}

.timeline-indicator .line {
  flex: 1;
  width: 2px;
  background: rgba(99, 102, 241, 0.2);
  margin-top: 4px;
}

.topic-content-box {
  flex: 1;
  min-width: 0;
}

.topic-top-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
}

.topic-time-badge {
  background: rgba(99, 102, 241, 0.2);
  color: #818cf8;
  border: 1px solid rgba(99, 102, 241, 0.35);
  border-radius: 4px;
  padding: 0.1rem 0.4rem;
  font-size: 0.72rem;
  font-weight: 600;
  transition: all 0.15s;
}

.topic-node-item:hover .topic-time-badge {
  background: #6366f1;
  color: #fff;
}

.topic-node-title {
  font-size: 0.82rem;
  font-weight: 600;
  color: #f1f5f9;
}

.topic-point-text {
  font-size: 0.78rem;
  color: #94a3b8;
  line-height: 1.45;
  margin: 0;
}

/* Tags Card */
.tags-clusters {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.tag-group {
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
}

.group-label {
  font-size: 0.75rem;
  color: #64748b;
  white-space: nowrap;
  padding-top: 0.15rem;
}

.pill-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.ai-tag-pill {
  background: rgba(16, 185, 129, 0.12);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.25);
  border-radius: 6px;
  padding: 0.15rem 0.5rem;
  font-size: 0.75rem;
}

.ai-kw-pill {
  background: rgba(14, 165, 233, 0.12);
  color: #38bdf8;
  border: 1px solid rgba(14, 165, 233, 0.25);
  border-radius: 6px;
  padding: 0.15rem 0.5rem;
  font-size: 0.72rem;
}

.apply-tags-btn {
  background: rgba(16, 185, 129, 0.2);
  border: 1px solid rgba(16, 185, 129, 0.4);
  color: #6ee7b7;
  border-radius: 6px;
  padding: 0.2rem 0.65rem;
  font-size: 0.72rem;
  cursor: pointer;
  transition: all 0.15s;
}

.apply-tags-btn:hover {
  background: #10b981;
  color: #fff;
}

/* Running Banner */
.ai-running-banner {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem 1.25rem;
  border-radius: 10px;
  margin-bottom: 1rem;
}

.ai-running-banner.transcribing {
  background: rgba(99, 102, 241, 0.12);
  border: 1px solid rgba(99, 102, 241, 0.35);
}

.ai-running-banner.analyzing {
  background: rgba(168, 85, 247, 0.12);
  border: 1px solid rgba(168, 85, 247, 0.35);
}

.running-info h4 {
  font-size: 0.85rem;
  color: #f1f5f9;
  margin: 0 0 0.2rem 0;
}

.running-info p {
  font-size: 0.75rem;
  color: #94a3b8;
  margin: 0;
}

/* Empty Prompt */
.ai-empty-prompt {
  text-align: center;
  padding: 2.5rem 1.5rem;
}

.empty-spark-icon {
  font-size: 2.5rem;
  margin-bottom: 0.75rem;
}

.ai-empty-prompt h3 {
  font-size: 1rem;
  color: #f1f5f9;
  margin: 0 0 0.5rem 0;
}

.ai-empty-prompt p {
  font-size: 0.8rem;
  color: #94a3b8;
  max-width: 380px;
  margin: 0 auto 1.5rem auto;
  line-height: 1.5;
}

.cta-primary-btn {
  background: linear-gradient(135deg, #4f46e5, #7c3aed);
  color: #fff;
  border: none;
  border-radius: 8px;
  padding: 0.65rem 1.25rem;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.18s;
  box-shadow: 0 4px 15px rgba(99, 102, 241, 0.3);
}

.cta-primary-btn:hover {
  filter: brightness(1.15);
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(99, 102, 241, 0.4);
}

.cta-primary-btn.compact {
  padding: 0.4rem 0.8rem;
  font-size: 0.78rem;
}

.cta-secondary-btn {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.18);
  color: #cbd5e1;
  border-radius: 8px;
  padding: 0.65rem 1.25rem;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  transition: all 0.18s ease;
}

.cta-secondary-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.16);
  border-color: rgba(255, 255, 255, 0.35);
  color: #ffffff;
  transform: translateY(-2px);
}

.cta-secondary-btn:disabled,
.cta-primary-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.cta-secondary-btn.compact {
  padding: 0.4rem 0.8rem;
  font-size: 0.78rem;
}

.empty-prompt-actions,
.no-sub-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  flex-wrap: wrap;
  margin-top: 1rem;
}

/* Transcribe Error Banner */
.transcribe-err-banner {
  margin: 1.25rem auto;
  max-width: 500px;
  padding: 0.85rem 1rem;
  background: rgba(239, 68, 68, 0.12);
  border: 1px solid rgba(239, 68, 68, 0.35);
  border-radius: 8px;
  text-align: left;
}

.transcribe-err-banner .err-header {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-weight: 600;
  color: #fca5a5;
  font-size: 0.82rem;
  margin-bottom: 0.35rem;
}

.transcribe-err-banner .err-desc {
  font-size: 0.76rem;
  color: #cbd5e1;
  line-height: 1.45;
  margin: 0 0 0.75rem 0;
  word-break: break-word;
}

.transcribe-err-banner .err-actions {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.transcribe-err-banner .btn-config-ai {
  background: rgba(99, 102, 241, 0.25);
  border: 1px solid rgba(99, 102, 241, 0.5);
  color: #c7d2fe;
  border-radius: 6px;
  padding: 0.35rem 0.75rem;
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.transcribe-err-banner .btn-config-ai:hover {
  background: rgba(99, 102, 241, 0.45);
  color: #ffffff;
}

.transcribe-err-banner .btn-local-fallback {
  background: rgba(168, 85, 247, 0.25);
  border: 1px solid rgba(168, 85, 247, 0.5);
  color: #e9d5ff;
  border-radius: 6px;
  padding: 0.35rem 0.75rem;
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.transcribe-err-banner .btn-local-fallback:hover {
  background: rgba(168, 85, 247, 0.45);
  color: #ffffff;
}

/* Tip Banner when metadata analysis exists but subtitles absent */
.subtitles-transcribe-tip-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.85rem;
  padding: 0.75rem 1rem;
  margin-bottom: 1rem;
  background: linear-gradient(90deg, rgba(99, 102, 241, 0.15) 0%, rgba(168, 85, 247, 0.1) 100%);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 8px;
}

.subtitles-transcribe-tip-banner .tip-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex: 1;
  min-width: 0;
}

.subtitles-transcribe-tip-banner .tip-spark {
  font-size: 1.1rem;
  flex-shrink: 0;
}

.subtitles-transcribe-tip-banner .tip-texts {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.subtitles-transcribe-tip-banner .tip-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: #e0e7ff;
}

.subtitles-transcribe-tip-banner .tip-sub {
  font-size: 0.72rem;
  color: #94a3b8;
  line-height: 1.35;
}

.subtitles-transcribe-tip-banner .tip-cta-btn {
  background: linear-gradient(135deg, #4f46e5, #7c3aed);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 0.35rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  flex-shrink: 0;
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  transition: all 0.18s ease;
}

.subtitles-transcribe-tip-banner .tip-cta-btn:hover:not(:disabled) {
  filter: brightness(1.15);
  transform: translateY(-1px);
}

.btn-spinner.mini {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

/* Transcript Tab */
.transcript-container {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  height: 100%;
}

.transcript-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.search-input-box {
  flex: 1;
  min-width: 180px;
  position: relative;
  display: flex;
  align-items: center;
}

.search-ic {
  position: absolute;
  left: 0.65rem;
  font-size: 0.8rem;
  color: #64748b;
}

.ts-search-input {
  width: 100%;
  background: rgba(30, 41, 59, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 0.45rem 2rem 0.45rem 2rem;
  color: #f1f5f9;
  font-size: 0.8rem;
  outline: none;
  transition: border-color 0.15s;
}

.ts-search-input:focus {
  border-color: #6366f1;
}

.clear-input-btn {
  position: absolute;
  right: 0.5rem;
  background: none;
  border: none;
  color: #64748b;
  cursor: pointer;
}

.tb-actions {
  display: flex;
  gap: 0.4rem;
}

.tb-act-btn {
  background: rgba(30, 41, 59, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cbd5e1;
  border-radius: 6px;
  padding: 0.35rem 0.65rem;
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.15s;
}

.tb-act-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.tb-act-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-regen {
  background: rgba(99, 102, 241, 0.15);
  border-color: rgba(99, 102, 241, 0.3);
  color: #a5b4fc;
}

.subtitles-scroll-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  overflow-y: auto;
  padding-right: 0.3rem;
}

.sub-item-row {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.5rem 0.7rem;
  border-radius: 6px;
  background: rgba(30, 41, 59, 0.3);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.15s;
}

.sub-item-row:hover {
  background: rgba(99, 102, 241, 0.15);
  border-color: rgba(99, 102, 241, 0.3);
}

.sub-timestamp {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.72rem;
  color: #818cf8;
  white-space: nowrap;
  background: rgba(99, 102, 241, 0.15);
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
}

.jump-ic {
  font-size: 0.65rem;
}

.sub-text {
  font-size: 0.82rem;
  color: #e2e8f0;
  line-height: 1.45;
}

.no-sub-notice {
  text-align: center;
  padding: 3rem 1rem;
  color: #94a3b8;
}

.no-sub-notice .ic {
  font-size: 2rem;
  display: block;
  margin-bottom: 0.5rem;
}

/* Settings Tab */
.config-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.config-sec-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: #f1f5f9;
  margin: 0;
}

.config-hint {
  font-size: 0.75rem;
  color: #64748b;
  margin: -0.5rem 0 0 0;
}

.preset-buttons-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.preset-btn {
  background: rgba(30, 41, 59, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #e2e8f0;
  border-radius: 6px;
  padding: 0.4rem 0.75rem;
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.15s;
}

.preset-btn:hover {
  background: rgba(99, 102, 241, 0.2);
  border-color: rgba(99, 102, 241, 0.4);
  color: #a5b4fc;
}

.config-divider {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 0.5rem 0;
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.85rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.form-group.full-width {
  grid-column: span 2;
}

.form-group label {
  font-size: 0.75rem;
  color: #94a3b8;
  font-weight: 500;
}

.form-input, .form-select {
  background: rgba(15, 23, 42, 0.8);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  padding: 0.45rem 0.65rem;
  color: #f1f5f9;
  font-size: 0.8rem;
  outline: none;
}

.form-input:focus, .form-select:focus {
  border-color: #6366f1;
}

.form-submit-row {
  margin-top: 1rem;
  display: flex;
  justify-content: flex-end;
}

/* Toast */
.drawer-toast {
  position: absolute;
  top: 1rem;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1001;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(99, 102, 241, 0.4);
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.5);
  color: #f1f5f9;
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-size: 0.8rem;
  pointer-events: none;
}

/* Spinners */
.spinner-lg {
  display: inline-block;
  width: 36px;
  height: 36px;
  border: 3px solid rgba(99, 102, 241, 0.2);
  border-top-color: #6366f1;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.spinner-md {
  display: inline-block;
  width: 22px;
  height: 22px;
  border: 2px solid rgba(99, 102, 241, 0.2);
  border-top-color: #6366f1;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.drawer-center-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 4rem 1rem;
  color: #94a3b8;
  font-size: 0.85rem;
}
</style>
