<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { CategoryStats, Tag, Collection } from '../types';

export interface CategorySelection {
  type: 'all' | 'smart' | 'format' | 'color' | 'tag' | 'folder' | 'collection';
  value: string;
  label: string;
}

const props = defineProps<{
  stats: CategoryStats | null;
  activeSelection: CategorySelection;
  isCollapsed: boolean;
}>();

const emit = defineEmits<{
  (e: 'select', selection: CategorySelection): void;
  (e: 'toggleCollapse'): void;
  (e: 'refreshStats'): void;
}>();

// New tag state
const isCreatingTag = ref(false);
const newTagName = ref('');
const newTagColor = ref('#3b82f6');

// New collection state
const isCreatingCollection = ref(false);
const newCollectionName = ref('');

const colorOptions = [
  { name: '红', value: 'red', bg: '#ef4444' },
  { name: '黄', value: 'yellow', bg: '#f59e0b' },
  { name: '绿', value: 'green', bg: '#10b981' },
  { name: '蓝', value: 'blue', bg: '#3b82f6' },
];

function selectItem(type: CategorySelection['type'], value: string, label: string) {
  emit('select', { type, value, label });
}

function isSelected(type: CategorySelection['type'], value: string) {
  return props.activeSelection.type === type && props.activeSelection.value === value;
}

async function handleCreateTag() {
  const name = newTagName.value.trim();
  if (!name) return;
  try {
    await invoke<Tag>('create_tag', { name, color: newTagColor.value });
    newTagName.value = '';
    isCreatingTag.value = false;
    emit('refreshStats');
  } catch (err) {
    console.warn('Failed to create tag:', err);
  }
}

async function handleDeleteTag(tag: Tag, event: Event) {
  event.stopPropagation();
  if (!confirm(`确定删除标签 "${tag.name}" 吗？关联视频不会被删除。`)) return;
  try {
    await invoke('delete_tag', { id: tag.id });
    if (props.activeSelection.type === 'tag' && props.activeSelection.value === String(tag.id)) {
      selectItem('all', 'all', '全部视频');
    }
    emit('refreshStats');
  } catch (err) {
    console.warn('Failed to delete tag:', err);
  }
}

async function handleCreateCollection() {
  const name = newCollectionName.value.trim();
  if (!name) return;
  try {
    await invoke<Collection>('create_collection', { name });
    newCollectionName.value = '';
    isCreatingCollection.value = false;
    emit('refreshStats');
  } catch (err) {
    console.warn('Failed to create collection:', err);
  }
}

async function handleDeleteCollection(coll: Collection, event: Event) {
  event.stopPropagation();
  if (!confirm(`确定删除合集 "${coll.name}" 吗？视频原文件不会被删除。`)) return;
  try {
    await invoke('delete_collection', { id: coll.id });
    if (props.activeSelection.type === 'collection' && props.activeSelection.value === String(coll.id)) {
      selectItem('all', 'all', '全部视频');
    }
    emit('refreshStats');
  } catch (err) {
    console.warn('Failed to delete collection:', err);
  }
}


</script>

