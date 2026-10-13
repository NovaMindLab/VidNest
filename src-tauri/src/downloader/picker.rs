use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

/// Returns the embedded JavaScript and CSS that provides the in-page video sniffer,
/// floating control dock, and quality selection modal on any webview-visited page.
pub fn get_video_picker_injected_script() -> &'static str {
    r#"
(function() {
  if (window.__vidnest_injected) return;
  window.__vidnest_injected = true;

  // 1. Navigation-based IPC helper (cancels navigation in Rust on_navigation)
  function sendAction(params) {
    try {
      const qs = new URLSearchParams(params).toString();
      const targetUrl = 'https://vidnest.local/action?' + qs;
      const a = document.createElement('a');
      a.href = targetUrl;
      a.style.display = 'none';
      document.documentElement.appendChild(a);
      a.click();
      setTimeout(function() { a.remove(); }, 300);
    } catch(e) {
      console.error('[VidNest] Send action error:', e);
    }
  }

  // 2. Inject CSS styles
  const style = document.createElement('style');
  style.id = 'vidnest-picker-style';
  style.textContent = `
    #vidnest-sniffer-bar {
      position: fixed;
      top: 14px;
      right: 20px;
      z-index: 2147483647;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
      font-size: 13px;
      line-height: 1.4;
      user-select: none;
      -webkit-user-select: none;
      box-sizing: border-box;
      transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }
    #vidnest-sniffer-bar * {
      box-sizing: border-box;
    }
    .vidnest-bar-card {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 8px 14px;
      background: rgba(15, 23, 42, 0.94);
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      border: 1px solid rgba(255, 255, 255, 0.16);
      border-radius: 40px;
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45), 0 0 0 1px rgba(0, 0, 0, 0.2);
      color: #f8fafc;
    }
    .vidnest-brand {
      display: flex;
      align-items: center;
      gap: 6px;
      cursor: grab;
      font-weight: 700;
      color: #ffffff;
      font-size: 13px;
    }
    .vidnest-brand:active {
      cursor: grabbing;
    }
    .vidnest-logo-icon {
      width: 22px;
      height: 22px;
      background: linear-gradient(135deg, #ef4444, #dc2626);
      border-radius: 6px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #ffffff;
      font-size: 11px;
      font-weight: bold;
      box-shadow: 0 2px 6px rgba(239, 68, 68, 0.4);
    }
    .vidnest-status-info {
      display: flex;
      align-items: center;
      gap: 6px;
      max-width: 240px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 12px;
      color: #cbd5e1;
    }
    .vidnest-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: #94a3b8;
      flex-shrink: 0;
    }
    .vidnest-dot.active {
      background: #10b981;
      box-shadow: 0 0 10px #10b981;
      animation: vidnest-pulse 1.8s infinite;
    }
    @keyframes vidnest-pulse {
      0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.7); }
      70% { transform: scale(1.1); box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
      100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0); }
    }
    .vidnest-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 14px;
      border-radius: 20px;
      font-size: 12px;
      font-weight: 600;
      cursor: pointer;
      border: none;
      transition: all 0.2s ease;
      text-decoration: none;
    }
    .vidnest-btn-primary {
      background: linear-gradient(135deg, #ef4444, #dc2626);
      color: #ffffff;
      box-shadow: 0 2px 8px rgba(239, 68, 68, 0.35);
    }
    .vidnest-btn-primary:hover {
      background: linear-gradient(135deg, #f87171, #ef4444);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(239, 68, 68, 0.5);
    }
    .vidnest-btn-primary:disabled {
      opacity: 0.6;
      cursor: not-allowed;
      transform: none;
    }
    .vidnest-btn-sub {
      background: rgba(255, 255, 255, 0.1);
      color: #e2e8f0;
      padding: 6px 10px;
    }
    .vidnest-btn-sub:hover {
      background: rgba(255, 255, 255, 0.18);
      color: #ffffff;
    }
    .vidnest-collapse-btn {
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: transparent;
      border: none;
      color: #94a3b8;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 14px;
    }
    .vidnest-collapse-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      color: #ffffff;
    }
    .vidnest-bar-mini {
      display: none;
      padding: 7px 14px;
      background: rgba(15, 23, 42, 0.94);
      backdrop-filter: blur(16px);
      border: 1px solid rgba(255, 255, 255, 0.16);
      border-radius: 20px;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
      color: #ffffff;
      cursor: pointer;
      align-items: center;
      gap: 8px;
      font-size: 12px;
      font-weight: 600;
    }
    .vidnest-bar-mini:hover {
      background: rgba(30, 41, 59, 0.96);
      border-color: rgba(239, 68, 68, 0.5);
    }

    /* Sites Navigation Modal & Quick Address Bar */
    #vidnest-sites-overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.75);
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
      z-index: 2147483647;
      display: flex;
      align-items: center;
      justify-content: center;
      animation: vidnest-fade-in 0.2s ease;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", sans-serif;
    }
    .vidnest-sites-card {
      background: #0f172a;
      border: 1px solid rgba(255, 255, 255, 0.16);
      border-radius: 20px;
      width: 660px;
      max-width: 92vw;
      box-shadow: 0 30px 70px rgba(0, 0, 0, 0.7);
      color: #f8fafc;
      overflow: hidden;
      display: flex;
      flex-direction: column;
      animation: vidnest-scale-up 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }
    .vidnest-nav-bar {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 12px 20px;
      background: rgba(255, 255, 255, 0.03);
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    }
    .vidnest-nav-icon-btn {
      width: 32px;
      height: 32px;
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.08);
      border: 1px solid rgba(255, 255, 255, 0.1);
      color: #cbd5e1;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      font-size: 13px;
      transition: all 0.15s;
    }
    .vidnest-nav-icon-btn:hover {
      background: rgba(255, 255, 255, 0.15);
      color: #ffffff;
    }
    .vidnest-nav-input-wrap {
      flex: 1;
      display: flex;
      align-items: center;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid rgba(255, 255, 255, 0.14);
      border-radius: 10px;
      padding: 0 10px;
      transition: border-color 0.2s;
    }
    .vidnest-nav-input-wrap:focus-within {
      border-color: #3b82f6;
    }
    .vidnest-nav-input {
      flex: 1;
      background: transparent;
      border: none;
      color: #ffffff;
      padding: 8px 6px;
      font-size: 13px;
      outline: none;
    }
    .vidnest-nav-go-btn {
      background: #3b82f6;
      border: none;
      color: #ffffff;
      padding: 5px 12px;
      border-radius: 6px;
      font-size: 12px;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.15s;
    }
    .vidnest-nav-go-btn:hover {
      background: #2563eb;
    }
    .vidnest-sites-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
      gap: 12px;
      padding: 20px;
      max-height: 420px;
      overflow-y: auto;
    }
    .vidnest-site-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 14px 10px;
      background: rgba(255, 255, 255, 0.04);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 14px;
      cursor: pointer;
      text-decoration: none;
      color: #f1f5f9;
      transition: all 0.2s ease;
      position: relative;
    }
    .vidnest-site-item:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.25);
      transform: translateY(-2px);
      box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
    }
    .vidnest-site-icon-box {
      width: 40px;
      height: 40px;
      border-radius: 12px;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 20px;
      font-weight: bold;
      margin-bottom: 8px;
      color: #ffffff;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    }
    .vidnest-site-name {
      font-size: 13px;
      font-weight: 600;
      margin-bottom: 2px;
      text-align: center;
    }
    .vidnest-site-tag {
      font-size: 10px;
      color: #94a3b8;
      text-align: center;
    }

    /* Modal Overlay & Card */
    #vidnest-modal-overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.72);
      backdrop-filter: blur(6px);
      -webkit-backdrop-filter: blur(6px);
      z-index: 2147483647;
      display: flex;
      align-items: center;
      justify-content: center;
      animation: vidnest-fade-in 0.2s ease;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    }
    @keyframes vidnest-fade-in {
      from { opacity: 0; }
      to { opacity: 1; }
    }
    .vidnest-modal-card {
      background: #0f172a;
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 18px;
      width: 480px;
      max-width: 90vw;
      box-shadow: 0 25px 60px rgba(0, 0, 0, 0.65);
      color: #f8fafc;
      overflow: hidden;
      animation: vidnest-scale-up 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }
    @keyframes vidnest-scale-up {
      from { transform: scale(0.92); opacity: 0; }
      to { transform: scale(1); opacity: 1; }
    }
    .vidnest-modal-header {
      padding: 16px 20px;
      background: rgba(255, 255, 255, 0.03);
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .vidnest-modal-title {
      font-size: 15px;
      font-weight: 700;
      display: flex;
      align-items: center;
      gap: 8px;
      color: #ffffff;
    }
    .vidnest-modal-close {
      background: none;
      border: none;
      color: #94a3b8;
      font-size: 18px;
      cursor: pointer;
      width: 28px;
      height: 28px;
      border-radius: 6px;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .vidnest-modal-close:hover {
      background: rgba(255, 255, 255, 0.1);
      color: #ffffff;
    }
    .vidnest-modal-body {
      padding: 20px;
      max-height: 70vh;
      overflow-y: auto;
    }
    .vidnest-video-preview {
      display: flex;
      gap: 14px;
      margin-bottom: 20px;
      background: rgba(255, 255, 255, 0.04);
      padding: 12px;
      border-radius: 12px;
      border: 1px solid rgba(255, 255, 255, 0.06);
    }
    .vidnest-video-thumb {
      width: 100px;
      height: 60px;
      border-radius: 8px;
      object-fit: cover;
      background: #1e293b;
      flex-shrink: 0;
    }
    .vidnest-video-meta {
      flex: 1;
      min-width: 0;
    }
    .vidnest-video-meta-title {
      font-size: 13px;
      font-weight: 600;
      color: #f1f5f9;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
      line-height: 1.4;
      margin-bottom: 4px;
    }
    .vidnest-video-meta-sub {
      font-size: 11px;
      color: #94a3b8;
    }
    .vidnest-options-label {
      font-size: 12px;
      font-weight: 600;
      color: #94a3b8;
      margin-bottom: 10px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    .vidnest-options-list {
      display: flex;
      flex-direction: column;
      gap: 8px;
      margin-bottom: 22px;
    }
    .vidnest-option-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 14px;
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 10px;
      cursor: pointer;
      transition: all 0.15s ease;
    }
    .vidnest-option-item:hover {
      background: rgba(255, 255, 255, 0.07);
      border-color: rgba(239, 68, 68, 0.4);
    }
    .vidnest-option-item.selected {
      background: rgba(239, 68, 68, 0.12);
      border-color: #ef4444;
    }
    .vidnest-option-left {
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .vidnest-option-radio {
      accent-color: #ef4444;
      width: 16px;
      height: 16px;
      cursor: pointer;
    }
    .vidnest-option-name {
      font-size: 13px;
      font-weight: 600;
      color: #f8fafc;
    }
    .vidnest-option-badge {
      font-size: 10px;
      padding: 1px 6px;
      border-radius: 4px;
      background: rgba(255, 255, 255, 0.08);
      color: #cbd5e1;
    }
    .vidnest-modal-actions {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 10px;
      padding-top: 14px;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }

    /* Toast */
    #vidnest-toast {
      position: fixed;
      top: 24px;
      left: 50%;
      transform: translateX(-50%);
      padding: 10px 20px;
      background: rgba(15, 23, 42, 0.96);
      backdrop-filter: blur(12px);
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 30px;
      color: #ffffff;
      font-size: 13px;
      font-weight: 600;
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
      z-index: 2147483647;
      display: none;
      align-items: center;
      gap: 8px;
    }
  `;
  document.documentElement.appendChild(style);

  // 3. Create DOM elements for sniffer bar
  const barContainer = document.createElement('div');
  barContainer.id = 'vidnest-sniffer-bar';
  barContainer.innerHTML = `
    <div class="vidnest-bar-card" id="vidnest-card">
      <div class="vidnest-brand" id="vidnest-drag-handle" title="按住可随意拖拽位置">
        <div class="vidnest-logo-icon">▶</div>
        <span>VidNest</span>
      </div>
      <button class="vidnest-btn vidnest-btn-sub" id="vidnest-switch-site-btn" title="切换到其他支持的热门视频网站或输入网址">
        <span>🌐 切换网站</span>
      </button>
      <div class="vidnest-status-info" id="vidnest-status-box">
        <span class="vidnest-dot" id="vidnest-dot"></span>
        <span id="vidnest-status-text">嗅探中...</span>
      </div>
      <button class="vidnest-btn vidnest-btn-primary" id="vidnest-download-btn">
        <span>⚡ 立即下载</span>
      </button>
      <button class="vidnest-btn vidnest-btn-sub" id="vidnest-send-main-btn" title="将视频链接发送至主界面下载中心">
        <span>📥 发送</span>
      </button>
      <button class="vidnest-collapse-btn" id="vidnest-min-btn" title="最小化浮窗">−</button>
    </div>
    <div class="vidnest-bar-mini" id="vidnest-mini">
      <div class="vidnest-logo-icon" style="width:18px;height:18px;font-size:9px;">▶</div>
      <span class="vidnest-dot active"></span>
      <span id="vidnest-mini-text">VidNest 嗅探</span>
    </div>
  `;
  document.documentElement.appendChild(barContainer);

  // 4. Create Toast element
  const toast = document.createElement('div');
  toast.id = 'vidnest-toast';
  document.documentElement.appendChild(toast);

  let toastTimer = null;
  window.__vidnest_show_toast = function(msg, type) {
    toast.textContent = (type === 'error' ? '⚠️ ' : '✓ ') + msg;
    toast.style.display = 'flex';
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(function() {
      toast.style.display = 'none';
    }, 2800);
  };

  window.__vidnest_set_inspecting = function(isInspecting) {
    const btn = document.getElementById('vidnest-download-btn');
    if (btn) {
      if (isInspecting) {
        btn.disabled = true;
        btn.innerHTML = '<span>⏳ 正在解析画质...</span>';
      } else {
        btn.disabled = false;
        btn.innerHTML = '<span>⚡ 立即下载</span>';
      }
    }
  };

  // 5. Video detection state
  let currentVideoUrl = null;
  let currentVideoTitle = null;

  function isSupportedVideoPage() {
    const href = window.location.href;
    const path = window.location.pathname;

    // YouTube
    if (href.includes('youtube.com/watch') || href.includes('youtu.be/')) return true;
    if (path.startsWith('/shorts/')) return true;

    // Bilibili
    if (href.includes('bilibili.com/video/') || href.includes('bilibili.com/bangumi/play/')) return true;

    // TikTok & Douyin
    if (href.includes('tiktok.com/@') && href.includes('/video/')) return true;
    if (href.includes('tiktok.com/t/')) return true;
    if (href.includes('douyin.com/video/') || href.includes('douyin.com/note/')) return true;

    // Twitter / X
    if ((href.includes('x.com/') || href.includes('twitter.com/')) && href.includes('/status/')) return true;

    // Instagram
    if (href.includes('instagram.com/p/') || href.includes('instagram.com/reel/') || href.includes('instagram.com/tv/')) return true;

    // Vimeo
    if (href.includes('vimeo.com/') && !href.endsWith('vimeo.com/')) return true;

    // Reddit
    if (href.includes('reddit.com/r/') && href.includes('/comments/')) return true;

    // Twitch
    if (href.includes('twitch.tv/videos/') || (href.includes('twitch.tv/') && href.includes('/clip/'))) return true;

    // Facebook
    if (href.includes('facebook.com/') && (href.includes('/videos/') || href.includes('/reel/') || href.includes('/watch'))) return true;

    // Xiaohongshu
    if (href.includes('xiaohongshu.com/explore/')) return true;

    // Weibo
    if (href.includes('weibo.com/') || href.includes('video.weibo.com/')) return true;

    // Kuaishou
    if (href.includes('kuaishou.com/short-video/')) return true;

    // Dailymotion
    if (href.includes('dailymotion.com/video/')) return true;

    // TED
    if (href.includes('ted.com/talks/')) return true;

    // Rumble
    if (href.includes('rumble.com/') && href.includes('.html')) return true;

    // SoundCloud
    if (href.includes('soundcloud.com/') && path.split('/').filter(Boolean).length >= 2) return true;

    // Generic HTML5 video tag with valid source
    const vids = document.querySelectorAll('video');
    for (let i = 0; i < vids.length; i++) {
      if ((vids[i].src || vids[i].currentSrc) && (vids[i].duration > 0 || !vids[i].paused)) {
        return true;
      }
    }
    return false;
  }

  function getPageVideoTitle() {
    // 1. YouTube specific
    const ytTitle = document.querySelector('h1.ytd-watch-metadata yt-formatted-string, h1.title, #title h1');
    if (ytTitle && ytTitle.textContent.trim()) return ytTitle.textContent.trim();

    // 2. Bilibili specific
    const biliTitle = document.querySelector('.video-title, .video-info-title, h1.video-title');
    if (biliTitle && biliTitle.textContent.trim()) return biliTitle.textContent.trim();

    // 3. OpenGraph / Twitter meta
    const ogTitle = document.querySelector('meta[property="og:title"], meta[name="twitter:title"]');
    if (ogTitle && ogTitle.content && ogTitle.content.trim()) {
      return ogTitle.content.trim();
    }

    // 4. Document title with common suffixes stripped
    const docTitle = document.title || '';
    return docTitle
      .replace(/ - YouTube$/, '')
      .replace(/_哔哩哔哩_bilibili$/, '')
      .replace(/ \| TikTok$/, '')
      .replace(/ on Vimeo$/, '')
      .replace(/ \/ X$/, '')
      .replace(/ on X$/, '')
      .trim();
  }

  function updateDetection() {
    const isVideo = isSupportedVideoPage();
    const dot = document.getElementById('vidnest-dot');
    const statusText = document.getElementById('vidnest-status-text');
    const btn = document.getElementById('vidnest-download-btn');
    const miniText = document.getElementById('vidnest-mini-text');

    if (isVideo) {
      currentVideoUrl = window.location.href;
      currentVideoTitle = getPageVideoTitle() || '当前视频';
      if (dot) dot.className = 'vidnest-dot active';
      if (statusText) {
        statusText.textContent = currentVideoTitle;
        statusText.title = currentVideoTitle;
      }
      if (miniText) miniText.textContent = '发现视频: ' + currentVideoTitle.substring(0, 10);
      if (btn) {
        btn.style.display = 'inline-flex';
        btn.title = '解析并选择画质下载';
      }
    } else {
      currentVideoUrl = window.location.href;
      currentVideoTitle = null;
      if (dot) dot.className = 'vidnest-dot';
      if (statusText) {
        statusText.textContent = '浏览中: 点击视频即可下载';
        statusText.title = '请在页面中点击任意感兴趣的视频';
      }
      if (miniText) miniText.textContent = 'VidNest 嗅探中';
    }
  }

  // Hook URL / SPA navigation changes
  setInterval(updateDetection, 600);
  window.addEventListener('popstate', updateDetection);
  window.addEventListener('yt-navigate-finish', updateDetection);

  // 6. Action handlers
  const downloadBtn = document.getElementById('vidnest-download-btn');
  downloadBtn.addEventListener('click', function() {
    const targetUrl = currentVideoUrl || window.location.href;
    sendAction({ type: 'inspect', url: targetUrl });
  });

  const sendMainBtn = document.getElementById('vidnest-send-main-btn');
  sendMainBtn.addEventListener('click', function() {
    const targetUrl = currentVideoUrl || window.location.href;
    sendAction({ type: 'send_to_main', url: targetUrl });
  });

  // Switch Site Modal Handler
  const switchSiteBtn = document.getElementById('vidnest-switch-site-btn');
  if (switchSiteBtn) {
    switchSiteBtn.addEventListener('click', function(e) {
      e.stopPropagation();
      showSitesModal();
    });
  }

  function showSitesModal() {
    let overlay = document.getElementById('vidnest-sites-overlay');
    if (overlay) {
      overlay.remove();
      return;
    }

    const popularSites = [
      { name: 'YouTube', url: 'https://www.youtube.com', bg: '#ef4444', icon: '▶', tag: '全球最大' },
      { name: 'TikTok', url: 'https://www.tiktok.com', bg: '#fe2c55', icon: '🎵', tag: '国际短视频' },
      { name: 'Twitter / X', url: 'https://x.com', bg: '#000000', icon: '𝕏', tag: '资讯·动态' },
      { name: 'Instagram', url: 'https://www.instagram.com', bg: 'linear-gradient(45deg, #f09433, #dc2743, #bc1888)', icon: '📷', tag: 'Reels短视频' },
      { name: 'Vimeo', url: 'https://vimeo.com', bg: '#1ab7ea', icon: '🎬', tag: '高清电影' },
      { name: 'Reddit', url: 'https://www.reddit.com', bg: '#ff4500', icon: '🤖', tag: '热帖社区' },
      { name: 'Twitch', url: 'https://www.twitch.tv', bg: '#9146ff', icon: '🎮', tag: '游戏直播' },
      { name: 'Facebook', url: 'https://www.facebook.com', bg: '#1877f2', icon: '👥', tag: 'Watch视频' },
      { name: 'DailyMotion', url: 'https://www.dailymotion.com', bg: '#0066dc', icon: '🎥', tag: '综合娱乐' },
      { name: 'TED Talks', url: 'https://www.ted.com', bg: '#e62b1e', icon: '💡', tag: '全球演讲' },
      { name: 'SoundCloud', url: 'https://soundcloud.com', bg: '#ff5500', icon: '🎧', tag: '原创音乐' },
      { name: 'Rumble', url: 'https://rumble.com', bg: '#85c742', icon: '🥊', tag: '自由视频' }
    ];

    overlay = document.createElement('div');
    overlay.id = 'vidnest-sites-overlay';

    let gridHtml = '';
    popularSites.forEach(function(s) {
      gridHtml += `
        <div class="vidnest-site-item" data-url="${s.url}">
          <div class="vidnest-site-icon-box" style="background:${s.bg}">
            ${s.icon}
          </div>
          <div class="vidnest-site-name">${s.name}</div>
          <div class="vidnest-site-tag">${s.tag}</div>
        </div>
      `;
    });

    overlay.innerHTML = `
      <div class="vidnest-sites-card">
        <div class="vidnest-modal-header">
          <div class="vidnest-modal-title">
            <div class="vidnest-logo-icon">🌐</div>
            <span>切换视频网站与导航</span>
          </div>
          <button class="vidnest-modal-close" id="vidnest-close-sites-btn">✕</button>
        </div>
        <div class="vidnest-nav-bar">
          <button class="vidnest-nav-icon-btn" id="vidnest-nav-back" title="后退">◀</button>
          <button class="vidnest-nav-icon-btn" id="vidnest-nav-fwd" title="前进">▶</button>
          <button class="vidnest-nav-icon-btn" id="vidnest-nav-reload" title="刷新">🔄</button>
          <div class="vidnest-nav-input-wrap">
            <input
              type="text"
              class="vidnest-nav-input"
              id="vidnest-nav-url"
              placeholder="输入任意视频网址 (如 youtube.com) 或关键词，按回车直达..."
              value="${window.location.href}"
            />
          </div>
          <button class="vidnest-nav-go-btn" id="vidnest-nav-go">前往 ➔</button>
        </div>
        <div class="vidnest-sites-grid">
          ${gridHtml}
        </div>
      </div>
    `;

    document.documentElement.appendChild(overlay);

    document.getElementById('vidnest-close-sites-btn').addEventListener('click', function() {
      overlay.remove();
    });
    overlay.addEventListener('click', function(e) {
      if (e.target === overlay) overlay.remove();
    });

    document.getElementById('vidnest-nav-back').addEventListener('click', function() {
      window.history.back();
    });
    document.getElementById('vidnest-nav-fwd').addEventListener('click', function() {
      window.history.forward();
    });
    document.getElementById('vidnest-nav-reload').addEventListener('click', function() {
      window.location.reload();
    });

    function navigateToInput() {
      const input = document.getElementById('vidnest-nav-url');
      let val = input.value.trim();
      if (!val) return;
      if (val.startsWith('http://') || val.startsWith('https://')) {
        window.location.href = val;
      } else if (val.includes('.') && !val.includes(' ')) {
        window.location.href = 'https://' + val;
      } else {
        window.location.href = 'https://www.bing.com/search?q=' + encodeURIComponent(val + ' 视频');
      }
    }

    document.getElementById('vidnest-nav-go').addEventListener('click', navigateToInput);
    document.getElementById('vidnest-nav-url').addEventListener('keydown', function(e) {
      if (e.key === 'Enter') navigateToInput();
    });

    const siteItems = overlay.querySelectorAll('.vidnest-site-item');
    siteItems.forEach(function(item) {
      item.addEventListener('click', function() {
        const u = item.getAttribute('data-url');
        if (u) {
          window.location.href = u;
        }
      });
    });
  }

  // Collapse / Expand toggle
  const minBtn = document.getElementById('vidnest-min-btn');
  const card = document.getElementById('vidnest-card');
  const mini = document.getElementById('vidnest-mini');

  minBtn.addEventListener('click', function(e) {
    e.stopPropagation();
    card.style.display = 'none';
    mini.style.display = 'flex';
  });

  mini.addEventListener('click', function() {
    mini.style.display = 'none';
    card.style.display = 'flex';
  });

  // Draggable floating bar
  let isDragging = false, startX = 0, startY = 0, initLeft = 0, initTop = 0;
  const handle = document.getElementById('vidnest-drag-handle');

  handle.addEventListener('mousedown', function(e) {
    isDragging = true;
    startX = e.clientX;
    startY = e.clientY;
    const rect = barContainer.getBoundingClientRect();
    initLeft = rect.left;
    initTop = rect.top;
    document.addEventListener('mousemove', onDragMove);
    document.addEventListener('mouseup', onDragEnd);
  });

  function onDragMove(e) {
    if (!isDragging) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    barContainer.style.left = Math.max(10, initLeft + dx) + 'px';
    barContainer.style.top = Math.max(10, initTop + dy) + 'px';
    barContainer.style.right = 'auto';
  }

  function onDragEnd() {
    isDragging = false;
    document.removeEventListener('mousemove', onDragMove);
    document.removeEventListener('mouseup', onDragEnd);
  }

  // 7. Modal Quality Options Dialog
  window.__vidnest_show_quality_modal = function(inspectData) {
    window.__vidnest_set_inspecting(false);

    // Remove existing modal if any
    const existing = document.getElementById('vidnest-modal-overlay');
    if (existing) existing.remove();

    const overlay = document.createElement('div');
    overlay.id = 'vidnest-modal-overlay';

    const durationSec = Math.round(inspectData.duration || 0);
    const durationStr = durationSec > 0
      ? `${Math.floor(durationSec / 60)}:${String(durationSec % 60).padStart(2, '0')}`
      : '--:--';

    // Build standard quality options
    const qualityTiers = [
      { id: '1080p', label: '1080P Full HD (超清)', format: 'bestvideo[height<=1080][protocol!=m3u8_native][protocol!=m3u8]+bestaudio/bestvideo[height<=1080]+bestaudio/best', badge: '推荐' },
      { id: '720p', label: '720P HD (高清)', format: 'bestvideo[height<=720][protocol!=m3u8_native][protocol!=m3u8]+bestaudio/bestvideo[height<=720]+bestaudio/best', badge: '极速' },
      { id: '480p', label: '480P 标清', format: 'bestvideo[height<=480][protocol!=m3u8_native][protocol!=m3u8]+bestaudio/bestvideo[height<=480]+bestaudio/best', badge: '流畅' },
      { id: 'mp3_320', label: '提取高品质 MP3 音频 (320kbps)', format: 'bestaudio[abr<=320][protocol!=m3u8_native][protocol!=m3u8]/bestaudio/best', badge: '音频' },
    ];

    let selectedFormat = qualityTiers[0].format;

    let optionsHtml = '';
    qualityTiers.forEach(function(tier, index) {
      const isChecked = index === 0 ? 'checked' : '';
      const isSelectedClass = index === 0 ? 'selected' : '';
      optionsHtml += `
        <div class="vidnest-option-item ${isSelectedClass}" data-fmt="${tier.format}">
          <div class="vidnest-option-left">
            <input type="radio" name="vidnest-quality-radio" class="vidnest-option-radio" ${isChecked} value="${tier.format}" />
            <span class="vidnest-option-name">${tier.label}</span>
          </div>
          <span class="vidnest-option-badge">${tier.badge}</span>
        </div>
      `;
    });

    overlay.innerHTML = `
      <div class="vidnest-modal-card">
        <div class="vidnest-modal-header">
          <div class="vidnest-modal-title">
            <div class="vidnest-logo-icon">▶</div>
            <span>VidNest 视频下载配置</span>
          </div>
          <button class="vidnest-modal-close" id="vidnest-close-modal-btn">✕</button>
        </div>
        <div class="vidnest-modal-body">
          <div class="vidnest-video-preview">
            <img src="${inspectData.thumbnail || ''}" class="vidnest-video-thumb" onerror="this.style.display='none'" />
            <div class="vidnest-video-meta">
              <div class="vidnest-video-meta-title" title="${inspectData.title}">${inspectData.title}</div>
              <div class="vidnest-video-meta-sub">作者: ${inspectData.uploader || '未知'} · 时长: ${durationStr}</div>
            </div>
          </div>
          <div class="vidnest-options-label">请选择下载画质或音频格式</div>
          <div class="vidnest-options-list" id="vidnest-options-container">
            ${optionsHtml}
          </div>
          <div class="vidnest-modal-actions">
            <button class="vidnest-btn vidnest-btn-sub" id="vidnest-modal-send-main">📥 发送至主应用</button>
            <button class="vidnest-btn vidnest-btn-primary" id="vidnest-modal-start-btn">
              <span>🚀 开始高速下载</span>
            </button>
          </div>
        </div>
      </div>
    `;

    document.documentElement.appendChild(overlay);

    // Option click handling
    const container = document.getElementById('vidnest-options-container');
    const items = container.querySelectorAll('.vidnest-option-item');
    items.forEach(function(item) {
      item.addEventListener('click', function() {
        items.forEach(function(el) { el.classList.remove('selected'); });
        item.classList.add('selected');
        const radio = item.querySelector('input[type="radio"]');
        if (radio) {
          radio.checked = true;
          selectedFormat = radio.value;
        }
      });
    });

    // Close modal
    document.getElementById('vidnest-close-modal-btn').addEventListener('click', function() {
      overlay.remove();
    });
    overlay.addEventListener('click', function(e) {
      if (e.target === overlay) overlay.remove();
    });

    // Start download action
    document.getElementById('vidnest-modal-start-btn').addEventListener('click', function() {
      const btn = this;
      btn.disabled = true;
      btn.innerHTML = '<span>⏳ 正在加入下载...</span>';

      sendAction({
        type: 'start_download',
        url: inspectData.url,
        format_id: selectedFormat,
        title: inspectData.title || '',
        author: inspectData.uploader || ''
      });
    });

    // Send to main window action
    document.getElementById('vidnest-modal-send-main').addEventListener('click', function() {
      sendAction({ type: 'send_to_main', url: inspectData.url });
      overlay.remove();
    });
  };

  window.__vidnest_on_download_success = function(msg) {
    const overlay = document.getElementById('vidnest-modal-overlay');
    if (overlay) overlay.remove();
    window.__vidnest_show_toast(msg || '已成功加入下载列表，并在后台高速下载！', 'success');
  };
})();
"#
}

