<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { t } from '../i18n';
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
let unlistenPickerUrl: UnlistenFn | null = null;
let unlistenPickerTask: UnlistenFn | null = null;
const thumbMap = ref<Record<string, string>>({});

export interface SupportedSiteItem {
  id: string;
  name: string;
  category: 'mainstream' | 'shorts' | 'social' | 'musicKnowledge';
  homeUrl: string;
  exampleUrl?: string;
  tagKey: string;
  color: string;
  iconText: string;
  descKey: string;
}

const siteCategoryDefs = [
  { key: 'catAll', id: 'all' },
  { key: 'catMainstream', id: 'mainstream' },
  { key: 'catShorts', id: 'shorts' },
  { key: 'catSocial', id: 'social' },
  { key: 'catMusicKnowledge', id: 'musicKnowledge' },
] as const;

type SiteCategory = typeof siteCategoryDefs[number]['id'];
const selectedSiteCategory = ref<SiteCategory>('all');
const siteSearchQuery = ref<string>('');

// Popular supported video platforms (yt-dlp powered)
const supportedSites: SupportedSiteItem[] = [
  {
    id: 'youtube',
    name: 'YouTube',
    category: 'mainstream',
    homeUrl: 'https://www.youtube.com',
    exampleUrl: 'https://www.youtube.com/watch?v=rRK8KP4Y68g',
    tagKey: 'siteTagYoutube',
    color: '#ff0000',
    iconText: '▶',
    descKey: 'siteDescYoutube',
  },
  {
    id: 'tiktok',
    name: 'TikTok',
    category: 'shorts',
    homeUrl: 'https://www.tiktok.com',
    exampleUrl: 'https://www.tiktok.com/@tiktok/video/7106594312292453678',
    tagKey: 'siteTagTiktok',
    color: '#fe2c55',
    iconText: '🎵',
    descKey: 'siteDescTiktok',
  },
  {
    id: 'twitter',
    name: 'Twitter / X',
    category: 'social',
    homeUrl: 'https://x.com',
    exampleUrl: 'https://x.com/NASA/status/1700000000000000000',
    tagKey: 'siteTagTwitter',
    color: '#000000',
    iconText: '𝕏',
    descKey: 'siteDescTwitter',
  },
  {
    id: 'instagram',
    name: 'Instagram',
    category: 'social',
    homeUrl: 'https://www.instagram.com',
    exampleUrl: 'https://www.instagram.com/reel/C3XXXXX/',
    tagKey: 'siteTagInstagram',
    color: '#e1306c',
    iconText: '📷',
    descKey: 'siteDescInstagram',
  },
  {
    id: 'vimeo',
    name: 'Vimeo',
    category: 'mainstream',
    homeUrl: 'https://vimeo.com',
    exampleUrl: 'https://vimeo.com/76979871',
    tagKey: 'siteTagVimeo',
    color: '#1ab7ea',
    iconText: '🎬',
    descKey: 'siteDescVimeo',
  },
  {
    id: 'reddit',
    name: 'Reddit',
    category: 'social',
    homeUrl: 'https://www.reddit.com',
    exampleUrl: 'https://www.reddit.com/r/videos/comments/xyz',
    tagKey: 'siteTagReddit',
    color: '#ff4500',
    iconText: '🤖',
    descKey: 'siteDescReddit',
  },
  {
    id: 'twitch',
    name: 'Twitch',
    category: 'mainstream',
    homeUrl: 'https://www.twitch.tv',
    exampleUrl: 'https://www.twitch.tv/videos/1234567890',
    tagKey: 'siteTagTwitch',
    color: '#9146ff',
    iconText: '🎮',
    descKey: 'siteDescTwitch',
  },
  {
    id: 'facebook',
    name: 'Facebook',
    category: 'social',
    homeUrl: 'https://www.facebook.com',
    exampleUrl: 'https://www.facebook.com/watch/?v=123',
    tagKey: 'siteTagFacebook',
    color: '#1877f2',
    iconText: '👥',
    descKey: 'siteDescFacebook',
  },
  {
    id: 'dailymotion',
    name: 'DailyMotion',
    category: 'mainstream',
    homeUrl: 'https://www.dailymotion.com',
    exampleUrl: 'https://www.dailymotion.com/video/x7tgad0',
    tagKey: 'siteTagDailymotion',
    color: '#0066dc',
    iconText: '🎥',
    descKey: 'siteDescDailymotion',
  },
  {
    id: 'ted',
    name: 'TED Talks',
    category: 'musicKnowledge',
    homeUrl: 'https://www.ted.com',
    exampleUrl: 'https://www.ted.com/talks/al_gore_the_case_for_optimism_on_climate_change',
    tagKey: 'siteTagTed',
    color: '#e62b1e',
    iconText: '💡',
    descKey: 'siteDescTed',
  },
  {
    id: 'soundcloud',
    name: 'SoundCloud',
    category: 'musicKnowledge',
    homeUrl: 'https://soundcloud.com',
    exampleUrl: 'https://soundcloud.com/user/track',
    tagKey: 'siteTagSoundcloud',
    color: '#ff5500',
    iconText: '🎧',
    descKey: 'siteDescSoundcloud',
  },
  {
    id: 'rumble',
    name: 'Rumble',
    category: 'mainstream',
    homeUrl: 'https://rumble.com',
    exampleUrl: 'https://rumble.com/v20847w-top-news.html',
    tagKey: 'siteTagRumble',
    color: '#85c742',
    iconText: '🥊',
    descKey: 'siteDescRumble',
  },
];

