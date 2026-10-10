<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { UrlInspectResult, DownloadTask } from '../types';

interface QualityOptionItem {
  id: string;
  category: 'video' | 'music';
  label: string;
  sizeStr: string;
  formatId: string;
}

const props = defineProps<{
  isStandaloneWindow?: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:activeCount', count: number): void;
  (e: 'viewInLibrary', payload: { fileName?: string; filePath?: string }): void;
}>();

// Main input & inspect state
const inputUrl = ref<string>('');
const isInspecting = ref<boolean>(false);
const inspectError = ref<string>('');
const inspectResult = ref<UrlInspectResult | null>(null);

// Quality selection (video / music)
const selectedOptionId = ref<string>('video-1080p');

// UI View tabs: 'parser' | 'tasks'
const currentView = ref<'parser' | 'tasks'>('parser');

// Task list & polling
const tasks = ref<DownloadTask[]>([]);
const isLoadingTasks = ref<boolean>(false);
const activeFilter = ref<'all' | 'active' | 'completed' | 'failed'>('all');
const toastMessage = ref<string>('');
let toastTimer: number | null = null;
let pollTimer: number | null = null;

// Supported sites list (Matching Image 1)
const supportedSites = [
  {
    name: 'YouTube',
    iconType: 'youtube',
    exampleUrl: 'https://www.youtube.com/watch?v=rRK8KP4Y68g',
    bg: '#f1f3f5',
    color: '#000000',
  },
  {
    name: 'vimeo',
    iconType: 'vimeo',
    exampleUrl: 'https://vimeo.com/76979871',
    bg: '#f1f3f5',
    color: '#000000',
  },
  {
    name: 'TED',
    iconType: 'ted',
    exampleUrl: 'https://www.ted.com/talks/al_gore_the_case_for_optimism_on_climate_change',
    bg: '#f1f3f5',
    color: '#e62b1e',
  },
  {
    name: 'OnlyFans',
    iconType: 'onlyfans',
    exampleUrl: '',
    bg: '#f1f3f5',
    color: '#00aff0',
  },
  {
    name: 'DailyMotion',
    iconType: 'dailymotion',
    exampleUrl: 'https://www.dailymotion.com/video/x7tgad0',
    bg: '#f1f3f5',
    color: '#000000',
  },
  {
    name: 'rumble',
    iconType: 'rumble',
    exampleUrl: 'https://rumble.com/v20847w-top-news.html',
    bg: '#f1f3f5',
    color: '#85c742',
  },
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
  return tasks.value.filter((t) => t.status === 'failed' || t.status === 'error' || t.status === 'cancelled');
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

// Extract Video & Music options based on inspection or smart presets
const videoOptions = computed<QualityOptionItem[]>(() => {
  if (!inspectResult.value) return [];
  const duration = inspectResult.value.duration || 600; // fallback 10 mins

  // Standard video heights
  const standardTiers = [
    { height: 1080, label: '1080P', baseMb: 435, defaultBitrate: 5500 },
    { height: 720, label: '720P', baseMb: 235, defaultBitrate: 2800 },
    { height: 480, label: '480P', baseMb: 126, defaultBitrate: 1500 },
    { height: 360, label: '360P', baseMb: 73, defaultBitrate: 850 },
    { height: 240, label: '240P', baseMb: 46, defaultBitrate: 500 },
    { height: 144, label: '144P', baseMb: 29, defaultBitrate: 300 },
  ];

  const formats = inspectResult.value.formats || [];

  return standardTiers.map((tier) => {
    // Find if specific format exists in inspect results
    const matchedFmt = formats.find((f) => {
      const res = f.resolution || '';
      return res.includes(`${tier.height}`) || (f.note && f.note.includes(`${tier.height}p`));
    });

    let sizeStr = '';
    if (matchedFmt && matchedFmt.filesize) {
      sizeStr = formatFileSize(matchedFmt.filesize);
    } else {
      // Calculate realistic estimate based on duration
      const estBytes = (duration * tier.defaultBitrate * 1024) / 8;
      sizeStr = formatFileSize(estBytes);
    }

    const formatId = matchedFmt?.format_id
      ? `${matchedFmt.format_id}+bestaudio/bestvideo[height<=${tier.height}]+bestaudio/best`
      : `bestvideo[height<=${tier.height}]+bestaudio/best[height<=${tier.height}]/best`;

    return {
      id: `video-${tier.height}p`,
      category: 'video',
      label: tier.label,
      sizeStr,
      formatId,
    };
  });
});

const musicOptions = computed<QualityOptionItem[]>(() => {
  if (!inspectResult.value) return [];
  const duration = inspectResult.value.duration || 600;

  const audioTiers = [
    { kbps: 64, label: '64kbps', defaultBitrate: 64 },
    { kbps: 128, label: '128kbps', defaultBitrate: 128 },
    { kbps: 320, label: '320kbps', defaultBitrate: 320 },
  ];

  return audioTiers.map((tier) => {
    const estBytes = (duration * tier.defaultBitrate * 1024) / 8;
    return {
      id: `music-${tier.kbps}k`,
      category: 'music',
      label: tier.label,
      sizeStr: formatFileSize(estBytes),
      formatId: `bestaudio[abr<=${tier.kbps}]/bestaudio/best`,
    };
  });
});

// Currently selected option object
const currentSelectedOption = computed<QualityOptionItem | undefined>(() => {
  const v = videoOptions.value.find((o) => o.id === selectedOptionId.value);
  if (v) return v;
  return musicOptions.value.find((o) => o.id === selectedOptionId.value);
});

// Trigger URL inspection
async function inspectVideoUrl() {
  const url = inputUrl.value.trim();
  if (!url) {
    inspectError.value = '请粘贴需要解析的视频链接 (Paste video link)';
    return;
  }
  inspectError.value = '';
  isInspecting.value = true;
  inspectResult.value = null;

  try {
    const res = await invoke<UrlInspectResult>('inspect_url', { url });
    inspectResult.value = res;
    // Default to 1080P or the first video option
    if (videoOptions.value.length > 0) {
      selectedOptionId.value = videoOptions.value[0].id;
    }
    showToast('视频信息解析成功！');
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    inspectError.value = `视频解析失败: ${msg}`;
  } finally {
    isInspecting.value = false;
  }
}

// Start download of the selected format
const isStartingDownload = ref<boolean>(false);
async function startSelectedDownload() {
  if (!inspectResult.value) return;
  const targetOption = currentSelectedOption.value;
  const formatId = targetOption ? targetOption.formatId : 'bestvideo+bestaudio/best';

  isStartingDownload.value = true;
  try {
    await invoke<DownloadTask>('start_download', {
      url: inspectResult.value.url,
      format_id: formatId,
      formatId: formatId,
    });
    showToast(`已开始下载: ${targetOption?.label || '最佳画质'}`);
    await fetchTasks();
    // Switch to task view or keep in parser with toast
    currentView.value = 'tasks';
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    showToast(`下载启动失败: ${msg}`);
  } finally {
    isStartingDownload.value = false;
  }
}

// Use supported site sample link
function handleSiteClick(site: typeof supportedSites[0]) {
  if (site.exampleUrl) {
    inputUrl.value = site.exampleUrl;
    inspectVideoUrl();
  } else {
    showToast(`请复制 ${site.name} 的视频链接并粘贴`);
  }
}

// Fetch download tasks from backend
async function fetchTasks() {
  isLoadingTasks.value = true;
  try {
    const list = await invoke<DownloadTask[]>('get_download_tasks');
    tasks.value = list || [];
    emit('update:activeCount', activeTasks.value.length);
  } catch (err: unknown) {
    console.warn('Failed to fetch download tasks:', err);
  } finally {
    isLoadingTasks.value = false;
  }
}

// Open standalone window
async function openInStandaloneWindow() {
  try {
    await invoke('open_ytd_window');
  } catch (err: unknown) {
    showToast(`打开独立窗口失败: ${err}`);
  }
}

// Task control operations
async function pauseTask(task: DownloadTask) {
  try {
    await invoke('pause_download_task', { task_id: task.id, taskId: task.id, id: task.id });
    showToast('任务已暂停');
    await fetchTasks();
  } catch (err: unknown) {
    showToast(`暂停失败: ${err}`);
  }
}

async function resumeTask(task: DownloadTask) {
  try {
    await invoke('resume_download_task', { task_id: task.id, taskId: task.id, id: task.id });
    showToast('任务已恢复');
    await fetchTasks();
  } catch (err: unknown) {
    showToast(`恢复失败: ${err}`);
  }
}

async function cancelTask(task: DownloadTask) {
  if (!confirm(`确定取消任务 "${task.title}" 吗？`)) return;
  try {
    await invoke('cancel_download_task', { task_id: task.id, taskId: task.id, id: task.id });
    showToast('任务已取消');
    await fetchTasks();
  } catch (err: unknown) {
    showToast(`取消失败: ${err}`);
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
  }
}

function viewInLibrary(task: DownloadTask) {
  emit('viewInLibrary', {
    fileName: task.title || undefined,
    filePath: task.target_path || undefined,
  });
}

async function retryTask(task: DownloadTask) {
  try {
    showToast(`正在重试下载: ${task.title || '视频'}`);
    await invoke('start_download', {
      url: task.url,
      formatId: task.format_id || undefined,
    });
    currentView.value = 'tasks';
    activeFilter.value = 'active';
    await fetchTasks();
  } catch (err: unknown) {
    showToast(`重试失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function pasteFromClipboard() {
  try {
    const text = await navigator.clipboard.readText();
    if (text) {
      inputUrl.value = text.trim();
      inspectError.value = '';
    }
  } catch {
    showToast('请直接在输入框按 ⌘V / Ctrl+V 粘贴');
  }
}

function formatFileSize(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return '0 MB';
  const mb = bytes / (1024 * 1024);
  if (mb < 1000) {
    return `${Math.round(mb)} MB`;
  }
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(1)} GB`;
}

onMounted(async () => {
  await fetchTasks();
  pollTimer = window.setInterval(fetchTasks, 1500);
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
  if (toastTimer) clearTimeout(toastTimer);
});
</script>

<template>
  <div class="ytd-container" :class="{ 'standalone-mode': props.isStandaloneWindow }">
    <!-- Top Bar Navigation (Window Split & Task Queue) -->
    <div class="ytd-topbar">
      <div class="topbar-left">
        <button
          class="topbar-tab-btn"
          :class="{ active: currentView === 'parser' }"
          @click="currentView = 'parser'"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span>嗅探下载器</span>
        </button>

        <button
          class="topbar-tab-btn"
          :class="{ active: currentView === 'tasks' }"
          @click="currentView = 'tasks'"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="8" y1="6" x2="21" y2="6"></line>
            <line x1="8" y1="12" x2="21" y2="12"></line>
            <line x1="8" y1="18" x2="21" y2="18"></line>
            <line x1="3" y1="6" x2="3.01" y2="6"></line>
            <line x1="3" y1="12" x2="3.01" y2="12"></line>
            <line x1="3" y1="18" x2="3.01" y2="18"></line>
          </svg>
          <span>下载任务列表</span>
          <span v-if="activeTasks.length > 0" class="badge-count">{{ activeTasks.length }}</span>
        </button>
      </div>

      <div class="topbar-right">
        <button
          v-if="!props.isStandaloneWindow"
          class="action-pill-btn"
          title="在新独立窗口中打开下载器"
          @click="openInStandaloneWindow"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
            <polyline points="15 3 21 3 21 9"></polyline>
            <line x1="10" y1="14" x2="21" y2="3"></line>
          </svg>
          <span>独立窗口</span>
        </button>
      </div>
    </div>

    <!-- MAIN PARSER VIEW (Matching Image 1 & Image 2) -->
    <div v-show="currentView === 'parser'" class="parser-wrapper">
      <!-- Title -->
      <h1 class="main-heading">Download Video and Music</h1>

      <!-- Giant Input Card (Matching Image 1 & Image 2) -->
      <div class="search-input-box" :class="{ 'has-error': !!inspectError }">
        <div class="input-inner">
          <input
            v-model="inputUrl"
            type="text"
            class="video-url-input"
            placeholder="Paste the video link here..."
            @keydown.enter="inspectVideoUrl"
          />
          <button
            v-if="!inputUrl"
            class="paste-btn"
            title="从剪贴板粘贴链接"
            @click="pasteFromClipboard"
          >
            📋 粘贴
          </button>
          <button
            v-else
            class="clear-btn"
            title="清空输入"
            @click="inputUrl = ''; inspectResult = null; inspectError = '';"
          >
            ✕
          </button>
        </div>

        <button
          class="download-trigger-btn"
          :disabled="isInspecting"
          @click="inspectVideoUrl"
        >
          <span v-if="isInspecting" class="spinner-small"></span>
          <span>{{ isInspecting ? '解析中...' : 'Download' }}</span>
        </button>
      </div>

      <!-- Error alert -->
      <div v-if="inspectError" class="error-banner">
        <span>⚠️ {{ inspectError }}</span>
      </div>

      <!-- STATE 1: Not Inspected Yet -> Show Supported Sites (Image 1) -->
      <div v-if="!inspectResult && !isInspecting" class="uninspected-state">
        <p class="copy-hint-text">Copy link from supported sites.</p>

        <h2 class="supported-heading">Supported sites</h2>

        <div class="supported-grid">
          <div
            v-for="site in supportedSites"
            :key="site.name"
            class="site-card"
            @click="handleSiteClick(site)"
          >
            <!-- YouTube Icon -->
            <template v-if="site.iconType === 'youtube'">
              <div class="site-icon-wrapper">
                <svg width="28" height="20" viewBox="0 0 28 20" fill="none">
                  <rect width="28" height="20" rx="5" fill="#FF0000"/>
                  <polygon points="11,5 19,10 11,15" fill="#FFFFFF"/>
                </svg>
              </div>
              <span class="site-name-text yt-font">YouTube</span>
            </template>

            <!-- Vimeo Icon -->
            <template v-else-if="site.iconType === 'vimeo'">
              <div class="site-icon-wrapper">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="#00ADEF">
                  <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm6.5 8.5c-.2 3.1-2.4 7.4-6.4 7.4-3.8 0-4.3-3.6-5.2-6.1-.5-1.5-.9-2.2-1.6-2.2-.2 0-.8.4-1.5 1.1l-.8-1c1.2-1.1 2.5-2.3 3.6-2.3 1.3 0 2.1.9 2.3 2.6.3 2 .8 4.7 1.7 4.7.7 0 1.6-1.5 1.7-2.7.2-1.4-.7-1.4-1.4-1.2.6-2 1.8-3 3.6-3 1.5 0 2.2 1 2 2.7z"/>
                </svg>
              </div>
              <span class="site-name-text vimeo-font">vimeo</span>
            </template>

            <!-- TED Icon -->
            <template v-else-if="site.iconType === 'ted'">
              <span class="ted-logo-text">TED</span>
            </template>

            <!-- OnlyFans Icon -->
            <template v-else-if="site.iconType === 'onlyfans'">
              <div class="site-icon-wrapper">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="#00AFF0">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 14c-2.21 0-4-1.79-4-4s1.79-4 4-4 4 1.79 4 4-1.79 4-4 4z"/>
                </svg>
              </div>
              <span class="site-name-text onlyfans-font">OnlyFans</span>
            </template>

            <!-- DailyMotion Icon -->
            <template v-else-if="site.iconType === 'dailymotion'">
              <div class="site-icon-wrapper">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="#000000">
                  <circle cx="12" cy="12" r="10"/>
                  <path d="M8 7h4a5 5 0 0 1 0 10H8V7z" fill="#FFFFFF"/>
                </svg>
              </div>
              <span class="site-name-text dm-font">DailyMotion</span>
            </template>

            <!-- Rumble Icon -->
            <template v-else-if="site.iconType === 'rumble'">
              <div class="site-icon-wrapper">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="#85C742">
                  <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm-2 14.5v-9l7 4.5z"/>
                </svg>
              </div>
              <span class="site-name-text rumble-font">rumble</span>
            </template>
          </div>
        </div>
      </div>

      <!-- STATE 2: Parsing in Progress Skeleton -->
      <div v-if="isInspecting" class="inspecting-skeleton">
        <div class="skeleton-spinner-box">
          <div class="spinner-large"></div>
          <p class="skeleton-text">正在深度解析多媒体视频流与音频格式...</p>
          <span class="skeleton-sub">借助 yt-dlp 核心引擎秒级萃取全分辨率列表</span>
        </div>
      </div>

      <!-- STATE 3: Inspected Success -> Show Format Options (Matching Image 2) -->
      <div v-if="inspectResult && !isInspecting" class="result-card">
        <!-- Left Column: Thumbnail + Video Title -->
        <div class="result-left-col">
          <div class="thumb-container">
            <img
              v-if="inspectResult.thumbnail"
              :src="inspectResult.thumbnail"
              :alt="inspectResult.title"
              class="video-thumbnail"
            />
            <div v-else class="thumb-fallback">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="1.5">
                <polygon points="6 3 20 12 6 21 6 3"></polygon>
              </svg>
            </div>
          </div>
          <h3 class="video-display-title" :title="inspectResult.title">
            {{ inspectResult.title }}
          </h3>
          <div v-if="inspectResult.uploader" class="video-uploader-tag">
            <span>👤 {{ inspectResult.uploader }}</span>
          </div>
        </div>

        <!-- Right Column: Video & Music Options Matrix -->
        <div class="result-right-col">
          <!-- Video Section -->
          <div class="format-section">
            <h4 class="format-heading">Video</h4>

            <div class="format-options-grid">
              <label
                v-for="opt in videoOptions"
                :key="opt.id"
                class="radio-option-item"
                :class="{ active: selectedOptionId === opt.id }"
              >
                <input
                  v-model="selectedOptionId"
                  type="radio"
                  name="download_format"
                  :value="opt.id"
                  class="native-radio-hidden"
                />
                <!-- Custom Radio Indicator (Matching Red Circle Dot in Image 2) -->
                <span class="custom-radio-circle"></span>
                <span class="option-label-text">{{ opt.label }} ({{ opt.sizeStr }})</span>
              </label>
            </div>
          </div>

          <!-- Divider -->
          <div class="format-divider"></div>

          <!-- Music Section -->
          <div class="format-section">
            <h4 class="format-heading">Music</h4>

            <div class="format-options-grid">
              <label
                v-for="opt in musicOptions"
                :key="opt.id"
                class="radio-option-item"
                :class="{ active: selectedOptionId === opt.id }"
              >
                <input
                  v-model="selectedOptionId"
                  type="radio"
                  name="download_format"
                  :value="opt.id"
                  class="native-radio-hidden"
                />
                <span class="custom-radio-circle"></span>
                <span class="option-label-text">{{ opt.label }} ({{ opt.sizeStr }})</span>
              </label>
            </div>
          </div>

          <!-- DOWNLOAD Big Red Button (Matching Image 2) -->
          <div class="download-action-row">
            <button
              class="giant-download-btn"
              :disabled="isStartingDownload"
              @click="startSelectedDownload"
            >
              <span v-if="isStartingDownload" class="spinner-small"></span>
              <span>DOWNLOAD</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- TASK QUEUE VIEW -->
    <div v-show="currentView === 'tasks'" class="tasks-wrapper">
      <div class="tasks-header-row">
        <div class="tasks-filter-group">
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'all' }"
            @click="activeFilter = 'all'"
          >
            全部 ({{ tasks.length }})
          </button>
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'active' }"
            @click="activeFilter = 'active'"
          >
            进行中 ({{ activeTasks.length }})
          </button>
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'completed' }"
            @click="activeFilter = 'completed'"
          >
            已完成 ({{ completedTasks.length }})
          </button>
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'failed' }"
            @click="activeFilter = 'failed'"
          >
            失败 ({{ failedTasks.length }})
          </button>
        </div>

        <div class="tasks-action-right">
          <button
            v-if="completedTasks.length > 0"
            class="clear-tasks-btn"
            @click="clearCompleted"
          >
            清空已完成
          </button>
        </div>
      </div>

      <!-- Empty State -->
      <div v-if="displayedTasks.length === 0" class="empty-tasks-box">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="1.5">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="7 10 12 15 17 10"/>
          <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        <p>暂无符合条件的下载任务</p>
        <button class="return-parser-btn" @click="currentView = 'parser'">
          去添加新视频下载
        </button>
      </div>

      <!-- Task Card List -->
      <div v-else class="tasks-card-list">
        <div
          v-for="task in displayedTasks"
          :key="task.id"
          class="task-item-card"
        >
          <div class="task-card-left">
            <div class="task-thumb-box">
              <img
                v-if="task.thumbnail"
                :src="task.thumbnail"
                class="task-thumb"
              />
              <div v-else class="task-thumb-placeholder">🎬</div>
            </div>
          </div>

          <div class="task-card-center">
            <div class="task-title-row">
              <span class="task-title" :title="task.title || undefined">{{ task.title }}</span>
              <span class="task-status-pill" :class="task.status">{{ task.status }}</span>
            </div>

            <!-- Progress bar -->
            <div class="progress-track">
              <div
                class="progress-fill"
                :class="task.status"
                :style="{ width: `${Math.min(100, Math.max(0, task.progress || 0))}%` }"
              ></div>
            </div>

            <div class="task-meta-row">
              <span>{{ (task.progress || 0).toFixed(1) }}%</span>
              <span v-if="task.speed_str">⚡ {{ task.speed_str }}</span>
              <span v-if="task.downloaded_bytes && task.total_bytes">
                {{ formatFileSize(task.downloaded_bytes) }} / {{ formatFileSize(task.total_bytes) }}
              </span>
              <span v-if="task.eta_str">⏳ 剩余 {{ task.eta_str }}</span>
            </div>

            <!-- Error message if error or failed -->
            <div v-if="(task.status === 'error' || task.status === 'failed') && task.error_log" class="task-error-tip">
              ⚠️ {{ task.error_log }}
            </div>
          </div>

          <div class="task-card-actions">
            <button
              v-if="task.status === 'error' || task.status === 'failed'"
              class="task-btn retry"
              title="重新下载"
              @click="retryTask(task)"
            >
              🔄 重试
            </button>
            <button
              v-if="task.status === 'downloading'"
              class="task-btn"
              title="暂停"
              @click="pauseTask(task)"
            >
              ⏸️
            </button>
            <button
              v-else-if="task.status === 'paused'"
              class="task-btn"
              title="继续"
              @click="resumeTask(task)"
            >
              ▶️
            </button>
            <button
              v-if="task.status === 'completed'"
              class="task-btn success"
              title="在媒体库中查看"
              @click="viewInLibrary(task)"
            >
              📁 查看
            </button>
            <button
              v-if="task.status !== 'completed'"
              class="task-btn danger"
              title="取消任务"
              @click="cancelTask(task)"
            >
              ✕
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Toast Notification -->
    <div v-if="toastMessage" class="toast-popup">
      {{ toastMessage }}
    </div>
  </div>
