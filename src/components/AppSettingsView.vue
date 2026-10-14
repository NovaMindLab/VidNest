<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { currentLocale, currentLocaleMode, setLocaleMode, SUPPORTED_LOCALES, t, type LocaleMode } from '../i18n';
import { themeMode, setTheme } from '../utils/theme';
import type { AiTranscriptionConfig } from '../types';

const activeLanguageName = computed(() => {
  const found = SUPPORTED_LOCALES.find(l => l.code === currentLocale.value);
  return found ? `${found.flag} ${found.nativeName}` : currentLocale.value;
});

function onLanguageSelectChange(val: string) {
  setLocaleMode(val as LocaleMode);
}

const emit = defineEmits<{
  (e: 'checkUpdate'): void;
}>();

const activeTab = ref<'general' | 'media' | 'downloader' | 'ai' | 'about'>('general');

// Preferences mock/storage state
const minFileSizeMb = ref<number>(
  Number(localStorage.getItem('vidnest_min_size') || '5')
);
const autoCheckYtdlp = ref<boolean>(
  localStorage.getItem('vidnest_ytdlp_auto_check') !== 'false'
);
const maxConcurrent = ref<number>(
  Number(localStorage.getItem('vidnest_max_concurrent') || '3')
);

// AI & Whisper settings state
const aiConfig = ref<AiTranscriptionConfig>({
  engine: 'api',
  api_url: 'https://api.openai.com/v1',
  api_key: '',
  model: 'whisper-1',
  language: 'auto',
  prompt: '',
  local_model_path: '',
});
const isSavingAi = ref<boolean>(false);
const aiSaveToast = ref<string>('');

async function loadAiSettings() {
  try {
    const cfg = await invoke<AiTranscriptionConfig>('get_ai_config');
    if (cfg) {
      aiConfig.value = cfg;
    }
  } catch (err) {
    console.warn('Failed to load ai config in settings:', err);
  }
}

async function saveAiSettings() {
  isSavingAi.value = true;
  aiSaveToast.value = '';
  try {
    await invoke('save_ai_config', { config: aiConfig.value });
    aiSaveToast.value = t('settings.ai.saveToast');
    setTimeout(() => {
      aiSaveToast.value = '';
    }, 2500);
  } catch (err) {
    aiSaveToast.value = t('settings.ai.saveFailed', { err: String(err) });
  } finally {
    isSavingAi.value = false;
  }
}

function applyAiPreset(preset: 'openai' | 'groq' | 'local_whisper_api' | 'ollama') {
  if (preset === 'openai') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'https://api.openai.com/v1';
    aiConfig.value.model = 'whisper-1';
  } else if (preset === 'groq') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'https://api.groq.com/openai/v1';
    aiConfig.value.model = 'whisper-large-v3';
  } else if (preset === 'local_whisper_api') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'http://localhost:8000/v1';
    aiConfig.value.model = 'Systran/faster-whisper-base';
  } else if (preset === 'ollama') {
    aiConfig.value.engine = 'api';
    aiConfig.value.api_url = 'http://localhost:11434/v1';
    aiConfig.value.model = 'whisper';
  }
}

function onMinSizeChange() {
  localStorage.setItem('vidnest_min_size', String(minFileSizeMb.value));
}

function onAutoCheckChange() {
  localStorage.setItem('vidnest_ytdlp_auto_check', String(autoCheckYtdlp.value));
}

function onMaxConcurrentChange() {
  localStorage.setItem('vidnest_max_concurrent', String(maxConcurrent.value));
}

function openExternal(url: string) {
  window.open(url, '_blank');
}

onMounted(() => {
  loadAiSettings();
});
</script>