const filteredSites = computed(() => {
  let list = supportedSites;
  if (selectedSiteCategory.value !== 'all') {
    list = list.filter((s) => s.category === selectedSiteCategory.value);
  }
  if (siteSearchQuery.value.trim()) {
    const q = siteSearchQuery.value.trim().toLowerCase();
    list = list.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        t('downloads.' + s.descKey).toLowerCase().includes(q) ||
        t('downloads.' + s.tagKey).toLowerCase().includes(q)
    );
  }
  return list;
});

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
    // Find if specific format exists in inspect results (prefer direct non-HLS streams)
    const matchedFmt = formats.find((f) => {
      const res = f.resolution || '';
      const note = f.note || '';
      const isM3u8 = note.toLowerCase().includes('m3u8') || note.toLowerCase().includes('hls');
      return !isM3u8 && (res.includes(`${tier.height}`) || note.includes(`${tier.height}p`));
    }) || formats.find((f) => {
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
      ? `${matchedFmt.format_id}+bestaudio/bestvideo[height<=${tier.height}][protocol!=m3u8_native][protocol!=m3u8]+bestaudio/bestvideo[height<=${tier.height}]+bestaudio/best[height<=${tier.height}]/best`
      : `bestvideo[height<=${tier.height}][protocol!=m3u8_native][protocol!=m3u8]+bestaudio/bestvideo[height<=${tier.height}]+bestaudio/best[height<=${tier.height}]/best`;

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
      formatId: `bestaudio[abr<=${tier.kbps}][protocol!=m3u8_native][protocol!=m3u8]/bestaudio[abr<=${tier.kbps}]/bestaudio/best`,
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
    inspectError.value = t('downloads.pasteLinkPrompt');
    return;
  }
  inspectError.value = '';
  isInspecting.value = true;
  inspectResult.value = null;

  try {
    const res = await invoke<UrlInspectResult>('inspect_url', { url });
    inspectResult.value = res;
    if (res.thumbnail) {
      thumbMap.value[res.url] = res.thumbnail;
    }
    // Default to 1080P or the first video option
    if (videoOptions.value.length > 0) {
      selectedOptionId.value = videoOptions.value[0].id;
    }
    showToast(t('downloads.inspectSuccess'));
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    inspectError.value = t('downloads.inspectFailed', { msg });
  } finally {
    isInspecting.value = false;
  }
}