</template>

<style scoped>
.ytd-container {
  width: 100%;
  min-height: 100vh;
  background-color: #f9fafb;
  color: #111827;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  padding: 30px 24px 60px;
  box-sizing: border-box;
}

.ytd-container.standalone-mode {
  padding: 20px 24px;
}

/* Topbar */
.ytd-topbar {
  max-width: 900px;
  margin: 0 auto 30px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.topbar-left {
  display: flex;
  gap: 12px;
}

.topbar-tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 18px;
  border-radius: 100px;
  border: 1px solid #e5e7eb;
  background: #ffffff;
  color: #4b5563;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.topbar-tab-btn:hover {
  background: #f3f4f6;
  color: #111827;
}

.topbar-tab-btn.active {
  background: #111827;
  color: #ffffff;
  border-color: #111827;
}

.badge-count {
  background: #e0382b;
  color: #fff;
  font-size: 11px;
  font-weight: 700;
  padding: 2px 7px;
  border-radius: 10px;
}

.action-pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  background: #ffffff;
  color: #374151;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.action-pill-btn:hover {
  background: #f3f4f6;
  border-color: #d1d5db;
}

/* Parser Wrapper */
.parser-wrapper {
  max-width: 860px;
  margin: 0 auto;
}

.main-heading {
  text-align: center;
  font-size: 32px;
  font-weight: 800;
  color: #111827;
  letter-spacing: -0.5px;
  margin-bottom: 28px;
}