<template>
  <aside class="category-sidebar" :class="{ collapsed: props.isCollapsed }">
    <!-- Header / Toggle Button -->
    <div class="sidebar-header">
      <div v-if="!props.isCollapsed" class="header-title-box">
        <svg class="header-ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="3" width="7" height="7"></rect>
          <rect x="14" y="3" width="7" height="7"></rect>
          <rect x="14" y="14" width="7" height="7"></rect>
          <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
        <span class="header-text">智能分类中枢</span>
      </div>
      <button
        class="toggle-collapse-btn"
        :title="props.isCollapsed ? '展开分类导航' : '收起分类导航'"
        @click="emit('toggleCollapse')"
      >
        <svg
          class="collapse-arrow"
          :class="{ reversed: props.isCollapsed }"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="15 18 9 12 15 6"></polyline>
        </svg>
      </button>
    </div>

    <!-- Scrollable Categories Content -->
    <div v-show="!props.isCollapsed" class="sidebar-scrollable-content">
      <!-- SECTION 1: 智能视图 (Smart Views) -->
      <div class="sidebar-section">
        <div class="section-title">智能视图</div>
        <div class="nav-list">
          <!-- All -->
          <button
            class="category-nav-btn"
            :class="{ active: isSelected('all', 'all') }"
            @click="selectItem('all', 'all', '全部视频')"
          >
            <span class="btn-icon">🎞️</span>
            <span class="btn-label">全部视频</span>
            <span class="btn-badge">{{ props.stats?.total_videos || 0 }}</span>
          </button>

          <!-- Starred Favorites -->
          <button
            class="category-nav-btn"
            :class="{ active: isSelected('smart', 'starred') }"
            @click="selectItem('smart', 'starred', '我的收藏')"
          >
            <span class="btn-icon">⭐</span>
            <span class="btn-label">我的收藏</span>
            <span class="btn-badge highlight">{{ props.stats?.starred_count || 0 }}</span>
          </button>

          <!-- Unwatched -->
          <button
            class="category-nav-btn"
            :class="{ active: isSelected('smart', 'unwatched') }"
            @click="selectItem('smart', 'unwatched', '待看清单')"
          >
            <span class="btn-icon">⏳</span>
            <span class="btn-label">待看清单</span>
            <span class="btn-badge">{{ props.stats?.unwatched_count || 0 }}</span>
          </button>

          <!-- In progress -->
          <button
            class="category-nav-btn"
            :class="{ active: isSelected('smart', 'in_progress') }"
            @click="selectItem('smart', 'in_progress', '正在观看')"
          >
            <span class="btn-icon">🟡</span>
            <span class="btn-label">正在观看</span>
            <span class="btn-badge">{{ props.stats?.in_progress_count || 0 }}</span>
          </button>

          <!-- Watched -->
          <button
            class="category-nav-btn"
            :class="{ active: isSelected('smart', 'watched') }"
            @click="selectItem('smart', 'watched', '已观看完')"
          >
            <span class="btn-icon">🟢</span>
            <span class="btn-label">已观看完</span>
            <span class="btn-badge">{{ props.stats?.watched_count || 0 }}</span>
          </button>
        </div>
      </div>

      <!-- SECTION 2: 内容形态 (Content Formats) -->
      <div class="sidebar-section">
        <div class="section-title">内容形态</div>
        <div class="nav-list">
          <button
            class="category-nav-btn"
            :class="{ active: isSelected('format', 'shorts') }"
            @click="selectItem('format', 'shorts', '竖屏 / 短视频')"
          >
            <span class="btn-icon">📱</span>
            <span class="btn-label">竖屏短视频</span>
            <span class="btn-badge">{{ props.stats?.shorts_count || 0 }}</span>
          </button>

          <button
            class="category-nav-btn"
            :class="{ active: isSelected('format', 'movies') }"
            @click="selectItem('format', 'movies', '电影 / 长片')"
          >
            <span class="btn-icon">🎬</span>
            <span class="btn-label">电影长片</span>
            <span class="btn-badge">{{ props.stats?.movies_count || 0 }}</span>
          </button>

          <button
            class="category-nav-btn"
            :class="{ active: isSelected('format', 'series') }"
            @click="selectItem('format', 'series', '系列 / 剧集')"
          >
            <span class="btn-icon">📺</span>
            <span class="btn-label">系列剧集</span>
            <span class="btn-badge">{{ props.stats?.series_count || 0 }}</span>
          </button>

          <button
            class="category-nav-btn"
            :class="{ active: isSelected('format', 'screen') }"
            @click="selectItem('format', 'screen', '屏幕录制')"
          >
            <span class="btn-icon">💻</span>
            <span class="btn-label">屏幕录制</span>
            <span class="btn-badge">{{ props.stats?.screen_count || 0 }}</span>
          </button>
        </div>
      </div>

      <!-- SECTION 3: 颜色与标签 (Tags) -->
      <div class="sidebar-section">
        <div class="section-title-row">
          <span class="section-title">标签与标记</span>
          <button
            class="btn-inline-add"
            title="添加新标签"
            @click="isCreatingTag = !isCreatingTag"
          >
            {{ isCreatingTag ? '取消' : '+ 新建' }}
          </button>
        </div>

        <!-- Color Dots Quick Filter Bar -->
        <div class="color-dots-bar">
          <button
            v-for="c in colorOptions"
            :key="c.value"
            class="color-dot-btn"
            :class="{ active: isSelected('color', c.value) }"
            :style="{ backgroundColor: c.bg }"
            :title="`筛选 ${c.name}色 标记`"
            @click="selectItem('color', c.value, `${c.name}色标记`)"
          ></button>
          <button
            v-if="props.activeSelection.type === 'color'"
            class="clear-color-btn"
            title="清除颜色过滤"
            @click="selectItem('all', 'all', '全部视频')"
          >
            ✕
          </button>
        </div>

        <!-- Inline Create Tag Box -->
        <div v-if="isCreatingTag" class="inline-create-box">
          <input
            v-model="newTagName"
            class="inline-input"
            placeholder="输入标签名..."
            @keydown.enter="handleCreateTag"
          />
          <button class="inline-submit-btn" @click="handleCreateTag">保存</button>
        </div>

        <!-- Custom User Tags -->
        <div class="nav-list tags-list">
          <div
            v-for="tag in props.stats?.tags || []"
            :key="tag.id"
            class="category-nav-btn tag-btn"
            :class="{ active: isSelected('tag', String(tag.id)) }"
            @click="selectItem('tag', String(tag.id), `#${tag.name}`)"
          >
            <span class="tag-hash">#</span>
            <span class="btn-label truncate">{{ tag.name }}</span>
            <span class="btn-badge">{{ tag.video_count || 0 }}</span>
            <button
              class="tag-delete-btn"
              title="删除标签"
              @click="handleDeleteTag(tag, $event)"
            >
              ✕
            </button>
          </div>
          <div v-if="!props.stats?.tags?.length" class="empty-hint">暂无自定义标签</div>
        </div>
      </div>

      <!-- SECTION 4: 自定义合集 (Collections) -->
      <div class="sidebar-section">
        <div class="section-title-row">
          <span class="section-title">合集与播单</span>
          <button
            class="btn-inline-add"
            title="新建合集"
            @click="isCreatingCollection = !isCreatingCollection"
          >
            {{ isCreatingCollection ? '取消' : '+ 新建' }}
          </button>
        </div>

        <!-- Inline Create Collection Box -->
        <div v-if="isCreatingCollection" class="inline-create-box">
          <input
            v-model="newCollectionName"
            class="inline-input"
            placeholder="输入合集名称..."
            @keydown.enter="handleCreateCollection"
          />
          <button class="inline-submit-btn" @click="handleCreateCollection">保存</button>
        </div>

        <div class="nav-list collections-list">
          <div
            v-for="coll in props.stats?.collections || []"
            :key="coll.id"
            class="category-nav-btn collection-btn"
            :class="{ active: isSelected('collection', String(coll.id)) }"
            @click="selectItem('collection', String(coll.id), coll.name)"
          >
            <span class="btn-icon">📚</span>
            <span class="btn-label truncate">{{ coll.name }}</span>
            <span class="btn-badge">{{ coll.video_count }}</span>
            <button
              class="tag-delete-btn"
              title="删除合集"
              @click="handleDeleteCollection(coll, $event)"
            >
              ✕
            </button>
          </div>
          <div v-if="!props.stats?.collections?.length" class="empty-hint">暂无自定义合集</div>
        </div>
      </div>
    </div>

    <!-- Collapsed Slim Bar View -->
    <div v-if="props.isCollapsed" class="collapsed-slim-bar">
      <button
        class="slim-icon-btn"
        :class="{ active: props.activeSelection.type === 'all' }"
        title="全部视频"
        @click="selectItem('all', 'all', '全部视频')"
      >
        🎞️
      </button>
      <button
        class="slim-icon-btn"
        :class="{ active: props.activeSelection.value === 'starred' }"
        title="我的收藏"
        @click="selectItem('smart', 'starred', '我的收藏')"
      >
        ⭐
      </button>
      <button
        class="slim-icon-btn"
        :class="{ active: props.activeSelection.value === 'unwatched' }"
        title="待看清单"
        @click="selectItem('smart', 'unwatched', '待看清单')"
      >
        ⏳
      </button>
      <button
        class="slim-icon-btn"
        :class="{ active: props.activeSelection.value === 'shorts' }"
        title="短视频"
        @click="selectItem('format', 'shorts', '竖屏 / 短视频')"
      >
        📱
      </button>
      <button
        class="slim-icon-btn"
        :class="{ active: props.activeSelection.value === 'movies' }"
        title="电影长片"
        @click="selectItem('format', 'movies', '电影 / 长片')"
      >
        🎬
      </button>
    </div>
  </aside>
