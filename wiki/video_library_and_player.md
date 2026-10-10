# 🎬 媒体资产库与独立弹窗视频播放器实现规范 (Video Library & Player Modal)

本文档阐述 VidNest 媒体库的高性能渲染方案、按天时间线组织，以及独立影院级弹窗视频播放器的底层工作原理。

---

## 📅 一、按天时间线分割 (Date-Grouped Timeline)

传统媒体库采用平铺或翻页式表格，无法直观呈现视频素材的时效脉络。VidNest 依据视频入库/创建时间戳（`created_at`）精准切分天级时间组：

- **智能日期标签**：
  - **今天**：例如 `今天 · 10月9日 (星期四)`
  - **昨天**：例如 `昨天 · 10月8日 (星期三)`
  - **今年内**：例如 `10月1日 · 星期三`
  - **往年/更早**：例如 `2025年12月20日 · 星期六`
- **组内聚合指标**：
  - 每个日期组顶部附带专有日历图标、分组标题、当天收录视频数（如 `12 部视频`）以及当日总文件体积统计（如 `共 2.45 GB`）。

---

## ⚡ 二、Grouped Virtual List 虚拟滚动渲染

当媒体库中存在上千或数万个视频时，若将所有 DOM 节点全部渲染进页面，浏览器会严重掉帧甚至内存溢出崩溃。

### 1. 数据结构扁平化 (Flattened Virtual Rows)
将不同维度的行结构统一转换为虚拟位置数组：
- **日期组标题行 (`header-row`)**：固定高度 48px；
- **网格卡片行 (`grid-row`)**：固定高度 268px（动态容纳 1~6 张视频卡片）；
- **列表项行 (`list-row`)**：固定高度 56px。

### 2. 二分查找可见视口与缓冲渲染
```typescript
// 基于 ResizeObserver 动态获取可视视口高度与当前 scrollTop
const minTop = Math.max(0, st - overscanPixels); // 上部 400px 缓冲
const maxBottom = st + vh + overscanPixels;      // 下部 400px 缓冲

// 二分查找可见行切片 (O(log N))
const visibleRows = positions.slice(startIdx, endIdx + 1);
```
- **渲染性能**：即便媒体库包含 10,000+ 部视频，真实挂载在 DOM 中的节点数始终保持在 15~25 个，保持 60/120fps 绝对丝滑滚动。
- **触底无限加载**：滚动接近底部 350px 时，自动触发 `loadNextBatch()` 无感追加下一批，彻底替代老旧的分页按钮。

---

## 🎥 三、独立弹窗视频播放器 (`VideoPlayerModal.vue`)

### 1. 本地大文件流式协议 (`convertFileSrc`)
在 Tauri 应用中，Webview 因安全策略禁止直接通过 `file://` 读取本地磁盘。VidNest 采用 Tauri 2 官方资产协议：
```typescript
import { convertFileSrc } from '@tauri-apps/api/core';
const videoUrl = convertFileSrc(video.file_path);
// 转化为 http://asset.localhost/... 或 asset://...
```
- **HTTP Range 寻道**：Tauri 底层流式服务器直接支持 `Range: bytes=start-end` 请求，几十 GB 的 4K 视频在进度条上点击任意位置可实现**毫秒级无感即时跳转**，无需预先读取完整视频到内存。

### 2. 影院级暗黑控制栏与功能矩阵
- **全格式支持**：MP4, WebM, MKV, MOV 等常见格式原生硬件解码；
- **画中画 (PiP)**：支持调用浏览器 Picture-in-Picture API，将视频悬浮于屏幕右下角跨窗口浏览；
- **快捷键系统**：
  - `Space`：播放 / 暂停；
  - `Left / Right`：后退 / 前进 5 秒；
  - `Up / Down`：音量微调 ±10%；
  - `M`：静音开关；
  - `F`：全屏切换；
  - `Esc`：关闭弹窗。
- **底部规格与系统互通**：实时显示当前视频的分辨率、FPS、编码器格式与比特率，并支持一键在操作系统资源管理器中定位（`revealItemInDir`）。
