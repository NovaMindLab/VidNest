<script setup lang="ts">
import { ref } from 'vue';
import { currentLocale, setLocale, t } from '../i18n';
import { themeMode, setTheme } from '../utils/theme';

const emit = defineEmits<{
  (e: 'checkUpdate'): void;
}>();

const activeTab = ref<'general' | 'media' | 'downloader' | 'about'>('general');

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
              <h3>{{ t('settings.language.title') }}</h3>
              <p>{{ t('settings.language.desc') }}</p>
            </div>
          </div>

          <div class="options-choice-grid">
            <button
              class="choice-btn"
              :class="{ active: currentLocale === 'zh-CN' }"
              @click="setLocale('zh-CN')"
            >
              <div class="choice-flag">🇨🇳</div>
              <div class="choice-text">
                <span class="choice-name">简体中文</span>
                <span class="choice-sub">Chinese (Simplified)</span>
              </div>
              <span v-if="currentLocale === 'zh-CN'" class="active-check">✓</span>
            </button>

            <button
              class="choice-btn"
              :class="{ active: currentLocale === 'en-US' }"
              @click="setLocale('en-US')"
            >
              <div class="choice-flag">🇺🇸</div>
              <div class="choice-text">
                <span class="choice-name">English</span>
                <span class="choice-sub">United States</span>
              </div>
              <span v-if="currentLocale === 'en-US'" class="active-check">✓</span>
            </button>
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
              <p>配置本地智能扫描深度、过滤条件以及源码误报清洗</p>
            </div>
          </div>

          <div class="setting-row-item">
            <div class="row-info">
              <span class="row-label">{{ t('settings.library.minSize') }}</span>
              <span class="row-sub">小于该体积的文件在扫描时将自动略过，降低碎片干扰</span>
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
              <p>管理解析引擎核心策略与下载队列参数</p>
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
              <span class="row-sub">限制同时执行的视频下载线程数量以保护带宽与 CPU 负载</span>
            </div>
            <div class="row-control">
              <select
                v-model.number="maxConcurrent"
                class="settings-select"
                @change="onMaxConcurrentChange"
              >
                <option :value="1">1</option>
                <option :value="2">2</option>
                <option :value="3">3 (推荐)</option>
                <option :value="5">5</option>
                <option :value="10">{{ t('settings.downloader.unlimited') }}</option>
              </select>
            </div>
          </div>
        </div>
      </section>

      <!-- 4. ABOUT VIDNEST -->
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
                <span>{{ t('settings.about.version') }} 2.0.0</span>
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
</style>
