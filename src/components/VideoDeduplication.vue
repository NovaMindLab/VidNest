<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Video, DuplicateGroup, DuplicateResolutionAction, ResolutionSummary, DedupStrategy } from '../types';
import { t } from '../i18n';

const emit = defineEmits<{
  (e: 'playVideo', video: Video): void;
  (e: 'navigateLibrary'): void;
}>();

const currentStrategy = ref<DedupStrategy>('exact_hash');
const isLoading = ref<boolean>(false);
const isResolving = ref<boolean>(false);
const duplicateGroups = ref<DuplicateGroup[]>([]);
const alsoDeleteDiskFile = ref<boolean>(false);
const showConfirmModal = ref<boolean>(false);
const toastMessage = ref<string>('');
let toastTimer: number | null = null;

// Map of groupId -> keepVideoId
const selections = ref<Record<string, number>>({});

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 3000);
}

async function fetchDuplicates() {
  isLoading.value = true;
  try {
    const groups = await invoke<DuplicateGroup[]>('find_duplicate_videos', {
      strategy: currentStrategy.value,
    });
    duplicateGroups.value = groups;
    applySmartSelect('best_quality');
  } catch (err: unknown) {
    showToast(`查重分析失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    isLoading.value = false;
  }
}

function setStrategy(strat: DedupStrategy) {
  if (currentStrategy.value === strat) return;
  currentStrategy.value = strat;
  fetchDuplicates();
}

// Smart Auto-Select logic
function applySmartSelect(rule: 'best_quality' | 'smallest_size' | 'newest' | 'oldest') {
  const newSelections: Record<string, number> = {};
  for (const group of duplicateGroups.value) {
    if (!group.videos || group.videos.length === 0) continue;
    const sorted = [...group.videos];

    switch (rule) {
      case 'best_quality':
        sorted.sort((a, b) => {
          const resA = (a.width || 0) * (a.height || 0);
          const resB = (b.width || 0) * (b.height || 0);
          if (resB !== resA) return resB - resA;
          return b.file_size - a.file_size;
        });
        break;
      case 'smallest_size':
        sorted.sort((a, b) => a.file_size - b.file_size);
        break;
      case 'newest':
        sorted.sort((a, b) => (b.updated_at || b.created_at) - (a.updated_at || a.created_at));
        break;
      case 'oldest':
        sorted.sort((a, b) => (a.created_at || a.updated_at) - (b.created_at || b.updated_at));
        break;
    }
    newSelections[group.group_id] = sorted[0].id;
  }
  selections.value = newSelections;
}

function setKeepVideo(groupId: string, videoId: number) {
  selections.value = {
    ...selections.value,
    [groupId]: videoId,
  };
}

// Computed statistics
const totalGroups = computed(() => duplicateGroups.value.length);

const totalRedundantVideos = computed(() => {
  let count = 0;
  for (const group of duplicateGroups.value) {
    const keepId = selections.value[group.group_id];
    if (keepId) {
      count += group.videos.filter((v) => v.id !== keepId).length;
    } else {
      count += Math.max(0, group.videos.length - 1);
    }
  }
  return count;
});

const totalReclaimableBytes = computed(() => {
  let bytes = 0;
  for (const group of duplicateGroups.value) {
    const keepId = selections.value[group.group_id];
    for (const v of group.videos) {
      if (v.id !== keepId) {
        bytes += v.file_size;
      }
    }
  }
  return bytes;
});

async function playVideo(video: Video) {
  try {
    await invoke('open_video_player_window', { video });
    showToast(`▶ 正在播放: ${video.file_name}`);
  } catch {
    emit('playVideo', video);
  }
}

async function revealFile(filePath: string) {
  try {
    await invoke('reveal_in_explorer', { path: filePath });
    showToast('📂 已在系统文件管理器中定位');
  } catch (err: unknown) {
    showToast(`定位失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function copyPath(filePath: string) {
  try {
    await navigator.clipboard.writeText(filePath);
    showToast('📋 文件路径已复制到剪贴板');
  } catch {
    showToast('复制失败');
  }
}

function promptResolve() {
  if (totalRedundantVideos.value === 0) {
    showToast('当前没有标记待清理的冗余视频');
    return;
  }
  showConfirmModal.value = true;
}

async function executeResolve() {
  isResolving.value = true;
  showConfirmModal.value = false;
  try {
    const actions: DuplicateResolutionAction[] = [];
    for (const group of duplicateGroups.value) {
      const keepId = selections.value[group.group_id];
      if (!keepId) continue;
      const deleteIds = group.videos.filter((v) => v.id !== keepId).map((v) => v.id);
      if (deleteIds.length > 0) {
        actions.push({
          groupId: group.group_id,
          keepVideoId: keepId,
          deleteVideoIds: deleteIds,
          deleteDiskFile: alsoDeleteDiskFile.value,
        });
      }
    }

    const res = await invoke<ResolutionSummary>('resolve_duplicate_videos', {
      actions,
    });

    showToast(res.message);
    await fetchDuplicates();
  } catch (err: unknown) {
    showToast(`去重清理失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    isResolving.value = false;
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds === undefined || isNaN(seconds)) return '--:--';
  const totalSecs = Math.floor(seconds);
  const hours = Math.floor(totalSecs / 3600);
  const minutes = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hours > 0) {
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

function formatDate(ts: number): string {
  if (!ts) return '--';
  const d = new Date(ts > 100000000000 ? ts : ts * 1000);
  return `${d.getFullYear()}-${(d.getMonth() + 1).toString().padStart(2, '0')}-${d.getDate().toString().padStart(2, '0')}`;
}

function getQualityTag(w?: number | null, h?: number | null): string {
  if (!w || !h) return '';
  const maxDim = Math.max(w, h);
  if (maxDim >= 3840) return '4K UHD';
  if (maxDim >= 2560) return '2K QHD';
  if (maxDim >= 1920) return '1080P';
  if (maxDim >= 1280) return '720P';
  return `${w}x${h}`;
}

onMounted(() => {
  fetchDuplicates();
});
</script>

<template>
  <div class="dedup-container">
    <!-- Top Strategy Header -->
    <header class="dedup-header-card">
      <div class="header-left">
        <div class="icon-avatar">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </div>
        <div>
          <h2 class="title">{{ t('dedup.title') }}</h2>
          <p class="subtitle">{{ t('dedup.subtitle') }}</p>
        </div>
      </div>

      <div class="header-actions">
        <button
          class="btn btn-primary btn-scan"
          :disabled="isLoading || isResolving"
          @click="fetchDuplicates"
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            class="refresh-icon"
            :class="{ spin: isLoading }"
          >
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
          </svg>
          <span>{{ isLoading ? t('dedup.scanning') : t('dedup.scanBtn') }}</span>
        </button>
      </div>
    </header>

    <!-- Strategy Selection Tabs -->
    <div class="strategy-selector-bar">
      <div class="strategy-tabs">
        <button
          class="strategy-tab-btn"
          :class="{ active: currentStrategy === 'exact_hash' }"
          @click="setStrategy('exact_hash')"
        >
          <span class="tab-indicator"></span>
          <span class="tab-title">{{ t('dedup.strategyExact') }}</span>
        </button>
        <button
          class="strategy-tab-btn"
          :class="{ active: currentStrategy === 'metadata' }"
          @click="setStrategy('metadata')"
        >
          <span class="tab-indicator"></span>
          <span class="tab-title">{{ t('dedup.strategyMetadata') }}</span>
        </button>
        <button
          class="strategy-tab-btn"
          :class="{ active: currentStrategy === 'fuzzy_name' }"
          @click="setStrategy('fuzzy_name')"
        >
          <span class="tab-indicator"></span>
          <span class="tab-title">{{ t('dedup.strategyFuzzy') }}</span>
        </button>
      </div>

      <div class="strategy-desc-banner">
        <span class="desc-tag">说明</span>
        <span class="desc-text">
          {{
            currentStrategy === 'exact_hash'
              ? t('dedup.strategyExactDesc')
              : currentStrategy === 'metadata'
              ? t('dedup.strategyMetadataDesc')
              : t('dedup.strategyFuzzyDesc')
          }}
        </span>
      </div>
    </div>

    <!-- Metrics Summary Cards -->
    <div class="metrics-grid">
      <div class="metric-card">
        <div class="metric-icon bg-blue">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
          </svg>
        </div>
        <div class="metric-info">
          <span class="metric-label">{{ t('dedup.statsGroups') }}</span>
          <span class="metric-value font-mono">{{ totalGroups }}</span>
        </div>
      </div>

      <div class="metric-card">
        <div class="metric-icon bg-amber">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="21 8 21 21 3 21 3 8"></polyline>
            <rect x="1" y="3" width="22" height="5"></rect>
            <line x1="10" y1="12" x2="14" y2="12"></line>
          </svg>
        </div>
        <div class="metric-info">
          <span class="metric-label">{{ t('dedup.statsDuplicates') }}</span>
          <span class="metric-value font-mono text-danger">{{ totalRedundantVideos }}</span>
        </div>
      </div>

      <div class="metric-card">
        <div class="metric-icon bg-emerald">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
            <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path>
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path>
          </svg>
        </div>
        <div class="metric-info">
          <span class="metric-label">{{ t('dedup.statsReclaimable') }}</span>
          <span class="metric-value font-mono text-success">{{ formatBytes(totalReclaimableBytes) }}</span>
        </div>
      </div>
    </div>

    <!-- Smart Action Bar -->
    <div v-if="duplicateGroups.length > 0" class="smart-select-toolbar">
      <span class="smart-title">{{ t('dedup.smartSelect') }}:</span>
      <div class="smart-btn-group">
        <button class="smart-btn" @click="applySmartSelect('best_quality')">
          {{ t('dedup.smartKeepBest') }}
        </button>
        <button class="smart-btn" @click="applySmartSelect('smallest_size')">
          {{ t('dedup.smartKeepSmallest') }}
        </button>
        <button class="smart-btn" @click="applySmartSelect('newest')">
          {{ t('dedup.smartKeepNewest') }}
        </button>
        <button class="smart-btn" @click="applySmartSelect('oldest')">
          {{ t('dedup.smartKeepOldest') }}
        </button>
      </div>
    </div>

    <!-- Main List of Duplicate Groups -->
    <main class="groups-pane">
      <div v-if="isLoading" class="loading-state">
        <div class="spinner-large"></div>
        <p class="loading-text">{{ t('dedup.scanning') }}</p>
      </div>

      <div v-else-if="duplicateGroups.length === 0" class="empty-state">
        <div class="empty-icon-circle">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
        </div>
        <h3 class="empty-title">{{ t('dedup.emptyTitle') }}</h3>
        <p class="empty-desc">{{ t('dedup.emptyDesc') }}</p>
      </div>

      <div v-else class="groups-list">
        <section
          v-for="(group, idx) in duplicateGroups"
          :key="group.group_id"
          class="group-card"
        >
          <!-- Group Header -->
          <div class="group-header">
            <div class="group-header-left">
              <span class="group-index-badge">组 #{{ idx + 1 }}</span>
              <span class="group-match-label">{{ group.match_label }}</span>
            </div>
            <div class="group-header-right font-mono">
              <span>{{ group.videos.length }} 个副本</span>
              <span class="separator">·</span>
              <span>总大小: {{ formatBytes(group.total_size) }}</span>
              <span class="separator">·</span>
              <span class="text-success font-semibold">清理释放: {{ formatBytes(group.reclaimable_size) }}</span>
            </div>
          </div>

          <!-- Group Videos Table/Card Row -->
          <div class="group-items-grid">
            <div
              v-for="video in group.videos"
              :key="video.id"
              class="video-row-card"
              :class="{
                'is-kept': selections[group.group_id] === video.id,
                'is-delete': selections[group.group_id] !== video.id,
              }"
              @click="setKeepVideo(group.group_id, video.id)"
            >
              <div class="row-selection-indicator">
                <input
                  type="radio"
                  :name="'group_' + group.group_id"
                  :checked="selections[group.group_id] === video.id"
                  @click.stop="setKeepVideo(group.group_id, video.id)"
                />
              </div>

              <!-- Thumbnail or Fallback -->
              <div class="video-thumb-box">
                <img
                  v-if="video.thumbnail_data"
                  :src="video.thumbnail_data"
                  class="thumb-img"
                  alt="thumbnail"
                />
                <div v-else class="thumb-placeholder">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <polygon points="5 3 19 12 5 21 5 3"></polygon>
                  </svg>
                </div>
                <span class="thumb-duration font-mono">{{ formatDuration(video.duration) }}</span>
              </div>

              <!-- Main Meta Details -->
              <div class="video-meta-body">
                <div class="meta-top-row">
                  <span class="file-name" :title="video.file_name">{{ video.file_name }}</span>
                  <span
                    class="status-pill"
                    :class="selections[group.group_id] === video.id ? 'pill-keep' : 'pill-del'"
                  >
                    {{ selections[group.group_id] === video.id ? t('dedup.badgeKeep') : t('dedup.badgeDelete') }}
                  </span>
                </div>

                <div class="file-path-row font-mono" :title="video.file_path">
                  {{ video.file_path }}
                </div>

                <div class="meta-tags-row font-mono">
                  <span class="tag-badge size-badge">{{ formatBytes(video.file_size) }}</span>
                  <span v-if="video.width" class="tag-badge res-badge">
                    {{ getQualityTag(video.width, video.height) }}
                  </span>
                  <span v-if="video.codec" class="tag-badge codec-badge">{{ video.codec.toUpperCase() }}</span>
                  <span class="tag-badge date-badge">{{ formatDate(video.created_at) }}</span>
                </div>
              </div>

              <!-- Actions -->
              <div class="video-actions-col" @click.stop>
                <button
                  class="act-btn btn-play"
                  title="播放预览"
                  @click="playVideo(video)"
                >
                  ▶ 播放
                </button>
                <button
                  class="act-btn"
                  title="在文件管理器中定位"
                  @click="revealFile(video.file_path)"
                >
                  📁 定位
                </button>
                <button
                  class="act-btn"
                  title="复制路径"
                  @click="copyPath(video.file_path)"
                >
                  📋 复制
                </button>
              </div>
            </div>
          </div>
        </section>
      </div>
    </main>

    <!-- Bottom Fixed Action Bar -->
    <footer v-if="duplicateGroups.length > 0" class="bottom-action-bar">
      <div class="action-bar-left">
        <label class="danger-checkbox-label">
          <input type="checkbox" v-model="alsoDeleteDiskFile" />
          <span class="checkbox-text">
            {{ t('dedup.deleteDiskLabel') }}
          </span>
        </label>
      </div>

      <div class="action-bar-right">
        <div class="summary-counts font-mono">
          待清理: <span class="highlight-count">{{ totalRedundantVideos }}</span> 个副本 ·
          释放约: <span class="highlight-bytes">{{ formatBytes(totalReclaimableBytes) }}</span>
        </div>
        <button
          class="btn btn-danger btn-resolve"
          :disabled="isResolving || totalRedundantVideos === 0"
          @click="promptResolve"
        >
          <span v-if="isResolving" class="spinner-small"></span>
          <span>{{ isResolving ? t('dedup.resolving') : t('dedup.resolveBtn') }}</span>
        </button>
      </div>
    </footer>

    <!-- Safe Confirmation Modal -->
    <div v-if="showConfirmModal" class="confirm-overlay" @click.self="showConfirmModal = false">
      <div class="confirm-card">
        <div class="modal-header">
          <div class="alert-icon-box">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
              <line x1="12" y1="9" x2="12" y2="13"></line>
              <line x1="12" y1="17" x2="12.01" y2="17"></line>
            </svg>
          </div>
          <div>
            <h3 class="modal-title">{{ t('dedup.confirmTitle') }}</h3>
            <p class="modal-subtitle">请仔细核对即将执行的清理操作</p>
          </div>
        </div>

        <div class="modal-body">
          <p class="confirm-text">
            {{ t('dedup.confirmDesc', { count: totalRedundantVideos }) }}
          </p>

          <div class="stats-preview-box font-mono">
            <div>处理组数: <strong>{{ totalGroups }}</strong> 组</div>
            <div>清理冗余: <strong>{{ totalRedundantVideos }}</strong> 个副本</div>
            <div>释放存储: <strong class="text-success">{{ formatBytes(totalReclaimableBytes) }}</strong></div>
            <div>保留主视频: <strong>{{ totalGroups }}</strong> 个</div>
          </div>

          <div v-if="alsoDeleteDiskFile" class="danger-warning-callout">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="callout-icon">
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="8" x2="12" y2="12"></line>
              <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <span>{{ t('dedup.confirmDiskWarning') }}</span>
          </div>
          <div v-else class="info-callout">
            <span>💡 提示：当前未勾选“删除原文件”，将仅从软件数据库移除多余记录，磁盘原文件将完整保留。</span>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showConfirmModal = false">取消</button>
          <button
            class="btn btn-danger"
            :disabled="isResolving"
            @click="executeResolve"
          >
            {{ alsoDeleteDiskFile ? '彻底清理并删除原文件' : '确认仅清理媒体库索引' }}
          </button>
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
.dedup-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
  padding: 1.5rem 1.75rem 5rem;
  gap: 1.25rem;
}

/* Header Card */
.dedup-header-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.icon-avatar {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.2), rgba(147, 51, 234, 0.2));
  color: #3b82f6;
  display: flex;
  align-items: center;
  justify-content: center;
}

.icon-avatar svg {
  width: 24px;
  height: 24px;
}

.title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.subtitle {
  margin: 0.25rem 0 0;
  font-size: 0.85rem;
  color: var(--text-muted);
}

.refresh-icon {
  width: 16px;
  height: 16px;
  margin-right: 6px;
}

.refresh-icon.spin {
  animation: spin 0.8s linear infinite;
}

/* Strategy Selector */
.strategy-selector-bar {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 12px;
  padding: 1rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.strategy-tabs {
  display: flex;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.strategy-tab-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  color: var(--text-muted);
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.strategy-tab-btn:hover {
  color: var(--text-primary);
  border-color: rgba(59, 130, 246, 0.4);
}

.strategy-tab-btn.active {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
  border-color: #3b82f6;
}

.tab-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
}

.strategy-tab-btn.active .tab-indicator {
  background: #3b82f6;
  box-shadow: 0 0 8px #3b82f6;
}

.strategy-desc-banner {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.8rem;
  color: var(--text-muted);
  background: rgba(0, 0, 0, 0.15);
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
}

.desc-tag {
  background: rgba(59, 130, 246, 0.2);
  color: #3b82f6;
  font-size: 0.72rem;
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 700;
}

/* Metrics Grid */
.metrics-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
}

.metric-card {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 12px;
  padding: 1rem 1.25rem;
  display: flex;
  align-items: center;
  gap: 1rem;
}

.metric-icon {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.metric-icon svg {
  width: 20px;
  height: 20px;
}

.bg-blue {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

.bg-amber {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

.bg-emerald {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
}

.metric-info {
  display: flex;
  flex-direction: column;
}

.metric-label {
  font-size: 0.78rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.metric-value {
  font-size: 1.45rem;
  font-weight: 800;
  color: var(--text-primary);
  margin-top: 2px;
}

.text-danger {
  color: #ef4444;
}

.text-success {
  color: #10b981;
}

/* Smart Select Toolbar */
.smart-select-toolbar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  padding: 0.65rem 1rem;
}

.smart-title {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-muted);
}

.smart-btn-group {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.smart-btn {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  color: var(--text-primary);
  padding: 0.35rem 0.75rem;
  border-radius: 6px;
  font-size: 0.78rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.smart-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.2);
}

/* Groups Pane */
.groups-pane {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.group-card {
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.06);
}

.group-header {
  background: rgba(0, 0, 0, 0.18);
  border-bottom: 1px solid var(--border-subtle);
  padding: 0.85rem 1.25rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.group-index-badge {
  background: rgba(59, 130, 246, 0.2);
  color: #3b82f6;
  font-size: 0.72rem;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 4px;
}

.group-match-label {
  font-size: 0.88rem;
  font-weight: 600;
  color: var(--text-primary);
}

.group-header-right {
  font-size: 0.8rem;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.separator {
  opacity: 0.4;
}

/* Group Items */
.group-items-grid {
  display: flex;
  flex-direction: column;
}

.video-row-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.85rem 1.25rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  cursor: pointer;
  transition: background 0.15s ease;
  position: relative;
}

.video-row-card:last-child {
  border-bottom: none;
}

.video-row-card:hover {
  background: rgba(255, 255, 255, 0.03);
}

.video-row-card.is-kept {
  background: rgba(16, 185, 129, 0.04);
  border-left: 4px solid #10b981;
}

.video-row-card.is-delete {
  border-left: 4px solid rgba(239, 68, 68, 0.4);
}

.row-selection-indicator input[type='radio'] {
  width: 18px;
  height: 18px;
  cursor: pointer;
  accent-color: #10b981;
}

.video-thumb-box {
  width: 100px;
  height: 60px;
  border-radius: 6px;
  background: #000;
  position: relative;
  overflow: hidden;
  flex-shrink: 0;
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}

.thumb-placeholder svg {
  width: 24px;
  height: 24px;
  opacity: 0.4;
}

.thumb-duration {
  position: absolute;
  bottom: 3px;
  right: 4px;
  background: rgba(0, 0, 0, 0.8);
  color: #fff;
  font-size: 0.65rem;
  padding: 1px 4px;
  border-radius: 3px;
}

.video-meta-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.meta-top-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.file-name {
  font-size: 0.92rem;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.status-pill {
  font-size: 0.68rem;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 4px;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

.pill-keep {
  background: rgba(16, 185, 129, 0.2);
  color: #10b981;
  border: 1px solid rgba(16, 185, 129, 0.35);
}

.pill-del {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
  border: 1px solid rgba(239, 68, 68, 0.3);
}

.file-path-row {
  font-size: 0.72rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.meta-tags-row {
  display: flex;
  gap: 0.4rem;
  flex-wrap: wrap;
}

.tag-badge {
  font-size: 0.7rem;
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--bg-input);
  color: var(--text-muted);
  border: 1px solid var(--border-subtle);
}

.res-badge {
  color: #3b82f6;
  border-color: rgba(59, 130, 246, 0.3);
}

.video-actions-col {
  display: flex;
  gap: 0.4rem;
  flex-shrink: 0;
}

.act-btn {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  color: var(--text-muted);
  font-size: 0.75rem;
  padding: 0.35rem 0.65rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.act-btn:hover {
  color: var(--text-primary);
  border-color: rgba(255, 255, 255, 0.2);
}

.btn-play {
  color: #3b82f6;
}

.btn-play:hover {
  background: rgba(59, 130, 246, 0.15);
  border-color: #3b82f6;
}

/* Bottom Fixed Action Bar */
.bottom-action-bar {
  position: fixed;
  bottom: 0;
  left: 240px;
  right: 0;
  background: var(--bg-card);
  border-top: 1px solid var(--border-subtle);
  backdrop-filter: blur(8px);
  padding: 0.85rem 1.75rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.25);
  z-index: 100;
}

.danger-checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  user-select: none;
}

.danger-checkbox-label input[type='checkbox'] {
  width: 16px;
  height: 16px;
  accent-color: #ef4444;
}

.checkbox-text {
  font-size: 0.82rem;
  color: var(--text-primary);
  font-weight: 500;
}

.action-bar-right {
  display: flex;
  align-items: center;
  gap: 1.25rem;
}

.summary-counts {
  font-size: 0.85rem;
  color: var(--text-muted);
}

.highlight-count {
  color: #ef4444;
  font-weight: 700;
}

.highlight-bytes {
  color: #10b981;
  font-weight: 700;
}

.btn-resolve {
  padding: 0.55rem 1.25rem;
  font-size: 0.88rem;
  font-weight: 600;
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

/* Modals */
.confirm-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 99999;
}

.confirm-card {
  width: 520px;
  max-width: 90vw;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.4);
}

.modal-header {
  padding: 1.25rem;
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  gap: 1rem;
}

.alert-icon-box {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
  display: flex;
  align-items: center;
  justify-content: center;
}

.alert-icon-box svg {
  width: 22px;
  height: 22px;
}

.modal-title {
  font-size: 1.05rem;
  font-weight: 700;
  margin: 0;
  color: var(--text-primary);
}

.modal-subtitle {
  font-size: 0.8rem;
  color: var(--text-muted);
  margin: 2px 0 0;
}

.modal-body {
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.confirm-text {
  font-size: 0.88rem;
  line-height: 1.5;
  color: var(--text-primary);
  margin: 0;
}

.stats-preview-box {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.5rem;
  font-size: 0.82rem;
  color: var(--text-muted);
}

.stats-preview-box strong {
  color: var(--text-primary);
}

.danger-warning-callout {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #ef4444;
  padding: 0.75rem;
  border-radius: 8px;
  font-size: 0.8rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.callout-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.info-callout {
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.25);
  color: #3b82f6;
  padding: 0.75rem;
  border-radius: 8px;
  font-size: 0.8rem;
}

.modal-footer {
  padding: 0.85rem 1.25rem;
  border-top: 1px solid var(--border-subtle);
  background: rgba(0, 0, 0, 0.1);
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

/* Toast */
.toast-popup {
  position: fixed;
  bottom: 70px;
  right: 24px;
  background: rgba(15, 23, 42, 0.95);
  color: #ffffff;
  padding: 10px 18px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.12);
  z-index: 999999;
  animation: toastFadeIn 0.2s ease-out;
  pointer-events: none;
}

/* Empty & Loading */
.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 1rem;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: 14px;
  text-align: center;
}

.spinner-large {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(59, 130, 246, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.loading-text {
  margin-top: 1rem;
  font-size: 0.9rem;
  color: var(--text-muted);
}

.empty-icon-circle {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-icon-circle svg {
  width: 30px;
  height: 30px;
}

.empty-title {
  margin: 1rem 0 0.25rem;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-primary);
}

.empty-desc {
  margin: 0;
  font-size: 0.85rem;
  color: var(--text-muted);
  max-width: 440px;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

@keyframes toastFadeIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