</template>

<style scoped>
.category-sidebar {
  width: 220px;
  flex-shrink: 0;
  background: #ffffff;
  border-right: 1px solid #e5e7eb;
  display: flex;
  flex-direction: column;
  transition: width 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
  user-select: none;
}

:root[data-theme='dark'] .category-sidebar,
.dark .category-sidebar {
  background: #111827;
  border-right-color: #1f2937;
}

.category-sidebar.collapsed {
  width: 48px;
}

/* Header */
.sidebar-header {
  height: 48px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #f3f4f6;
  flex-shrink: 0;
}

:root[data-theme='dark'] .sidebar-header,
.dark .sidebar-header {
  border-bottom-color: #1f2937;
}

.header-title-box {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #111827;
  font-weight: 700;
  font-size: 13.5px;
}

:root[data-theme='dark'] .header-title-box,
.dark .header-title-box {
  color: #f9fafb;
}

.header-ico {
  width: 16px;
  height: 16px;
  color: #3b82f6;
}

.toggle-collapse-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  color: #6b7280;
  padding: 4px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.toggle-collapse-btn:hover {
  background: #f3f4f6;
  color: #111827;
}

:root[data-theme='dark'] .toggle-collapse-btn:hover,
.dark .toggle-collapse-btn:hover {
  background: #1f2937;
  color: #f9fafb;
}

