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

// Task lists and filters
const tasks = ref<DownloadTask[]>([]);
const isLoadingTasks = ref<boolean>(false);
const activeFilter = ref<'all' | 'active' | 'completed' | 'failed'>('all');
const toastMessage = ref<string>('');
let toastTimer: number | null = null;
let pollTimer: number | null = null;

// Presets for video quality
const qualityPresets = [
  { id: 'best', label: '最佳画质', desc: '最佳音画混流 (Best)', formatId: 'bestvideo+bestaudio/best' },
  { id: '1080p', label: '1080P 高清', desc: 'Full HD 1080P', formatId: 'bestvideo[height<=1080]+bestaudio/best[height<=1080]' },
  { id: '720p', label: '720P 标清', desc: 'HD 720P', formatId: 'bestvideo[height<=720]+bestaudio/best[height<=720]' },
  { id: 'audio_only', label: '仅提取音频', desc: 'MP3/M4A 纯音频', formatId: 'bestaudio/best' },
  { id: 'custom', label: '自定义流格式', desc: '指定格式流 ID', formatId: '' },
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
  return `ID: ${fmt.format_id} [${fmt.ext}] ${fmt.resolution}${notePart}${sizePart}`;
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

// Auto-install yt-dlp engine when missing
const isInstallingEngine = ref<boolean>(false);
const isYtdlpMissingError = computed(() => {
  const err = inspectError.value.toLowerCase();
  return err.includes('yt-dlp') || err.includes('ytdl') || err.includes('未找到可用的');
});

async function handleInstallEngine() {
  isInstallingEngine.value = true;
  try {
    showToast('正在下载并部署独立托管 yt-dlp 引擎，请稍候...');
    const res = await invoke<{ success: boolean; message: string }>('update_ytdlp');
    if (res.success) {
      showToast('yt-dlp 核心引擎安装成功！正在重新解析...');
      inspectError.value = '';
      if (inputUrl.value.trim()) {
        await inspectVideoUrl();
      }
    } else {
      inspectError.value = `引擎安装失败: ${res.message}`;
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    inspectError.value = `引擎安装失败: ${msg}`;
  } finally {
    isInstallingEngine.value = false;
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

// Confirm download from inspect preview
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
    inspectResult.value = null;
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
    showToast('任务已暂停');
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
    showToast('任务已恢复');
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
    showToast('任务已取消');
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
    showToast('已重新加入下载队列');
    await fetchTasks();
  } catch {
    try {
      await invoke('start_download', {
        url: task.url,
        format_id: 'bestvideo+bestaudio/best',
        formatId: 'bestvideo+bestaudio/best',
      });
      showToast('已重新提交下载');
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
    showToast('已移除任务记录');
    await fetchTasks();
  } catch {
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

async function copyFilePath(path: string | null | undefined) {
  if (!path) return;
  try {
    await navigator.clipboard.writeText(path);
    showToast('文件路径已复制到剪贴板！');
  } catch {
    const input = document.createElement('input');
    input.value = path;
    document.body.appendChild(input);
    input.select();
    document.execCommand('copy');
    document.body.removeChild(input);
    showToast('文件路径已复制到剪贴板！');
  }
}

function viewInLibrary(task: DownloadTask) {
  emit('viewInLibrary', {
    fileName: task.title,
    filePath: task.target_path || undefined,
  });
}

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
  <div class="download-split-container">
    <!-- Toast Notification -->
    <transition name="fade">
      <div v-if="toastMessage" class="toast-box">
        <svg class="toast-ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
        <span>{{ toastMessage }}</span>
      </div>
    </transition>

    <!-- LEFT PANE: Download Studio (下载工坊与配置中心) -->
    <aside class="studio-pane">
      <div class="studio-header">
        <div class="studio-title-row">
          <div class="studio-icon-box">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
          </div>
          <div>
            <h3 class="studio-heading">视频下载工坊</h3>
            <span class="studio-sub">支持 B站、YouTube、TikTok 等</span>
          </div>
        </div>
      </div>

      <div class="studio-scroll-area">
        <!-- URL Input Block -->
        <div class="studio-section">
          <label class="studio-label">粘贴视频链接 (URL)：</label>
          <div class="url-input-box">
            <input
              v-model="inputUrl"
              class="url-input-field font-mono"
              placeholder="https://www.bilibili.com/video/BV... 或 YouTube 链接"
              @keyup.enter="inspectVideoUrl"
            />
            <button
              v-if="inputUrl"
              class="btn-clear-url"
              title="清空"
              @click="inputUrl = ''"
            >
              ✕
            </button>
            <button
              class="btn-paste-url"
              title="从剪贴板粘贴"
              @click="pasteFromClipboard"
            >
              粘贴
            </button>
          </div>

          <div class="studio-actions-row">
            <button
              class="btn btn-secondary flex-1"
              :disabled="isInspecting || !inputUrl.trim()"
              @click="inspectVideoUrl"
            >
              <span v-if="isInspecting" class="spinner-sm"></span>
              <span>{{ isInspecting ? '解析中...' : '解析元信息 (Inspect)' }}</span>
            </button>

            <button
              class="btn btn-primary flex-1"
              :disabled="isAddingDownload || !inputUrl.trim()"
              @click="quickAddDownload"
            >
              <span v-if="isAddingDownload" class="spinner-sm"></span>
              <span>{{ isAddingDownload ? '提交中...' : '快速下载' }}</span>
            </button>
          </div>

          <div v-if="inspectError" class="alert-error-mini">
            <div class="alert-error-content">
              <span class="alert-error-icon">⚠️</span>
              <span class="alert-error-text">{{ inspectError }}</span>
            </div>
            <button
              v-if="isYtdlpMissingError"
              type="button"
              class="btn-install-engine"
              :disabled="isInstallingEngine"
              @click="handleInstallEngine"
            >
              <span v-if="isInstallingEngine" class="spinner-sm"></span>
              <span>{{ isInstallingEngine ? '正在下载配置引擎，请稍候...' : '⚡️ 点击一键安装 yt-dlp 核心引擎并重试' }}</span>
            </button>
          </div>
        </div>

        <!-- Quality Preset Selection -->
        <div class="studio-section">
          <label class="studio-label">首选下载画质预设：</label>
          <div class="preset-cards-list">
            <label
              v-for="p in qualityPresets"
              :key="p.id"
              class="preset-item-card"
              :class="{ active: selectedFormatPreset === p.id }"
            >
              <input
                type="radio"
                name="quality"
                :value="p.id"
                v-model="selectedFormatPreset"
                class="hidden-radio"
              />
              <div class="preset-content">
                <span class="p-title">{{ p.label }}</span>
                <span class="p-desc">{{ p.desc }}</span>
              </div>
              <span class="check-mark font-mono" v-if="selectedFormatPreset === p.id">✓</span>
            </label>
          </div>
        </div>

        <!-- Embedded Inspect Preview Card (当解析成功时，直接在左侧展示预览和确认按钮) -->
        <div v-if="inspectResult" class="studio-section inspect-card-embedded">
          <div class="inspect-card-top">
            <span class="inspect-tag">已解析视频元信息</span>
            <button class="btn-close-inspect" @click="inspectResult = null">✕</button>
          </div>

          <div class="inspect-thumb-wrap">
            <img
              v-if="inspectResult.thumbnail"
              :src="inspectResult.thumbnail"
              class="inspect-thumb-img"
              :alt="inspectResult.title"
              referrerpolicy="no-referrer"
            />
            <div v-else class="inspect-thumb-ph">▶</div>
            <span v-if="inspectResult.duration" class="inspect-dur font-mono">
              {{ formatDuration(inspectResult.duration) }}
            </span>
          </div>

          <div class="inspect-info-wrap">
            <h4 class="inspect-title" :title="inspectResult.title">{{ inspectResult.title }}</h4>
            <div class="inspect-tags-row">
              <span v-if="inspectResult.extractor" class="tag-chip extractor-chip">
                {{ inspectResult.extractor.toUpperCase() }}
              </span>
              <span v-if="inspectResult.uploader" class="tag-chip">
                UP: {{ inspectResult.uploader }}
              </span>
            </div>

            <!-- Custom format selector if needed -->
            <div v-if="selectedFormatPreset === 'custom' || (inspectResult.formats && inspectResult.formats.length > 0)" class="format-select-sub">
              <label class="custom-f-lbl">可选流格式：</label>
              <select v-model="selectedCustomFormatId" class="select-format-dropdown font-mono">
                <option
                  v-for="fmt in inspectResult.formats || []"
                  :key="fmt.format_id"
                  :value="fmt.format_id"
                >
                  {{ formatStreamSummary(fmt) }}
                </option>
              </select>
            </div>

            <button
              class="btn btn-primary btn-full confirm-dl-btn"
              :disabled="isAddingDownload"
              @click="confirmInspectDownload"
            >
              <span v-if="isAddingDownload" class="spinner-sm"></span>
              <span>{{ isAddingDownload ? '启动中...' : '确认并开始下载任务' }}</span>
            </button>
          </div>
        </div>
      </div>
    </aside>

    <!-- RIGHT PANE: Task Queue & Monitor (任务看板与下载队列) -->
    <main class="queue-pane">
      <!-- Queue Filter Header -->
      <header class="queue-header">
        <div class="filter-tabs-row">
          <button
            class="q-tab-btn"
            :class="{ active: activeFilter === 'all' }"
            @click="activeFilter = 'all'"
          >
            全部任务
            <span class="q-badge-count">{{ tasks.length }}</span>
          </button>
          <button
            class="q-tab-btn"
            :class="{ active: activeFilter === 'active' }"
            @click="activeFilter = 'active'"
          >
            进行中
            <span class="q-badge-count hl-blue">{{ activeTasks.length }}</span>
          </button>
          <button
            class="q-tab-btn"
            :class="{ active: activeFilter === 'completed' }"
            @click="activeFilter = 'completed'"
          >
            已完成
            <span class="q-badge-count hl-green">{{ completedTasks.length }}</span>
          </button>
          <button
            class="q-tab-btn"
            :class="{ active: activeFilter === 'failed' }"
            @click="activeFilter = 'failed'"
          >
            异常/失败
            <span class="q-badge-count hl-red">{{ failedTasks.length }}</span>
          </button>
        </div>

        <div class="queue-tools-row">
          <button
            v-if="completedTasks.length > 0"
            class="btn-text-tool"
            @click="clearCompleted"
          >
            清空已完成
          </button>
          <button
            class="btn-refresh-tool"
            :class="{ 'spin-anim': isLoadingTasks }"
            title="刷新列表"
            @click="fetchTasks"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6"></path>
              <path d="M1 20v-6h6"></path>
              <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
            </svg>
          </button>
        </div>
      </header>

      <!-- Tasks List View -->
      <div class="queue-scroll-area">
        <!-- Empty State -->
        <div v-if="displayedTasks.length === 0" class="empty-queue-box">
          <div class="empty-icon-circle">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
          </div>
          <h3 class="empty-q-title">
            {{ activeFilter === 'all' ? '暂无下载任务' : `当前无${activeFilter === 'active' ? '进行中' : activeFilter === 'completed' ? '已完成' : '失败'}任务` }}
          </h3>
          <p class="empty-q-desc">
            在左侧下载工坊输入视频链接，解析并点击开始下载，视频将自动下载并归档至本地媒体库。
          </p>
        </div>

        <!-- Task Item Cards -->
        <div v-else class="task-cards-list">
          <div
            v-for="task in displayedTasks"
            :key="task.id"
            class="task-row-card"
          >
            <!-- Thumbnail Box -->
            <div class="task-thumb-col">
              <img
                v-if="task.thumbnail"
                :src="task.thumbnail"
                :alt="task.title"
                class="task-thumb-pic"
                referrerpolicy="no-referrer"
              />
              <div v-else class="task-thumb-ph">▶</div>
              <span class="status-corner-pill badge font-mono" :class="getStatusBadgeClass(task.status)">
                {{ getStatusText(task.status) }}
              </span>
            </div>

            <!-- Main Task Body -->
            <div class="task-info-col">
              <div class="task-top-line">
                <h4 class="task-title-text" :title="task.title">{{ task.title || task.url }}</h4>
                <span class="task-created-time font-mono">{{ formatDate(task.created_at) }}</span>
              </div>

              <!-- Active Downloading Progress -->
              <div v-if="task.status === 'downloading' || task.status === 'pending' || task.status === 'paused'" class="task-progress-box">
                <div class="progress-track">
                  <div
                    class="progress-fill"
                    :class="{ 'pulse-bar': task.status === 'downloading', 'paused-bar': task.status === 'paused' }"
                    :style="{ width: `${Math.min(Math.max(task.progress || 0, 0), 100)}%` }"
                  ></div>
                </div>

                <div class="progress-numbers-line">
                  <div class="p-num-left font-mono">
                    <span class="p-pct">{{ (task.progress || 0).toFixed(1) }}%</span>
                    <span class="p-bytes">{{ formatFileSize(task.downloaded_bytes) }} / {{ task.total_bytes > 0 ? formatFileSize(task.total_bytes) : '计算中...' }}</span>
                  </div>
                  <div class="p-num-right font-mono">
                    <span v-if="task.speed_str" class="p-speed">⚡ {{ task.speed_str }}</span>
                    <span v-if="task.eta_str" class="p-eta">ETA: {{ task.eta_str }}</span>
                  </div>
                </div>
              </div>

              <!-- Completed Meta -->
              <div v-else-if="task.status === 'completed'" class="task-completed-box">
                <div class="comp-path font-mono" :title="task.target_path || ''">
                  <span class="comp-label">文件路径：</span>{{ task.target_path || '默认下载目录' }}
                </div>
                <span class="comp-size font-mono">{{ formatFileSize(task.total_bytes || task.downloaded_bytes) }}</span>
              </div>

              <!-- Failed Alert -->
              <div v-else-if="task.status === 'failed' || task.status === 'cancelled'" class="task-failed-box">
                <span class="fail-msg">{{ task.error_message || (task.status === 'cancelled' ? '用户已取消下载' : '下载失败，网络中断或防盗链限制') }}</span>
              </div>

              <!-- Source URL -->
              <div class="task-source-line">
                <span class="source-tag">来源:</span>
                <a :href="task.url" target="_blank" class="source-url-link font-mono" :title="task.url">{{ task.url }}</a>
              </div>
            </div>

            <!-- Task Controls -->
            <div class="task-controls-col">
              <template v-if="task.status === 'downloading'">
                <button class="btn-ctrl btn-pause" @click="pauseTask(task)" title="暂停">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="6" y="4" width="4" height="16"></rect>
                    <rect x="14" y="4" width="4" height="16"></rect>
                  </svg>
                  <span>暂停</span>
                </button>
                <button class="btn-ctrl btn-cancel" @click="cancelTask(task)" title="取消">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                  <span>取消</span>
                </button>
              </template>

              <template v-else-if="task.status === 'paused'">
                <button class="btn-ctrl btn-resume" @click="resumeTask(task)" title="继续">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="5 3 19 12 5 21 5 3"></polygon>
                  </svg>
                  <span>继续</span>
                </button>
                <button class="btn-ctrl btn-cancel" @click="cancelTask(task)" title="取消">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                  <span>取消</span>
                </button>
              </template>

              <template v-else-if="task.status === 'completed'">
                <button class="btn-ctrl btn-view-lib" @click="viewInLibrary(task)" title="在媒体库中查看">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="2" y="2" width="20" height="20" rx="2.18"></rect>
                    <line x1="7" y1="2" x2="7" y2="22"></line>
                  </svg>
                  <span>查看媒体库</span>
                </button>
                <button class="btn-ctrl btn-copy" @click="copyFilePath(task.target_path)" title="复制文件路径">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="9" y="9" width="13" height="13" rx="2"></rect>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                  </svg>
                  <span>复制路径</span>
                </button>
              </template>

              <template v-else-if="task.status === 'failed' || task.status === 'cancelled'">
                <button class="btn-ctrl btn-retry" @click="retryTask(task)" title="重新下载">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="1 4 1 10 7 10"></polyline>
                    <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
                  </svg>
                  <span>重试</span>
                </button>
                <button class="btn-ctrl btn-delete" @click="deleteTask(task)" title="删除记录">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
    </main>
  </div>
</template>

<style scoped>
.download-split-container {
  display: flex;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background-color: var(--bg-app);
}

/* Toast */
.toast-box {
  position: fixed;
  top: 1.25rem;
  right: 1.5rem;
  z-index: 1000;
  background-color: #1e293b;
  color: #f8fafc;
  padding: 0.65rem 1.15rem;
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.85rem;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.toast-ico {
  width: 16px;
  height: 16px;
  color: #10b981;
}

/* Left Studio Pane */
.studio-pane {
  width: 380px;
  min-width: 380px;
  max-width: 380px;
  height: 100%;
  background-color: var(--bg-card);
  border-right: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.studio-header {
  height: 50px;
  min-height: 50px;
  padding: 0 1.25rem;
  display: flex;
  align-items: center;
  border-bottom: 1px solid var(--border-subtle);
  background-color: var(--bg-header);
}

.studio-title-row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.studio-icon-box {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  background: linear-gradient(135deg, #3b82f6, #6366f1);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.studio-icon-box svg {
  width: 15px;
  height: 15px;
}

.studio-heading {
  margin: 0;
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--text-primary);
}

.studio-sub {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.studio-scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.studio-section {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.studio-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.url-input-box {
  position: relative;
  display: flex;
  align-items: center;
}

.url-input-field {
  width: 100%;
  padding: 0.55rem 3.5rem 0.55rem 0.75rem;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 0.82rem;
  outline: none;
}

.url-input-field:focus {
  border-color: #3b82f6;
}

.btn-clear-url {
  position: absolute;
  right: 2.4rem;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.75rem;
}

.btn-paste-url {
  position: absolute;
  right: 4px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 5px;
  color: #3b82f6;
  font-size: 0.7rem;
  font-weight: 600;
  padding: 2px 6px;
  cursor: pointer;
}

.studio-actions-row {
  display: flex;
  gap: 0.65rem;
}

.flex-1 {
  flex: 1;
}

.alert-error-mini {
  padding: 0.65rem 0.85rem;
  background-color: #fef2f2;
  color: #991b1b;
  border-radius: 8px;
  font-size: 0.78rem;
  border: 1px solid #fecaca;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.alert-error-content {
  display: flex;
  align-items: flex-start;
  gap: 0.4rem;
  line-height: 1.45;
}

.alert-error-text {
  word-break: break-all;
}

.btn-install-engine {
  align-self: flex-start;
  background: #dc2626;
  color: #ffffff;
  border: none;
  border-radius: 6px;
  padding: 0.4rem 0.85rem;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  transition: all 0.2s ease;
  box-shadow: 0 1px 2px rgba(220, 38, 38, 0.2);
}

.btn-install-engine:hover:not(:disabled) {
  background: #b91c1c;
}

.btn-install-engine:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Preset Cards */
.preset-cards-list {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.preset-item-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.6rem 0.85rem;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  cursor: pointer;
  transition: all 0.15s ease;
}

.preset-item-card:hover {
  background: var(--btn-hover);
}

.preset-item-card.active {
  border-color: #3b82f6;
  background: var(--nav-active-bg);
}

.hidden-radio {
  display: none;
}

.preset-content {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.p-title {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-primary);
}

.p-desc {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.check-mark {
  color: #3b82f6;
  font-size: 0.85rem;
  font-weight: 700;
}

/* Embedded Inspect Preview Card */
.inspect-card-embedded {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  padding: 0.85rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.inspect-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.inspect-tag {
  font-size: 0.72rem;
  font-weight: 700;
  color: #3b82f6;
  text-transform: uppercase;
}

.btn-close-inspect {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.85rem;
}

.inspect-thumb-wrap {
  position: relative;
  width: 100%;
  padding-top: 56.25%;
  border-radius: 7px;
  overflow: hidden;
  background: #090e17;
}

.inspect-thumb-img {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.inspect-thumb-ph {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
}

.inspect-dur {
  position: absolute;
  bottom: 6px;
  right: 6px;
  background: rgba(0, 0, 0, 0.75);
  color: white;
  padding: 2px 5px;
  border-radius: 4px;
  font-size: 0.7rem;
}

.inspect-info-wrap {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.inspect-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  line-height: 1.35;
  color: var(--text-primary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.inspect-tags-row {
  display: flex;
  gap: 0.4rem;
}

.tag-chip {
  font-size: 0.7rem;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--bg-card);
  color: var(--text-muted);
  font-weight: 500;
}

.extractor-chip {
  background: #dbeafe;
  color: #1e40af;
  font-weight: 700;
}

.format-select-sub {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.custom-f-lbl {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.select-format-dropdown {
  width: 100%;
  padding: 0.4rem;
  border-radius: 6px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 0.75rem;
  outline: none;
}

.confirm-dl-btn {
  margin-top: 0.35rem;
}

/* RIGHT PANE: Task Queue & Monitor */
.queue-pane {
  flex: 1;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-app);
}

.queue-header {
  height: 50px;
  min-height: 50px;
  padding: 0 1.25rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-header);
}

.filter-tabs-row {
  display: flex;
  gap: 0.4rem;
}

.q-tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.75rem;
  border-radius: 7px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.q-tab-btn:hover {
  background: var(--btn-hover);
}

.q-tab-btn.active {
  background: var(--bg-input);
  color: #3b82f6;
  font-weight: 600;
}

.q-badge-count {
  font-size: 0.7rem;
  padding: 0.05rem 0.4rem;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.2);
  color: var(--text-muted);
}

.hl-blue { background: rgba(59, 130, 246, 0.15); color: #3b82f6; }
.hl-green { background: rgba(16, 185, 129, 0.15); color: #10b981; }
.hl-red { background: rgba(239, 68, 68, 0.15); color: #ef4444; }

.queue-tools-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.btn-text-tool {
  background: none;
  border: none;
  font-size: 0.75rem;
  color: var(--text-muted);
  cursor: pointer;
}

.btn-text-tool:hover {
  color: #ef4444;
}

.btn-refresh-tool {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
}

.btn-refresh-tool svg {
  width: 15px;
  height: 15px;
}

.queue-scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

/* Empty Queue */
.empty-queue-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 4rem 2rem;
  text-align: center;
}

.empty-icon-circle {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: var(--bg-input);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  margin-bottom: 1rem;
}

.empty-icon-circle svg {
  width: 30px;
  height: 30px;
}

.empty-q-title {
  margin: 0 0 0.4rem;
  font-size: 1.05rem;
  color: var(--text-primary);
}

.empty-q-desc {
  margin: 0;
  font-size: 0.82rem;
  color: var(--text-muted);
  max-width: 440px;
}

/* Task Row Cards */
.task-cards-list {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.task-row-card {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 11px;
  padding: 0.85rem;
  display: flex;
  gap: 1rem;
  align-items: center;
  transition: all 0.15s ease;
}

.task-row-card:hover {
  border-color: rgba(59, 130, 246, 0.3);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.task-thumb-col {
  position: relative;
  width: 120px;
  height: 68px;
  min-width: 120px;
  border-radius: 7px;
  overflow: hidden;
  background: #090e17;
}

.task-thumb-pic {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.task-thumb-ph {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  font-size: 1rem;
}

.status-corner-pill {
  position: absolute;
  top: 4px;
  left: 4px;
  font-size: 0.65rem;
  padding: 1px 5px;
  border-radius: 4px;
}

.badge-primary { background: #2563eb; color: white; }
.badge-secondary { background: #475569; color: white; }
.badge-success { background: #15803d; color: white; }
.badge-warning { background: #b45309; color: white; }
.badge-danger { background: #b91c1c; color: white; }

.task-info-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  overflow: hidden;
}

.task-top-line {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 0.5rem;
}

.task-title-text {
  margin: 0;
  font-size: 0.88rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text-primary);
}

.task-created-time {
  font-size: 0.72rem;
  color: var(--text-muted);
  white-space: nowrap;
}

/* Progress bar */
.task-progress-box {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.progress-track {
  width: 100%;
  height: 6px;
  background: var(--bg-input);
  border-radius: 999px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #3b82f6;
  border-radius: 999px;
  transition: width 0.2s ease;
}

.pulse-bar {
  background: linear-gradient(90deg, #3b82f6, #60a5fa, #3b82f6);
  background-size: 200% 100%;
  animation: shimmer 2s infinite linear;
}

.paused-bar {
  background: #f59e0b;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.progress-numbers-line {
  display: flex;
  justify-content: space-between;
  font-size: 0.72rem;
}

.p-num-left {
  display: flex;
  gap: 0.5rem;
}

.p-pct {
  font-weight: 700;
  color: #3b82f6;
}

.p-bytes {
  color: var(--text-muted);
}

.p-num-right {
  display: flex;
  gap: 0.75rem;
  color: var(--text-muted);
}

.p-speed {
  color: #10b981;
  font-weight: 600;
}

/* Completed & Failed boxes */
.task-completed-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.comp-path {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 480px;
}

.comp-label {
  color: var(--text-muted);
}

.comp-size {
  color: var(--text-primary);
  font-weight: 600;
}

.task-failed-box {
  font-size: 0.75rem;
  color: #ef4444;
}

.task-source-line {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.7rem;
}

.source-tag {
  color: var(--text-muted);
}

.source-url-link {
  color: var(--text-muted);
  text-decoration: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 460px;
}

.source-url-link:hover {
  text-decoration: underline;
  color: #3b82f6;
}

/* Task controls */
.task-controls-col {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.btn-ctrl {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.35rem 0.65rem;
  border-radius: 6px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-input);
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-ctrl:hover {
  background: var(--btn-hover);
}

.btn-ctrl svg {
  width: 13px;
  height: 13px;
}

.btn-delete:hover, .btn-cancel:hover {
  color: #ef4444;
  border-color: rgba(239, 68, 68, 0.4);
}

.btn-view-lib {
  background: #eff6ff;
  color: #1e40af;
  border-color: #bfdbfe;
}

.btn-view-lib:hover {
  background: #dbeafe;
}

/* Global button helpers */
.btn-full {
  width: 100%;
}

.spinner-sm {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}

.spin-anim {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
