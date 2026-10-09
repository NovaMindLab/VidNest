<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { HealthReport, YtDlpUpdateCheck, YtDlpUpdateResult } from './types';
import VideoLibrary from './components/VideoLibrary.vue';
import DownloadCenter from './components/DownloadCenter.vue';

const currentTab = ref<'library' | 'downloads' | 'system'>('library');
const activeDownloadsCount = ref<number>(0);

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
  <div class="app-layout">
    <!-- Header -->
    <header class="app-header">
      <div class="header-left">
        <div class="logo-badge">
          <svg class="app-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="5 3 19 12 5 21 5 3"></polygon>
          </svg>
        </div>
        <div>
          <h1 class="header-title">Local Video Intelligence</h1>
          <p class="header-subtitle">开源本地 AI 视频管理工具 · 本地优先 (Local-First)</p>
        </div>
      </div>

      <!-- Center Navigation Tabs -->
      <nav class="nav-tabs">
        <button
          class="nav-tab-btn"
          :class="{ active: currentTab === 'library' }"
          @click="currentTab = 'library'"
        >
          <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect>
            <line x1="7" y1="2" x2="7" y2="22"></line>
            <line x1="17" y1="2" x2="17" y2="22"></line>
            <line x1="2" y1="12" x2="22" y2="12"></line>
          </svg>
          本地媒体库
        </button>

        <button
          class="nav-tab-btn"
          :class="{ active: currentTab === 'downloads' }"
          @click="currentTab = 'downloads'"
        >
          <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
          下载中心
          <span v-if="activeDownloadsCount > 0" class="tab-badge-count">
            {{ activeDownloadsCount }}
          </span>
        </button>

        <button
          class="nav-tab-btn"
          :class="{ active: currentTab === 'system' }"
          @click="currentTab = 'system'"
        >
          <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
          </svg>
          环境与更新引擎
        </button>
      </nav>

      <div class="header-right">
        <span
          v-if="healthReport"
          class="status-pill"
          :class="healthReport.status === 'healthy' ? 'pill-healthy' : 'pill-degraded'"
        >
          <span class="status-dot"></span>
          {{ healthReport.status === 'healthy' ? '就绪 (Healthy)' : '环境受限' }}
        </span>
        <button class="btn btn-secondary btn-header" :disabled="isLoading" @click="fetchHealth">
          <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
          </svg>
          {{ isLoading ? '检测中...' : '刷新状态' }}
        </button>
      </div>
    </header>

    <!-- Main Content -->
    <main class="app-main">
      <div v-if="errorMessage" class="alert alert-error">
        <strong>健康检查异常：</strong> {{ errorMessage }}
      </div>

      <!-- TAB 1: VIDEO LIBRARY -->
      <div v-if="currentTab === 'library'">
        <VideoLibrary />
      </div>

      <!-- TAB 2: DOWNLOAD CENTER -->
      <div v-else-if="currentTab === 'downloads'">
        <DownloadCenter
          @update:active-count="onActiveCountChange"
          @view-in-library="handleViewInLibrary"
        />
      </div>

      <!-- TAB 3: SYSTEM HEALTH & YT-DLP UPDATER -->
      <div v-else>
        <!-- Overview Cards -->
        <section class="section-grid">
          <div class="card stat-card">
            <div class="card-header-sm">应用版本</div>
            <div class="stat-value">{{ healthReport?.app_version || '0.1.0' }}</div>
            <div class="stat-desc">Tauri 2 + Rust + Vue 3</div>
          </div>

          <div class="card stat-card">
            <div class="card-header-sm">宿主系统</div>
            <div class="stat-value">
              {{ healthReport?.os.os }} ({{ healthReport?.os.arch }})
            </div>
            <div class="stat-desc">Family: {{ healthReport?.os.family || 'Windows' }}</div>
          </div>

          <div class="card stat-card">
            <div class="card-header-sm">IPC 延迟</div>
            <div class="stat-value">
              {{ pingLatency !== null ? `${pingLatency} ms` : '-' }}
            </div>
            <div class="stat-desc">Rust Native Bridge</div>
          </div>

          <div class="card stat-card">
            <div class="card-header-sm">检查时间</div>
            <div class="stat-value">
              {{ healthReport ? formatTime(healthReport.timestamp_ms) : '-' }}
            </div>
            <div class="stat-desc">自动基线监控</div>
          </div>
        </section>

        <!-- yt-dlp Auto-Update Dedicated Center -->
        <section class="content-block">
          <div class="section-header-flex">
            <h2 class="section-title">yt-dlp 下载引擎与自动更新中心</h2>
            <span class="managed-tag" :class="ytdlpCheck?.is_managed ? 'tag-managed' : 'tag-system'">
              {{ ytdlpCheck?.is_managed ? '模式：应用内置独立托管 (可自更新)' : '模式：系统环境版本' }}
            </span>
          </div>

          <div class="card ytdlp-manager-card">
            <div class="ytdlp-grid">
              <div class="ytdlp-item">
                <span class="ytdlp-label">当前运行版本</span>
                <span class="ytdlp-value">{{ ytdlpCheck?.current_version || '未检测到' }}</span>
              </div>
              <div class="ytdlp-item">
                <span class="ytdlp-label">官方最新发布</span>
                <span class="ytdlp-value font-mono">
                  {{ ytdlpCheck?.latest_version || (isCheckingYtdlp ? '获取中...' : '未知') }}
                </span>
              </div>
              <div class="ytdlp-item">
                <span class="ytdlp-label">更新状态</span>
                <div>
                  <span
                    v-if="ytdlpCheck?.has_update"
                    class="badge badge-warning"
                  >
                    发现新版本
                  </span>
                  <span
                    v-else-if="ytdlpCheck?.current_version"
                    class="badge badge-success"
                  >
                    已是最新版
                  </span>
                  <span v-else class="badge badge-danger">未安装</span>
                </div>
              </div>
              <div class="ytdlp-item">
                <span class="ytdlp-label">二进制位置</span>
                <span class="ytdlp-path" :title="ytdlpCheck?.executable_path || ''">
                  {{ ytdlpCheck?.executable_path || '未定位' }}
                </span>
              </div>
            </div>

            <div class="ytdlp-actions">
              <div class="action-buttons-left">
                <button
                  class="btn btn-secondary"
                  :disabled="isCheckingYtdlp || isUpdatingYtdlp"
                  @click="checkYtdlpUpdate"
                >
                  <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
                  <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="7 10 12 15 17 10"></polyline>
                    <line x1="12" y1="15" x2="12" y2="3"></line>
                  </svg>
                  {{ isUpdatingYtdlp ? '正在自动更新...' : (ytdlpCheck?.has_update ? '立即更新 yt-dlp' : '下载/强制重装最新托管版') }}
                </button>
              </div>

              <div class="auto-update-toggle">
                <label class="toggle-label">
                  <input type="checkbox" v-model="autoCheckYtdlpOnLaunch" />
                  <span>应用启动时自动检查更新</span>
                </label>
              </div>
            </div>

            <div v-if="ytdlpUpdateMsg" class="ytdlp-feedback" :class="ytdlpUpdateSuccess === false ? 'feedback-err' : 'feedback-info'">
              <span class="feedback-icon">{{ ytdlpUpdateSuccess === false ? '⚠️' : 'ℹ️' }}</span>
              <span>{{ ytdlpUpdateMsg }}</span>
            </div>
          </div>
        </section>

        <!-- Tool Diagnostics -->
        <section class="content-block">
          <h2 class="section-title">核心工具链与引擎检测</h2>
          <div class="tools-grid">
            <div
              v-for="tool in healthReport?.tools || []"
              :key="tool.name"
              class="card tool-card"
              :class="{ 'tool-card-ok': tool.found, 'tool-card-missing': !tool.found }"
            >
              <div class="tool-top">
                <span class="tool-name">{{ tool.name }}</span>
                <span class="badge" :class="tool.found ? 'badge-success' : 'badge-danger'">
                  {{ tool.found ? '已就绪' : '缺失' }}
                </span>
              </div>
              <p class="tool-desc">{{ tool.description }}</p>
              <div class="tool-meta">
                <span class="tool-label">版本：</span>
                <span class="tool-version" :title="tool.version || '未检测到版本'">
                  {{ tool.version || '未安装或无法执行' }}
                </span>
              </div>
              <div v-if="tool.path" class="tool-meta" style="margin-top: 0.35rem">
                <span class="tool-label">路径：</span>
                <span class="tool-version" :title="tool.path">{{ tool.path }}</span>
              </div>
            </div>
          </div>
        </section>

        <!-- Storage & Directory Health -->
        <section class="content-block">
          <h2 class="section-title">本地存储与缓存环境</h2>
          <div class="card storage-card">
            <div class="storage-row">
              <div class="storage-info">
                <div class="storage-name">应用持久化数据目录 (App Data Directory)</div>
                <div class="storage-path">{{ healthReport?.storage.app_data_dir || '-' }}</div>
              </div>
              <span
                class="badge"
                :class="healthReport?.storage.app_data_writable ? 'badge-success' : 'badge-danger'"
              >
                {{ healthReport?.storage.app_data_writable ? '读写正常' : '写权限受限' }}
              </span>
            </div>

            <div class="storage-divider"></div>

            <div class="storage-row">
              <div class="storage-info">
                <div class="storage-name">系统临时目录 (Temp Directory)</div>
                <div class="storage-path">{{ healthReport?.storage.temp_dir || '-' }}</div>
              </div>
              <span
                class="badge"
                :class="healthReport?.storage.temp_writable ? 'badge-success' : 'badge-danger'"
              >
                {{ healthReport?.storage.temp_writable ? '读写正常' : '写权限受限' }}
              </span>
            </div>
          </div>
        </section>

        <!-- Rust Command IPC Test -->
        <section class="content-block">
          <h2 class="section-title">前后端通信联调控制台 (IPC Command Baseline)</h2>
          <div class="card test-card">
            <div class="test-row">
              <div class="input-group">
                <label for="echo-input" class="input-label">发送测试消息至 Rust 后端：</label>
                <div class="input-action-row">
                  <input
                    id="echo-input"
                    v-model="echoInput"
                    class="text-input"
                    placeholder="输入测试内容..."
                    @keyup.enter="testEcho"
                  />
                  <button class="btn btn-primary" @click="testEcho">调用 Echo Command</button>
                  <button class="btn btn-secondary" @click="testPing">发送 Ping</button>
                </div>
              </div>
            </div>

            <div v-if="echoOutput || pingOutput" class="console-box">
              <div v-if="pingOutput" class="console-line">
                <span class="console-prompt">[Rust Ping]</span> {{ pingOutput }} (耗时: {{ pingLatency }}ms)
              </div>
              <div v-if="echoOutput" class="console-line">
                <span class="console-prompt">[Rust Echo]</span> {{ echoOutput }}
              </div>
            </div>
          </div>
        </section>
      </div>
    </main>

    <!-- Footer -->
    <footer class="app-footer">
      <span>Local Video Intelligence · Local-First Architecture</span>
      <span>阶段三：yt-dlp 视频下载引擎与来源追溯中心已就绪</span>
    </footer>
  </div>
