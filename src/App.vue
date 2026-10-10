<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { HealthReport, YtDlpUpdateCheck, YtDlpUpdateResult } from './types';
import VideoLibrary from './components/VideoLibrary.vue';
import DownloadCenter from './components/DownloadCenter.vue';
import AppUpdateModal from './components/AppUpdateModal.vue';

const currentTab = ref<'library' | 'downloads' | 'system'>('library');
const activeDownloadsCount = ref<number>(0);
const showUpdateModal = ref<boolean>(false);

function onActiveCountChange(count: number) {
  activeDownloadsCount.value = count;
}

function handleViewInLibrary(_payload: { fileName?: string; filePath?: string }) {
  currentTab.value = 'library';
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
const echoInput = ref<string>('视频媒体库与 FTS5 引擎基线测试');
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
    ytdlpUpdateMsg.value = `检查更新失败: ${err instanceof Error ? err.message : String(err)}`;
    ytdlpUpdateSuccess.value = false;
  } finally {
    isCheckingYtdlp.value = false;
  }
}

async function triggerYtdlpUpdate() {
  isUpdatingYtdlp.value = true;
  ytdlpUpdateMsg.value = '正在下载并自动更新 yt-dlp 核心引擎，请稍候...';
  ytdlpUpdateSuccess.value = null;
  try {
    const res = await invoke<YtDlpUpdateResult>('update_ytdlp');
    ytdlpUpdateSuccess.value = res.success;
    ytdlpUpdateMsg.value = `[${res.method === 'github_download' ? 'GitHub 独立托管下载' : '原生更新'}] ${res.message}`;
    await fetchHealth();
    await checkYtdlpUpdate();
  } catch (err: unknown) {
    ytdlpUpdateSuccess.value = false;
    ytdlpUpdateMsg.value = `更新失败: ${err instanceof Error ? err.message : String(err)}`;
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
    pingOutput.value = `Ping 失败: ${err}`;
  }
}

async function testEcho() {
  try {
    const res = await invoke<string>('echo', { message: echoInput.value });
    echoOutput.value = res;
  } catch (err: unknown) {
    echoOutput.value = `Echo 失败: ${err}`;
  }
}

function formatTime(timestampMs: number): string {
  if (!timestampMs) return '-';
  const d = new Date(timestampMs);
  return d.toLocaleTimeString();
}

onMounted(async () => {
  await fetchHealth();
  await testPing();
  if (autoCheckYtdlpOnLaunch.value) {
    await checkYtdlpUpdate();
  }
});
</script>