.collapse-arrow {
  width: 16px;
  height: 16px;
  transition: transform 0.25s ease;
}

.collapse-arrow.reversed {
  transform: rotate(180deg);
}

/* Scrollable Content */
.sidebar-scrollable-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px 10px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.sidebar-scrollable-content::-webkit-scrollbar {
  width: 4px;
}

.sidebar-scrollable-content::-webkit-scrollbar-thumb {
  background: rgba(156, 163, 175, 0.4);
  border-radius: 4px;
}

/* Section */
.sidebar-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.section-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 6px 4px;
}

.section-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #9ca3af;
  padding: 2px 6px;
}

.btn-inline-add {
  font-size: 11px;
  color: #3b82f6;
  background: transparent;
  border: none;
  cursor: pointer;
  font-weight: 600;
  padding: 2px 4px;
  border-radius: 4px;
}

.btn-inline-add:hover {
  background: rgba(59, 130, 246, 0.1);
}

/* Nav list */
.nav-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.category-nav-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  background: transparent;
  border: none;
  cursor: pointer;
  color: #4b5563;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.15s ease;
  text-align: left;
  width: 100%;
}

:root[data-theme='dark'] .category-nav-btn,
.dark .category-nav-btn {
  color: #9ca3af;
}

.category-nav-btn:hover {
  background: #f3f4f6;
  color: #111827;
}

