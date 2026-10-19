<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import VideoPlayerModal from './VideoPlayerModal.vue';
import type { ClippedVideo, MasterVideoClipsGroup, ClippedVideoFilter, Video } from '../types';

const emit = defineEmits<{
  (e: 'navigateLibrary'): void;
  (e: 'navigateClipper', video?: Video): void;
}>();

const groups = ref<MasterVideoClipsGroup[]>([]);
const isLoading = ref<boolean>(false);
const searchQuery = ref<string>('');
const isMergedOnly = ref<boolean>(false);
const sortBy = ref<string>('created_desc');
const collapsedGroupKeys = ref<Set<string>>(new Set());

// Delete Modal State
const showDeleteModal = ref<boolean>(false);
const deleteTargetType = ref<'single' | 'batch'>('single');
const singleClipToDelete = ref<ClippedVideo | null>(null);
const masterGroupToDelete = ref<MasterVideoClipsGroup | null>(null);
const alsoDeleteDiskFile = ref<boolean>(false);
const isDeleting = ref<boolean>(false);

// Video Player Modal State
const isPlayerVisible = ref<boolean>(false);
const activePlayerVideo = ref<Video | null>(null);

// Toast Notification
const toastMessage = ref<string>('');
let toastTimer: number | null = null;

function showToast(msg: string) {
  toastMessage.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
  }, 2600);
}

// Helpers
function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function formatDuration(sec: number | null | undefined): string {
  if (!sec || isNaN(sec) || sec < 0) return '00:00';
  const total = Math.floor(sec);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) {
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

function msToTimeStr(ms: number): string {
  if (isNaN(ms) || ms < 0) ms = 0;
  const sec = ms / 1000.0;
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  const mm = m.toString().padStart(2, '0');
  const ss = s.toString().padStart(2, '0');
  if (h > 0) {
    return `${h.toString().padStart(2, '0')}:${mm}:${ss}`;
  }
  return `${mm}:${ss}`;
}

function formatDate(epochMs: number): string {
  if (!epochMs) return '-';
  const d = new Date(epochMs);
  return `${d.getFullYear()}-${(d.getMonth() + 1).toString().padStart(2, '0')}-${d.getDate().toString().padStart(2, '0')} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`;
}

// Data Fetching
async function fetchClippedGroups() {
  isLoading.value = true;
  try {
    const filter: ClippedVideoFilter = {
      search: searchQuery.value.trim() || undefined,
      isMergedOnly: isMergedOnly.value || undefined,
      sortBy: sortBy.value,
    };
    const res = await invoke<MasterVideoClipsGroup[]>('list_clipped_video_groups', { filter });
    groups.value = res || [];
  } catch (err: unknown) {
    showToast('获取裁剪视频列表失败: ' + String(err));
  } finally {
    isLoading.value = false;
  }
}

// Stats
const totalMasterVideosCount = computed(() => groups.value.length);
const totalClipsCount = computed(() =>
  groups.value.reduce((acc, g) => acc + g.totalClips, 0)
);
const grandTotalDuration = computed(() =>
  groups.value.reduce((acc, g) => acc + g.totalDuration, 0)
);
const grandTotalSize = computed(() =>
  groups.value.reduce((acc, g) => acc + g.totalSize, 0)
);

// Toggle Group Collapse
function toggleGroupCollapse(key: string) {
  if (collapsedGroupKeys.value.has(key)) {
    collapsedGroupKeys.value.delete(key);
  } else {
    collapsedGroupKeys.value.add(key);
  }
}

function expandAllGroups() {
  collapsedGroupKeys.value.clear();
}

function collapseAllGroups() {
  for (const g of groups.value) {
    collapsedGroupKeys.value.add(g.sourceVideoPath);
  }
}

// Action: Copy Path
async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    showToast('已复制文件路径至剪贴板');
  } catch {
    showToast('复制路径失败');
  }
}

// Action: Reveal in Folder
async function revealPath(path: string) {
  try {
    await revealItemInDir(path);
  } catch (err: unknown) {
    showToast('打开所在目录失败: ' + String(err));
  }
}

