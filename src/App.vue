<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { HealthReport, YtDlpUpdateCheck, YtDlpUpdateResult, AppUpdateInfo, Video } from './types';
import VideoLibrary from './components/VideoLibrary.vue';
import VideoClipperStudio from './components/VideoClipperStudio.vue';
import ClippedVideosManager from './components/ClippedVideosManager.vue';
import VideoDeduplication from './components/VideoDeduplication.vue';
import ExtractedAudiosView from './components/ExtractedAudiosView.vue';
import DownloadCenter from './components/DownloadCenter.vue';
import AppSettingsView from './components/AppSettingsView.vue';
import AppUpdateModal from './components/AppUpdateModal.vue';
import StandalonePlayerWindow from './components/StandalonePlayerWindow.vue';
import { t, currentLocale, toggleLocale, SUPPORTED_LOCALES } from './i18n';
import { initTheme, activeTheme, toggleTheme } from './utils/theme';

const activeLocaleInfo = computed(() => {
  const found = SUPPORTED_LOCALES.find(l => l.code === currentLocale.value);
  return found || { flag: '🌐', code: 'EN', name: 'English' };
});

const urlParams = new URLSearchParams(window.location.search);
const isPlayerWindow = ref(urlParams.get('window') === 'player');
const isYtdWindow = ref(urlParams.get('window') === 'ytd');

const currentTab = ref<'library' | 'clipper' | 'clips' | 'dedup' | 'audios' | 'downloads' | 'system' | 'settings'>('library');
const clipperActiveVideo = ref<Video | null>(null);
const activeDownloadsCount = ref<number>(0);
const showUpdateModal = ref<boolean>(false);
const discoveredUpdate = ref<AppUpdateInfo | null>(null);
const showStartupUpdateNotice = ref<boolean>(false);

function onActiveCountChange(count: number) {
  activeDownloadsCount.value = count;
}

function handleViewInLibrary(_payload: { fileName?: string; filePath?: string }) {
  currentTab.value = 'library';
}

function handleNavigateClipper(video?: Video) {
  if (video) {
    clipperActiveVideo.value = video;
  }
  currentTab.value = 'clipper';
}

const healthReport = ref<HealthReport | null>(null);
const isLoading = ref<boolean>(false);
const errorMessage = ref<string>('');

// yt-dlp updater state
const ytdlpCheck = ref<YtDlpUpdateCheck | null>(null);
const isCheckingYtdlp = ref<boolean>(false);
const isUpdatingYtdlp = ref<boolean>(false);
const ytdlpUpdateMsg = ref<string>('');
const ytdlpUpdateSuccess = ref<boolean | null>(null);
const autoCheckYtdlpOnLaunch = ref<boolean>(true);

// IPC console
const echoInput = ref<string>('VidNest IPC Benchmark');
const echoOutput = ref<string>('');
const pingOutput = ref<string>('');
const pingLatency = ref<number | null>(null);

async function fetchHealth() {
  isLoading.value = true;
  errorMessage.value = '';
  try {
    const report = await invoke<HealthReport>('system_health_check');
    healthReport.value = report;
  } catch (err: unknown) {
    errorMessage.value = err instanceof Error ? err.message : String(err);
  } finally {
    isLoading.value = false;
  }
}

async function checkYtdlpUpdate() {
  isCheckingYtdlp.value = true;
  ytdlpUpdateMsg.value = '';
  try {
    const res = await invoke<YtDlpUpdateCheck>('check_ytdlp_update');
    ytdlpCheck.value = res;
    ytdlpUpdateMsg.value = res.message;
    ytdlpUpdateSuccess.value = true;
  } catch (err: unknown) {
    ytdlpUpdateMsg.value = `${t('update.checkFailed')}: ${err instanceof Error ? err.message : String(err)}`;
    ytdlpUpdateSuccess.value = false;
  } finally {
    isCheckingYtdlp.value = false;
  }
}

