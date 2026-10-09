<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { FormatOption, UrlInspectResult, DownloadTask } from '../types';

const emit = defineEmits<{
  (e: 'update:activeCount', count: number): void;
  (e: 'viewInLibrary', payload: { fileName?: string; filePath?: string }): void;
}>();

// Input and state
const inputUrl = ref<string>('');
const isInspecting = ref<boolean>(false);
const isAddingDownload = ref<boolean>(false);
const inspectError = ref<string>('');
const inspectResult = ref<UrlInspectResult | null>(null);

// Quality selection state
const selectedFormatPreset = ref<string>('best');
const selectedCustomFormatId = ref<string>('');
const showInspectModal = ref<boolean>(false);

// Task lists and filters
const tasks = ref<DownloadTask[]>([]);
const isLoadingTasks = ref<boolean>(false);
const activeFilter = ref<'all' | 'active' | 'completed' | 'failed'>('all');
const toastMessage = ref<string>('');
let toastTimer: number | null = null;
let pollTimer: number | null = null;

// Presets for video quality
const qualityPresets = [
  { id: 'best', label: '最佳画质 (Best Video + Audio)', formatId: 'bestvideo+bestaudio/best' },
  { id: '1080p', label: '1080P 高清 (1080p Full HD)', formatId: 'bestvideo[height<=1080]+bestaudio/best[height<=1080]' },
  { id: '720p', label: '720P 标清 (720p HD)', formatId: 'bestvideo[height<=720]+bestaudio/best[height<=720]' },
  { id: '480p', label: '480P 流畅 (480p SD)', formatId: 'bestvideo[height<=480]+bestaudio/best[height<=480]' },
  { id: 'audio_only', label: '仅提取音频 (Audio Only MP3/M4A)', formatId: 'bestaudio/best' },
  { id: 'custom', label: '自定义格式 ID...', formatId: '' },
];

// Computed tasks categories
const activeTasks = computed(() => {
  return tasks.value.filter(
    (t) => t.status === 'downloading' || t.status === 'pending' || t.status === 'paused'
  );
});

const completedTasks = computed(() => {
  return tasks.value.filter((t) => t.status === 'completed');
});

const failedTasks = computed(() => {
  return tasks.value.filter((t) => t.status === 'failed' || t.status === 'cancelled');
});

const displayedTasks = computed(() => {
  if (activeFilter.value === 'active') return activeTasks.value;
  if (activeFilter.value === 'completed') return completedTasks.value;
  if (activeFilter.value === 'failed') return failedTasks.value;
  return tasks.value;
});

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2600);
}

// Fetch download tasks from backend
async function fetchTasks() {
  isLoadingTasks.value = true;
  try {
    const list = await invoke<DownloadTask[]>('get_download_tasks');
    tasks.value = list || [];
    emit('update:activeCount', activeTasks.value.length);
  } catch (err: unknown) {
    // If command not yet implemented or error occurs, handle gracefully
    const msg = err instanceof Error ? err.message : String(err);
    if (!msg.includes('not found') && !msg.includes('unknown')) {
      console.warn('Failed to fetch download tasks:', msg);
    }
  } finally {
    isLoadingTasks.value = false;
  }
}

function formatStreamSummary(fmt: FormatOption): string {
  const notePart = fmt.note ? ` | ${fmt.note}` : '';
  const sizePart = fmt.filesize ? ` | ${formatFileSize(fmt.filesize)}` : '';
  return `ID: ${fmt.format_id} | [${fmt.ext}] ${fmt.resolution}${notePart}${sizePart}`;
}

// Inspect video metadata
async function inspectVideoUrl() {
  const url = inputUrl.value.trim();
  if (!url) {
    inspectError.value = '请输入需要解析的视频链接 (URL)';
    return;
  }
  inspectError.value = '';
  isInspecting.value = true;
  inspectResult.value = null;

  try {
    const res = await invoke<UrlInspectResult>('inspect_url', { url });
    inspectResult.value = res;
    showInspectModal.value = true;
    selectedFormatPreset.value = 'best';
    selectedCustomFormatId.value = res.formats?.[0]?.format_id || '';
    showToast('元信息解析成功！');
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    inspectError.value = `视频解析失败: ${msg}`;
  } finally {
    isInspecting.value = false;
  }
}

// Quick add download with default best quality
async function quickAddDownload() {
  const url = inputUrl.value.trim();
  if (!url) {
    inspectError.value = '请输入需要下载的视频链接 (URL)';
    return;
  }
  inspectError.value = '';
  isAddingDownload.value = true;

  try {
    await invoke<DownloadTask>('start_download', {
      url,
      format_id: 'bestvideo+bestaudio/best',
      formatId: 'bestvideo+bestaudio/best',
    });
    inputUrl.value = '';
    showToast('已加入下载任务队列');
    await fetchTasks();
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    inspectError.value = `添加下载失败: ${msg}`;
  } finally {
    isAddingDownload.value = false;
  }
}