// Action: Play Clip in Modal
function playClip(clip: ClippedVideo) {
  activePlayerVideo.value = {
    id: -clip.id,
    file_path: clip.filePath,
    file_name: clip.fileName,
    file_hash: '',
    file_size: clip.fileSize,
    duration: clip.duration || 0,
    width: null,
    height: null,
    codec: null,
    created_at: clip.createdAt,
    updated_at: clip.createdAt,
    status: 'ready',
    thumbnail_path: null,
    thumbnail_data: null,
    is_starred: false,
    watch_status: 'watched',
    playback_progress: 0,
    tags: [],
  };
  isPlayerVisible.value = true;
}

// Action: Re-clip in Studio
function reClipMasterVideo(group: MasterVideoClipsGroup) {
  const masterVideoObj: Video = {
    id: group.videoId || Date.now(),
    file_path: group.sourceVideoPath,
    file_name: group.sourceVideoTitle,
    file_hash: '',
    file_size: 0,
    duration: 0,
    width: null,
    height: null,
    codec: null,
    created_at: group.latestCreatedAt,
    updated_at: group.latestCreatedAt,
    status: 'ready',
    thumbnail_path: null,
    thumbnail_data: null,
    is_starred: false,
    watch_status: 'unwatched',
    playback_progress: 0,
    tags: [],
  };
  emit('navigateClipper', masterVideoObj);
}

// Deletion Handlers
function promptDeleteSingleClip(clip: ClippedVideo) {
  deleteTargetType.value = 'single';
  singleClipToDelete.value = clip;
  masterGroupToDelete.value = null;
  alsoDeleteDiskFile.value = false;
  showDeleteModal.value = true;
}

function promptDeleteMasterGroup(group: MasterVideoClipsGroup) {
  deleteTargetType.value = 'batch';
  masterGroupToDelete.value = group;
  singleClipToDelete.value = null;
  alsoDeleteDiskFile.value = false;
  showDeleteModal.value = true;
}

async function confirmDelete() {
  isDeleting.value = true;
  try {
    if (deleteTargetType.value === 'single' && singleClipToDelete.value) {
      await invoke('delete_clipped_video', {
        id: singleClipToDelete.value.id,
        deleteDiskFile: alsoDeleteDiskFile.value,
      });
      showToast(
        `切片已删除${alsoDeleteDiskFile.value ? '（已同步清除磁盘文件）' : ''}`
      );
    } else if (deleteTargetType.value === 'batch' && masterGroupToDelete.value) {
      const count = await invoke<number>('batch_delete_master_clips', {
        sourceVideoPath: masterGroupToDelete.value.sourceVideoPath,
        deleteDiskFile: alsoDeleteDiskFile.value,
      });
      showToast(
        `已清理该主视频的 ${count} 个分段切片${alsoDeleteDiskFile.value ? '（已清除本地文件）' : ''}`
      );
    }
    showDeleteModal.value = false;
    await fetchClippedGroups();
  } catch (err: unknown) {
    showToast('删除失败: ' + String(err));
  } finally {
    isDeleting.value = false;
  }
}

onMounted(() => {
  fetchClippedGroups();
});
</script>