/* Giant Search Input Box (Matching Image 1 & Image 2) */
.search-input-box {
  background: #ffffff;
  border: 2.5px solid #111827;
  border-radius: 12px;
  padding: 6px 6px 6px 18px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.04);
  transition: all 0.2s ease;
}

.search-input-box:focus-within {
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.08);
}

.search-input-box.has-error {
  border-color: #ef4444;
}

.input-inner {
  display: flex;
  align-items: center;
  flex: 1;
  margin-right: 12px;
}

.video-url-input {
  width: 100%;
  border: none;
  outline: none;
  font-size: 16px;
  color: #111827;
  background: transparent;
  padding: 8px 0;
}

.video-url-input::placeholder {
  color: #9ca3af;
}

.paste-btn, .clear-btn {
  background: none;
  border: none;
  color: #6b7280;
  font-size: 13px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  white-space: nowrap;
}

.paste-btn:hover, .clear-btn:hover {
  background: #f3f4f6;
  color: #111827;
}

.download-trigger-btn {
  background-color: #d32f2f;
  color: #ffffff;
  border: none;
  border-radius: 8px;
  padding: 12px 34px;
  font-size: 16px;
  font-weight: 700;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  white-space: nowrap;
}

.download-trigger-btn:hover:not(:disabled) {
  background-color: #b71c1c;
  transform: translateY(-1px);
}