// Start download of the selected format
const isStartingDownload = ref<boolean>(false);
async function startSelectedDownload() {
  if (!inspectResult.value) return;
  const targetOption = currentSelectedOption.value;
  const formatId = targetOption ? targetOption.formatId : 'bestvideo[protocol!=m3u8_native][protocol!=m3u8]+bestaudio/bestvideo+bestaudio/best';

  isStartingDownload.value = true;
  try {
    if (inspectResult.value.thumbnail) {
      thumbMap.value[inspectResult.value.url] = inspectResult.value.thumbnail;
    }
    await invoke<DownloadTask>('start_download', {
      url: inspectResult.value.url,
      format_id: formatId,
      formatId: formatId,
      title: inspectResult.value.title || undefined,
      author: inspectResult.value.uploader || undefined,
    });
    showToast(t('downloads.downloadStarted', { label: targetOption?.label || t('downloads.bestQuality') }));
    await fetchTasks();
    scheduleNextPoll();
    // Switch to task view or keep in parser with toast
    currentView.value = 'tasks';
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    showToast(t('downloads.downloadStartFailed', { msg }));
  } finally {
    isStartingDownload.value = false;
  }
}

// Open dedicated Webview Video Sniffer / Picker
async function openWebviewPicker(url?: string) {
  try {
    const targetUrl = url || inputUrl.value.trim() || 'https://www.youtube.com';
    await invoke('open_video_picker_window', { initialUrl: targetUrl });
    const siteObj = supportedSites.find((s) =>
      targetUrl.includes(s.homeUrl.replace('https://', '').replace('www.', ''))
    );
    const siteName = siteObj ? siteObj.name : t('downloads.defaultSiteName');
    showToast(t('downloads.browserSnifferOpened', { site: siteName }));
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    showToast(t('downloads.browserSnifferOpenFailed', { msg }));
  }
}

function openSiteInWebview(site: SupportedSiteItem) {
  openWebviewPicker(site.homeUrl);
}

function fillExampleUrl(site: SupportedSiteItem) {
  if (site.exampleUrl) {
    inputUrl.value = site.exampleUrl;
    inspectVideoUrl();
    showToast(t('downloads.sampleUrlFilled', { site: site.name }));
  } else {
    showToast(t('downloads.copyLinkInstruction', { site: site.name }));
  }
}

async function copySiteUrl(site: SupportedSiteItem) {
  try {
    await navigator.clipboard.writeText(site.homeUrl);
    showToast(t('downloads.siteUrlCopied', { site: site.name }));
  } catch {
    showToast(t('downloads.copyFailed'));
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
    showToast(t('downloads.openWindowFailed', { err: String(err) }));
  }
}

// Task control operations
async function pauseTask(task: DownloadTask) {
  try {
    await invoke('pause_download_task', { task_id: task.id, taskId: task.id, id: task.id });
    showToast(t('downloads.taskPaused'));
    await fetchTasks();
  } catch (err: unknown) {
    showToast(t('downloads.pauseFailed', { err: String(err) }));
  }
}

async function resumeTask(task: DownloadTask) {
  try {
    await invoke('resume_download_task', { task_id: task.id, taskId: task.id, id: task.id });
    showToast(t('downloads.taskResumed'));
    await fetchTasks();
    scheduleNextPoll();
  } catch (err: unknown) {
    showToast(t('downloads.resumeFailed', { err: String(err) }));
  }
}

async function cancelTask(task: DownloadTask) {
  if (!confirm(t('downloads.confirmCancel', { title: task.title || '' }))) return;
  try {
    await invoke('cancel_download_task', { task_id: task.id, taskId: task.id, id: task.id });
    showToast(t('downloads.taskCancelled'));
    await fetchTasks();
  } catch (err: unknown) {
    showToast(t('downloads.cancelFailed', { err: String(err) }));
  }
}