// Confirm download from inspect preview modal
async function confirmInspectDownload() {
  if (!inspectResult.value) return;
  const url = inspectResult.value.url;
  let targetFormatId: string | undefined = undefined;

  if (selectedFormatPreset.value === 'custom') {
    targetFormatId = selectedCustomFormatId.value.trim() || undefined;
  } else {
    const foundPreset = qualityPresets.find((p) => p.id === selectedFormatPreset.value);
    targetFormatId = foundPreset?.formatId || undefined;
  }

  isAddingDownload.value = true;
  try {
    await invoke<DownloadTask>('start_download', {
      url,
      format_id: targetFormatId,
      formatId: targetFormatId,
    });
    showInspectModal.value = false;
    inputUrl.value = '';
    showToast('已确认并开始下载任务');
    await fetchTasks();
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    alert(`启动下载失败: ${msg}`);
  } finally {
    isAddingDownload.value = false;
  }
}

// Task control operations
async function pauseTask(task: DownloadTask) {
  try {
    await invoke('pause_download_task', {
      task_id: task.id,
      taskId: task.id,
      id: task.id,
    });
    showToast(`任务已暂停`);
    await fetchTasks();
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    alert(`暂停失败: ${msg}`);
  }
}

async function resumeTask(task: DownloadTask) {
  try {
    await invoke('resume_download_task', {
      task_id: task.id,
      taskId: task.id,
      id: task.id,
    });
    showToast(`任务已恢复`);
    await fetchTasks();
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    alert(`恢复失败: ${msg}`);
  }
}

async function cancelTask(task: DownloadTask) {
  if (!confirm(`确定取消任务 "${task.title}" 吗？`)) return;
  try {
    await invoke('cancel_download_task', {
      task_id: task.id,
      taskId: task.id,
      id: task.id,
    });
    showToast(`任务已取消`);
    await fetchTasks();
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    alert(`取消失败: ${msg}`);
  }
}

async function retryTask(task: DownloadTask) {
  try {
    await invoke('retry_download_task', {
      task_id: task.id,
      taskId: task.id,
      id: task.id,
      url: task.url,
    });
    showToast(`已重新加入下载队列`);
    await fetchTasks();
  } catch (err: unknown) {
    // If retry_download_task command not found, fall back to start_download
    try {
      await invoke('start_download', {
        url: task.url,
        format_id: 'bestvideo+bestaudio/best',
        formatId: 'bestvideo+bestaudio/best',
      });
      showToast(`已重新提交下载`);
      await fetchTasks();
    } catch (innerErr: unknown) {
      const msg = innerErr instanceof Error ? innerErr.message : String(innerErr);
      alert(`重试失败: ${msg}`);
    }
  }
}

async function deleteTask(task: DownloadTask) {
  try {
    await invoke('delete_download_task', {
      task_id: task.id,
      taskId: task.id,
      id: task.id,
    });
    showToast(`已移除任务记录`);
    await fetchTasks();
  } catch {
    // Local remove if backend command is simple
    tasks.value = tasks.value.filter((t) => t.id !== task.id);
  }
}

async function clearCompleted() {
  if (completedTasks.value.length === 0) return;
  try {
    await invoke('clear_completed_tasks');
    await fetchTasks();
    showToast('已清空已完成记录');
  } catch {
    tasks.value = tasks.value.filter((t) => t.status !== 'completed');
    showToast('已清空已完成记录');
  }
}

// Copy file path to clipboard
async function copyFilePath(path: string | null | undefined) {
  if (!path) return;
  try {
    await navigator.clipboard.writeText(path);
    showToast('文件路径已复制到剪贴板！');
  } catch {
    // Fallback
    const input = document.createElement('input');
    input.value = path;
    document.body.appendChild(input);
    input.select();
    document.execCommand('copy');
    document.body.removeChild(input);
    showToast('文件路径已复制到剪贴板！');
  }
}

// View video in media library
function viewInLibrary(task: DownloadTask) {
  emit('viewInLibrary', {
    fileName: task.title,
    filePath: task.target_path || undefined,
  });
}

// Paste from clipboard helper
async function pasteFromClipboard() {
  try {
    const text = await navigator.clipboard.readText();
    if (text) {
      inputUrl.value = text.trim();
      inspectError.value = '';
    }
  } catch (err) {
    console.warn('Clipboard read failed:', err);
  }
}