.download-trigger-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.error-banner {
  margin-top: 14px;
  padding: 10px 16px;
  background-color: #fee2e2;
  border-left: 4px solid #ef4444;
  border-radius: 6px;
  color: #b91c1c;
  font-size: 14px;
}

/* Uninspected State (Matching Image 1) */
.uninspected-state {
  margin-top: 14px;
  text-align: center;
}

.copy-hint-text {
  font-size: 15px;
  color: #6b7280;
  margin-bottom: 40px;
}

.supported-heading {
  font-size: 24px;
  font-weight: 700;
  color: #111827;
  margin-bottom: 24px;
}

.supported-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  max-width: 620px;
  margin: 0 auto;
}

.site-card {
  background-color: #f1f3f5;
  border-radius: 8px;
  padding: 14px 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
}

.site-card:hover {
  background-color: #e5e7eb;
  transform: translateY(-2px);
}

.site-icon-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
}

.site-name-text {
  font-size: 15px;
  font-weight: 700;
  color: #111827;
}

.yt-font {
  font-family: 'YouTube Sans', Roboto, sans-serif;
  letter-spacing: -0.5px;
}

.vimeo-font {
  font-style: italic;
  font-weight: 800;
}

.ted-logo-text {
  color: #e62b1e;
  font-size: 20px;
  font-weight: 900;
  letter-spacing: 0.5px;
}