<template>
  <div class="settings-view-container">
    <!-- Header -->
    <div class="settings-header">
      <div class="header-titles">
        <h1 class="settings-title">{{ t('settings.title') }}</h1>
        <p class="settings-subtitle">{{ t('settings.subtitle') }}</p>
      </div>

      <!-- Tab Buttons -->
      <div class="settings-tabs-nav">
        <button
          class="tab-nav-btn"
          :class="{ active: activeTab === 'general' }"
          @click="activeTab = 'general'"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="5"></circle>
            <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"></path>
          </svg>
          <span>{{ t('settings.tabs.general') }}</span>
        </button>

        <button
          class="tab-nav-btn"
          :class="{ active: activeTab === 'media' }"
          @click="activeTab = 'media'"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect>
            <line x1="7" y1="2" x2="7" y2="22"></line>
            <line x1="17" y1="2" x2="17" y2="22"></line>
            <line x1="2" y1="12" x2="22" y2="12"></line>
          </svg>
          <span>{{ t('settings.tabs.media') }}</span>
        </button>

        <button
          class="tab-nav-btn"
          :class="{ active: activeTab === 'downloader' }"
          @click="activeTab = 'downloader'"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
          <span>{{ t('settings.tabs.download') }}</span>
        </button>

        <button
          class="tab-nav-btn"
          :class="{ active: activeTab === 'ai' }"
          @click="activeTab = 'ai'"
        >
          <span style="font-size: 1rem; line-height: 1;">✨</span>
          <span>{{ t('settings.tabs.ai') }}</span>
        </button>

        <button
          class="tab-nav-btn"
          :class="{ active: activeTab === 'about' }"
          @click="activeTab = 'about'"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="16" x2="12" y2="12"></line>
            <line x1="12" y1="8" x2="12.01" y2="8"></line>
          </svg>
          <span>{{ t('settings.tabs.about') }}</span>
        </button>
      </div>
    </div>

    <!-- Content Sections -->
    <div class="settings-content-body">
      <!-- 1. GENERAL / APPEARANCE -->
      <section v-show="activeTab === 'general'" class="settings-section">
        <!-- Language Block -->
        <div class="settings-card">
          <div class="card-header-row">
            <div class="card-icon-badge">🌐</div>
            <div class="card-title-group">
              <div class="title-with-pill">
                <h3>{{ t('settings.language.title') }}</h3>
                <span class="active-lang-badge">
                  <span class="pulse-dot"></span>
                  {{ currentLocaleMode === 'auto' ? t('settings.language.auto') + ' · ' + activeLanguageName : activeLanguageName }}
                </span>
              </div>
              <p>{{ t('settings.language.desc') }}</p>
            </div>
          </div>

          <div class="language-selector-wrapper">
            <div class="language-select-row">
              <div class="language-select-box">
                <div class="select-label-icon">🌐</div>
                <select
                  class="language-native-select"
                  :value="currentLocaleMode"
                  @change="onLanguageSelectChange(($event.target as HTMLSelectElement).value)"
                >
                  <option value="auto">
                    {{ t('settings.language.auto') }}
                  </option>
                  <option
                    v-for="loc in SUPPORTED_LOCALES"
                    :key="loc.code"
                    :value="loc.code"
                  >
                    {{ loc.flag }} {{ loc.nativeName }} ({{ loc.englishName }})
                  </option>
                </select>
                <div class="select-arrow">▼</div>
              </div>
            </div>

            <!-- Quick Language Switch Pills -->
            <div class="quick-lang-pills">
              <button
                class="lang-pill-btn"
                :class="{ active: currentLocaleMode === 'auto' }"
                :title="t('settings.language.auto')"
                @click="setLocaleMode('auto')"
              >
                <span class="pill-flag">🌐</span>
                <span class="pill-text">{{ t('settings.theme.auto') }}</span>
              </button>

              <button
                v-for="loc in SUPPORTED_LOCALES"
                :key="loc.code"
                class="lang-pill-btn"
                :class="{ active: currentLocaleMode === loc.code }"
                :title="`${loc.nativeName} (${loc.englishName})`"
                @click="setLocaleMode(loc.code)"
              >
                <span class="pill-flag">{{ loc.flag }}</span>
                <span class="pill-text">{{ loc.name }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Theme & Appearance Block -->
        <div class="settings-card">
          <div class="card-header-row">
            <div class="card-icon-badge">🎨</div>
            <div class="card-title-group">
              <h3>{{ t('settings.theme.title') }}</h3>
              <p>{{ t('settings.theme.desc') }}</p>
            </div>
          </div>

          <div class="theme-cards-grid">
            <!-- Dark Mode -->
            <div
              class="theme-card"
              :class="{ active: themeMode === 'dark' }"
              @click="setTheme('dark')"
            >
              <div class="theme-preview dark-preview">
                <div class="preview-sidebar"></div>
                <div class="preview-body">
                  <div class="preview-bar"></div>
                  <div class="preview-content"></div>
                </div>
              </div>
              <div class="theme-card-info">
                <div class="theme-title-row">
                  <span class="theme-title">{{ t('settings.theme.dark') }}</span>
                  <span v-if="themeMode === 'dark'" class="theme-badge">Active</span>
                </div>
                <p class="theme-desc">{{ t('settings.theme.darkDesc') }}</p>
              </div>
            </div>

            <!-- Light Mode -->
            <div
              class="theme-card"
              :class="{ active: themeMode === 'light' }"
              @click="setTheme('light')"
            >
              <div class="theme-preview light-preview">
                <div class="preview-sidebar"></div>
                <div class="preview-body">
                  <div class="preview-bar"></div>
                  <div class="preview-content"></div>
                </div>
              </div>
              <div class="theme-card-info">
                <div class="theme-title-row">
                  <span class="theme-title">{{ t('settings.theme.light') }}</span>
                  <span v-if="themeMode === 'light'" class="theme-badge">Active</span>
                </div>
                <p class="theme-desc">{{ t('settings.theme.lightDesc') }}</p>
              </div>
            </div>

            <!-- Auto Mode -->
            <div
              class="theme-card"
              :class="{ active: themeMode === 'auto' }"
              @click="setTheme('auto')"
            >
              <div class="theme-preview auto-preview">
                <div class="half-dark"></div>
                <div class="half-light"></div>
              </div>
              <div class="theme-card-info">
                <div class="theme-title-row">
                  <span class="theme-title">{{ t('settings.theme.auto') }}</span>
                  <span v-if="themeMode === 'auto'" class="theme-badge">Active</span>
                </div>
                <p class="theme-desc">{{ t('settings.theme.autoDesc') }}</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 2. MEDIA & LIBRARY PREFERENCES -->
      <section v-show="activeTab === 'media'" class="settings-section">
        <div class="settings-card">
          <div class="card-header-row">
            <div class="card-icon-badge">📁</div>
            <div class="card-title-group">
              <h3>{{ t('settings.library.title') }}</h3>
              <p>{{ t('settings.library.desc') }}</p>
            </div>
          </div>

          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.library.minSize') }}</span>
              <span class="row-sub">{{ t('settings.library.minSizeSub') }}</span>
            </div>
            <div class="row-control">
              <input
                v-model.number="minFileSizeMb"
                type="number"
                min="0"
                max="500"
                class="settings-num-input"
                @change="onMinSizeChange"
              />
              <span class="unit-text">MB</span>
            </div>
          </div>

          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.library.tsCheck') }}</span>
              <span class="row-sub">{{ t('settings.library.tsCheckDesc') }}</span>
            </div>
            <div class="row-control">
              <span class="status-tag-active">🛡️ {{ t('settings.library.enabled') }}</span>
            </div>
          </div>

          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.library.tsPurge') }}</span>
              <span class="row-sub">{{ t('settings.library.tsPurgeDesc') }}</span>
            </div>
            <div class="row-control">
              <span class="status-tag-active">🧹 {{ t('settings.library.enabled') }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 3. DOWNLOADER PREFERENCES -->
      <section v-show="activeTab === 'downloader'" class="settings-section">
        <div class="settings-card">
          <div class="card-header-row">
            <div class="card-icon-badge">⚡</div>
            <div class="card-title-group">
              <h3>{{ t('settings.downloader.title') }}</h3>
              <p>{{ t('settings.downloader.desc') }}</p>
            </div>
          </div>

          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.downloader.ytdlpAuto') }}</span>
              <span class="row-sub">{{ t('settings.downloader.ytdlpAutoDesc') }}</span>
            </div>
            <div class="row-control">
              <label class="switch-toggle">
                <input
                  v-model="autoCheckYtdlp"
                  type="checkbox"
                  @change="onAutoCheckChange"
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.downloader.concurrent') }}</span>
              <span class="row-sub">{{ t('settings.downloader.concurrentSub') }}</span>
            </div>
            <div class="row-control">
              <select
                v-model.number="maxConcurrent"
                class="settings-select"
                @change="onMaxConcurrentChange"
              >
                <option :value="1">1</option>
                <option :value="2">2</option>
                <option :value="3">{{ t('settings.downloader.concurrentRecommended') }}</option>
                <option :value="5">5</option>
                <option :value="10">{{ t('settings.downloader.unlimited') }}</option>
              </select>
            </div>
          </div>
        </div>
      </section>

      <!-- 4. AI & WHISPER MODEL CONFIGURATION -->
      <section v-show="activeTab === 'ai'" class="settings-section">
        <div class="settings-card">
          <div class="card-header-row">
            <div class="card-icon-badge">✨</div>
            <div class="card-title-group">
              <h3>{{ t('settings.ai.title') }}</h3>
              <p>{{ t('settings.ai.desc') }}</p>
            </div>
          </div>

          <!-- Engine choice -->
          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.engineLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.engineSub') }}</span>
            </div>
            <div class="row-control">
              <select v-model="aiConfig.engine" class="settings-select">
                <option value="api">{{ t('settings.ai.engineApi') }}</option>
                <option value="local">{{ t('settings.ai.engineLocal') }}</option>
              </select>
            </div>
          </div>

          <!-- Quick Presets when API -->
          <div v-if="aiConfig.engine === 'api'" class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.presetLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.presetSub') }}</span>
            </div>
            <div class="row-control-presets">
              <button class="preset-pill-btn" @click="applyAiPreset('openai')">{{ t('settings.ai.presetOpenai') }}</button>
              <button class="preset-pill-btn" @click="applyAiPreset('groq')">{{ t('settings.ai.presetGroq') }}</button>
              <button class="preset-pill-btn" @click="applyAiPreset('local_whisper_api')">{{ t('settings.ai.presetLocalApi') }}</button>
              <button class="preset-pill-btn" @click="applyAiPreset('ollama')">{{ t('settings.ai.presetOllama') }}</button>
            </div>
          </div>

          <!-- API URL -->
          <div v-if="aiConfig.engine === 'api'" class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.baseUrlLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.baseUrlSub') }}</span>
            </div>
            <div class="row-control wide">
              <input
                v-model="aiConfig.api_url"
                type="text"
                class="settings-text-input"
                placeholder="https://api.openai.com/v1"
              />
            </div>
          </div>

          <!-- API Key -->
          <div v-if="aiConfig.engine === 'api'" class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.apiKeyLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.apiKeySub') }}</span>
            </div>
            <div class="row-control wide">
              <input
                v-model="aiConfig.api_key"
                type="password"
                class="settings-text-input"
                placeholder="sk-..."
              />
            </div>
          </div>

          <!-- Model Name -->
          <div v-if="aiConfig.engine === 'api'" class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.modelLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.modelSub') }}</span>
            </div>
            <div class="row-control wide">
              <input
                v-model="aiConfig.model"
                type="text"
                class="settings-text-input"
                placeholder="whisper-1"
              />
            </div>
          </div>

          <!-- Local Model Path -->
          <div v-if="aiConfig.engine === 'local'" class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.localWeightLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.localWeightSub') }}</span>
            </div>
            <div class="row-control wide">
              <input
                v-model="aiConfig.local_model_path"
                type="text"
                class="settings-text-input"
                :placeholder="t('settings.ai.localWeightPlaceholder')"
              />
            </div>
          </div>

          <!-- Target Language -->
          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.langLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.langSub') }}</span>
            </div>
            <div class="row-control">
              <select v-model="aiConfig.language" class="settings-select">
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
          </div>

          <!-- Prompt -->
          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.ai.promptLabel') }}</span>
              <span class="row-sub">{{ t('settings.ai.promptSub') }}</span>
            </div>
            <div class="row-control wide">
              <input
                v-model="aiConfig.prompt"
                type="text"
                class="settings-text-input"
                :placeholder="t('settings.ai.promptPlaceholder')"
              />
            </div>
          </div>

          <!-- Save Button Bar -->
          <div class="setting-action-footer">
            <span v-if="aiSaveToast" class="ai-save-toast">{{ aiSaveToast }}</span>
            <button
              class="btn-save-ai-settings"
              :disabled="isSavingAi"
              @click="saveAiSettings"
            >
              <span>{{ isSavingAi ? t('ai.saving') : t('settings.ai.saveBtn') }}</span>
            </button>
          </div>
        </div>
      </section>

      <!-- 5. ABOUT VIDNEST -->
      <section v-show="activeTab === 'about'" class="settings-section">
        <div class="settings-card about-card">
          <div class="about-hero">
            <div class="about-app-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polygon points="6 3 20 12 6 21 6 3"></polygon>
              </svg>
            </div>
            <div class="about-hero-text">
              <h2>VidNest</h2>
              <p class="about-version-line">
                <span>{{ t('settings.about.version') }} 2.7.0</span>
                <span class="stable-tag">{{ t('settings.about.stable') }}</span>
              </p>
            </div>
          </div>

          <p class="about-statement">{{ t('settings.about.credits') }}</p>

          <div class="about-actions-grid">
            <button class="about-btn primary" @click="emit('checkUpdate')">
              <span class="btn-icon">⚡</span>
              <span>{{ t('settings.about.checkNow') }}</span>
            </button>

            <button
              class="about-btn secondary"
              @click="openExternal('https://github.com/NovaMindLab/VidNest')"
            >
              <span class="btn-icon">🐙</span>
              <span>{{ t('settings.about.openGithub') }}</span>
            </button>

            <button
              class="about-btn secondary"
              @click="openExternal('https://vidnest.novamindlab.com')"
            >
              <span class="btn-icon">🌐</span>
              <span>{{ t('settings.about.openOfficial') }}</span>
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-view-container {
  width: 100%;
  max-width: 960px;
  margin: 0 auto;
  padding: 10px 0 60px;
}

.settings-header {
  margin-bottom: 28px;
}

.header-titles {
  margin-bottom: 20px;
}

.settings-title {
  font-size: 26px;
  font-weight: 800;
  color: var(--text-primary, #ffffff);
  margin: 0 0 6px 0;
  letter-spacing: -0.5px;
}

.settings-subtitle {
  font-size: 14px;
  color: var(--text-muted, #8b949e);
  margin: 0;
}

.settings-tabs-nav {
  display: flex;
  gap: 8px;
  border-bottom: 1px solid var(--border-color, #30363d);
  padding-bottom: 12px;
}

.tab-nav-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-muted, #8b949e);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-nav-btn:hover {
  background: var(--bg-hover, rgba(255, 255, 255, 0.06));
  color: var(--text-primary, #ffffff);
}

.tab-nav-btn.active {
  background: var(--bg-card, #161b22);
  border-color: var(--border-color, #30363d);
  color: var(--accent-primary, #58a6ff);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.settings-content-body {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.settings-card {
  background: var(--bg-card, #161b22);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
}

.card-header-row {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 20px;
}

.card-icon-badge {
  font-size: 22px;
  width: 44px;
  height: 44px;
  border-radius: 10px;
  background: var(--bg-badge, rgba(255, 255, 255, 0.05));
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-title-group h3 {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary, #ffffff);
  margin: 0 0 4px 0;
}

.card-title-group p {
  font-size: 13px;
  color: var(--text-muted, #8b949e);
  margin: 0;
}

.title-with-pill {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 4px;
}

.title-with-pill h3 {
  margin: 0 !important;
}

.active-lang-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  color: var(--accent-primary, #58a6ff);
  background: var(--accent-subtle, rgba(88, 166, 255, 0.1));
  border: 1px solid rgba(88, 166, 255, 0.25);
}

.pulse-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #3fb950;
  box-shadow: 0 0 6px rgba(63, 185, 80, 0.6);
}

.language-selector-wrapper {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.language-select-row {
  max-width: 480px;
}

.language-select-box {
  position: relative;
  display: flex;
  align-items: center;
  background: var(--bg-item, rgba(255, 255, 255, 0.04));
  border: 1.5px solid var(--border-color, #30363d);
  border-radius: 10px;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.language-select-box:hover,
.language-select-box:focus-within {
  border-color: var(--accent-primary, #58a6ff);
  background: var(--bg-hover, rgba(255, 255, 255, 0.07));
  box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.15);
}

.select-label-icon {
  padding-left: 14px;
  font-size: 18px;
  pointer-events: none;
}

.language-native-select {
  flex: 1;
  appearance: none;
  background: transparent;
  border: none;
  color: var(--text-primary, #ffffff);
  font-size: 14px;
  font-weight: 600;
  padding: 12px 36px 12px 10px;
  outline: none;
  cursor: pointer;
  border-radius: 10px;
}

.language-native-select option {
  background: #161b22;
  color: #f0f6fc;
}

.select-arrow {
  position: absolute;
  right: 14px;
  font-size: 10px;
  color: var(--text-muted, #8b949e);
  pointer-events: none;
}

.quick-lang-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.lang-pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border-color, #30363d);
  background: var(--bg-item, rgba(255, 255, 255, 0.03));
  color: var(--text-secondary, #c9d1d9);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.lang-pill-btn:hover {
  border-color: var(--accent-primary, #58a6ff);
  color: var(--text-primary, #ffffff);
  background: var(--bg-hover, rgba(255, 255, 255, 0.06));
}

.lang-pill-btn.active {
  border-color: var(--accent-primary, #58a6ff);
  background: var(--accent-subtle, rgba(88, 166, 255, 0.15));
  color: var(--accent-primary, #58a6ff);
  font-weight: 700;
}

.pill-flag {
  font-size: 14px;
}

/* Language Choice Grid */
.options-choice-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 14px;
}

.choice-btn {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px;
  border-radius: 10px;
  border: 1.5px solid var(--border-color, #30363d);
  background: var(--bg-item, rgba(255, 255, 255, 0.03));
  color: var(--text-primary, #ffffff);
  cursor: pointer;
  text-align: left;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  position: relative;
}

.choice-btn:hover {
  border-color: var(--accent-primary, #58a6ff);
  background: var(--bg-hover, rgba(255, 255, 255, 0.06));
}

.choice-btn.active {
  border-color: var(--accent-primary, #58a6ff);
  background: var(--accent-subtle, rgba(88, 166, 255, 0.12));
}

.choice-flag {
  font-size: 24px;
}

.choice-text {
  display: flex;
  flex-direction: column;
}

.choice-name {
  font-size: 14px;
  font-weight: 700;
}

.choice-sub {
  font-size: 12px;
  color: var(--text-muted, #8b949e);
}

.active-check {
  margin-left: auto;
  color: var(--accent-primary, #58a6ff);
  font-weight: 800;
  font-size: 16px;
}

/* Theme Cards Grid */
.theme-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
}

.theme-card {
  border: 1.5px solid var(--border-color, #30363d);
  border-radius: 12px;
  overflow: hidden;
  background: var(--bg-item, rgba(255, 255, 255, 0.03));
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.theme-card:hover {
  border-color: var(--accent-primary, #58a6ff);
  transform: translateY(-2px);
}

.theme-card.active {
  border-color: var(--accent-primary, #58a6ff);
  background: var(--accent-subtle, rgba(88, 166, 255, 0.08));
  box-shadow: 0 4px 14px rgba(88, 166, 255, 0.15);
}

.theme-preview {
  height: 90px;
  position: relative;
  border-bottom: 1px solid var(--border-color, #30363d);
}

.dark-preview {
  background: #0d1117;
}

.dark-preview .preview-sidebar {
  width: 25%;
  height: 100%;
  background: #161b22;
  border-right: 1px solid #30363d;
  position: absolute;
}

.dark-preview .preview-body {
  left: 25%;
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  padding: 10px;
}

.dark-preview .preview-bar {
  height: 10px;
  width: 60%;
  background: #30363d;
  border-radius: 4px;
  margin-bottom: 8px;
}

.dark-preview .preview-content {
  height: 36px;
  background: #21262d;
  border-radius: 6px;
}

.light-preview {
  background: #f6f8fa;
}

.light-preview .preview-sidebar {
  width: 25%;
  height: 100%;
  background: #ffffff;
  border-right: 1px solid #d0d7de;
  position: absolute;
}

.light-preview .preview-body {
  left: 25%;
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  padding: 10px;
}

.light-preview .preview-bar {
  height: 10px;
  width: 60%;
  background: #d0d7de;
  border-radius: 4px;
  margin-bottom: 8px;
}

.light-preview .preview-content {
  height: 36px;
  background: #ffffff;
  border: 1px solid #d0d7de;
  border-radius: 6px;
}

.auto-preview {
  display: flex;
}

.auto-preview .half-dark {
  width: 50%;
  height: 100%;
  background: #0d1117;
}

.auto-preview .half-light {
  width: 50%;
  height: 100%;
  background: #f6f8fa;
}

.theme-card-info {
  padding: 14px 16px;
}

.theme-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.theme-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary, #ffffff);
}

.theme-badge {
  font-size: 11px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--accent-primary, #58a6ff);
  color: #ffffff;
}

.theme-desc {
  font-size: 12px;
  color: var(--text-muted, #8b949e);
  margin: 0;
  line-height: 1.4;
}

/* Rows in Cards */
.setting-row-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 0;
  border-bottom: 1px solid var(--border-color, rgba(255, 255, 255, 0.08));
}

.setting-row-item:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.row-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.row-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #ffffff);
}

.row-sub {
  font-size: 12px;
  color: var(--text-muted, #8b949e);
}

.row-control {
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-num-input {
  width: 70px;
  background: var(--bg-input, #0d1117);
  border: 1px solid var(--border-color, #30363d);
  color: var(--text-primary, #ffffff);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 13px;
  text-align: right;
}

.unit-text {
  font-size: 13px;
  color: var(--text-muted, #8b949e);
}

.status-tag-active {
  font-size: 12px;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: 6px;
  background: rgba(35, 134, 54, 0.2);
  color: #3fb950;
  border: 1px solid rgba(63, 185, 80, 0.3);
}

.settings-select {
  background: var(--bg-input, #0d1117);
  border: 1px solid var(--border-color, #30363d);
  color: var(--text-primary, #ffffff);
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 13px;
}

/* Switch Toggle */
.switch-toggle {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
}

.switch-toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--border-color, #30363d);
  transition: 0.3s;
  border-radius: 24px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: 0.3s;
  border-radius: 50%;
}

input:checked + .toggle-slider {
  background-color: var(--accent-primary, #58a6ff);
}

input:checked + .toggle-slider:before {
  transform: translateX(20px);
}

/* About Card */
.about-hero {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}

.about-app-icon {
  width: 56px;
  height: 56px;
  border-radius: 14px;
  background: linear-gradient(135deg, #1f6feb, #238636);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
}

.about-app-icon svg {
  width: 28px;
  height: 28px;
}

.about-hero-text h2 {
  font-size: 22px;
  font-weight: 800;
  margin: 0 0 4px 0;
  color: var(--text-primary, #ffffff);
}

.about-version-line {
  margin: 0;
  font-size: 13px;
  color: var(--text-muted, #8b949e);
  display: flex;
  align-items: center;
  gap: 8px;
}

.stable-tag {
  font-size: 11px;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(88, 166, 255, 0.15);
  color: var(--accent-primary, #58a6ff);
}

.about-statement {
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-muted, #8b949e);
  margin-bottom: 22px;
}

.about-actions-grid {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.about-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 18px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.about-btn.primary {
  background: var(--accent-primary, #1f6feb);
  color: #ffffff;
  border: none;
}

.about-btn.primary:hover {
  background: #388bfd;
}

.about-btn.secondary {
  background: var(--bg-item, rgba(255, 255, 255, 0.05));
  border: 1px solid var(--border-color, #30363d);
  color: var(--text-primary, #ffffff);
}

.about-btn.secondary:hover {
  background: var(--bg-hover, rgba(255, 255, 255, 0.1));
}

/* AI Settings specific styles */
.row-control.wide {
  flex: 1;
  max-width: 480px;
}

.settings-text-input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color, #30363d);
  background: var(--bg-input, #0d1117);
  color: var(--text-primary, #ffffff);
  font-size: 13px;
  outline: none;
  transition: all 0.2s ease;
}

.settings-text-input:focus {
  border-color: #6366f1;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.row-control-presets {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.preset-pill-btn {
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border-color, rgba(255, 255, 255, 0.12));
  background: var(--bg-item, rgba(255, 255, 255, 0.04));
  color: var(--text-muted, #c9d1d9);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.preset-pill-btn:hover {
  background: rgba(99, 102, 241, 0.15);
  border-color: rgba(99, 102, 241, 0.4);
  color: #a5b4fc;
}

.setting-action-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  padding-top: 16px;
  margin-top: 8px;
  border-top: 1px solid var(--border-color, rgba(255, 255, 255, 0.06));
}

.ai-save-toast {
  font-size: 13px;
  color: #4ade80;
  font-weight: 500;
}

.btn-save-ai-settings {
  padding: 9px 20px;
  background: linear-gradient(135deg, #6366f1, #a855f7);
  border: none;
  border-radius: 8px;
  color: #ffffff;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
  transition: all 0.2s ease;
}

.btn-save-ai-settings:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(168, 85, 247, 0.4);
}

.btn-save-ai-settings:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