<template>
  <div class="desktop-app">
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
        <div class="nav-section-title">核心功能</div>

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
          <span class="nav-item-label">本地媒体库</span>
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
          <span class="nav-item-label">视频下载中心</span>
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
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
            </svg>
          </div>
          <span class="nav-item-label">环境与引擎状态</span>
        </button>
      </nav>

      <!-- Sidebar Footer Status -->
      <div class="sidebar-footer">
        <div class="health-card">
          <div class="health-row">
            <span class="health-status-dot" :class="healthReport?.status === 'healthy' ? 'dot-green' : 'dot-yellow'"></span>
            <span class="health-title">{{ healthReport?.status === 'healthy' ? '引擎就绪' : '环境受限' }}</span>
            <span v-if="pingLatency !== null" class="latency-badge">{{ pingLatency }}ms</span>
          </div>
          <div class="health-meta">
            <span>{{ healthReport?.os.os || 'Desktop' }} · v{{ healthReport?.app_version || '0.1.0' }}</span>
          </div>
        </div>

        <button class="btn-refresh-sidebar" :disabled="isLoading" @click="fetchHealth" title="刷新系统健康状态">
          <svg class="refresh-icon" :class="{ 'spin-anim': isLoading }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
          </svg>
          <span>{{ isLoading ? '检测中...' : '检查环境' }}</span>
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
            {{ currentTab === 'library' ? '本地媒体资产库' : currentTab === 'downloads' ? 'yt-dlp 在线下载中心' : '系统运行环境与引擎中心' }}
          </h2>
        </div>

        <div class="header-actions">
          <button class="btn-check-update" title="检查新版本 (应用内差分增量升级)" @click="showUpdateModal = true">
            <span class="update-icon">⚡</span>
            <span>检查更新</span>
          </button>
          <span class="engine-badge" v-if="ytdlpCheck?.current_version">
            yt-dlp: {{ ytdlpCheck.current_version }}
          </span>
          <span class="localfirst-pill">
            <span class="pulse-indicator"></span>
            本地优先
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
          <VideoLibrary />
        </div>

        <!-- VIEW 2: DOWNLOAD CENTER -->
        <div v-else-if="currentTab === 'downloads'" class="view-panel">
          <DownloadCenter
            @update:active-count="onActiveCountChange"
            @view-in-library="handleViewInLibrary"
          />
        </div>

        <!-- VIEW 3: SYSTEM & UPDATER (SPLIT 2-COLUMN DASHBOARD) -->
        <div v-else class="system-dashboard-view">
          <!-- Overview Mini Cards -->
          <div class="stat-banner-grid">
            <div class="mini-stat-card">
              <span class="stat-title">应用版本</span>
              <span class="stat-number">{{ healthReport?.app_version || '0.1.0' }}</span>
              <span class="stat-sub">Tauri 2 + Rust + Vue 3</span>
            </div>
            <div class="mini-stat-card">
              <span class="stat-title">操作系统</span>
              <span class="stat-number">{{ healthReport?.os.os }} ({{ healthReport?.os.arch }})</span>
              <span class="stat-sub">{{ healthReport?.os.family || 'Desktop' }}</span>
            </div>
            <div class="mini-stat-card">
              <span class="stat-title">Rust IPC 响应延迟</span>
              <span class="stat-number accent-num">{{ pingLatency !== null ? `${pingLatency} ms` : '-' }}</span>
              <span class="stat-sub">Native Core Bridge</span>
            </div>
            <div class="mini-stat-card">
              <span class="stat-title">健康检测时间</span>
              <span class="stat-number">{{ healthReport ? formatTime(healthReport.timestamp_ms) : '-' }}</span>
              <span class="stat-sub">自动基线监控</span>
            </div>
          </div>

          <!-- Split Two Columns -->
          <div class="system-split-row">
            <!-- Left: yt-dlp dedicated manager -->
            <div class="split-col-left">
              <div class="card ytdlp-dashboard-card">
                <div class="card-title-bar">
                  <div class="title-with-pill">
                    <h3 class="panel-h3">yt-dlp 引擎自动更新控制台</h3>
                    <span class="mode-pill" :class="ytdlpCheck?.is_managed ? 'mode-managed' : 'mode-system'">
                      {{ ytdlpCheck?.is_managed ? '独立托管模式' : '系统环境变量' }}
                    </span>
                  </div>
                </div>

                <div class="meta-field-list">
                  <div class="meta-field-item">
                    <span class="field-k">当前运行版本</span>
                    <span class="field-v font-mono highlight-v">{{ ytdlpCheck?.current_version || '未检测到' }}</span>
                  </div>
                  <div class="meta-field-item">
                    <span class="field-k">官方最新发布</span>
                    <span class="field-v font-mono">{{ ytdlpCheck?.latest_version || (isCheckingYtdlp ? '获取中...' : '未知') }}</span>
                  </div>
                  <div class="meta-field-item">
                    <span class="field-k">更新状态</span>
                    <div>
                      <span v-if="ytdlpCheck?.has_update" class="badge badge-warning">发现新版本</span>
                      <span v-else-if="ytdlpCheck?.current_version" class="badge badge-success">已是最新版本</span>
                      <span v-else class="badge badge-danger">未安装</span>
                    </div>
                  </div>
                  <div class="meta-field-item">
                    <span class="field-k">二进制路径</span>
                    <span class="field-path font-mono" :title="ytdlpCheck?.executable_path || ''">
                      {{ ytdlpCheck?.executable_path || '未定位' }}
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
                      {{ isCheckingYtdlp ? '检查中...' : '检查新版本' }}
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
                      {{ isUpdatingYtdlp ? '更新进行中...' : (ytdlpCheck?.has_update ? '立即更新 yt-dlp' : '重装/更新最新版本') }}
                    </button>
                  </div>

                  <label class="auto-chk-label">
                    <input type="checkbox" v-model="autoCheckYtdlpOnLaunch" />
                    <span>启动应用时自动静默检查更新</span>
                  </label>
                </div>

                <div v-if="ytdlpUpdateMsg" class="ytdlp-feedback-box" :class="ytdlpUpdateSuccess === false ? 'fb-err' : 'fb-ok'">
                  <span>{{ ytdlpUpdateMsg }}</span>
                </div>
              </div>
            </div>

            <!-- Right: Tools, Storage, IPC -->
            <div class="split-col-right">
              <!-- Tools Grid -->
              <div class="card section-card">
                <h3 class="panel-h3">底层工具链与多媒体引擎检测</h3>
                <div class="tools-list">
                  <div
                    v-for="tool in healthReport?.tools || []"
                    :key="tool.name"
                    class="tool-item-row"
                  >
                    <div class="tool-left">
                      <div class="tool-title-row">
                        <span class="tool-badge-dot" :class="tool.found ? 'dot-ok' : 'dot-err'"></span>
                        <span class="tool-title-name font-mono">{{ tool.name }}</span>
                        <span class="badge" :class="tool.found ? 'badge-success' : 'badge-danger'">
                          {{ tool.found ? '已就绪' : '缺失' }}
                        </span>
                      </div>
                      <span class="tool-description-text">{{ tool.description }}</span>
                    </div>
                    <div class="tool-right font-mono">
                      <span class="tool-ver-text">{{ tool.version || '未安装' }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Storage & Cache -->
              <div class="card section-card">
                <h3 class="panel-h3">本地存储与缓存环境</h3>
                <div class="storage-list">
                  <div class="storage-item">
                    <div class="storage-title">应用持久化数据目录 (App Data)</div>
                    <div class="storage-val font-mono">{{ healthReport?.storage.app_data_dir || '-' }}</div>
                  </div>
                  <div class="storage-item">
                    <div class="storage-title">临时处理缓存目录 (Temp Dir)</div>
                    <div class="storage-val font-mono">{{ healthReport?.storage.temp_dir || '-' }}</div>
                  </div>
                </div>
              </div>

              <!-- IPC Console -->
              <div class="card section-card">
                <h3 class="panel-h3">前后端通信联调控制台 (IPC Bridge)</h3>
                <div class="ipc-input-row">
                  <input
                    v-model="echoInput"
                    class="ipc-input font-mono"
                    placeholder="输入测试内容..."
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
/* Global CSS variables & Reset */
:root {
  --bg-app: #f8fafc;
  --bg-sidebar: #ffffff;
  --bg-header: #ffffff;
  --bg-card: #ffffff;
  --bg-input: #f1f5f9;
  --text-primary: #0f172a;
  --text-secondary: #334155;
  --text-muted: #64748b;
  --border-subtle: #e2e8f0;
  --btn-hover: #e2e8f0;
  --nav-active-bg: #eff6ff;
  --badge-blue-bg: #dbeafe;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-app: #080c14;
    --bg-sidebar: #0e1524;
    --bg-header: #0e1524;
    --bg-card: #111a2d;
    --bg-input: #172238;
    --text-primary: #f8fafc;
    --text-secondary: #cbd5e1;
    --text-muted: #8896ab;
    --border-subtle: #1f2c45;
    --btn-hover: #1e2c47;
    --nav-active-bg: #1e3a8a33;
    --badge-blue-bg: #1e3a8a;
  }

  .badge-success { background: #064e3b; color: #6ee7b7; }
  .badge-warning { background: #78350f; color: #fde68a; }
  .badge-danger { background: #7f1d1d; color: #fca5a5; }

  .mode-managed { background: #1e3a8a; color: #93c5fd; }
  .mode-system { background: #1e2c47; color: #cbd5e1; }

  .fb-ok { background: #172554; color: #93c5fd; border-color: #1e40af; }
  .fb-err { background: #450a0a; color: #fca5a5; border-color: #991b1b; }
}

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
</style>