<template>
  <div class="clipped-manager-container">
    <!-- Top Control Bar -->
    <header class="manager-header-card">
      <div class="header-left-col">
        <div class="header-nav-btns">
          <button
            class="btn-nav-back"
            title="返回媒体库列表"
            @click="emit('navigateLibrary')"
          >
            <span class="back-arrow">←</span>
            <span>返回媒体库</span>
          </button>
          <button
            class="btn-to-clipper"
            title="进入视频阶段与智能裁剪工坊"
            @click="emit('navigateClipper')"
          >
            <span>✂️ 视频裁剪工坊</span>
          </button>
        </div>

        <div class="header-title-box">
          <h2 class="manager-title">✂️ 视频裁剪与分段管理</h2>
          <p class="manager-subtitle">
            按主视频归集分段，一处集中管理全部分段切片、高潮提取与拼接成片
          </p>
        </div>
      </div>

      <!-- Search & Filters -->
      <div class="header-right-col">
        <div class="search-input-wrap">
          <span class="search-icon">🔍</span>
          <input
            v-model="searchQuery"
            type="text"
            class="search-input"
            placeholder="搜索主视频名称、切片标题或路径..."
            @input="fetchClippedGroups"
          />
          <button
            v-if="searchQuery"
            class="btn-clear-search"
            @click="searchQuery = ''; fetchClippedGroups();"
          >
            ✕
          </button>
        </div>

        <div class="filter-controls-row">
          <label class="checkbox-filter-label" title="仅查看由多个分段拼接而成的完整合集成片">
            <input
              v-model="isMergedOnly"
              type="checkbox"
              @change="fetchClippedGroups"
            />
            <span>🔗 仅看合集拼接成片</span>
          </label>

          <select
            v-model="sortBy"
            class="select-sort"
            @change="fetchClippedGroups"
          >
            <option value="created_desc">⏱️ 按创建时间 (最新优先)</option>
            <option value="created_asc">⏱️ 按创建时间 (最早优先)</option>
            <option value="duration_desc">⏳ 按时长从长到短</option>
            <option value="title_asc">🔤 按名称字母排序</option>
          </select>

          <button
            class="btn-refresh"
            :disabled="isLoading"
            title="刷新数据"
            @click="fetchClippedGroups"
          >
            <span>🔄 刷新</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Stats Summary Ribbon -->
    <section class="stats-ribbon-card">
      <div class="stat-item">
        <span class="stat-label">🎬 主视频数</span>
        <span class="stat-value highlight-blue">{{ totalMasterVideosCount }}</span>
      </div>
      <div class="stat-divider"></div>
      <div class="stat-item">
        <span class="stat-label">✂️ 分段切片总数</span>
        <span class="stat-value highlight-orange">{{ totalClipsCount }}</span>
      </div>
      <div class="stat-divider"></div>
      <div class="stat-item">
        <span class="stat-label">⏳ 切片总时长</span>
        <span class="stat-value">{{ formatDuration(grandTotalDuration) }}</span>
      </div>
      <div class="stat-divider"></div>
      <div class="stat-item">
        <span class="stat-label">💾 占用总空间</span>
        <span class="stat-value">{{ formatBytes(grandTotalSize) }}</span>
      </div>

      <div class="ribbon-actions">
        <button
          v-if="collapsedGroupKeys.size > 0"
          class="btn-text-action"
          @click="expandAllGroups"
        >
          ➕ 展开全部
        </button>
        <button
          v-else-if="groups.length > 0"
          class="btn-text-action"
          @click="collapseAllGroups"
        >
          ➖ 折叠全部
        </button>
      </div>
    </section>

    <!-- Main Content: Master Videos Groups List -->
    <main class="groups-scroll-area">
      <!-- Loading State -->
      <div v-if="isLoading" class="manager-loading-state">
        <div class="spinner-wave"></div>
        <p>正在载入裁剪视频与分段数据...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="groups.length === 0" class="manager-empty-state">
        <div class="empty-icon-art">✂️</div>
        <h3>暂无已裁剪的视频记录</h3>
        <p>
          在媒体库中选择任意视频，点击工具栏中的「✂️ 裁剪」划分阶段或提取高潮片段，导出的切片将自动汇总于此。
        </p>
        <div class="empty-cta-btns">
          <button class="btn-cta primary" @click="emit('navigateLibrary')">
            🎬 前往媒体库选择视频
          </button>
          <button class="btn-cta ghost" @click="emit('navigateClipper')">
            ✂️ 打开裁剪工坊
          </button>
        </div>
      </div>

      <!-- Groups List (1 Master Video -> Multiple Clipped Segments) -->
      <div v-else class="groups-list">
        <div
          v-for="group in groups"
          :key="group.sourceVideoPath"
          class="master-group-card"
        >
          <!-- 1. Master Video Header -->
          <div
            class="master-header-row"
            @click="toggleGroupCollapse(group.sourceVideoPath)"
          >
            <div class="master-left-info">
              <span
                class="collapse-toggle-arrow"
                :class="{ collapsed: collapsedGroupKeys.has(group.sourceVideoPath) }"
              >
                ▼
              </span>

              <div class="master-icon-badge">🎬</div>

              <div class="master-texts">
                <div class="master-title-line">
                  <h4 class="master-video-title" :title="group.sourceVideoTitle">
                    {{ group.sourceVideoTitle }}
                  </h4>
                  <span class="badge-clips-count">
                    ✂️ {{ group.totalClips }} 个分段
                  </span>
                </div>
                <div class="master-meta-line">
                  <span class="meta-path font-mono" :title="group.sourceVideoPath">
                    {{ group.sourceVideoPath }}
                  </span>
                  <span class="meta-dot">·</span>
                  <span class="meta-summary">
                    总计 {{ formatDuration(group.totalDuration) }} · {{ formatBytes(group.totalSize) }}
                  </span>
                  <span class="meta-dot">·</span>
                  <span class="meta-date">
                    最新剪辑: {{ formatDate(group.latestCreatedAt) }}
                  </span>
                </div>
              </div>
            </div>

            <!-- Master Action Buttons -->
            <div class="master-right-actions" @click.stop>
              <button
                class="btn-m-act reclip-btn"
                title="重新载入此主视频进入裁剪工坊继续编辑"
                @click="reClipMasterVideo(group)"
              >
                <span>✂️ 继续剪辑</span>
              </button>
              <button
                class="btn-m-act locate-btn"
                title="定位原主视频所在文件夹"
                @click="revealPath(group.sourceVideoPath)"
              >
                <span>📁 原片目录</span>
              </button>
              <button
                class="btn-m-act copy-btn"
                title="复制原视频绝对路径"
                @click="copyPath(group.sourceVideoPath)"
              >
                <span>📋 路径</span>
              </button>
              <button
                class="btn-m-act delete-group-btn"
                title="批量清理该主视频下的全部裁剪切片"
                @click="promptDeleteMasterGroup(group)"
              >
                <span>🗑️ 批量清理</span>
              </button>
            </div>
          </div>

          <!-- 2. Children: Segment Clips List (Collapsible) -->
          <div
            v-if="!collapsedGroupKeys.has(group.sourceVideoPath)"
            class="children-clips-wrapper"
          >
            <div class="clips-grid">
              <div
                v-for="(clip, idx) in group.clips"
                :key="clip.id"
                class="clip-item-card"
                :class="{ 'is-merged-card': clip.isMerged }"
              >
                <!-- Clip Top Header: Index & Title -->
                <div class="clip-card-header">
                  <div class="clip-title-group">
                    <span class="clip-num-badge">#{{ idx + 1 }}</span>
                    <h5 class="clip-card-title" :title="clip.title">
                      {{ clip.title }}
                    </h5>
                  </div>

                  <div class="clip-badge-tags">
                    <span v-if="clip.isMerged" class="tag-merged">
                      🔗 拼接合集
                    </span>
                    <span
                      v-else-if="clip.title.includes('高潮') || clip.title.includes('亮点') || clip.title.includes('精彩')"
                      class="tag-highlight"
                    >
                      🔥 高能亮点
                    </span>
                  </div>
                </div>

                <!-- Clip Meta Details -->
                <div class="clip-meta-grid">
                  <div class="meta-cell">
                    <span class="cell-label">原片时间区间</span>
                    <span class="cell-value time font-mono">
                      {{ msToTimeStr(clip.startMs) }} ~ {{ msToTimeStr(clip.endMs) }}
                    </span>
                  </div>
                  <div class="meta-cell">
                    <span class="cell-label">片段时长</span>
                    <span class="cell-value font-mono">
                      {{ formatDuration(clip.duration) }}
                    </span>
                  </div>
                  <div class="meta-cell">
                    <span class="cell-label">文件体积</span>
                    <span class="cell-value font-mono">
                      {{ formatBytes(clip.fileSize) }}
                    </span>
                  </div>
                  <div class="meta-cell">
                    <span class="cell-label">生成时间</span>
                    <span class="cell-value text-muted">
                      {{ formatDate(clip.createdAt) }}
                    </span>
                  </div>
                </div>

                <div class="clip-file-path-row font-mono" :title="clip.filePath">
                  {{ clip.fileName }}
                </div>

                <!-- Clip Action Buttons -->
                <div class="clip-actions-row">
                  <button
                    class="btn-clip-act play-btn"
                    title="在内置播放器中试听预览"
                    @click="playClip(clip)"
                  >
                    <span>▶ 播放</span>
                  </button>
                  <button
                    class="btn-clip-act dir-btn"
                    title="在系统文件管理器中定位该切片文件"
                    @click="revealPath(clip.filePath)"
                  >
                    <span>📁 定位</span>
                  </button>
                  <button
                    class="btn-clip-act copy-btn"
                    title="复制切片绝对路径"
                    @click="copyPath(clip.filePath)"
                  >
                    <span>📋 复制</span>
                  </button>
                  <button
                    class="btn-clip-act del-btn"
                    title="删除此分段切片记录"
                    @click="promptDeleteSingleClip(clip)"
                  >
                    <span>🗑️ 删除</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- Delete Confirmation Modal -->
    <div
      v-if="showDeleteModal"
      class="modal-backdrop-layer"
      @click.self="showDeleteModal = false"
    >
      <div class="delete-dialog-card">
        <div class="dialog-header">
          <span class="warn-icon">⚠️</span>
          <h4>
            {{ deleteTargetType === 'single' ? '删除切片确认' : '批量清理主视频切片确认' }}
          </h4>
        </div>

        <div class="dialog-body">
          <p v-if="deleteTargetType === 'single' && singleClipToDelete">
            确定要从管理列表中移除片段
            <strong>"{{ singleClipToDelete.title }}"</strong> 吗？
          </p>
          <p v-else-if="deleteTargetType === 'batch' && masterGroupToDelete">
            确定要清理主视频
            <strong>"{{ masterGroupToDelete.sourceVideoTitle }}"</strong>
            名下的全部 <strong>{{ masterGroupToDelete.totalClips }}</strong> 个分段切片吗？
          </p>

          <label class="disk-delete-checkbox-label">
            <input v-model="alsoDeleteDiskFile" type="checkbox" />
            <span class="checkbox-text">
              <strong>同时永久从本地磁盘中删除实际视频文件</strong>
              <small class="warn-subtext">（注意：此操作不可逆，文件将彻底移除）</small>
            </span>
          </label>
        </div>

        <div class="dialog-footer">
          <button
            class="btn-dialog-cancel"
            :disabled="isDeleting"
            @click="showDeleteModal = false"
          >
            取消
          </button>
          <button
            class="btn-dialog-danger"
            :disabled="isDeleting"
            @click="confirmDelete"
          >
            {{ isDeleting ? '正在删除...' : '确认删除' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Video Player Preview Modal -->
    <VideoPlayerModal
      :video="activePlayerVideo"
      :visible="isPlayerVisible"
      @close="isPlayerVisible = false"
    />

    <!-- Toast Notification -->
    <transition name="fade">
      <div v-if="toastMessage" class="toast-chip-floating">
        {{ toastMessage }}
      </div>
    </transition>
  </div>
</template>

<style scoped>
.clipped-manager-container {
  display: flex;
  flex-direction: column;
  gap: 1.1rem;
  padding: 1.25rem;
  height: 100%;
  box-sizing: border-box;
  background: var(--bg-base, #0d1117);
  color: var(--text-color, #e6edf3);
  overflow-y: auto;
}

/* Header Card */
.manager-header-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.5rem;
  padding: 1rem 1.25rem;
  background: var(--bg-card, rgba(22, 27, 34, 0.8));
  border: 1px solid var(--border-color, rgba(255, 255, 255, 0.1));
  border-radius: 12px;
  backdrop-filter: blur(16px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
  flex-wrap: wrap;
}

.header-left-col {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  min-width: 0;
}

.header-nav-btns {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.btn-nav-back,
.btn-to-clipper {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.4rem 0.8rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-color, #e6edf3);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-nav-back:hover,
.btn-to-clipper:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #60a5fa;
  transform: translateY(-1px);
}

.manager-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: #fff;
}

.manager-subtitle {
  margin: 0.2rem 0 0 0;
  font-size: 0.78rem;
  color: #94a3b8;
}

.header-right-col {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  align-items: flex-end;
}

.search-input-wrap {
  position: relative;
  width: 320px;
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 0.85rem;
  color: #64748b;
}

.search-input {
  width: 100%;
  padding: 0.45rem 1.8rem 0.45rem 2rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 0.82rem;
  box-sizing: border-box;
}

.search-input:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.25);
}

.btn-clear-search {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  font-size: 0.75rem;
}

.filter-controls-row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.checkbox-filter-label {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.78rem;
  color: #cbd5e1;
  cursor: pointer;
}

.select-sort {
  padding: 0.4rem 0.65rem;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.78rem;
  outline: none;
  cursor: pointer;
}

.btn-refresh {
  padding: 0.4rem 0.75rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
  font-size: 0.78rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-refresh:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.16);
}

/* Stats Ribbon */
.stats-ribbon-card {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  padding: 0.75rem 1.25rem;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 8px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.stat-label {
  font-size: 0.75rem;
  color: #94a3b8;
}

.stat-value {
  font-size: 0.95rem;
  font-weight: 700;
  color: #f1f5f9;
}

.stat-value.highlight-blue {
  color: #60a5fa;
}

.stat-value.highlight-orange {
  color: #fb923c;
}

.stat-divider {
  width: 1px;
  height: 18px;
  background: rgba(255, 255, 255, 0.1);
}

.ribbon-actions {
  margin-left: auto;
}

.btn-text-action {
  background: none;
  border: none;
  color: #38bdf8;
  font-size: 0.78rem;
  cursor: pointer;
  padding: 2px 6px;
}

.btn-text-action:hover {
  text-decoration: underline;
}

/* Groups Area */
.groups-scroll-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.manager-loading-state,
.manager-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3.5rem 1.5rem;
  text-align: center;
  color: #94a3b8;
}

.spinner-wave {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(59, 130, 246, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-icon-art {
  font-size: 3rem;
  margin-bottom: 0.75rem;
}

.empty-cta-btns {
  display: flex;
  gap: 0.75rem;
  margin-top: 1.25rem;
}

.btn-cta {
  padding: 0.55rem 1.2rem;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-cta.primary {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
  color: #fff;
  border: none;
}

.btn-cta.primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 14px rgba(59, 130, 246, 0.4);
}

.btn-cta.ghost {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
}

.btn-cta.ghost:hover {
  background: rgba(255, 255, 255, 0.15);
}

/* Master Group Card */
.master-group-card {
  background: rgba(22, 27, 34, 0.65);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.2s ease;
}

.master-group-card:hover {
  border-color: rgba(255, 255, 255, 0.16);
}

.master-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 1.25rem;
  background: rgba(255, 255, 255, 0.02);
  cursor: pointer;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  user-select: none;
}

.master-header-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.master-left-info {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  min-width: 0;
  flex: 1;
}

.collapse-toggle-arrow {
  font-size: 0.7rem;
  color: #94a3b8;
  transition: transform 0.2s ease;
}

.collapse-toggle-arrow.collapsed {
  transform: rotate(-90deg);
}

.master-icon-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: rgba(59, 130, 246, 0.12);
  border: 1px solid rgba(59, 130, 246, 0.25);
  font-size: 1.15rem;
  flex-shrink: 0;
}

.master-texts {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.master-title-line {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.master-video-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 700;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 420px;
}

.badge-clips-count {
  font-size: 0.72rem;
  font-weight: 600;
  background: rgba(251, 146, 60, 0.15);
  border: 1px solid rgba(251, 146, 60, 0.35);
  color: #fb923c;
  padding: 1px 6px;
  border-radius: 4px;
}

.master-meta-line {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.74rem;
  color: #94a3b8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meta-path {
  color: #64748b;
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.master-right-actions {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-shrink: 0;
}

.btn-m-act {
  padding: 0.32rem 0.65rem;
  border-radius: 5px;
  font-size: 0.74rem;
  font-weight: 600;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  transition: all 0.15s ease;
}

.btn-m-act:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

.btn-m-act.reclip-btn {
  background: rgba(245, 158, 11, 0.12);
  border-color: rgba(245, 158, 11, 0.3);
  color: #fbbf24;
}

.btn-m-act.reclip-btn:hover {
  background: rgba(245, 158, 11, 0.25);
  color: #fff;
}

.btn-m-act.delete-group-btn:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.4);
  color: #f87171;
}

/* Children Clips Grid */
.children-clips-wrapper {
  padding: 0.85rem 1.25rem 1.1rem;
}

.clips-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 0.85rem;
}