.onlyfans-font {
  color: #00aff0;
  font-weight: 700;
}

.dm-font {
  font-weight: 700;
}

.rumble-font {
  color: #111827;
  font-weight: 700;
}

/* Skeleton Loading */
.inspecting-skeleton {
  margin-top: 40px;
  background: #ffffff;
  border-radius: 16px;
  padding: 48px 24px;
  text-align: center;
  border: 1px solid #e5e7eb;
}

.skeleton-spinner-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
}

.spinner-large {
  width: 40px;
  height: 40px;
  border: 4px solid #f3f4f6;
  border-top-color: #d32f2f;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.skeleton-text {
  font-size: 16px;
  font-weight: 600;
  color: #111827;
}

.skeleton-sub {
  font-size: 13px;
  color: #6b7280;
}

/* Result Card (Matching Image 2) */
.result-card {
  margin-top: 36px;
  background: #ffffff;
  border-radius: 16px;
  border: 1px solid #eef2f6;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.05);
  padding: 28px;
  display: grid;
  grid-template-columns: 260px 1fr;
  gap: 32px;
}

/* Left Column */
.result-left-col {
  display: flex;
  flex-direction: column;
}

.thumb-container {
  width: 100%;
  aspect-ratio: 16/9;
  border-radius: 10px;
  overflow: hidden;
  background-color: #e5e7eb;
}