/// Handles actions triggered from the webview picker's navigation requests.
pub async fn handle_picker_action(app: AppHandle, action_url: Url) {
    let mut action_type = String::new();
    let mut video_url = String::new();
    let mut format_id = None;
    let mut title = None;
    let mut author = None;

    for (k, v) in action_url.query_pairs() {
        match k.as_ref() {
            "type" => action_type = v.to_string(),
            "url" => video_url = v.to_string(),
            "format_id" => format_id = Some(v.to_string()),
            "title" => title = Some(v.to_string()),
            "author" => author = Some(v.to_string()),
            _ => {}
        }
    }

    let app_data_path = app.path().app_data_dir().unwrap_or_else(|_| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("LocalVideoIntelligence")
    });

    match action_type.as_str() {
        "inspect" => {
            if video_url.is_empty() {
                return;
            }
            if let Some(win) = app.get_webview_window("video-picker-window") {
                let _ = win.eval("window.__vidnest_set_inspecting && window.__vidnest_set_inspecting(true);");
            }

            let inspect_res = crate::downloader::inspect_url(&video_url, Some(&app_data_path)).await;

            if let Some(win) = app.get_webview_window("video-picker-window") {
                match inspect_res {
                    Ok(res) => {
                        if let Ok(json) = serde_json::to_string(&res) {
                            let eval_str = format!(
                                "window.__vidnest_show_quality_modal && window.__vidnest_show_quality_modal({});",
                                json
                            );
                            let _ = win.eval(&eval_str);
                        }
                    }
                    Err(e) => {
                        let err_json = serde_json::to_string(&format!("解析失败: {}", e)).unwrap_or_default();
                        let eval_str = format!(
                            "window.__vidnest_set_inspecting && window.__vidnest_set_inspecting(false); window.__vidnest_show_toast && window.__vidnest_show_toast({}, 'error');",
                            err_json
                        );
                        let _ = win.eval(&eval_str);
                    }
                }
            }
        }
        "start_download" => {
            if video_url.is_empty() {
                return;
            }
            if let Some(db) = app.try_state::<crate::db::Database>() {
                let manager = crate::downloader::DownloadManager::global();
                let res = manager
                    .start_download(
                        video_url.clone(),
                        format_id,
                        title,
                        author,
                        app_data_path,
                        (*db).clone(),
                    )
                    .await;

                if let Some(win) = app.get_webview_window("video-picker-window") {
                    match res {
                        Ok(task) => {
                            let _ = win.eval(
                                "window.__vidnest_on_download_success && window.__vidnest_on_download_success('已成功加入下载列表！');",
                            );
                            let _ = app.emit("picker-task-started", &task);
                        }
                        Err(e) => {
                            let err_json = serde_json::to_string(&format!("启动下载失败: {}", e)).unwrap_or_default();
                            let eval_str = format!(
                                "window.__vidnest_show_toast && window.__vidnest_show_toast({}, 'error');",
                                err_json
                            );
                            let _ = win.eval(&eval_str);
                        }
                    }
                }
            }
        }
        "send_to_main" => {
            if video_url.is_empty() {
                return;
            }
            let target_win = app
                .get_webview_window("main")
                .or_else(|| app.get_webview_window("ytd-window"));

            if let Some(main_win) = target_win {
                let _ = main_win.show();
                let _ = main_win.unminimize();
                let _ = main_win.set_focus();
                let _ = main_win.emit("picker-select-url", &video_url);
            }
            if let Some(win) = app.get_webview_window("video-picker-window") {
                let _ = win.eval(
                    "window.__vidnest_show_toast && window.__vidnest_show_toast('已发送至主窗口下载中心！', 'success');",
                );
            }
        }
        "open_main" => {
            let target_win = app
                .get_webview_window("main")
                .or_else(|| app.get_webview_window("ytd-window"));

            if let Some(main_win) = target_win {
                let _ = main_win.show();
                let _ = main_win.unminimize();
                let _ = main_win.set_focus();
            }
        }
        _ => {}
    }
}