.clip-item-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 0.75rem;
  background: rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 0.85rem;
  transition: all 0.18s ease;
}

.clip-item-card:hover {
  background: rgba(0, 0, 0, 0.5);
  border-color: rgba(59, 130, 246, 0.35);
  transform: translateY(-2px);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
}

.clip-item-card.is-merged-card {
  border-color: rgba(16, 185, 129, 0.3);
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.05), rgba(0, 0, 0, 0.35));
}

.clip-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.5rem;
}

.clip-title-group {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  flex: 1;
}

.clip-num-badge {
  font-size: 0.72rem;
  font-weight: 700;
  color: #818cf8;
  background: rgba(129, 140, 248, 0.15);
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}

.clip-card-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.clip-badge-tags {
  display: flex;
  gap: 0.3rem;
  flex-shrink: 0;
}

.tag-merged {
  font-size: 0.65rem;
  font-weight: 700;
  background: rgba(16, 185, 129, 0.2);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.35);
  padding: 1px 5px;
  border-radius: 4px;
}

.tag-highlight {
  font-size: 0.65rem;
  font-weight: 700;
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
  border: 1px solid rgba(239, 68, 68, 0.35);
  padding: 1px 5px;
  border-radius: 4px;
}

.clip-meta-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
  background: rgba(255, 255, 255, 0.02);
  padding: 0.5rem;
  border-radius: 6px;
}