.video-thumbnail {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.thumb-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.video-display-title {
  margin-top: 12px;
  font-size: 13.5px;
  font-weight: 500;
  line-height: 1.5;
  color: #4b5563;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.video-uploader-tag {
  margin-top: 8px;
  font-size: 12px;
  color: #9ca3af;
}

/* Right Column */
.result-right-col {
  display: flex;
  flex-direction: column;
}

.format-section {
  display: flex;
  flex-direction: column;
}

.format-heading {
  font-size: 19px;
  font-weight: 700;
  color: #111827;
  margin-bottom: 14px;
}

.format-options-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  row-gap: 12px;
  column-gap: 24px;
}

.radio-option-item {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  user-select: none;
  font-size: 14.5px;
  color: #4b5563;
  transition: color 0.15s ease;
}

.radio-option-item:hover {
  color: #111827;
}

.radio-option-item.active {
  color: #d32f2f;
  font-weight: 600;
}

.native-radio-hidden {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

/* Custom Radio Circle (Matching Red Circle Dot in Image 2) */
.custom-radio-circle {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 1.8px solid #d1d5db;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background-color: #ffffff;
  transition: all 0.15s ease;
}

.radio-option-item.active .custom-radio-circle {
  border-color: #d32f2f;
}

.radio-option-item.active .custom-radio-circle::after {
  content: '';
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background-color: #d32f2f;
}

.format-divider {
  height: 1px;
  background-color: #e5e7eb;
  margin: 22px 0 18px;
}

/* Action row */
.download-action-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 26px;
}