/// Creates or activates the native Webview browser picker window for YouTube and other video sites.
pub fn open_video_picker_window_sync(app: &AppHandle, initial_url: Option<String>) -> Result<(), String> {
    let target = initial_url
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://www.youtube.com".to_string());

    if let Some(win) = app.get_webview_window("video-picker-window") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        let js = format!(
            "window.location.href = {};",
            serde_json::to_string(&target).unwrap_or_default()
        );
        let _ = win.eval(&js);
        return Ok(());
    }

    let parsed_url: Url = target
        .parse()
        .map_err(|e| format!("无效的初始网址: {}", e))?;

    let init_script = get_video_picker_injected_script();
    let app_handle = app.clone();

    let win = WebviewWindowBuilder::new(
        app,
        "video-picker-window",
        WebviewUrl::External(parsed_url),
    )
    .title("VidNest - 网页视频嗅探与下载")
    .inner_size(1200.0, 800.0)
    .min_inner_size(720.0, 480.0)
    .resizable(true)
    .center()
    .initialization_script(init_script)
    .on_navigation(move |url| {
        if url.host_str() == Some("vidnest.local") || url.scheme() == "vidnest" {
            let action_url = url.clone();
            let app_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                handle_picker_action(app_clone, action_url).await;
            });
            return false;
        }
        true
    })
    .build()
    .map_err(|e| format!("创建视频嗅探窗口失败: {}", e))?;

    let _ = win.set_focus();
    Ok(())
}