:root[data-theme='dark'] .category-nav-btn:hover,
.dark .category-nav-btn:hover {
  background: #1f2937;
  color: #f9fafb;
}

.category-nav-btn.active {
  background: #eff6ff;
  color: #2563eb;
  font-weight: 600;
}

:root[data-theme='dark'] .category-nav-btn.active,
.dark .category-nav-btn.active {
  background: rgba(37, 99, 235, 0.2);
  color: #60a5fa;
}

.btn-icon {
  font-size: 13px;
  flex-shrink: 0;
}

.btn-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-badge {
  font-size: 10.5px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 10px;
  background: #f3f4f6;
  color: #6b7280;
  flex-shrink: 0;
}

:root[data-theme='dark'] .btn-badge,
.dark .btn-badge {
  background: #1f2937;
  color: #9ca3af;
}

.btn-badge.highlight {
  background: #fef3c7;
  color: #d97706;
}

:root[data-theme='dark'] .btn-badge.highlight,
.dark .btn-badge.highlight {
  background: rgba(217, 119, 6, 0.25);
  color: #fbbf24;
}

/* Color dots bar */
.color-dots-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px 8px;
}

.color-dot-btn {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: transform 0.15s, border-color 0.15s;
}

.color-dot-btn:hover {
  transform: scale(1.25);
}

.color-dot-btn.active {
  border-color: #111827;
  transform: scale(1.3);
}

:root[data-theme='dark'] .color-dot-btn.active,
.dark .color-dot-btn.active {
  border-color: #ffffff;
}

.clear-color-btn {
  font-size: 11px;
  color: #9ca3af;
  background: transparent;
  border: none;
  cursor: pointer;
  margin-left: auto;
}

/* Inline Create Box */
.inline-create-box {
  display: flex;
  gap: 4px;
  padding: 4px 6px 8px;
}

.inline-input {
  flex: 1;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid #d1d5db;
  font-size: 12px;
  background: #ffffff;
  color: #111827;
  outline: none;
}

:root[data-theme='dark'] .inline-input,
.dark .inline-input {
  background: #1f2937;
  border-color: #374151;
  color: #f9fafb;
}

.inline-submit-btn {
  padding: 4px 8px;
  border-radius: 4px;
  background: #2563eb;
  color: #ffffff;
  border: none;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}

/* Tags & Collection items */
.tag-hash {
  font-weight: 700;
  color: #3b82f6;
  font-size: 12px;
}

.tag-btn, .collection-btn {
  position: relative;
}

.tag-delete-btn {
  display: none;
  background: transparent;
  border: none;
  cursor: pointer;
  color: #9ca3af;
  font-size: 10px;
  padding: 2px;
}

.tag-btn:hover .tag-delete-btn,
.collection-btn:hover .tag-delete-btn {
  display: block;
}

.tag-delete-btn:hover {
  color: #ef4444;
}

.empty-hint {
  font-size: 11.5px;
  color: #9ca3af;
  padding: 4px 8px;
  font-style: italic;
}

/* Collapsed view */
.collapsed-slim-bar {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 12px 0;
}

.slim-icon-btn {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: transparent;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  transition: all 0.2s;
}

.slim-icon-btn:hover {
  background: #f3f4f6;
}

:root[data-theme='dark'] .slim-icon-btn:hover,
.dark .slim-icon-btn:hover {
  background: #1f2937;
}

.slim-icon-btn.active {
  background: #eff6ff;
  border: 1px solid #bfdbfe;
}

:root[data-theme='dark'] .slim-icon-btn.active,
.dark .slim-icon-btn.active {
  background: rgba(37, 99, 235, 0.2);
  border-color: rgba(96, 165, 250, 0.4);
}
</style>
