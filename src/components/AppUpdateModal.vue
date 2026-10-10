<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppUpdateInfo, UpdateProgressPayload, UpdateInstallResult } from '../types';

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const updateInfo = ref<AppUpdateInfo | null>(null);
const isChecking = ref(false);
const isDownloading = ref(false);
const isReady = ref(false);
const errorMessage = ref('');
const progress = ref<UpdateProgressPayload | null>(null);

let unlistenProgress: UnlistenFn | null = null;

function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

async function checkForUpdate(mock = false) {
  isChecking.value = true;
  errorMessage.value = '';
  try {
    const res = await invoke<AppUpdateInfo>('check_app_update', { mock });
    updateInfo.value = res;
  } catch (err: unknown) {
    errorMessage.value = `检查更新失败: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    isChecking.value = false;
  }
}

async function startUpdate() {
  if (!updateInfo.value) return;
  isDownloading.value = true;
  errorMessage.value = '';
  progress.value = {
    status: 'downloading',
    downloaded_bytes: 0,
    total_bytes: updateInfo.value.download_size_bytes,
    percentage: 0,
    speed_mbps: 0,
    message: '正在初始化下载链接...',
  };

  try {
    const res = await invoke<UpdateInstallResult>('download_and_install_update', {
      downloadUrl: updateInfo.value.download_url,
      totalBytes: updateInfo.value.download_size_bytes,
      sha256: updateInfo.value.sha256,
    });
    if (res.success) {
      isReady.value = true;
    } else {
      errorMessage.value = res.message;
    }
  } catch (err: unknown) {
    errorMessage.value = `下载更新失败: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    isDownloading.value = false;
  }
}

async function relaunch() {
  try {
    await invoke('install_update_and_relaunch');
  } catch (err) {
    console.warn('install_update_and_relaunch failed, falling back to relaunch_app:', err);
    try {
      await invoke('relaunch_app');
    } catch (e) {
      console.error('Failed to relaunch:', e);
    }
  }
}

async function openInstallerFolder() {
  try {
    await invoke('open_update_installer_folder');
  } catch (err) {
    console.error('Failed to open installer folder:', err);
  }
}

function close() {
  emit('close');
}

watch(
  () => props.visible,
  (val) => {
    if (val && !updateInfo.value) {
      checkForUpdate(false);
    }
  }
);

onMounted(async () => {
  unlistenProgress = await listen<UpdateProgressPayload>('app-update-progress', (event) => {
    progress.value = event.payload;
    if (event.payload.status === 'ready') {
      isReady.value = true;
      isDownloading.value = false;
    }
  });
});

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress();
  }
});
</script>