.meta-cell {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.cell-label {
  font-size: 0.65rem;
  color: #64748b;
}

.cell-value {
  font-size: 0.74rem;
  color: #e2e8f0;
  font-weight: 500;
}

.cell-value.time {
  color: #38bdf8;
}

.cell-value.text-muted {
  font-size: 0.68rem;
  color: #94a3b8;
}

.clip-file-path-row {
  font-size: 0.68rem;
  color: #64748b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.clip-actions-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 0.55rem;
}

.btn-clip-act {
  flex: 1;
  padding: 0.32rem 0;
  font-size: 0.72rem;
  font-weight: 600;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cbd5e1;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-clip-act:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.btn-clip-act.play-btn {
  background: rgba(59, 130, 246, 0.18);
  border-color: rgba(59, 130, 246, 0.35);
  color: #93c5fd;
}

.btn-clip-act.play-btn:hover {
  background: rgba(59, 130, 246, 0.35);
  color: #fff;
}

.btn-clip-act.del-btn:hover {
  background: rgba(239, 68, 68, 0.2);
  border-color: rgba(239, 68, 68, 0.35);
  color: #f87171;
}

/* Delete Dialog */
.modal-backdrop-layer {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.delete-dialog-card {
  width: 440px;
  background: #161b22;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6);
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem 1.25rem;
  background: rgba(239, 68, 68, 0.1);
  border-bottom: 1px solid rgba(239, 68, 68, 0.2);
}

.warn-icon {
  font-size: 1.25rem;
}

.dialog-header h4 {
  margin: 0;
  font-size: 1rem;
  color: #fca5a5;
}

.dialog-body {
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  font-size: 0.85rem;
  color: #cbd5e1;
}

.dialog-body p {
  margin: 0;
  line-height: 1.5;
}

.disk-delete-checkbox-label {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.75rem;
  background: rgba(239, 68, 68, 0.08);
  border: 1px solid rgba(239, 68, 68, 0.2);
  border-radius: 6px;
  cursor: pointer;
}

.checkbox-text {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  font-size: 0.78rem;
  color: #fca5a5;
}

.warn-subtext {
  font-size: 0.7rem;
  color: #f87171;
}

.dialog-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0.85rem 1.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.2);
}

.btn-dialog-cancel {
  padding: 0.45rem 1rem;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cbd5e1;
  font-size: 0.82rem;
  cursor: pointer;
}

.btn-dialog-danger {
  padding: 0.45rem 1.15rem;
  border-radius: 6px;
  background: #dc2626;
  border: none;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-dialog-danger:hover:not(:disabled) {
  background: #ef4444;
}

/* Toast */
.toast-chip-floating {
  position: fixed;
  bottom: 24px;
  right: 24px;
  padding: 0.6rem 1.1rem;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(59, 130, 246, 0.4);
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.45);
  border-radius: 8px;
  color: #fff;
  font-size: 0.85rem;
  z-index: 2000;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>