.giant-download-btn {
  background-color: #d32f2f;
  color: #ffffff;
  border: none;
  border-radius: 6px;
  padding: 12px 34px;
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 0.6px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  box-shadow: 0 4px 12px rgba(211, 47, 47, 0.25);
}

.giant-download-btn:hover:not(:disabled) {
  background-color: #b71c1c;
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(211, 47, 47, 0.35);
}

.giant-download-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Task Management View */
.tasks-wrapper {
  max-width: 860px;
  margin: 0 auto;
}

.tasks-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.tasks-filter-group {
  display: flex;
  gap: 8px;
}

.filter-pill {
  padding: 6px 14px;
  border-radius: 20px;
  border: 1px solid #e5e7eb;
  background: #ffffff;
  color: #4b5563;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.filter-pill.active {
  background: #111827;
  color: #ffffff;
  border-color: #111827;
}

.clear-tasks-btn {
  background: none;
  border: none;
  color: #6b7280;
  font-size: 13px;
  cursor: pointer;
}

.clear-tasks-btn:hover {
  color: #ef4444;
}

.empty-tasks-box {
  background: #ffffff;
  border-radius: 14px;
  padding: 48px;
  text-align: center;
  border: 1px solid #e5e7eb;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: #6b7280;
}

.return-parser-btn {
  margin-top: 8px;
  padding: 8px 18px;
  border-radius: 8px;
  background: #d32f2f;
  color: #ffffff;
  border: none;
  font-weight: 600;
  cursor: pointer;
}