<template>
  <div v-if="visible" class="update-modal-backdrop" @click.self="close">
    <div class="update-modal-container">
      <!-- Modal Header -->
      <div class="update-header">
        <div class="header-title-box">
          <span class="header-icon">🚀</span>
          <div>
            <h3 class="update-title">应用内差分增量升级</h3>
            <span class="update-subtitle">VidNest 现代化软件更新交付中心</span>
          </div>
        </div>
        <button class="btn-close" title="关闭 (Esc)" @click="close">✕</button>
      </div>

      <!-- Loading State -->
      <div v-if="isChecking" class="update-body state-loading">
        <div class="spinner-lg"></div>
        <p class="loading-text">正在查询并计算增量差分补丁，请稍候...</p>
      </div>

      <!-- Error State -->
      <div v-else-if="errorMessage" class="update-body state-error">
        <div class="err-icon">⚠️</div>
        <p class="err-msg">{{ errorMessage }}</p>
        <button class="btn-retry" @click="checkForUpdate(false)">重新检查更新</button>
      </div>

      <!-- Up to Date -->
      <div v-else-if="updateInfo && !updateInfo.has_update" class="update-body state-uptodate">
        <div class="success-icon">✓</div>
        <h4 class="uptodate-h4">当前已是最新版本</h4>
        <p class="uptodate-p">当前版本 v{{ updateInfo.current_version }}，无需更新。</p>
        <button class="btn-primary" @click="close">好的</button>
      </div>

      <!-- Has Update Content -->
      <div v-else-if="updateInfo && updateInfo.has_update" class="update-body">
        <!-- Versions & Delta Badge -->
        <div class="version-banner">
          <div class="ver-col">
            <span class="ver-label">当前版本</span>
            <span class="ver-val v-cur">v{{ updateInfo.current_version }}</span>
          </div>
          <div class="arrow-sep">➔</div>
          <div class="ver-col">
            <span class="ver-label">最新版本</span>
            <span class="ver-val v-new">v{{ updateInfo.latest_version }}</span>
          </div>

          <div v-if="updateInfo.is_differential" class="delta-badge" title="差分增量更新仅下载新旧版本差异字节">
            <span class="delta-bolt">⚡</span>
            <div class="delta-texts">
              <span class="delta-t1">差分增量包</span>
              <span class="delta-t2">节约 78% 流量</span>
            </div>
          </div>
        </div>

        <!-- Size comparison bar -->
        <div class="size-specs-box">
          <div class="size-spec">
            <span class="s-k">下载体积:</span>
            <span class="s-v highlight">{{ formatBytes(updateInfo.download_size_bytes) }}</span>
          </div>
          <div class="size-spec">
            <span class="s-k">全量包体积:</span>
            <span class="s-v strike">{{ formatBytes(updateInfo.original_size_bytes) }}</span>
          </div>
          <div class="size-spec">
            <span class="s-k">发布时间:</span>
            <span class="s-v">{{ updateInfo.published_at ? new Date(updateInfo.published_at).toLocaleDateString() : '刚刚' }}</span>
          </div>
        </div>

        <!-- Release Notes Section -->
        <div class="notes-container">
          <h4 class="notes-h4">更新日志与新特性:</h4>
          <div class="notes-markdown">
            <pre class="notes-text">{{ updateInfo.release_notes }}</pre>
          </div>
        </div>

        <!-- Progress Bar if Downloading or Ready -->
        <div v-if="isDownloading || isReady" class="progress-section">
          <div class="progress-labels">
            <span class="p-msg">{{ progress?.message || '正在准备下载...' }}</span>
            <span class="p-pct">{{ progress ? `${progress.percentage.toFixed(1)}%` : '0%' }}</span>
          </div>
          <div class="progress-bar-bg">
            <div
              class="progress-bar-fill"
              :class="{ 'fill-ready': isReady }"
              :style="{ width: `${progress?.percentage || 0}%` }"
            ></div>
          </div>
          <div class="progress-stats">
            <span class="stat-item">已下载: {{ formatBytes(progress?.downloaded_bytes || 0) }} / {{ formatBytes(progress?.total_bytes || 0) }}</span>
            <span v-if="isDownloading" class="stat-item stat-speed">速度: {{ (progress?.speed_mbps || 0).toFixed(1) }} MB/s</span>
            <span v-if="isReady" class="stat-item stat-success">✓ 安装包校验通过</span>
          </div>
        </div>

        <!-- Action Buttons Footer -->
        <div class="update-actions">
          <button class="btn-cancel" :disabled="isDownloading" @click="close">
            {{ isReady ? '稍后安装' : '暂不更新' }}
          </button>

          <button v-if="isReady" class="btn-cancel" @click="openInstallerFolder" title="打开下载好的安装包所在目录">
            📁 打开安装包目录
          </button>

          <button v-if="!isDownloading && !isReady" class="btn-primary-action" @click="startUpdate">
            ⚡ 立即在应用内升级 ({{ formatBytes(updateInfo.download_size_bytes) }})
          </button>

          <button v-else-if="isDownloading" class="btn-primary-action disabled" disabled>
            <span class="spinner-sm"></span>
            正在下载中...
          </button>

          <button v-else-if="isReady" class="btn-relaunch-action" @click="relaunch">
            🚀 立即运行安装包并更新
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.update-modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(12px);
  z-index: 1050;
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

.update-modal-container {
  width: 100%;
  max-width: 580px;
  background: #0b1220;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 14px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.update-header {
  height: 58px;
  padding: 0 1.25rem;
  background: #0f182c;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.header-title-box {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.header-icon {
  font-size: 1.4rem;
}

.update-title {
  margin: 0;
  font-size: 0.96rem;
  font-weight: 700;
  color: #f1f5f9;
}

.update-subtitle {
  font-size: 0.72rem;
  color: #64748b;
}

.btn-close {
  background: none;
  border: none;
  color: #94a3b8;
  font-size: 1.1rem;
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
}

.btn-close:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
}