async function clearCompleted() {
  if (completedTasks.value.length === 0) return;
  try {
    await invoke('clear_completed_tasks');
    await fetchTasks();
    showToast(t('downloads.clearedCompleted'));
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

function getTaskThumbnail(task: DownloadTask): string | undefined {
  if (task.thumbnail) return task.thumbnail;
  if (thumbMap.value[task.url]) return thumbMap.value[task.url];
  if (inspectResult.value && inspectResult.value.url === task.url && inspectResult.value.thumbnail) {
    return inspectResult.value.thumbnail;
  }
  return undefined;
}

async function retryTask(task: DownloadTask) {
  try {
    showToast(t('downloads.retryingDownload', { title: task.title || t('downloads.defaultVideoTitle') }));
    await invoke('start_download', {
      url: task.url,
      format_id: task.format_id || undefined,
      formatId: task.format_id || undefined,
      title: task.title || undefined,
      author: task.author || undefined,
    });
    currentView.value = 'tasks';
    activeFilter.value = 'active';
    await fetchTasks();
    scheduleNextPoll();
  } catch (err: unknown) {
    showToast(t('downloads.retryFailed', { err: err instanceof Error ? err.message : String(err) }));
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
    showToast(t('downloads.pasteShortcutHint'));
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

function scheduleNextPoll() {
  if (pollTimer) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
  // 智能降频轮询：有正在下载/排队任务时以 1.5s 刷新；空闲时降频为 8s，释放主进程 IPC
  const interval = activeTasks.value.length > 0 ? 1500 : 8000;
  pollTimer = window.setTimeout(async () => {
    await fetchTasks();
    scheduleNextPoll();
  }, interval);
}

onMounted(async () => {
  await fetchTasks();
  scheduleNextPoll();

  try {
    unlistenPickerUrl = await listen<string>('picker-select-url', (event) => {
      if (event.payload) {
        inputUrl.value = event.payload;
        currentView.value = 'parser';
        inspectVideoUrl();
        showToast(t('downloads.snifferLoadedUrl'));
      }
    });

    unlistenPickerTask = await listen<DownloadTask>('picker-task-started', async () => {
      showToast(t('downloads.snifferTaskAdded'));
      await fetchTasks();
      scheduleNextPoll();
    });
  } catch (err) {
    console.warn('Failed to listen to picker events:', err);
  }
});

onUnmounted(() => {
  if (pollTimer) clearTimeout(pollTimer);
  if (toastTimer) clearTimeout(toastTimer);
  if (unlistenPickerUrl) {
    unlistenPickerUrl();
    unlistenPickerUrl = null;
  }
  if (unlistenPickerTask) {
    unlistenPickerTask();
    unlistenPickerTask = null;
  }
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
          <span>{{ t('downloads.tabParser') }}</span>
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
          <span>{{ t('downloads.tabTasks') }}</span>
          <span v-if="activeTasks.length > 0" class="badge-count">{{ activeTasks.length }}</span>
        </button>
      </div>

      <div class="topbar-right">
        <button
          class="action-pill-btn browser-picker-btn"
          :title="t('downloads.browserPickerTitle')"
          @click="openWebviewPicker()"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="2" y1="12" x2="22" y2="12"></line>
            <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
          </svg>
          <span>{{ t('downloads.browserPickerBtn') }}</span>
        </button>

        <button
          v-if="!props.isStandaloneWindow"
          class="action-pill-btn"
          :title="t('downloads.standaloneTitle')"
          @click="openInStandaloneWindow"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
            <polyline points="15 3 21 3 21 9"></polyline>
            <line x1="10" y1="14" x2="21" y2="3"></line>
          </svg>
          <span>{{ t('downloads.standaloneBtn') }}</span>
        </button>
      </div>
    </div>

    <!-- MAIN PARSER VIEW (Matching Image 1 & Image 2) -->
    <div v-show="currentView === 'parser'" class="parser-wrapper">
      <!-- Title -->
      <h1 class="main-heading">{{ t('downloads.titleHeading') }}</h1>

      <!-- Giant Input Card (Matching Image 1 & Image 2) -->
      <div class="search-input-box" :class="{ 'has-error': !!inspectError }">
        <div class="input-inner">
          <input
            v-model="inputUrl"
            type="text"
            class="video-url-input"
            :placeholder="t('downloads.urlInputPlaceholder')"
            @keydown.enter="inspectVideoUrl"
          />
          <button
            v-if="!inputUrl"
            class="paste-btn"
            :title="t('downloads.btnPaste')"
            @click="pasteFromClipboard"
          >
            {{ t('downloads.btnPaste') }}
          </button>
          <button
            v-else
            class="clear-btn"
            :title="t('common.cancel')"
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
          <span>{{ isInspecting ? t('downloads.btnInspecting') : t('downloads.btnDownload') }}</span>
        </button>
      </div>

      <!-- Error alert -->
      <div v-if="inspectError" class="error-banner">
        <span>⚠️ {{ inspectError }}</span>
      </div>

      <!-- STATE 1: Not Inspected Yet -> Show Supported Sites (Image 1) -->
      <div v-if="!inspectResult && !isInspecting" class="uninspected-state">
        <!-- Quick Webview Picker Banner -->
        <div class="quick-picker-banner" @click="openWebviewPicker()">
          <div class="picker-banner-left">
            <div class="picker-banner-icon">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <polygon points="10 8 16 12 10 16 10 8" fill="currentColor"></polygon>
              </svg>
            </div>
            <div class="picker-banner-info">
              <div class="picker-banner-title">{{ t('downloads.quickPickerBannerTitle') }}</div>
              <div class="picker-banner-sub">{{ t('downloads.quickPickerBannerSub') }}</div>
            </div>
          </div>
          <button class="picker-banner-action" type="button">
            <span>{{ t('downloads.quickPickerBrowseNow') }}</span>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
          </button>
        </div>

        <div class="sites-header-bar">
          <div class="sites-title-group">
            <h2 class="supported-heading">{{ t('downloads.popularHeading') }}</h2>
            <span class="sites-count-badge">{{ t('downloads.popularSub', { count: supportedSites.length }) }}</span>
          </div>
          <div class="site-search-wrap">
            <input
              v-model="siteSearchQuery"
              class="site-search-input"
              :placeholder="t('downloads.searchSitePlaceholder')"
            />
          </div>
        </div>

        <!-- Category Filter Pills -->
        <div class="site-category-pills">
          <button
            v-for="cat in siteCategoryDefs"
            :key="cat.key"
            class="site-cat-pill"
            :class="{ active: selectedSiteCategory === cat.id }"
            @click="selectedSiteCategory = cat.id"
          >
            {{ t('downloads.' + cat.key) }}
          </button>
        </div>

        <!-- Supported Sites Grid -->
        <div class="supported-grid">
          <div
            v-for="site in filteredSites"
            :key="site.id"
            class="site-card"
          >
            <div class="site-card-top" @click="openSiteInWebview(site)">
              <div class="site-icon-wrapper" :style="{ backgroundColor: site.color }">
                <span class="site-icon-glyph">{{ site.iconText }}</span>
              </div>
              <div class="site-card-meta">
                <div class="site-name-row">
                  <span class="site-name-text">{{ site.name }}</span>
                  <span class="site-tag-badge">{{ t('downloads.' + site.tagKey) }}</span>
                </div>
                <div class="site-desc-text" :title="t('downloads.' + site.descKey)">{{ t('downloads.' + site.descKey) }}</div>
              </div>
            </div>

            <div class="site-card-footer">
              <button
                class="btn-site-act btn-open-picker"
                :title="t('downloads.browserPickerTitle')"
                @click.stop="openSiteInWebview(site)"
              >
                {{ t('downloads.siteActFind') }}
              </button>
              <button
                v-if="site.exampleUrl"
                class="btn-site-act btn-fill-example"
                @click.stop="fillExampleUrl(site)"
              >
                {{ t('downloads.siteActExample') }}
              </button>
              <button
                class="btn-site-act btn-copy-link"
                @click.stop="copySiteUrl(site)"
              >
                🔗
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- STATE 2: Parsing in Progress Skeleton -->
      <div v-if="isInspecting" class="inspecting-skeleton">
        <div class="skeleton-spinner-box">
          <div class="spinner-large"></div>
          <p class="skeleton-text">{{ t('downloads.inspectingTitle') }}</p>
          <span class="skeleton-sub">{{ t('downloads.inspectingSub') }}</span>
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
            <h4 class="format-heading">{{ t('downloads.videoHeading') }}</h4>

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
            <h4 class="format-heading">{{ t('downloads.musicHeading') }}</h4>

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
              <span>{{ t('downloads.giantDownload') }}</span>
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
            {{ t('downloads.tasksFilterAll', { count: tasks.length }) }}
          </button>
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'active' }"
            @click="activeFilter = 'active'"
          >
            {{ t('downloads.tasksFilterActive', { count: activeTasks.length }) }}
          </button>
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'completed' }"
            @click="activeFilter = 'completed'"
          >
            {{ t('downloads.tasksFilterCompleted', { count: completedTasks.length }) }}
          </button>
          <button
            class="filter-pill"
            :class="{ active: activeFilter === 'failed' }"
            @click="activeFilter = 'failed'"
          >
            {{ t('downloads.tasksFilterFailed', { count: failedTasks.length }) }}
          </button>
        </div>

        <div class="tasks-action-right">
          <button
            v-if="completedTasks.length > 0"
            class="clear-tasks-btn"
            @click="clearCompleted"
          >
            {{ t('downloads.clearCompleted') }}
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
        <p>{{ t('downloads.emptyTasks') }}</p>
        <button class="return-parser-btn" @click="currentView = 'parser'">
          {{ t('downloads.addNewDownload') }}
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
                v-if="getTaskThumbnail(task)"
                :src="getTaskThumbnail(task)"
                class="task-thumb"
              />
              <div v-else class="task-thumb-placeholder">🎬</div>
            </div>
          </div>

          <div class="task-card-center">
            <div class="task-title-row">
              <div class="task-title-wrapper">
                <span class="task-title" :title="task.title || task.url">{{ task.title || task.url }}</span>
                <span v-if="task.author" class="task-author-badge">{{ task.author }}</span>
              </div>
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
              <span v-if="task.eta || task.eta_str">⏳ {{ t('downloads.remaining', { eta: task.eta || task.eta_str || '' }) }}</span>
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
              :title="t('downloads.btnRetry')"
              @click="retryTask(task)"
            >
              {{ t('downloads.btnRetry') }}
            </button>
            <button
              v-if="task.status === 'downloading'"
              class="task-btn"
              :title="t('downloads.btnPause')"
              @click="pauseTask(task)"
            >
              ⏸️
            </button>
            <button
              v-else-if="task.status === 'paused'"
              class="task-btn"
              :title="t('downloads.btnResume')"
              @click="resumeTask(task)"
            >
              ▶️
            </button>
            <button
              v-if="task.status === 'completed'"
              class="task-btn success"
              :title="t('downloads.btnViewLibrary')"
              @click="viewInLibrary(task)"
            >
              {{ t('downloads.btnViewLibrary') }}
            </button>
            <button
              v-if="task.status !== 'completed'"
              class="task-btn danger"
              :title="t('downloads.btnCancel')"
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
  height: 100%;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  background-color: #f9fafb;
  color: #111827;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  padding: 30px 24px 60px;
  box-sizing: border-box;
  scroll-behavior: smooth;
}

.ytd-container::-webkit-scrollbar {
  width: 8px;
}

.ytd-container::-webkit-scrollbar-track {
  background: transparent;
}

.ytd-container::-webkit-scrollbar-thumb {
  background: #d1d5db;
  border-radius: 4px;
}

.ytd-container::-webkit-scrollbar-thumb:hover {
  background: #9ca3af;
}

.ytd-container.standalone-mode {
  height: 100vh;
  padding: 20px 24px 60px;
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

.browser-picker-btn {
  background: linear-gradient(135deg, #fef2f2, #fff1f2);
  border-color: #fecaca;
  color: #dc2626;
  font-weight: 600;
}

.browser-picker-btn:hover {
  background: linear-gradient(135deg, #fee2e2, #fecdd3);
  border-color: #fca5a5;
  color: #b91c1c;
  transform: translateY(-1px);
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

/* Quick Webview Picker Banner */
.quick-picker-banner {
  max-width: 620px;
  margin: 10px auto 26px;
  background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  padding: 14px 18px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  cursor: pointer;
  box-shadow: 0 4px 18px rgba(15, 23, 42, 0.12);
  transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  text-align: left;
}

.quick-picker-banner:hover {
  transform: translateY(-2px);
  border-color: rgba(239, 68, 68, 0.4);
  box-shadow: 0 8px 24px rgba(239, 68, 68, 0.18);
}

.picker-banner-left {
  display: flex;
  align-items: center;
  gap: 14px;
}

.picker-banner-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.35);
}

.picker-banner-info {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.picker-banner-title {
  font-size: 14.5px;
  font-weight: 700;
  color: #f8fafc;
  letter-spacing: -0.2px;
}

.picker-banner-sub {
  font-size: 12px;
  color: #94a3b8;
  line-height: 1.4;
}

.picker-banner-action {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 7px 14px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.12);
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.15);
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: all 0.2s ease;
}

.quick-picker-banner:hover .picker-banner-action {
  background: #ef4444;
  border-color: #ef4444;
}

.sites-header-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  max-width: 960px;
  margin: 0 auto 16px;
  flex-wrap: wrap;
  gap: 12px;
}

.sites-title-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
  text-align: left;
}

.supported-heading {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary, #111827);
  margin: 0;
}

.sites-count-badge {
  font-size: 12px;
  color: var(--text-muted, #64748b);
}

.site-search-wrap {
  width: 240px;
}

.site-search-input {
  width: 100%;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border-subtle, #e2e8f0);
  background: var(--bg-input, #ffffff);
  color: var(--text-primary, #0f172a);
  font-size: 12px;
  outline: none;
  transition: border-color 0.15s;
}

.site-search-input:focus {
  border-color: #3b82f6;
}

.site-category-pills {
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: 960px;
  margin: 0 auto 18px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.site-cat-pill {
  padding: 5px 14px;
  border-radius: 20px;
  background: var(--bg-input, #f1f5f9);
  border: 1px solid var(--border-subtle, #e2e8f0);
  color: var(--text-secondary, #475569);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.site-cat-pill:hover {
  background: var(--btn-hover, #e2e8f0);
  color: var(--text-primary, #0f172a);
}

.site-cat-pill.active {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #ffffff;
  font-weight: 600;
  box-shadow: 0 2px 8px rgba(59, 130, 246, 0.3);
}

.supported-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 14px;
  max-width: 960px;
  margin: 0 auto;
}

.site-card {
  background: var(--bg-card, #ffffff);
  border: 1px solid var(--border-subtle, #e2e8f0);
  border-radius: 12px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 12px;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  text-align: left;
}

.site-card:hover {
  transform: translateY(-2px);
  border-color: rgba(59, 130, 246, 0.4);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
}

.site-card-top {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  cursor: pointer;
}

.site-icon-wrapper {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.15);
}

.site-icon-glyph {
  font-size: 20px;
  color: #ffffff;
  font-weight: bold;
}

.site-card-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.site-name-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.site-name-text {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary, #0f172a);
}

.site-tag-badge {
  font-size: 10px;
  font-weight: 600;
  color: #3b82f6;
  background: rgba(59, 130, 246, 0.08);
  border: 1px solid rgba(59, 130, 246, 0.2);
  padding: 1px 6px;
  border-radius: 4px;
  white-space: nowrap;
}

.site-desc-text {
  font-size: 11.5px;
  color: var(--text-muted, #64748b);
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.site-card-footer {
  display: flex;
  align-items: center;
  gap: 6px;
  border-top: 1px solid var(--border-subtle, #f1f5f9);
  padding-top: 10px;
}

.btn-site-act {
  padding: 5px 10px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s ease;
}

.btn-open-picker {
  flex: 1;
  background: var(--bg-input, #f1f5f9);
  color: var(--text-primary, #0f172a);
  border-color: var(--border-subtle, #e2e8f0);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.btn-open-picker:hover {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #ffffff;
}

.btn-fill-example {
  background: transparent;
  color: var(--text-muted, #64748b);
  border-color: var(--border-subtle, #e2e8f0);
}

.btn-fill-example:hover {
  background: var(--btn-hover, #e2e8f0);
  color: var(--text-primary, #0f172a);
}

.btn-copy-link {
  background: transparent;
  color: var(--text-muted, #64748b);
  border-color: var(--border-subtle, #e2e8f0);
  padding: 5px 8px;
}

.btn-copy-link:hover {
  background: var(--btn-hover, #e2e8f0);
  color: var(--text-primary, #0f172a);
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

.task-title-wrapper {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
}

.task-title {
  font-size: 14px;
  font-weight: 600;
  color: #111827;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 420px;
}

.task-author-badge {
  font-size: 11px;
  color: #6b7280;
  background: #f3f4f6;
  padding: 1px 6px;
  border-radius: 4px;
  white-space: nowrap;
  flex-shrink: 0;
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