async function triggerYtdlpUpdate() {
  isUpdatingYtdlp.value = true;
  ytdlpUpdateMsg.value = t('system.updating');
  ytdlpUpdateSuccess.value = null;
  try {
    const res = await invoke<YtDlpUpdateResult>('update_ytdlp');
    ytdlpUpdateSuccess.value = res.success;
    ytdlpUpdateMsg.value = `[${res.method === 'github_download' ? t('system.modeManaged') : t('system.modeSystem')}] ${res.message}`;
    await fetchHealth();
    await checkYtdlpUpdate();
  } catch (err: unknown) {
    ytdlpUpdateSuccess.value = false;
    ytdlpUpdateMsg.value = `${t('common.failed')}: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    isUpdatingYtdlp.value = false;
  }
}

async function testPing() {
  const start = performance.now();
  try {
    const res = await invoke<string>('ping');
    pingLatency.value = Math.round(performance.now() - start);
    pingOutput.value = res;
  } catch (err: unknown) {
    pingOutput.value = `Ping ${t('common.failed')}: ${err}`;
  }
}

async function testEcho() {
  try {
    const res = await invoke<string>('echo', { message: echoInput.value });
    echoOutput.value = res;
  } catch (err: unknown) {
    echoOutput.value = `Echo ${t('common.failed')}: ${err}`;
  }
}

function formatTime(timestampMs: number): string {
  if (!timestampMs) return '-';
  const d = new Date(timestampMs);
  return d.toLocaleTimeString();
}

async function checkAppUpdateOnLaunch() {
  try {
    const res = await invoke<AppUpdateInfo>('check_app_update', { mock: false });
    if (res && res.has_update) {
      discoveredUpdate.value = res;
      showStartupUpdateNotice.value = true;
    }
  } catch (err: unknown) {
    console.warn('[VidNest Startup Update Check] Failed silently:', err);
  }
}

onMounted(() => {
  initTheme();
  if (isPlayerWindow.value || isYtdWindow.value) {
    return;
  }
  // 非阻塞并行触发系统健康检测与 IPC 探针，首屏毫秒级就绪
  fetchHealth();
  testPing();
  if (autoCheckYtdlpOnLaunch.value) {
    // 延迟 1.5s 在后台静默检查，绝不争抢首屏与冷启动主流程资源
    setTimeout(() => {
      checkYtdlpUpdate();
    }, 1500);
  }
  // 延迟 2.5s 在后台静默检查应用版本更新并轻量提示
  setTimeout(() => {
    checkAppUpdateOnLaunch();
  }, 2500);
});
</script>

<template>
  <StandalonePlayerWindow v-if="isPlayerWindow" />
  <DownloadCenter
    v-else-if="isYtdWindow"
    :is-standalone-window="true"
    @view-in-library="handleViewInLibrary"
  />
  <div v-else class="desktop-app">
    <!-- LEFT SIDEBAR -->
    <aside class="app-sidebar">
      <!-- App Brand -->
      <div class="sidebar-brand">
        <div class="brand-logo">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="brand-icon">
            <polygon points="6 3 20 12 6 21 6 3"></polygon>
          </svg>
        </div>
        <div class="brand-meta">
          <span class="brand-name">VidNest</span>
          <span class="brand-tag">Local Intelligence</span>
        </div>
      </div>

      <!-- Navigation Menu -->
      <nav class="sidebar-nav">
        <div class="nav-section-title">{{ t('nav.core') }}</div>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'library' }"
          @click="currentTab = 'library'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="2" width="20" height="20" rx="3"></rect>
              <line x1="7" y1="2" x2="7" y2="22"></line>
              <line x1="17" y1="2" x2="17" y2="22"></line>
              <line x1="2" y1="12" x2="22" y2="12"></line>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.library') }}</span>
        </button>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'clips' }"
          @click="currentTab = 'clips'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="6" cy="6" r="3"></circle>
              <circle cx="6" cy="18" r="3"></circle>
              <line x1="20" y1="4" x2="8.12" y2="15.88"></line>
              <line x1="14.47" y1="14.48" x2="20" y2="20"></line>
              <line x1="8.12" y1="8.12" x2="12" y2="12"></line>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.clips') }}</span>
        </button>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'dedup' }"
          @click="currentTab = 'dedup'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.dedup') }}</span>
        </button>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'audios' }"
          @click="currentTab = 'audios'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18V5l12-2v13"></path>
              <circle cx="6" cy="18" r="3"></circle>
              <circle cx="18" cy="16" r="3"></circle>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.audios') }}</span>
        </button>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'downloads' }"
          @click="currentTab = 'downloads'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.downloads') }}</span>
          <span v-if="activeDownloadsCount > 0" class="nav-badge">
            {{ activeDownloadsCount }}
          </span>
        </button>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'system' }"
          @click="currentTab = 'system'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
              <line x1="8" y1="21" x2="16" y2="21"></line>
              <line x1="12" y1="17" x2="12" y2="21"></line>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.system') }}</span>
        </button>

        <div class="nav-section-title" style="margin-top: 14px;">{{ t('nav.preferences') }}</div>

        <button
          class="nav-item"
          :class="{ active: currentTab === 'settings' }"
          @click="currentTab = 'settings'"
        >
          <div class="nav-item-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
            </svg>
          </div>
          <span class="nav-item-label">{{ t('nav.settings') }}</span>
          <span v-if="discoveredUpdate?.has_update" class="nav-update-dot" :title="t('update.newVersionFound')"></span>
        </button>
      </nav>

      <!-- Sidebar Footer Status -->
      <div class="sidebar-footer">
        <div class="health-card">
          <div class="health-row">
            <span class="health-status-dot" :class="healthReport?.status === 'healthy' ? 'dot-green' : 'dot-yellow'"></span>
            <span class="health-title">{{ healthReport?.status === 'healthy' ? t('health.ready') : t('health.restricted') }}</span>
            <span v-if="pingLatency !== null" class="latency-badge">{{ pingLatency }}ms</span>
          </div>
          <div class="health-meta">
            <span>{{ healthReport?.os.os || 'Desktop' }} · v{{ healthReport?.app_version || '0.1.0' }}</span>
          </div>
        </div>

        <button class="btn-refresh-sidebar" :disabled="isLoading" @click="fetchHealth" :title="t('health.refresh')">
          <svg class="refresh-icon" :class="{ 'spin-anim': isLoading }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
          </svg>
          <span>{{ isLoading ? t('health.checking') : t('health.checkEnv') }}</span>
        </button>
      </div>
    </aside>

    <!-- RIGHT MAIN WORKSPACE -->
    <main class="app-workspace">
      <!-- Workspace Header -->
      <header class="workspace-header">
        <div class="header-breadcrumb">
          <span class="breadcrumb-prefix">VidNest</span>
          <span class="breadcrumb-separator">/</span>
          <h2 class="breadcrumb-current">
            {{
              currentTab === 'library'
                ? t('header.libraryTitle')
                : currentTab === 'clipper'
                ? t('header.clipperTitle')
                : currentTab === 'clips'
                ? t('header.clipsTitle')
                : currentTab === 'dedup'
                ? t('header.dedupTitle')
                : currentTab === 'audios'
                ? t('header.audiosTitle')
                : currentTab === 'downloads'
                ? t('header.downloadsTitle')
                : currentTab === 'system'
                ? t('header.systemTitle')
                : t('header.settingsTitle')
            }}
          </h2>
        </div>

        <div class="header-actions">
          <!-- Quick Theme Toggle -->
          <button
            class="btn-quick-action"
            :title="activeTheme === 'dark' ? t('header.lightMode') : t('header.darkMode')"
            @click="toggleTheme"
          >
            <span>{{ activeTheme === 'dark' ? '☀️' : '🌙' }}</span>
          </button>

          <!-- Quick Language Toggle -->
          <button
            class="btn-quick-action"
            :title="t('header.switchLanguage')"
            @click="toggleLocale"
          >
            <span class="lang-text">{{ activeLocaleInfo.flag }} {{ activeLocaleInfo.code.split('-')[0].toUpperCase() }}</span>
          </button>

          <button
            class="btn-check-update"
            :class="{ 'has-new-update': discoveredUpdate?.has_update }"
            :title="t('header.checkUpdate')"
            @click="showUpdateModal = true"
          >
            <span class="update-icon">⚡</span>
            <span>{{ t('header.checkUpdate') }}</span>
            <span v-if="discoveredUpdate?.has_update" class="update-badge-pill">
              {{ t('update.badgeNew') }}
            </span>
          </button>
          <span class="engine-badge" v-if="ytdlpCheck?.current_version">
            yt-dlp: {{ ytdlpCheck.current_version }}
          </span>
          <span class="localfirst-pill">
            <span class="pulse-indicator"></span>
            {{ t('app.localFirst') }}
          </span>
        </div>
      </header>

      <!-- Workspace Body View Container -->
      <div class="workspace-body">
        <div v-if="errorMessage" class="global-alert-error">
          <svg class="alert-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
          <span>{{ errorMessage }}</span>
        </div>

        <!-- VIEW 1: VIDEO LIBRARY -->
        <div v-if="currentTab === 'library'" class="view-panel">
          <VideoLibrary
            @navigate-dedup="currentTab = 'dedup'"
            @navigate-audios="currentTab = 'audios'"
            @navigate-clipper="handleNavigateClipper"
          />
        </div>

        <!-- VIEW 1.2: VIDEO CLIPPER STUDIO -->
        <div v-else-if="currentTab === 'clipper'" class="view-panel">
          <VideoClipperStudio
            :initial-video="clipperActiveVideo"
            @navigate-library="currentTab = 'library'"
            @navigate-clips="currentTab = 'clips'"
          />
        </div>

        <!-- VIEW 1.3: CLIPPED VIDEOS MANAGER -->
        <div v-else-if="currentTab === 'clips'" class="view-panel">
          <ClippedVideosManager
            @navigate-library="currentTab = 'library'"
            @navigate-clipper="handleNavigateClipper"
          />
        </div>

        <!-- VIEW 1.5: VIDEO DEDUPLICATION -->
        <div v-else-if="currentTab === 'dedup'" class="view-panel">
          <VideoDeduplication @navigate-library="currentTab = 'library'" />
        </div>

        <!-- VIEW 1.8: EXTRACTED AUDIOS -->
        <div v-else-if="currentTab === 'audios'" class="view-panel">
          <ExtractedAudiosView @navigate-library="currentTab = 'library'" />
        </div>

        <!-- VIEW 2: DOWNLOAD CENTER -->
        <div v-else-if="currentTab === 'downloads'" class="view-panel">
          <DownloadCenter
            @update:active-count="onActiveCountChange"
            @view-in-library="handleViewInLibrary"
          />
        </div>

        <!-- VIEW 3: SETTINGS VIEW -->
        <div v-else-if="currentTab === 'settings'" class="view-panel">
          <AppSettingsView @check-update="showUpdateModal = true" />
        </div>

        <!-- VIEW 4: SYSTEM & UPDATER (SPLIT 2-COLUMN DASHBOARD) -->
        <div v-else class="system-dashboard-view">
          <!-- Overview Mini Cards -->
          <div class="stat-banner-grid">
            <div class="mini-stat-card">
              <span class="stat-title">{{ t('system.appVersion') }}</span>
              <span class="stat-number">{{ healthReport?.app_version || '0.1.0' }}</span>
              <span class="stat-sub">Tauri 2 + Rust + Vue 3</span>
            </div>
            <div class="mini-stat-card">
              <span class="stat-title">{{ t('system.os') }}</span>
              <span class="stat-number">{{ healthReport?.os.os }} ({{ healthReport?.os.arch }})</span>
              <span class="stat-sub">{{ healthReport?.os.family || 'Desktop' }}</span>
            </div>
            <div class="mini-stat-card">
              <span class="stat-title">{{ t('system.ipcLatency') }}</span>
              <span class="stat-number accent-num">{{ pingLatency !== null ? `${pingLatency} ms` : '-' }}</span>
              <span class="stat-sub">Native Core Bridge</span>
            </div>
            <div class="mini-stat-card">
              <span class="stat-title">{{ t('system.healthCheckTime') }}</span>
              <span class="stat-number">{{ healthReport ? formatTime(healthReport.timestamp_ms) : '-' }}</span>
              <span class="stat-sub">{{ t('system.baselineMonitoring') }}</span>
            </div>
          </div>

          <!-- Split Two Columns -->
          <div class="system-split-row">
            <!-- Left: yt-dlp dedicated manager -->
            <div class="split-col-left">
              <div class="card ytdlp-dashboard-card">
                <div class="card-title-bar">
                  <div class="title-with-pill">
                    <h3 class="panel-h3">{{ t('system.ytdlpConsoleTitle') }}</h3>
                    <span class="mode-pill" :class="ytdlpCheck?.is_managed ? 'mode-managed' : 'mode-system'">
                      {{ ytdlpCheck?.is_managed ? t('system.modeManaged') : t('system.modeSystem') }}
                    </span>
                  </div>
                </div>

                <div class="meta-field-list">
                  <div class="meta-field-item">
                    <span class="field-k">{{ t('system.currentVersion') }}</span>
                    <span class="field-v font-mono highlight-v">{{ ytdlpCheck?.current_version || t('system.notDetected') }}</span>
                  </div>
                  <div class="meta-field-item">
                    <span class="field-k">{{ t('system.latestRelease') }}</span>
                    <span class="field-v font-mono">{{ ytdlpCheck?.latest_version || (isCheckingYtdlp ? t('system.fetching') : t('system.unknown')) }}</span>
                  </div>
                  <div class="meta-field-item">
                    <span class="field-k">{{ t('system.updateStatus') }}</span>
                    <div>
                      <span v-if="ytdlpCheck?.has_update" class="badge badge-warning">{{ t('system.newVersionFound') }}</span>
                      <span v-else-if="ytdlpCheck?.current_version" class="badge badge-success">{{ t('system.upToDate') }}</span>
                      <span v-else class="badge badge-danger">{{ t('system.notInstalled') }}</span>
                    </div>
                  </div>
                  <div class="meta-field-item">
                    <span class="field-k">{{ t('system.binaryPath') }}</span>
                    <span class="field-path font-mono" :title="ytdlpCheck?.executable_path || ''">
                      {{ ytdlpCheck?.executable_path || t('system.notLocated') }}
                    </span>
                  </div>
                </div>

                <div class="updater-actions-box">
                  <div class="updater-btns">
                    <button
                      class="btn btn-secondary"
                      :disabled="isCheckingYtdlp || isUpdatingYtdlp"
                      @click="checkYtdlpUpdate"
                    >
                      <svg class="btn-sm-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="11" cy="11" r="8"></circle>
                        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                      </svg>
                      {{ isCheckingYtdlp ? t('system.checking') : t('system.checkNewVersion') }}
                    </button>

                    <button
                      class="btn btn-primary"
                      :disabled="isUpdatingYtdlp"
                      @click="triggerYtdlpUpdate"
                    >
                      <svg class="btn-sm-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                        <polyline points="7 10 12 15 17 10"></polyline>
                        <line x1="12" y1="15" x2="12" y2="3"></line>
                      </svg>
                      {{ isUpdatingYtdlp ? t('system.updating') : (ytdlpCheck?.has_update ? t('system.updateYtdlp') : t('system.reinstallUpdate')) }}
                    </button>
                  </div>

                  <label class="auto-chk-label">
                    <input type="checkbox" v-model="autoCheckYtdlpOnLaunch" />
                    <span>{{ t('system.autoCheckOnLaunch') }}</span>
                  </label>
                </div>

                <div v-if="ytdlpUpdateMsg" class="ytdlp-feedback-box" :class="ytdlpUpdateSuccess === false ? 'fb-err' : 'fb-ok'">
                  <span>{{ ytdlpUpdateMsg }}</span>
                </div>
              </div>
            </div>

            <!-- Right: Storage, IPC -->
            <div class="split-col-right">
              <!-- Storage & Cache -->
              <div class="card section-card">
                <h3 class="panel-h3">{{ t('system.storageEnvTitle') }}</h3>
                <div class="storage-list">
                  <div class="storage-item">
                    <div class="storage-title">{{ t('system.appDataDir') }}</div>
                    <div class="storage-val font-mono">{{ healthReport?.storage.app_data_dir || '-' }}</div>
                  </div>
                  <div class="storage-item">
                    <div class="storage-title">{{ t('system.tempDir') }}</div>
                    <div class="storage-val font-mono">{{ healthReport?.storage.temp_dir || '-' }}</div>
                  </div>
                </div>
              </div>

              <!-- IPC Console -->
              <div class="card section-card">
                <h3 class="panel-h3">{{ t('system.ipcConsoleTitle') }}</h3>
                <div class="ipc-input-row">
                  <input
                    v-model="echoInput"
                    class="ipc-input font-mono"
                    :placeholder="t('system.testInputPlaceholder')"
                    @keyup.enter="testEcho"
                  />
                  <button class="btn btn-primary btn-sm" @click="testEcho">Echo</button>
                  <button class="btn btn-secondary btn-sm" @click="testPing">Ping</button>
                </div>
                <div v-if="echoOutput || pingOutput" class="ipc-console-output font-mono">
                  <div v-if="pingOutput">>>> [Rust Ping]: {{ pingOutput }} ({{ pingLatency }}ms)</div>
                  <div v-if="echoOutput">>>> [Rust Echo]: {{ echoOutput }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- In-App Differential Updater Modal -->
    <AppUpdateModal :visible="showUpdateModal" @close="showUpdateModal = false" />

    <!-- Startup Update Notification Floating Toast -->
    <transition name="toast-slide">
      <div v-if="showStartupUpdateNotice && discoveredUpdate?.has_update" class="startup-update-toast">
        <div class="toast-glow"></div>
        <div class="toast-body">
          <div class="toast-icon-box">🚀</div>
          <div class="toast-content">
            <div class="toast-header-row">
              <span class="toast-title">{{ t('update.newVersionFound') }}</span>
              <span class="toast-ver-badge">v{{ discoveredUpdate.latest_version }}</span>
            </div>
            <p class="toast-desc">
              {{ t('update.newVersionFoundDesc', { version: discoveredUpdate.latest_version }) }}
            </p>
          </div>
          <button class="toast-btn-close" @click="showStartupUpdateNotice = false" :title="t('common.close')">✕</button>
        </div>
        <div class="toast-action-row">
          <button class="btn-toast-dismiss" @click="showStartupUpdateNotice = false">
            {{ t('update.dismiss') }}
          </button>
          <button class="btn-toast-upgrade" @click="showStartupUpdateNotice = false; showUpdateModal = true">
            {{ t('update.upgradeNow') }}
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* App Shell Layout */
.desktop-app {
  display: flex;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background-color: var(--bg-app);
  color: var(--text-primary);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  user-select: none;
}

/* Sidebar */
.app-sidebar {
  width: 240px;
  min-width: 240px;
  max-width: 240px;
  height: 100vh;
  background-color: var(--bg-sidebar);
  border-right: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  padding: 1.25rem 1rem;
  box-sizing: border-box;
  z-index: 50;
}

.sidebar-brand {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0 0.5rem 1.25rem;
  border-bottom: 1px solid var(--border-subtle);
}

.brand-logo {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: linear-gradient(135deg, #3b82f6 0%, #6366f1 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.35);
}

.brand-icon {
  width: 20px;
  height: 20px;
}

.brand-meta {
  display: flex;
  flex-direction: column;
}

.brand-name {
  font-size: 1.15rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--text-primary);
}

.brand-tag {
  font-size: 0.72rem;
  font-weight: 500;
  color: var(--text-muted);
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding-top: 1.25rem;
  flex: 1;
}

.nav-section-title {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  padding: 0 0.65rem 0.5rem;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.65rem 0.85rem;
  border-radius: 9px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.88rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease-in-out;
  position: relative;
  text-align: left;
}

.nav-item:hover {
  background-color: var(--btn-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background-color: var(--nav-active-bg);
  color: #3b82f6;
  font-weight: 600;
}

.nav-item-icon {
  display: flex;
  align-items: center;
  justify-content: center;
}

.nav-item-icon svg {
  width: 18px;
  height: 18px;
}

.nav-item-label {
  flex: 1;
}

.nav-badge {
  background-color: #ef4444;
  color: #ffffff;
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
}

.nav-update-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #ef4444;
  box-shadow: 0 0 8px rgba(239, 68, 68, 0.85);
  animation: pulse-update-dot 2s infinite ease-in-out;
  margin-left: auto;
}

@keyframes pulse-update-dot {
  0% { transform: scale(0.9); opacity: 0.8; }
  50% { transform: scale(1.3); opacity: 1; }
  100% { transform: scale(0.9); opacity: 0.8; }
}

.sidebar-footer {
  padding-top: 1rem;
  border-top: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.health-card {
  background-color: var(--bg-card);
  padding: 0.75rem;
  border-radius: 10px;
  border: 1px solid var(--border-subtle);
}

.health-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8rem;
  font-weight: 600;
}

.health-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.dot-green {
  background-color: #10b981;
  box-shadow: 0 0 8px rgba(16, 185, 129, 0.5);
}

.dot-yellow {
  background-color: #f59e0b;
}

.health-title {
  flex: 1;
  color: var(--text-primary);
}

.latency-badge {
  font-size: 0.7rem;
  font-family: ui-monospace, SFMono-Regular, monospace;
  color: #3b82f6;
  background-color: var(--badge-blue-bg);
  padding: 1px 6px;
  border-radius: 4px;
}

.health-meta {
  font-size: 0.72rem;
  color: var(--text-muted);
  margin-top: 0.35rem;
}

.btn-refresh-sidebar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.5rem;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 0.78rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-refresh-sidebar:hover:not(:disabled) {
  background: var(--btn-hover);
  color: var(--text-primary);
}

.refresh-icon {
  width: 14px;
  height: 14px;
}

.spin-anim {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Workspace */
.app-workspace {
  flex: 1;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: var(--bg-app);
}

.workspace-header {
  height: 52px;
  min-height: 52px;
  padding: 0 1.5rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-subtle);
  background-color: var(--bg-header);
}

.header-breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.breadcrumb-prefix {
  font-size: 0.85rem;
  color: var(--text-muted);
  font-weight: 500;
}

.breadcrumb-separator {
  color: var(--border-subtle);
  font-size: 0.85rem;
}

.breadcrumb-current {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.btn-check-update {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.25rem 0.75rem;
  border-radius: 999px;
  background: rgba(59, 130, 246, 0.12);
  border: 1px solid rgba(59, 130, 246, 0.3);
  color: #60a5fa;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-check-update:hover {
  background: rgba(59, 130, 246, 0.22);
  border-color: rgba(59, 130, 246, 0.5);
  color: #ffffff;
  transform: translateY(-1px);
}

.btn-check-update.has-new-update {
  background: rgba(239, 68, 68, 0.12);
  border-color: rgba(239, 68, 68, 0.45);
  color: #f87171;
}

.update-badge-pill {
  background: #ef4444;
  color: #ffffff;
  font-size: 0.65rem;
  font-weight: 700;
  padding: 0.05rem 0.4rem;
  border-radius: 999px;
  margin-left: 0.2rem;
  animation: pulse-update-dot 2s infinite ease-in-out;
}

.btn-quick-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.25rem 0.65rem;
  border-radius: 999px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  color: var(--text-primary);
  font-size: 0.85rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-quick-action:hover {
  background: var(--btn-hover);
  border-color: var(--accent-primary, #3b82f6);
  transform: translateY(-1px);
}

.lang-text {
  font-size: 0.72rem;
  font-weight: 700;
}

.update-icon {
  font-size: 0.85rem;
}

.engine-badge {
  font-size: 0.75rem;
  font-family: ui-monospace, SFMono-Regular, monospace;
  padding: 0.2rem 0.6rem;
  border-radius: 6px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  color: var(--text-secondary);
}

.localfirst-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.75rem;
  font-weight: 500;
  padding: 0.25rem 0.65rem;
  border-radius: 999px;
  background-color: rgba(16, 185, 129, 0.12);
  color: #10b981;
}

.pulse-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #10b981;
}

.workspace-body {
  flex: 1;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
}

.view-panel {
  flex: 1;
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.global-alert-error {
  padding: 0.75rem 1.25rem;
  background-color: #fee2e2;
  color: #991b1b;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.85rem;
}

.alert-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

/* System Dashboard Split View */
.system-dashboard-view {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.stat-banner-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 1rem;
}

.mini-stat-card {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 12px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.stat-title {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
}

.stat-number {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-sub {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.accent-num {
  color: #3b82f6;
}

.system-split-row {
  display: grid;
  grid-template-columns: 420px 1fr;
  gap: 1.25rem;
  align-items: start;
}

.split-col-left, .split-col-right {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.card {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 12px;
  padding: 1.25rem;
}

.panel-h3 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
}

.title-with-pill {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.mode-pill {
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.2rem 0.55rem;
  border-radius: 6px;
}

.mode-managed {
  background: #dbeafe;
  color: #1e40af;
}

.mode-system {
  background: var(--btn-hover);
  color: var(--text-secondary);
}

.meta-field-list {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  margin: 1.25rem 0;
  padding: 1rem;
  background: var(--bg-input);
  border-radius: 9px;
}

.meta-field-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.82rem;
}

.field-k {
  color: var(--text-muted);
}

.field-v {
  font-weight: 600;
  color: var(--text-primary);
}

.highlight-v {
  color: #3b82f6;
}

.field-path {
  font-size: 0.75rem;
  color: var(--text-muted);
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.updater-actions-box {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.updater-btns {
  display: flex;
  gap: 0.75rem;
}

.auto-chk-label {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.8rem;
  color: var(--text-muted);
  cursor: pointer;
}

.ytdlp-feedback-box {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  font-size: 0.82rem;
}

.fb-ok {
  background: #eff6ff;
  color: #1e40af;
  border: 1px solid #bfdbfe;
}

.fb-err {
  background: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
}

/* Tools item rows */
.section-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.tools-list {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}

.tool-item-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.65rem 0.85rem;
  background: var(--bg-input);
  border-radius: 8px;
}

.tool-left {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.tool-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.tool-badge-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.dot-ok { background-color: #10b981; }
.dot-err { background-color: #ef4444; }

.tool-title-name {
  font-weight: 700;
  font-size: 0.88rem;
}

.tool-description-text {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.tool-ver-text {
  font-size: 0.78rem;
  color: var(--text-secondary);
}

/* Storage list */
.storage-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.storage-item {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.65rem 0.85rem;
  background: var(--bg-input);
  border-radius: 8px;
}

.storage-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.storage-val {
  font-size: 0.75rem;
  color: var(--text-muted);
  word-break: break-all;
}

/* IPC input */
.ipc-input-row {
  display: flex;
  gap: 0.5rem;
}

.ipc-input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.82rem;
  outline: none;
}

.ipc-console-output {
  background: #090d16;
  color: #38bdf8;
  padding: 0.75rem;
  border-radius: 8px;
  font-size: 0.78rem;
  line-height: 1.5;
}

/* Common components & buttons */
.badge {
  font-size: 0.7rem;
  font-weight: 600;
  padding: 0.15rem 0.5rem;
  border-radius: 4px;
}

.badge-success { background: #dcfce7; color: #15803d; }
.badge-warning { background: #fef3c7; color: #b45309; }
.badge-danger { background: #fee2e2; color: #b91c1c; }

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  padding: 0.55rem 0.95rem;
  border-radius: 8px;
  font-size: 0.84rem;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: all 0.15s ease;
}

.btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.btn-primary {
  background: #2563eb;
  color: #ffffff;
}

.btn-primary:hover:not(:disabled) {
  background: #1d4ed8;
}

.btn-secondary {
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-subtle);
}

.btn-secondary:hover:not(:disabled) {
  background: var(--btn-hover);
}

.btn-sm {
  padding: 0.4rem 0.75rem;
  font-size: 0.78rem;
}

.btn-sm-icon {
  width: 15px;
  height: 15px;
}

.font-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}
</style>

<style>
/* Global CSS variables & Theme Modes */
:root,
html[data-theme='dark'] {
  --bg-app: #080c14;
  --bg-sidebar: #0e1524;
  --bg-header: #0e1524;
  --bg-card: #111a2d;
  --bg-input: #172238;
  --border-color: #1f2c45;
  --border-subtle: #1f2c45;
  --text-primary: #f8fafc;
  --text-secondary: #cbd5e1;
  --text-muted: #8896ab;
  --btn-hover: #1e2c47;
  --nav-active-bg: #1e3a8a33;
  --badge-blue-bg: #1e3a8a;
  --accent-primary: #3b82f6;
  --bg-hover: rgba(255, 255, 255, 0.06);
  --bg-badge: rgba(255, 255, 255, 0.05);
  --bg-item: rgba(255, 255, 255, 0.03);
  --accent-subtle: rgba(59, 130, 246, 0.12);
}

html[data-theme='light'] {
  --bg-app: #f8fafc;
  --bg-sidebar: #ffffff;
  --bg-header: #ffffff;
  --bg-card: #ffffff;
  --bg-input: #f1f5f9;
  --border-color: #e2e8f0;
  --border-subtle: #e2e8f0;
  --text-primary: #0f172a;
  --text-secondary: #334155;
  --text-muted: #64748b;
  --btn-hover: #f1f5f9;
  --nav-active-bg: #eff6ff;
  --badge-blue-bg: #dbeafe;
  --accent-primary: #2563eb;
  --bg-hover: rgba(0, 0, 0, 0.05);
  --bg-badge: rgba(0, 0, 0, 0.04);
  --bg-item: #f8fafc;
  --accent-subtle: rgba(37, 99, 235, 0.1);
}

/* Status Badges */
.badge-success { background: #064e3b; color: #6ee7b7; }
.badge-warning { background: #78350f; color: #fde68a; }
.badge-danger { background: #7f1d1d; color: #fca5a5; }

.mode-managed { background: #1e3a8a; color: #93c5fd; }
.mode-system { background: #1e2c47; color: #cbd5e1; }

.fb-ok { background: #172554; color: #93c5fd; border-color: #1e40af; }
.fb-err { background: #450a0a; color: #fca5a5; border-color: #991b1b; }

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
}

/* Custom Sleek Scrollbar */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.25);
  border-radius: 999px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.45);
}

/* Startup Update Floating Toast Card */
.startup-update-toast {
  position: fixed;
  bottom: 24px;
  right: 24px;
  width: 360px;
  max-width: calc(100vw - 48px);
  background: var(--bg-card, #1e293b);
  border: 1px solid rgba(59, 130, 246, 0.35);
  border-radius: 14px;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45), 0 0 20px rgba(59, 130, 246, 0.15);
  overflow: hidden;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  backdrop-filter: blur(12px);
}

.toast-glow {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, #3b82f6, #8b5cf6, #ec4899);
}

.toast-body {
  display: flex;
  align-items: flex-start;
  padding: 1rem 1rem 0.75rem;
  gap: 0.75rem;
  position: relative;
}

.toast-icon-box {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: rgba(59, 130, 246, 0.15);
  border: 1px solid rgba(59, 130, 246, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  flex-shrink: 0;
}

.toast-content {
  flex: 1;
  min-width: 0;
}

.toast-header-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.3rem;
}

.toast-title {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--text-primary);
}

.toast-ver-badge {
  font-size: 0.7rem;
  font-weight: 700;
  color: #60a5fa;
  background: rgba(59, 130, 246, 0.15);
  padding: 0.1rem 0.4rem;
  border-radius: 6px;
}

.toast-desc {
  font-size: 0.78rem;
  color: var(--text-secondary);
  line-height: 1.4;
  margin: 0;
  word-break: break-word;
}

.toast-btn-close {
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.85rem;
  padding: 0.2rem;
  line-height: 1;
  border-radius: 4px;
  transition: all 0.15s ease;
}

.toast-btn-close:hover {
  color: var(--text-primary);
  background: var(--btn-hover);
}

.toast-action-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 0 1rem 0.85rem;
}

.btn-toast-dismiss {
  padding: 0.35rem 0.75rem;
  font-size: 0.78rem;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-toast-dismiss:hover {
  background: var(--btn-hover);
  color: var(--text-primary);
}

.btn-toast-upgrade {
  padding: 0.35rem 0.9rem;
  font-size: 0.78rem;
  font-weight: 600;
  border-radius: 7px;
  border: none;
  background: linear-gradient(135deg, #2563eb, #3b82f6);
  color: #ffffff;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(37, 99, 235, 0.35);
  transition: all 0.15s ease;
}

.btn-toast-upgrade:hover {
  background: linear-gradient(135deg, #1d4ed8, #2563eb);
  box-shadow: 0 4px 12px rgba(37, 99, 235, 0.5);
  transform: translateY(-1px);
}

/* Toast Transition */
.toast-slide-enter-active,
.toast-slide-leave-active {
  transition: all 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}

.toast-slide-enter-from {
  transform: translateY(20px) scale(0.95);
  opacity: 0;
}

.toast-slide-leave-to {
  transform: translateY(20px) scale(0.95);
  opacity: 0;
}
</style>