.update-body {
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.state-loading, .state-error, .state-uptodate {
  align-items: center;
  text-align: center;
  padding: 2.5rem 1.5rem;
}

.spinner-lg {
  width: 36px;
  height: 36px;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 1rem;
}

.loading-text {
  font-size: 0.85rem;
  color: #94a3b8;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.err-icon {
  font-size: 2.2rem;
  margin-bottom: 0.5rem;
}

.err-msg {
  color: #fca5a5;
  font-size: 0.85rem;
  margin-bottom: 1rem;
}

.btn-retry {
  background: #3b82f6;
  color: white;
  border: none;
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 0.8rem;
  cursor: pointer;
}

.success-icon {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
  margin-bottom: 0.75rem;
}

.uptodate-h4 {
  margin: 0 0 0.3rem;
  font-size: 1.05rem;
  color: #f8fafc;
}

.uptodate-p {
  margin: 0 0 1.25rem;
  font-size: 0.82rem;
  color: #94a3b8;
}

.btn-primary {
  background: #3b82f6;
  color: white;
  border: none;
  padding: 7px 18px;
  border-radius: 6px;
  font-size: 0.82rem;
  cursor: pointer;
}

/* Version Banner */
.version-banner {
  background: #080e1a;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 9px;
  padding: 0.75rem 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.ver-col {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.ver-label {
  font-size: 0.68rem;
  color: #64748b;
  text-transform: uppercase;
}

.ver-val {
  font-size: 0.95rem;
  font-weight: 700;
}

.v-cur {
  color: #94a3b8;
}

.v-new {
  color: #60a5fa;
}

.arrow-sep {
  color: #475569;
  font-size: 1.1rem;
}

.delta-badge {
  background: rgba(16, 185, 129, 0.15);
  border: 1px solid rgba(16, 185, 129, 0.35);
  border-radius: 7px;
  padding: 4px 8px;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.delta-bolt {
  font-size: 1.1rem;
  color: #34d399;
}

.delta-texts {
  display: flex;
  flex-direction: column;
}

.delta-t1 {
  font-size: 0.72rem;
  font-weight: 700;
  color: #34d399;
}

.delta-t2 {
  font-size: 0.65rem;
  color: #a7f3d0;
}

/* Size specs */
.size-specs-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: rgba(255, 255, 255, 0.03);
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.75rem;
}

.size-spec {
  display: flex;
  align-items: center;
  gap: 0.3rem;
}

.s-k {
  color: #64748b;
}

.s-v {
  color: #cbd5e1;
}

.s-v.highlight {
  color: #38bdf8;
  font-weight: 700;
}

.s-v.strike {
  text-decoration: line-through;
  color: #64748b;
}

/* Release notes */
.notes-container {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.notes-h4 {
  margin: 0;
  font-size: 0.78rem;
  color: #94a3b8;
  font-weight: 600;
}

.notes-markdown {
  background: #080d17;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  padding: 0.75rem;
  max-height: 150px;
  overflow-y: auto;
}

.notes-text {
  margin: 0;
  font-family: inherit;
  font-size: 0.75rem;
  color: #cbd5e1;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.5;
}

/* Progress bar */
.progress-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  background: #080e1a;
  border: 1px solid rgba(59, 130, 246, 0.2);
  border-radius: 8px;
  padding: 0.75rem;
}

.progress-labels {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
}

.p-msg {
  color: #93c5fd;
  font-weight: 600;
}

.p-pct {
  color: #3b82f6;
  font-weight: 700;
}

.progress-bar-bg {
  width: 100%;
  height: 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #3b82f6, #60a5fa);
  transition: width 0.15s ease;
}

.progress-bar-fill.fill-ready {
  background: linear-gradient(90deg, #10b981, #34d399);
}

.progress-stats {
  display: flex;
  justify-content: space-between;
  font-size: 0.7rem;
  color: #64748b;
}

.stat-speed {
  color: #38bdf8;
  font-weight: 600;
}

.stat-success {
  color: #34d399;
  font-weight: 600;
}

/* Footer Actions */
.update-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.6rem;
  margin-top: 0.5rem;
}

.btn-cancel {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #94a3b8;
  padding: 7px 14px;
  border-radius: 6px;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-cancel:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.btn-primary-action {
  background: #3b82f6;
  border: none;
  color: #ffffff;
  padding: 7px 16px;
  border-radius: 6px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.4rem;
  transition: all 0.15s ease;
}

.btn-primary-action:hover {
  background: #2563eb;
}

.btn-relaunch-action {
  background: #10b981;
  border: none;
  color: #ffffff;
  padding: 7px 16px;
  border-radius: 6px;
  font-size: 0.82rem;
  font-weight: 700;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(16, 185, 129, 0.4);
  transition: all 0.15s ease;
}

.btn-relaunch-action:hover {
  background: #059669;
}

.spinner-sm {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}
</style>