// Formatting helpers
function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || isNaN(seconds)) return '--:--';
  const totalSecs = Math.floor(seconds);
  const hours = Math.floor(totalSecs / 3600);
  const minutes = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hours > 0) {
    return `${hours.toString().padStart(2, '0')}:${minutes
      .toString()
      .padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

function formatFileSize(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

function formatDate(timestamp: number | null | undefined): string {
  if (!timestamp) return '-';
  const d = new Date(timestamp > 1e11 ? timestamp : timestamp * 1000);
  return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function getStatusBadgeClass(status: string): string {
  switch (status) {
    case 'downloading':
      return 'badge-primary';
    case 'completed':
      return 'badge-success';
    case 'paused':
      return 'badge-warning';
    case 'failed':
    case 'cancelled':
      return 'badge-danger';
    default:
      return 'badge-secondary';
  }
}

function getStatusText(status: string): string {
  switch (status) {
    case 'downloading':
      return '下载中';
    case 'pending':
      return '排队中';
    case 'paused':
      return '已暂停';
    case 'completed':
      return '已完成';
    case 'failed':
      return '下载失败';
    case 'cancelled':
      return '已取消';
    default:
      return status;
  }
}

onMounted(() => {
  fetchTasks();
  // Poll tasks every 1.5 seconds to refresh progress and status
  pollTimer = window.setInterval(() => {
    fetchTasks();
  }, 1500);
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
  if (toastTimer) clearTimeout(toastTimer);
});
</script>

<template>
  <div class="download-center">
    <!-- Toast Notification -->
    <transition name="fade">
      <div v-if="toastMessage" class="toast-notification">
        <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
        <span>{{ toastMessage }}</span>
      </div>
    </transition>

    <!-- Top Input Card -->
    <section class="card input-section">
      <div class="section-title-row">
        <div class="title-with-icon">
          <div class="icon-wrap-blue">
            <svg class="action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
          </div>
          <div>
            <h2 class="card-title">yt-dlp 在线视频解析与高速下载引擎</h2>
            <p class="card-subtitle">支持 YouTube、Bilibili、Twitter、TikTok 等全球主流音视频平台，支持多画质自选与音画自动混流</p>
          </div>
        </div>
      </div>

      <div class="url-input-container">
        <div class="input-wrapper">
          <svg class="input-prefix-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
          </svg>
          <input
            v-model="inputUrl"
            type="text"
            class="url-input"
            placeholder="粘贴视频 URL 链接 (例如：https://www.bilibili.com/video/BV... 或 https://youtube.com/watch?v=...)"
            @keyup.enter="inspectVideoUrl"
          />
          <button
            v-if="inputUrl"
            class="icon-clear-btn"
            title="清空输入"
            @click="inputUrl = ''"
          >
            ✕
          </button>
          <button
            class="paste-btn"
            title="从剪贴板粘贴"
            @click="pasteFromClipboard"
          >
            <svg class="btn-sm-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
            粘贴
          </button>
        </div>

        <div class="action-buttons-group">
          <button
            class="btn btn-secondary inspect-btn"
            :disabled="isInspecting || !inputUrl.trim()"
            @click="inspectVideoUrl"
          >
            <svg
              class="btn-icon"
              :class="{ 'animate-spin': isInspecting }"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle cx="11" cy="11" r="8"></circle>
              <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            </svg>
            {{ isInspecting ? '解析元数据中...' : '解析视频元信息 (Inspect)' }}
          </button>

          <button
            class="btn btn-primary quick-download-btn"
            :disabled="isAddingDownload || !inputUrl.trim()"
            @click="quickAddDownload"
          >
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
            {{ isAddingDownload ? '提交中...' : '快速添加下载' }}
          </button>
        </div>
      </div>

      <div v-if="inspectError" class="alert alert-danger">
        <svg class="alert-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
        <span>{{ inspectError }}</span>
      </div>
    </section>

    <!-- Video Metadata Inspect Modal / Dialog -->
    <transition name="modal">
      <div v-if="showInspectModal && inspectResult" class="modal-overlay" @click.self="showInspectModal = false">
        <div class="modal-card">
          <div class="modal-header">
            <div class="modal-title-wrap">
              <div class="modal-badge-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
              </div>
              <div>
                <h3 class="modal-title">视频元信息解析预览</h3>
                <span class="modal-subtitle">选择首选清晰度格式并确认加入下载队列</span>
              </div>
            </div>
            <button class="modal-close-btn" @click="showInspectModal = false">✕</button>
          </div>

          <div class="modal-body">
            <!-- Video Preview Banner -->
            <div class="preview-media-box">
              <div class="media-thumb-wrap">
                <img
                  v-if="inspectResult.thumbnail"
                  :src="inspectResult.thumbnail"
                  :alt="inspectResult.title"
                  class="preview-img"
                  referrerpolicy="no-referrer"
                />
                <div v-else class="preview-placeholder">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect>
                    <polygon points="10 8 16 12 10 16 10 8"></polygon>
                  </svg>
                </div>
                <span v-if="inspectResult.duration" class="thumb-duration-pill">
                  {{ formatDuration(inspectResult.duration) }}
                </span>
              </div>

              <div class="media-details">
                <h4 class="media-title" :title="inspectResult.title">{{ inspectResult.title }}</h4>
                <div class="media-meta-tags">
                  <span v-if="inspectResult.uploader" class="meta-tag">
                    <svg class="tag-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
                      <circle cx="12" cy="7" r="4"></circle>
                    </svg>
                    {{ inspectResult.uploader }}
                  </span>
                  <span v-if="inspectResult.extractor" class="meta-tag tag-extractor">
                    <svg class="tag-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"></circle>
                      <line x1="2" y1="12" x2="22" y2="12"></line>
                      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                    </svg>
                    {{ inspectResult.extractor.toUpperCase() }}
                  </span>
                  <span class="meta-tag">
                    <svg class="tag-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"></circle>
                      <polyline points="12 6 12 12 14 14"></polyline>
                    </svg>
                    {{ formatDuration(inspectResult.duration) }}
                  </span>
                </div>
                <div class="media-url-snippet font-mono" :title="inspectResult.url">
                  {{ inspectResult.url }}
                </div>
              </div>
            </div>

            <!-- Quality & Format Selectors -->
            <div class="format-selection-area">
              <label class="form-section-label">画质与编码格式配置：</label>

              <div class="format-radio-grid">
                <label
                  v-for="preset in qualityPresets"
                  :key="preset.id"
                  class="preset-radio-card"
                  :class="{ active: selectedFormatPreset === preset.id }"
                >
                  <input
                    type="radio"
                    name="qualityPreset"
                    :value="preset.id"
                    v-model="selectedFormatPreset"
                  />
                  <div class="preset-info">
                    <span class="preset-name">{{ preset.label }}</span>
                    <span v-if="preset.formatId" class="preset-id font-mono">{{ preset.formatId }}</span>
                  </div>
                </label>
              </div>

              <!-- Detailed Formats Dropdown if Custom is Selected -->
              <div v-if="selectedFormatPreset === 'custom' || (inspectResult.formats && inspectResult.formats.length > 0)" class="custom-formats-box">
                <label class="custom-label">解析到的可用格式流列表 (Available Streams)：</label>
                <select v-model="selectedCustomFormatId" class="select-input">
                  <option
                    v-for="fmt in inspectResult.formats || []"
                    :key="fmt.format_id"
                    :value="fmt.format_id"
                  >
                    {{ formatStreamSummary(fmt) }}
                  </option>
                </select>
              </div>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showInspectModal = false">取消</button>
            <button
              class="btn btn-primary"
              :disabled="isAddingDownload"
              @click="confirmInspectDownload"
            >
              <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
              {{ isAddingDownload ? '正在启动...' : '确认并开始下载' }}
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- Task Queue Controls & Tabs -->
    <section class="queue-header-row">
      <div class="filter-tabs">
        <button
          class="filter-tab-btn"
          :class="{ active: activeFilter === 'all' }"
          @click="activeFilter = 'all'"
        >
          全部任务
          <span class="count-pill">{{ tasks.length }}</span>
        </button>

        <button
          class="filter-tab-btn"
          :class="{ active: activeFilter === 'active' }"
          @click="activeFilter = 'active'"
        >
          进行中
          <span class="count-pill pill-blue">{{ activeTasks.length }}</span>
        </button>

        <button
          class="filter-tab-btn"
          :class="{ active: activeFilter === 'completed' }"
          @click="activeFilter = 'completed'"
        >
          已完成
          <span class="count-pill pill-green">{{ completedTasks.length }}</span>
        </button>

        <button
          class="filter-tab-btn"
          :class="{ active: activeFilter === 'failed' }"
          @click="activeFilter = 'failed'"
        >
          失败 / 异常
          <span class="count-pill pill-red">{{ failedTasks.length }}</span>
        </button>
      </div>

      <div class="header-tools">
        <button
          v-if="completedTasks.length > 0"
          class="btn-text"
          @click="clearCompleted"
        >
          <svg class="tool-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
          清空已完成记录
        </button>
        <button
          class="btn-icon-refresh"
          :class="{ 'animate-spin': isLoadingTasks }"
          :disabled="isLoadingTasks"
          title="刷新任务状态"
          @click="fetchTasks"
        >
          <svg class="tool-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
          </svg>
        </button>
      </div>
    </section>

    <!-- Empty State -->
    <div v-if="displayedTasks.length === 0" class="empty-state-card card">
      <div class="empty-icon-wrap">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
          <polyline points="7 10 12 15 17 10"></polyline>
          <line x1="12" y1="15" x2="12" y2="3"></line>
        </svg>
      </div>
      <h3 class="empty-title">
        {{ activeFilter === 'all' ? '暂无下载任务' : `暂无${activeFilter === 'active' ? '进行中' : activeFilter === 'completed' ? '已完成' : '失败'}任务` }}
      </h3>
      <p class="empty-desc">
        在上方输入框粘贴视频 URL 并点击“解析视频元信息”或“快速添加下载”，即可自动建立下载管线并归档至媒体库。
      </p>
    </div>

    <!-- Task List Cards -->
    <div v-else class="tasks-container">
      <!-- ACTIVE TASKS SECTION -->
      <div v-for="task in displayedTasks" :key="task.id" class="task-card card">
        <!-- Thumbnail -->
        <div class="task-thumb-wrap">
          <img
            v-if="task.thumbnail"
            :src="task.thumbnail"
            :alt="task.title"
            class="task-thumb"
            referrerpolicy="no-referrer"
          />
          <div v-else class="task-thumb-placeholder">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
          </div>
          <span class="status-badge-corner badge" :class="getStatusBadgeClass(task.status)">
            {{ getStatusText(task.status) }}
          </span>
        </div>

        <!-- Task Main Body -->
        <div class="task-body">
          <div class="task-title-row">
            <h3 class="task-title" :title="task.title">{{ task.title || task.url }}</h3>
            <span class="task-time">{{ formatDate(task.created_at) }}</span>
          </div>

          <!-- Active Download Progress Area -->
          <div v-if="task.status === 'downloading' || task.status === 'pending' || task.status === 'paused'" class="progress-section">
            <div class="progress-bar-track">
              <div
                class="progress-bar-fill"
                :class="{ 'bar-pulsing': task.status === 'downloading', 'bar-paused': task.status === 'paused' }"
                :style="{ width: `${Math.min(Math.max(task.progress || 0, 0), 100)}%` }"
              ></div>
            </div>

            <div class="progress-stats-row">
              <div class="stats-left">
                <span class="stat-percentage font-mono">{{ (task.progress || 0).toFixed(1) }}%</span>
                <span class="stat-size font-mono">
                  {{ formatFileSize(task.downloaded_bytes) }} / {{ task.total_bytes > 0 ? formatFileSize(task.total_bytes) : '计算中...' }}
                </span>
              </div>

              <div class="stats-right">
                <span v-if="task.speed_str" class="stat-speed font-mono">
                  <svg class="stat-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polyline>
                  </svg>
                  {{ task.speed_str }}
                </span>
                <span v-if="task.eta_str" class="stat-eta font-mono">
                  <svg class="stat-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"></circle>
                    <polyline points="12 6 12 12 16 14"></polyline>
                  </svg>
                  ETA: {{ task.eta_str }}
                </span>
              </div>
            </div>
          </div>

          <!-- Completed State Details -->
          <div v-else-if="task.status === 'completed'" class="completed-meta-row">
            <div class="path-display-box">
              <span class="path-label">存储路径：</span>
              <span class="path-value font-mono" :title="task.target_path || ''">
                {{ task.target_path || '默认下载目录' }}
              </span>
            </div>
            <div class="completed-size-tag">
              大小：{{ formatFileSize(task.total_bytes || task.downloaded_bytes) }}
            </div>
          </div>

          <!-- Failed / Error State Details -->
          <div v-else-if="task.status === 'failed' || task.status === 'cancelled'" class="failed-alert-box">
            <svg class="failed-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="8" x2="12" y2="12"></line>
              <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <span class="failed-text">
              {{ task.error_message || (task.status === 'cancelled' ? '用户已取消该下载任务' : '下载失败，可能是网络中断或防盗链限制') }}
            </span>
          </div>

          <!-- Original Source URL Tag -->
          <div class="source-url-bar">
            <span class="url-label">来源链接：</span>
            <a :href="task.url" target="_blank" class="url-link font-mono" :title="task.url">
              {{ task.url }}
            </a>
          </div>
        </div>

        <!-- Task Actions Controls -->
        <div class="task-actions">
          <!-- Controls for Active Tasks -->
          <template v-if="task.status === 'downloading'">
            <button class="btn-action btn-pause" title="暂停任务" @click="pauseTask(task)">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="6" y="4" width="4" height="16"></rect>
                <rect x="14" y="4" width="4" height="16"></rect>
              </svg>
              <span>暂停</span>
            </button>
            <button class="btn-action btn-cancel" title="取消下载" @click="cancelTask(task)">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
              <span>取消</span>
            </button>
          </template>

          <template v-else-if="task.status === 'paused'">
            <button class="btn-action btn-resume" title="恢复下载" @click="resumeTask(task)">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
              </svg>
              <span>恢复</span>
            </button>
            <button class="btn-action btn-cancel" title="取消下载" @click="cancelTask(task)">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
              <span>取消</span>
            </button>
          </template>

          <!-- Controls for Completed Tasks -->
          <template v-else-if="task.status === 'completed'">
            <button
              class="btn-action btn-view-lib"
              title="在本地媒体库中查看"
              @click="viewInLibrary(task)"
            >
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect>
                <line x1="7" y1="2" x2="7" y2="22"></line>
                <line x1="17" y1="2" x2="17" y2="22"></line>
                <line x1="2" y1="12" x2="22" y2="12"></line>
              </svg>
              <span>在媒体库中查看</span>
            </button>

            <button
              class="btn-action btn-copy-path"
              title="复制文件绝对路径"
              @click="copyFilePath(task.target_path)"
            >
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              <span>复制路径</span>
            </button>
          </template>

          <!-- Controls for Failed Tasks -->
          <template v-else-if="task.status === 'failed' || task.status === 'cancelled'">
            <button class="btn-action btn-retry" title="重新下载" @click="retryTask(task)">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="1 4 1 10 7 10"></polyline>
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
              </svg>
              <span>重新下载</span>
            </button>
            <button class="btn-action btn-delete" title="删除记录" @click="deleteTask(task)">
              <svg class="action-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
              <span>删除</span>
            </button>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.download-center {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

/* Toast */
.toast-notification {
  position: fixed;
  top: 1.5rem;
  right: 2rem;
  z-index: 1000;
  background-color: #1e293b;
  color: #f8fafc;
  padding: 0.75rem 1.25rem;
  border-radius: 10px;
  box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.toast-icon {
  width: 18px;
  height: 18px;
  color: #10b981;
}

/* Input Section */
.input-section {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 14px;
}

.section-title-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.title-with-icon {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.icon-wrap-blue {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  background: linear-gradient(135deg, #3b82f6, #1d4ed8);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  box-shadow: 0 4px 10px rgba(59, 130, 246, 0.25);
}

.action-icon {
  width: 22px;
  height: 22px;
}

.card-title {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text-color);
}

.card-subtitle {
  margin: 0.2rem 0 0;
  font-size: 0.82rem;
  color: var(--text-muted);
}

.url-input-container {
  display: flex;
  gap: 0.85rem;
  align-items: center;
  flex-wrap: wrap;
}

.input-wrapper {
  flex: 1;
  min-width: 320px;
  position: relative;
  display: flex;
  align-items: center;
}

.input-prefix-icon {
  position: absolute;
  left: 0.9rem;
  width: 18px;
  height: 18px;
  color: var(--text-muted);
  pointer-events: none;
}

.url-input {
  width: 100%;
  padding: 0.75rem 5.5rem 0.75rem 2.6rem;
  border-radius: 10px;
  border: 1px solid var(--border-color);
  background-color: var(--input-bg);
  color: var(--text-color);
  font-size: 0.92rem;
  outline: none;
  transition: all 0.2s;
  box-sizing: border-box;
}

.url-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.icon-clear-btn {
  position: absolute;
  right: 3.4rem;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 0.25rem;
  font-size: 0.9rem;
}

.icon-clear-btn:hover {
  color: var(--text-color);
}

.paste-btn {
  position: absolute;
  right: 0.5rem;
  background-color: var(--btn-sec-bg);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 0.35rem 0.6rem;
  font-size: 0.78rem;
  color: var(--text-muted);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  transition: all 0.2s;
}

.paste-btn:hover {
  background-color: var(--btn-sec-hover);
  color: var(--text-color);
}

.btn-sm-icon {
  width: 13px;
  height: 13px;
}

.action-buttons-group {
  display: flex;
  gap: 0.65rem;
}

.inspect-btn {
  padding: 0.75rem 1.15rem;
}

.quick-download-btn {
  padding: 0.75rem 1.25rem;
}

.alert-danger {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  background-color: #fee2e2;
  color: #991b1b;
  border: 1px solid #f87171;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  font-size: 0.88rem;
}

.alert-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

/* Modal styles */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1.5rem;
}

.modal-card {
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 16px;
  width: 100%;
  max-width: 680px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 40px -10px rgba(0, 0, 0, 0.3);
  overflow: hidden;
}

.modal-header {
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.modal-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.modal-badge-icon {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: #eff6ff;
  color: #2563eb;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-badge-icon svg {
  width: 18px;
  height: 18px;
}

.modal-title {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
}

.modal-subtitle {
  font-size: 0.78rem;
  color: var(--text-muted);
}

.modal-close-btn {
  background: none;
  border: none;
  font-size: 1.2rem;
  color: var(--text-muted);
  cursor: pointer;
  padding: 0.3rem 0.5rem;
  border-radius: 6px;
}

.modal-close-btn:hover {
  background: var(--btn-sec-bg);
  color: var(--text-color);
}

.modal-body {
  padding: 1.5rem;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.preview-media-box {
  display: flex;
  gap: 1.2rem;
  background: var(--bg-color);
  padding: 1rem;
  border-radius: 12px;
  border: 1px solid var(--border-color);
}

.media-thumb-wrap {
  position: relative;
  width: 180px;
  height: 105px;
  border-radius: 8px;
  overflow: hidden;
  background: #0f172a;
  flex-shrink: 0;
}

.preview-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.preview-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
}

.preview-placeholder svg {
  width: 36px;
  height: 36px;
}

.thumb-duration-pill {
  position: absolute;
  bottom: 6px;
  right: 6px;
  background: rgba(0, 0, 0, 0.8);
  color: white;
  font-size: 0.72rem;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-family: ui-monospace, monospace;
}

.media-details {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  overflow: hidden;
}

.media-title {
  margin: 0;
  font-size: 0.98rem;
  font-weight: 600;
  line-height: 1.35;
  color: var(--text-color);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.media-meta-tags {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin: 0.4rem 0;
}

.meta-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  padding: 0.2rem 0.5rem;
  border-radius: 6px;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.tag-extractor {
  background: #f0fdf4;
  color: #166534;
  border-color: #bbf7d0;
  font-weight: 600;
}

.tag-icon {
  width: 12px;
  height: 12px;
}

.media-url-snippet {
  font-size: 0.75rem;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.format-selection-area {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.form-section-label {
  font-size: 0.88rem;
  font-weight: 600;
  color: var(--text-color);
}

.format-radio-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 0.65rem;
}

.preset-radio-card {
  display: flex;
  align-items: flex-start;
  gap: 0.65rem;
  padding: 0.75rem 0.9rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-color);
  cursor: pointer;
  transition: all 0.2s;
}

.preset-radio-card:hover {
  border-color: #93c5fd;
}

.preset-radio-card.active {
  border-color: #3b82f6;
  background-color: rgba(59, 130, 246, 0.05);
}

.preset-info {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.preset-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-color);
}

.preset-id {
  font-size: 0.72rem;
  color: var(--text-muted);
}

.custom-formats-box {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-top: 0.5rem;
}

.custom-label {
  font-size: 0.8rem;
  color: var(--text-muted);
}

.select-input {
  width: 100%;
  padding: 0.65rem 0.85rem;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--input-bg);
  color: var(--text-color);
  font-size: 0.85rem;
  outline: none;
}

.modal-footer {
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

/* Queue Header */
.queue-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 1rem;
}

.filter-tabs {
  display: flex;
  background: var(--btn-sec-bg);
  padding: 3px;
  border-radius: 10px;
  gap: 4px;
}

.filter-tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.45rem 0.9rem;
  border-radius: 8px;
  border: none;
  background: none;
  color: var(--text-muted);
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.filter-tab-btn:hover {
  color: var(--text-color);
}

.filter-tab-btn.active {
  background: var(--card-bg);
  color: #2563eb;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.count-pill {
  font-size: 0.72rem;
  padding: 0.1rem 0.45rem;
  border-radius: 9999px;
  background: var(--border-color);
  color: var(--text-muted);
  font-weight: 700;
}

.pill-blue {
  background: #dbeafe;
  color: #1d4ed8;
}

.pill-green {
  background: #dcfce7;
  color: #15803d;
}

.pill-red {
  background: #fee2e2;
  color: #b91c1c;
}

.header-tools {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.btn-text {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 0.82rem;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.4rem 0.6rem;
  border-radius: 6px;
}

.btn-text:hover {
  color: #ef4444;
  background: var(--btn-sec-bg);
}

.btn-icon-refresh {
  background: var(--btn-sec-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: var(--text-muted);
  transition: all 0.2s;
}

.btn-icon-refresh:hover {
  color: var(--text-color);
  background: var(--btn-sec-hover);
}

.tool-icon {
  width: 15px;
  height: 15px;
}

/* Empty State */
.empty-state-card {
  padding: 3.5rem 2rem;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  border-radius: 14px;
  border: 1px dashed var(--border-color);
  background: var(--card-bg);
}

.empty-icon-wrap {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: var(--bg-color);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}

.empty-icon-wrap svg {
  width: 32px;
  height: 32px;
}

.empty-title {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text-color);
}

.empty-desc {
  margin: 0;
  font-size: 0.85rem;
  color: var(--text-muted);
  max-width: 480px;
  line-height: 1.5;
}

/* Task Container */
.tasks-container {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.task-card {
  padding: 1.15rem 1.25rem;
  display: flex;
  gap: 1.25rem;
  align-items: center;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  transition: transform 0.15s, box-shadow 0.15s;
}

.task-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.task-thumb-wrap {
  position: relative;
  width: 140px;
  height: 82px;
  border-radius: 8px;
  overflow: hidden;
  background: #0f172a;
  flex-shrink: 0;
}

.task-thumb {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.task-thumb-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
}

.task-thumb-placeholder svg {
  width: 28px;
  height: 28px;
}

.status-badge-corner {
  position: absolute;
  top: 6px;
  left: 6px;
  font-size: 0.68rem;
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
}

.badge-primary {
  background-color: #2563eb;
  color: white;
}

.badge-secondary {
  background-color: #64748b;
  color: white;
}

.task-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.task-title-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.task-title {
  margin: 0;
  font-size: 0.98rem;
  font-weight: 600;
  color: var(--text-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-time {
  font-size: 0.75rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

/* Progress bar */
.progress-section {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.progress-bar-track {
  width: 100%;
  height: 7px;
  background: var(--bg-color);
  border-radius: 9999px;
  overflow: hidden;
  border: 1px solid var(--border-color);
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #3b82f6, #2563eb);
  border-radius: 9999px;
  transition: width 0.3s ease;
}

.bar-pulsing {
  background: linear-gradient(90deg, #3b82f6, #60a5fa);
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

.bar-paused {
  background: #f59e0b;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.75; }
}

.progress-stats-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.stats-left {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.stat-percentage {
  font-weight: 700;
  color: #2563eb;
}

.stat-size {
  color: var(--text-muted);
}

.stats-right {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.stat-speed {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  color: #10b981;
  font-weight: 600;
}

.stat-eta {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  color: var(--text-muted);
}

.stat-icon {
  width: 12px;
  height: 12px;
}

/* Completed row */
.completed-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-color);
  padding: 0.4rem 0.65rem;
  border-radius: 6px;
  font-size: 0.78rem;
}

.path-display-box {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  overflow: hidden;
  margin-right: 0.75rem;
}

.path-label {
  color: var(--text-muted);
  flex-shrink: 0;
}

.path-value {
  color: var(--text-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.completed-size-tag {
  color: var(--text-muted);
  flex-shrink: 0;
}

/* Failed alert */
.failed-alert-box {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background-color: #fee2e2;
  color: #b91c1c;
  padding: 0.4rem 0.75rem;
  border-radius: 6px;
  font-size: 0.78rem;
  border: 1px solid #fca5a5;
}

.failed-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.failed-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Source URL Bar */
.source-url-bar {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.75rem;
}

.url-label {
  color: var(--text-muted);
}

.url-link {
  color: #3b82f6;
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.url-link:hover {
  text-decoration: underline;
}

/* Task Actions */
.task-actions {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 110px;
}

.btn-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  padding: 0.45rem 0.75rem;
  border-radius: 8px;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid var(--border-color);
  background: var(--btn-sec-bg);
  color: var(--text-color);
  transition: all 0.2s;
  white-space: nowrap;
}

.btn-action:hover {
  background: var(--btn-sec-hover);
}

.action-btn-icon {
  width: 14px;
  height: 14px;
}

.btn-pause {
  color: #d97706;
}

.btn-pause:hover {
  background: #fef3c7;
  border-color: #fde68a;
}

.btn-resume {
  color: #2563eb;
}

.btn-resume:hover {
  background: #dbeafe;
  border-color: #bfdbfe;
}

.btn-cancel {
  color: #64748b;
}

.btn-cancel:hover {
  color: #ef4444;
  background: #fee2e2;
  border-color: #fca5a5;
}

.btn-view-lib {
  background: #eff6ff;
  color: #1d4ed8;
  border-color: #bfdbfe;
}

.btn-view-lib:hover {
  background: #dbeafe;
}

.btn-copy-path {
  color: var(--text-muted);
}

.btn-retry {
  background: #fee2e2;
  color: #b91c1c;
  border-color: #fca5a5;
}

.btn-retry:hover {
  background: #fecaca;
}

.btn-delete {
  color: var(--text-muted);
}

.btn-delete:hover {
  color: #ef4444;
}

/* Animations */
.animate-spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.25s ease;
}

.fade-enter-from, .fade-leave-to {
  opacity: 0;
}

.modal-enter-active, .modal-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.modal-enter-from, .modal-leave-to {
  opacity: 0;
  transform: scale(0.96);
}

/* Responsive */
@media (max-width: 768px) {
  .task-card {
    flex-direction: column;
    align-items: stretch;
  }
  .task-thumb-wrap {
    width: 100%;
    height: 160px;
  }
  .task-actions {
    flex-direction: row;
  }
  .preview-media-box {
    flex-direction: column;
  }
  .media-thumb-wrap {
    width: 100%;
    height: 160px;
  }
}
</style>