</template>

<style scoped>
.app-layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-color);
  color: var(--text-color);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
}

.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 2rem;
  background-color: var(--header-bg);
  border-bottom: 1px solid var(--border-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.logo-badge {
  width: 40px;
  height: 40px;
  background: linear-gradient(135deg, #3b82f6, #1d4ed8);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  box-shadow: 0 4px 10px rgba(59, 130, 246, 0.3);
}

.app-icon {
  width: 22px;
  height: 22px;
}

.header-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.header-subtitle {
  margin: 0.15rem 0 0;
  font-size: 0.8rem;
  color: var(--text-muted);
}

.nav-tabs {
  display: flex;
  background-color: var(--btn-sec-bg);
  padding: 3px;
  border-radius: 10px;
  gap: 4px;
}

.nav-tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1.15rem;
  border-radius: 8px;
  border: none;
  background: none;
  color: var(--text-muted);
  font-size: 0.88rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
}

.nav-tab-btn:hover {
  color: var(--text-color);
}

.nav-tab-btn.active {
  background-color: var(--card-bg);
  color: #2563eb;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.tab-badge-count {
  background-color: #ef4444;
  color: white;
  font-size: 0.72rem;
  font-weight: 700;
  padding: 0.1rem 0.45rem;
  border-radius: 9999px;
  line-height: 1.1;
  min-width: 14px;
  text-align: center;
  box-shadow: 0 1px 3px rgba(239, 68, 68, 0.4);
}

.tab-icon {
  width: 16px;
  height: 16px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.btn-header {
  padding: 0.45rem 0.85rem;
  font-size: 0.82rem;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.3rem 0.75rem;
  border-radius: 9999px;
  font-size: 0.8rem;
  font-weight: 600;
}

.pill-healthy {
  background-color: #dcfce7;
  color: #15803d;
}

.pill-degraded {
  background-color: #fef3c7;
  color: #b45309;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background-color: currentColor;
}

.app-main {
  flex: 1;
  padding: 2rem;
  max-width: 1300px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 1.25rem;
  margin-bottom: 2rem;
}

.card {
  background-color: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 1.25rem;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.stat-card {
  display: flex;
  flex-direction: column;
}

.card-header-sm {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-bottom: 0.5rem;
}

.stat-value {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-color);
  margin-bottom: 0.25rem;
}

.stat-desc {
  font-size: 0.8rem;
  color: var(--text-muted);
}

.content-block {
  margin-bottom: 2rem;
}

.section-header-flex {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.section-title {
  font-size: 1.15rem;
  font-weight: 600;
  margin: 0;
  color: var(--text-color);
}

.managed-tag {
  font-size: 0.78rem;
  padding: 0.25rem 0.65rem;
  border-radius: 6px;
  font-weight: 600;
}

.tag-managed {
  background-color: #dbeafe;
  color: #1e40af;
}

.tag-system {
  background-color: #f3f4f6;
  color: #4b5563;
}

.ytdlp-manager-card {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
}

.ytdlp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
  background-color: var(--bg-color);
  padding: 1rem;
  border-radius: 8px;
}

.ytdlp-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.ytdlp-label {
  font-size: 0.78rem;
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
}

.ytdlp-value {
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--text-color);
}

.ytdlp-path {
  font-size: 0.78rem;
  color: var(--text-muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ytdlp-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}

.action-buttons-left {
  display: flex;
  gap: 0.75rem;
}

.auto-update-toggle {
  display: flex;
  align-items: center;
}

.toggle-label {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.85rem;
  color: var(--text-muted);
  cursor: pointer;
}

.ytdlp-feedback {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  font-size: 0.88rem;
}

.feedback-info {
  background-color: #eff6ff;
  color: #1e40af;
  border: 1px solid #bfdbfe;
}

.feedback-err {
  background-color: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 1.25rem;
}

.tool-card {
  display: flex;
  flex-direction: column;
  transition: transform 0.2s, box-shadow 0.2s;
}

.tool-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.tool-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.tool-name {
  font-size: 1.1rem;
  font-weight: 700;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.tool-desc {
  font-size: 0.85rem;
  color: var(--text-muted);
  margin: 0 0 1rem;
  flex: 1;
  line-height: 1.4;
}

.tool-meta {
  background-color: var(--bg-color);
  padding: 0.6rem 0.85rem;
  border-radius: 8px;
  font-size: 0.8rem;
  display: flex;
  align-items: baseline;
  gap: 0.4rem;
  overflow: hidden;
}

.tool-label {
  color: var(--text-muted);
  white-space: nowrap;
}

.tool-version {
  color: var(--text-color);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge {
  display: inline-block;
  padding: 0.25rem 0.6rem;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
}

.badge-success {
  background-color: #dcfce7;
  color: #15803d;
}

.badge-warning {
  background-color: #fef3c7;
  color: #b45309;
}

.badge-danger {
  background-color: #fee2e2;
  color: #b91c1c;
}

.storage-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.storage-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.storage-name {
  font-size: 0.95rem;
  font-weight: 600;
  margin-bottom: 0.25rem;
}

.storage-path {
  font-size: 0.82rem;
  color: var(--text-muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  word-break: break-all;
}

.storage-divider {
  height: 1px;
  background-color: var(--border-color);
}

.test-card {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.input-label {
  display: block;
  font-size: 0.88rem;
  font-weight: 600;
  margin-bottom: 0.5rem;
}

.input-action-row {
  display: flex;
  gap: 0.75rem;
}

.text-input {
  flex: 1;
  padding: 0.65rem 0.85rem;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background-color: var(--input-bg);
  color: var(--text-color);
  font-size: 0.95rem;
  outline: none;
  transition: border-color 0.2s;
}

.text-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.65rem 1.15rem;
  border-radius: 8px;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: all 0.2s;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-primary {
  background-color: #2563eb;
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background-color: #1d4ed8;
}

.btn-secondary {
  background-color: var(--btn-sec-bg);
  color: var(--text-color);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover:not(:disabled) {
  background-color: var(--btn-sec-hover);
}

.btn-icon {
  width: 16px;
  height: 16px;
}

.console-box {
  background-color: #0f172a;
  color: #f8fafc;
  padding: 1rem;
  border-radius: 8px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.console-prompt {
  color: #38bdf8;
  font-weight: 600;
}

.app-footer {
  padding: 1rem 2rem;
  border-top: 1px solid var(--border-color);
  font-size: 0.8rem;
  color: var(--text-muted);
  display: flex;
  justify-content: space-between;
  background-color: var(--header-bg);
}

.alert-error {
  background-color: #fee2e2;
  color: #991b1b;
  border: 1px solid #f87171;
  padding: 0.85rem 1.25rem;
  border-radius: 8px;
  margin-bottom: 1.5rem;
}
</style>

<style>
:root {
  --bg-color: #f8fafc;
  --header-bg: #ffffff;
  --card-bg: #ffffff;
  --text-color: #0f172a;
  --text-muted: #64748b;
  --border-color: #e2e8f0;
  --input-bg: #ffffff;
  --btn-sec-bg: #f1f5f9;
  --btn-sec-hover: #e2e8f0;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-color: #0b0f19;
    --header-bg: #111827;
    --card-bg: #111827;
    --text-color: #f9fafb;
    --text-muted: #9ca3af;
    --border-color: #1f2937;
    --input-bg: #1f2937;
    --btn-sec-bg: #1f2937;
    --btn-sec-hover: #374151;
  }

  .pill-healthy {
    background-color: #064e3b;
    color: #6ee7b7;
  }

  .pill-degraded {
    background-color: #78350f;
    color: #fde68a;
  }

  .badge-success {
    background-color: #064e3b;
    color: #6ee7b7;
  }

  .badge-warning {
    background-color: #78350f;
    color: #fde68a;
  }

  .badge-danger {
    background-color: #7f1d1d;
    color: #fca5a5;
  }

  .alert-error {
    background-color: #450a0a;
    color: #fca5a5;
    border-color: #991b1b;
  }

  .tag-managed {
    background-color: #1e3a8a;
    color: #93c5fd;
  }

  .tag-system {
    background-color: #374151;
    color: #d1d5db;
  }

  .feedback-info {
    background-color: #172554;
    color: #93c5fd;
    border-color: #1e40af;
  }

  .feedback-err {
    background-color: #450a0a;
    color: #fca5a5;
    border-color: #991b1b;
  }
}

body {
  margin: 0;
  padding: 0;
  overflow-x: hidden;
}
</style>