.tasks-card-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.task-item-card {
  background: #ffffff;
  border: 1px solid #e5e7eb;
  border-radius: 12px;
  padding: 16px;
  display: grid;
  grid-template-columns: 80px 1fr auto;
  gap: 16px;
  align-items: center;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.03);
}

.task-thumb-box {
  width: 80px;
  height: 52px;
  border-radius: 6px;
  overflow: hidden;
  background: #e5e7eb;
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
  font-size: 20px;
}

.task-card-center {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.task-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.task-title {
  font-size: 14px;
  font-weight: 600;
  color: #111827;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 480px;
}

.task-status-pill {
  font-size: 11px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 4px;
  text-transform: uppercase;
}

.task-status-pill.downloading {
  background: #dbeafe;
  color: #1d4ed8;
}

.task-status-pill.completed {
  background: #dcfce7;
  color: #15803d;
}

.task-status-pill.paused {
  background: #fef3c7;
  color: #b45309;
}

.task-status-pill.failed, .task-status-pill.error, .task-status-pill.cancelled {
  background: #fee2e2;
  color: #b91c1c;
}

.progress-track {
  width: 100%;
  height: 6px;
  background: #f3f4f6;
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #2563eb;
  transition: width 0.3s ease;
}

.progress-fill.completed {
  background: #16a34a;
}

.progress-fill.paused {
  background: #d97706;
}

.progress-fill.failed, .progress-fill.error {
  background: #dc2626;
}

.task-error-tip {
  font-size: 11px;
  color: #dc2626;
  background: #fef2f2;
  border: 1px solid #fee2e2;
  border-radius: 4px;
  padding: 4px 8px;
  margin-top: 4px;
  word-break: break-all;
}

.task-meta-row {
  display: flex;
  gap: 14px;
  font-size: 12px;
  color: #6b7280;
}

.task-card-actions {
  display: flex;
  gap: 8px;
}

.task-btn {
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid #e5e7eb;
  background: #ffffff;
  color: #374151;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.task-btn:hover {
  background: #f3f4f6;
}

.task-btn.success {
  background: #16a34a;
  color: #ffffff;
  border-color: #16a34a;
}

.task-btn.danger {
  color: #dc2626;
}

.task-btn.retry {
  background: #ef4444;
  color: #ffffff;
  border-color: #ef4444;
}

.task-btn.retry:hover {
  background: #dc2626;
}

/* Spinner */
.spinner-small {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Toast */
.toast-popup {
  position: fixed;
  bottom: 30px;
  left: 50%;
  transform: translateX(-50%);
  background: #111827;
  color: #ffffff;
  padding: 10px 22px;
  border-radius: 100px;
  font-size: 13.5px;
  font-weight: 500;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
  z-index: 1000;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translate(-50%, 10px); }
  to { opacity: 1; transform: translate(-50%, 0); }
}

@media (max-width: 768px) {
  .result-card {
    grid-template-columns: 1fr;
  }
  .supported-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
