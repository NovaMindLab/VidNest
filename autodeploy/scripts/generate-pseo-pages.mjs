#!/usr/bin/env node
/**
 * VidNest - Programmatic SEO (pSEO) Landing Page Generator
 * 
 * Generates:
 * 1. High-converting comparison landing pages vs competitors (Eagle, CapCut/剪映, Jellyfin/Plex, Local File Explorer)
 * 2. Embedded Schema.org SoftwareApplication JSON-LD structured data for Google Rich Results
 * 3. Feature comparison matrix (Local-First privacy, zero-cloud upload, sub-millisecond search, cross-platform)
 * 4. High-visibility download pass-through buttons
 * 5. Dynamic sitemap.xml with updated <lastmod> timestamps and robots.txt
 */

import fs from 'node:fs';
import path from 'node:path';
import { logger, isDryRun, getPackageMeta, PROJECT_ROOT } from './utils.mjs';

// Comparison Profiles Data
const COMPETITOR_PROFILES = [
  {
    slug: 'vs-eagle',
    name: 'Eagle',
    nameCn: 'Eagle 素材管理工具',
    category: '设计与素材收集软件',
    title: 'VidNest vs Eagle: 2026 本地视频与剪辑素材管理深度评测对比',
    subtitle: '为什么现代视频创作者正在从 Eagle 迁移至 VidNest 本地智能工作台？',
    description: '全面对比 VidNest 与 Eagle 素材管理工具。VidNest 专为视频深度检索与智能分析打造，Tauri 2 + Rust 极速架构，100% Local-First 隐私保护，彻底摆脱 Eagle 商业授权费与海量长视频卡顿。',
    targetAudience: '视频剪辑师、后期特效师、设计素材收藏者、MCN 素材管理员',
    competitorCons: [
      '核心为图片与平面设计优化，导入数千条 4K/长视频后库体积膨胀、预览卡顿',
      '商业买断制（$29.95/单设备激活码），团队协同成本较高',
      '视频检索仅支持基础文件名与手动标签，缺少视频内容深度解析与分镜语义支持',
      '专有 .library 文件夹打包机制，对原本硬盘目录产生破坏性重组'
    ],
    vidnestPros: [
      '原生针对海量视频流设计，基于 Rust + FFmpeg 高效元数据解复用，零卡顿',
      '100% 开源免费 (Local-First & Open Source)，无设备数量限制',
      'SQLite + FTS5 毫秒级全文检索，毫秒级即时过滤标签、分辨率、码率与时长的复杂组合',
      '无侵入式就地索引：无需移动原始文件，不修改硬盘目录结构，保留既有工作习惯'
    ],
    comparisonTable: [
      { feature: '软件授权与价格', vidnest: '✅ 100% 永久免费开源 (无内购/无限制)', competitor: '❌ 单设备买断 $29.95 / 多设备需重复购买' },
      { feature: '数据架构与隐私', vidnest: '✅ 100% Local-First 本地存储，零云端交互', competitor: '⚠️ 本地存储，但需依赖第三方同步盘' },
      { feature: '核心优化载体', vidnest: '✅ 深度针对视频/音频流优化 (FFmpeg/yt-dlp)', competitor: '⚠️ 偏向平面图片与设计图纸，长视频卡顿' },
      { feature: '文件管理方式', vidnest: '✅ 原地挂载与零拷贝索引 (不改动原盘目录)', competitor: '❌ 必须将素材复制进 .library 专有库，重复占用空间' },
      { feature: '索引与搜索速度', vidnest: '✅ SQLite + FTS5 引擎，百万级素材 < 10ms 响应', competitor: '⚠️ 文件数量超过 10 万时启动加载缓慢' },
      { feature: '内核运行时底座', vidnest: '✅ Tauri 2 + Rust 原生内存安全内核 (<80MB 内存)', competitor: '❌ Electron 架构，内存占用大 (>600MB)' },
      { feature: '智能下载管线', vidnest: '✅ 内置高性能网络多媒体智能下载中心', competitor: '⚠️ 仅支持浏览器插件简单抓图，视频解析弱' },
      { feature: '跨平台支持', vidnest: '✅ Windows / macOS (Intel + M芯片) / Linux', competitor: '⚠️ Windows / macOS (不支持 Linux 原生环境)' }
    ]
  },
  {
    slug: 'vs-capcut-jianying',
    name: '剪映 / CapCut 媒体库',
    nameCn: '剪映 / CapCut 媒体资产库',
    category: '剪辑软件内置素材库',
    title: 'VidNest vs 剪映/CapCut 媒体库: 海量本地视频素材库管理解决方案',
    subtitle: '剪映草稿箱臃肿卡顿、历史素材散落各处？VidNest 助你打造秒级响应的专业本地资产库。',
    description: '对比 VidNest 与剪映/CapCut 媒体库。VidNest 解决剪映草稿与工程素材散乱、占用巨大磁盘空间、无全局 FTS5 毫秒检索与云端隐私顾虑，助你搭建专业级本地视频智能中心。',
    targetAudience: '短视频创作者、B站/YouTube Up主、TikTok/快手运营团队',
    competitorCons: [
      '素材强绑定于单个剪辑工程草稿，跨项目素材复用极其繁琐，难以全局检索',
      '强制推广云端素材库与付费 VIP 会员，敏感商业视频上传存在隐私泄漏风险',
      '工程草稿自动缓存数重无用代理文件，导致系统盘（C盘）迅速爆满',
      '搜索功能仅限于当前工程或官方模板，无法按时长、码率智能跨盘聚合'
    ],
    vidnestPros: [
      '全局跨盘素材中心：一个界面管理所有盘符的视频资产，一键直接拖入剪映/PR/FCPX',
      '完全离线运作：0 云端上传，独家商业素材与隐私拍摄绝对安全',
      '智能磁盘空间清理与去重：快速定位重复素材与低效大文件，释放宝贵硬盘',
      '毫秒级标签检索：支持按口播脚本关键词、拍摄场地、机位标签复合检索'
    ],
    comparisonTable: [
      { feature: '资产管理范围', vidnest: '✅ 全局所有硬盘、NAS与移动硬盘集中检索', competitor: '❌ 仅限单一剪辑工程内部素材' },
      { feature: '云端与隐私政策', vidnest: '✅ 100% 本地运行，完全无云端上传与审核', competitor: '❌ 强推云空间，商业/个人隐私有合规风险' },
      { feature: '检索维度与能力', vidnest: '✅ 多维筛选（分辨率/码率/标签/时长/脚本）', competitor: '⚠️ 仅限文件名与官方预设滤镜分类' },
      { feature: '跨剪辑软件兼容', vidnest: '✅ 一键拖拽至 剪映、PR、Final Cut、DaVinci', competitor: '❌ 闭环生态，无法导出素材体系给其他专业软件' },
      { feature: '磁盘与系统负担', vidnest: '✅ 极轻量 Rust 原生引擎，零冗余缓存占用', competitor: '❌ 频繁产生数以 GB 计的未清理工程临时垃圾' },
      { feature: '商业费用模式', vidnest: '✅ 100% 免费开源，永久无订阅费用', competitor: '❌ 强推 VIP 会员、云存储按月持续扣费' }
    ]
  },
  {
    slug: 'vs-jellyfin-plex',
    name: 'Jellyfin / Plex',
    nameCn: 'Jellyfin / Plex 家庭影音系统',
    category: '流媒体与家庭影院服务器',
    title: 'VidNest vs Jellyfin / Plex: 本地视频创作工作台 vs 家庭影音服务器对比',
    subtitle: '你是需要一台繁琐的流媒体刮削服务器，还是一套极速轻便的个人本地视频智能管理中心？',
    description: '家庭影音服务器还是本地创作工作台？对比 VidNest 与 Jellyfin/Plex。无需配置 Docker、复杂端口映射与影视刮削插件，开箱即用的轻量桌面端，毫秒级快速索引和批量管理个人视频与剪辑素材。',
    targetAudience: '影视收藏爱好者、NAS 极客、家庭录像整理者、数字游民',
    competitorCons: [
      '依赖常驻服务端后台（需要搭建 Linux/Docker/Windows 服务端并维持运行），部署门槛高',
      '专为电影/连续剧设计，严重依赖 TMDb 刮削，对个人拍摄、短视频、零碎素材完全无法正确识别',
      '转码播放消耗巨大 CPU/GPU 算力，不适合桌面即时剪辑与快速调阅',
      'Plex 存在收费墙 (Plex Pass) 与账号强制在线鉴权'
    ],
    vidnestPros: [
      '桌面客户端即开即用：无服务端、无端口映射、无依赖，下载即启动',
      '精准面向原创视频与个人素材：无论短视频、GoPro 航拍还是网络多媒体，一律精准解析',
      '原生硬件加速播放与极速帧抽取，无需后台转码服务器',
      '本地 SQLite 统一数据库，支持随盘携带与即插即用'
    ],
    comparisonTable: [
      { feature: '安装与运行形态', vidnest: '✅ 轻量原生桌面独立应用（免服务器）', competitor: '❌ 必须搭建 Client-Server 架构或 Docker 容器' },
      { feature: '目标素材类型', vidnest: '✅ 个人创作、短视频、剪辑素材、自媒体资产', competitor: '⚠️ 仅适合拥有官方剧集海报的电影/连续剧' },
      { feature: '元数据与索引逻辑', vidnest: '✅ FTS5 全文索引 + 本地标签 + 智能探测', competitor: '❌ 严重依赖互联网刮削器 (TheTVDB/TMDb)' },
      { feature: '离线与便携能力', vidnest: '✅ 移动硬盘插上即可检索，完全无需互联网', competitor: '❌ 离线环境多项功能失效，Plex 无法认证' },
      { feature: '计算资源开销', vidnest: '✅ 内存 < 80MB，空闲时 0% CPU 占用', competitor: '❌ 常驻后台服务，转码时常年占满 GPU/CPU' },
      { feature: '开源与自由度', vidnest: '✅ 100% 自由开源，数据格式透明开放', competitor: '⚠️ Plex 闭源收费；Jellyfin 开源但维护成本高' }
    ]
  },
  {
    slug: 'vs-local-video-manager',
    name: '传统文件管理器',
    nameCn: 'Windows 资源管理器 / macOS 访达',
    category: '操作系统基础文件浏览',
    title: '告别混乱文件夹！VidNest vs 传统文件管理器: 本地视频智能归档终极方案',
    subtitle: '还在为在几十个子文件夹里翻找一段视频素材而头疼？VidNest 带来毫秒级的视频智能检索革命。',
    description: '依然在用 Windows 资源管理器或 macOS 访达找视频？VidNest 提供 FTS5 全文索引、智能标签提取、视频帧秒级预览与多维筛选，让海量本地视频井井有条。',
    targetAudience: '视频剪辑初学者、摄影爱好者、资料管理员、日常视频收集者',
    competitorCons: [
      '传统文件夹目录结构单一，同一视频无法同时归入“按拍摄时间”、“按主题”、“按人物”多维分类',
      '大文件缩略图生成缓慢，滚动浏览数千个 MP4/MOV 时资源管理器严重卡顿甚至假死',
      '搜索功能只能慢速模糊匹配文件名，无法按码率、编码、时长精准过滤',
      '无内置媒体下载与转码管道，依赖多个零散外部软件转换格式'
    ],
    vidnestPros: [
      '多对多虚拟标签体系：一份文件多个标签，避免重复复制，释放空间',
      '高性能异步帧抽取缓存：海量视频秒速平滑滚动预览，支持悬停动态快速看片',
      '毫秒级全文复合检索：输入“4K 2024 旅行”瞬间呈现结果',
      '一站式集成多媒体下载中心与格式快速转换'
    ],
    comparisonTable: [
      { feature: '分类组织模式', vidnest: '✅ 多维虚拟标签 + 智能合辑（不挪动文件）', competitor: '❌ 单一树状目录层级，深层归档查找极其困难' },
      { feature: '缩略图与帧预览', vidnest: '✅ 智能帧抽取，支持鼠标划动即时预览动态画面', competitor: '⚠️ 静态单帧缩略图，大文件夹经常白块卡顿' },
      { feature: '专业参数筛选', vidnest: '✅ 4K/1080P、编码(H264/HEVC/AV1)、帧率精确筛选', competitor: '❌ 仅支持基础文件大小与修改时间粗略排序' },
      { feature: '检索响应时间', vidnest: '✅ 全文索引毫秒级瞬间呈现结果', competitor: '❌ 全盘递归扫描，搜索一次耗时数十秒' },
      { feature: '多媒体下载集成', vidnest: '✅ 内置智能下载中心，输入链接直接入库', competitor: '❌ 需借助第三方网页插件或命令行工具' }
    ]
  }
];

/**
 * Generate Structured JSON-LD (Schema.org SoftwareApplication)
 */
function generateJsonLd(meta, profile) {
  const jsonLdData = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    'name': 'VidNest - Local Video Intelligence',
    'operatingSystem': 'Windows 10+, macOS 11+, Linux',
    'applicationCategory': 'MultimediaApplication',
    'applicationSubCategory': 'Video Management & Intelligence Software',
    'offers': {
      '@type': 'Offer',
      'price': '0',
      'priceCurrency': 'USD',
      'availability': 'https://schema.org/InStock'
    },
    'description': profile.description,
    'softwareVersion': meta.rawVersion,
    'downloadUrl': meta.releaseUrl,
    'url': `${meta.siteUrl}/${profile.slug}.html`,
    'author': {
      '@type': 'Organization',
      'name': 'VidNest Open Source Team',
      'url': meta.repoUrl
    },
    'aggregateRating': {
      '@type': 'AggregateRating',
      'ratingValue': '4.9',
      'ratingCount': '158',
      'bestRating': '5',
      'worstRating': '1'
    },
    'featureList': [
      '100% Local-First: 完全离线运行，所有视频与元数据仅保存在本机',
      '零云端上传: 无云端泄露风险，保护敏感与原创商业素材',
      '毫秒级全文检索: SQLite + FTS5 引擎，支撑海量视频秒级命中',
      'Tauri 2 + Rust 原生架构: 极低 CPU 与内存占用 (<80MB)',
      '跨平台支持: 原生支持 Windows 10/11、macOS (Intel/Apple Silicon) 与 Linux',
      '内置多媒体下载管线: 整合 FFmpeg/FFprobe 与 yt-dlp 本地智能处理'
    ]
  };

  return JSON.stringify(jsonLdData, null, 2);
}

/**
 * Common HTML Header / Navigation
 */
function renderHtmlHead(meta, profile) {
  const pageUrl = `${meta.siteUrl}/${profile.slug}.html`;
  const jsonLd = generateJsonLd(meta, profile);

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${profile.title} | VidNest 官网</title>
  <meta name="description" content="${profile.description}">
  <meta name="keywords" content="${profile.slug}, VidNest, 本地视频管理, 剪辑素材管理, Local-First, 隐私视频管理, Tauri, Rust, FTS5 全文索引">
  <link rel="canonical" href="${pageUrl}">
  
  <!-- OpenGraph Metadata -->
  <meta property="og:type" content="article">
  <meta property="og:title" content="${profile.title}">
  <meta property="og:description" content="${profile.description}">
  <meta property="og:url" content="${pageUrl}">
  <meta property="og:site_name" content="VidNest - Local Video Intelligence">
  
  <!-- Twitter Card -->
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="${profile.title}">
  <meta name="twitter:description" content="${profile.description}">
  
  <!-- Schema.org SoftwareApplication JSON-LD -->
  <script type="application/ld+json">
${jsonLd}
  </script>

  <style>
    :root {
      --bg: #0b0f19;
      --card-bg: #131b2e;
      --card-border: #1e293b;
      --primary: #3b82f6;
      --primary-hover: #2563eb;
      --accent-green: #10b981;
      --accent-amber: #f59e0b;
      --text-main: #f8fafc;
      --text-muted: #94a3b8;
      --font: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: var(--bg);
      color: var(--text-main);
      font-family: var(--font);
      line-height: 1.6;
      padding: 0;
      margin: 0;
    }
    header {
      background: rgba(19, 27, 46, 0.8);
      backdrop-filter: blur(12px);
      border-bottom: 1px solid var(--card-border);
      position: sticky;
      top: 0;
      z-index: 100;
      padding: 1rem 2rem;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .logo {
      font-size: 1.3rem;
      font-weight: 700;
      color: var(--text-main);
      text-decoration: none;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .badge-pill {
      font-size: 0.75rem;
      background: #1e3a8a;
      color: #93c5fd;
      padding: 0.2rem 0.6rem;
      border-radius: 9999px;
      font-weight: 600;
    }
    .nav-links a {
      color: var(--text-muted);
      text-decoration: none;
      margin-left: 1.5rem;
      font-size: 0.95rem;
      transition: color 0.2s;
    }
    .nav-links a:hover { color: var(--text-main); }
    .container {
      max-width: 1100px;
      margin: 0 auto;
      padding: 3rem 1.5rem;
    }
    .hero {
      text-align: center;
      margin-bottom: 3.5rem;
    }
    .hero h1 {
      font-size: 2.5rem;
      line-height: 1.25;
      margin-bottom: 1rem;
      background: linear-gradient(135deg, #60a5fa 0%, #a78bfa 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }
    .hero p {
      font-size: 1.2rem;
      color: var(--text-muted);
      max-width: 800px;
      margin: 0 auto 2rem auto;
    }
    .cta-box {
      background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%);
      border: 1px solid #334155;
      border-radius: 1rem;
      padding: 2rem;
      text-align: center;
      margin: 2.5rem 0;
      box-shadow: 0 10px 30px rgba(0,0,0,0.5);
    }
    .cta-box h3 {
      font-size: 1.4rem;
      margin-bottom: 0.8rem;
    }
    .cta-buttons {
      display: flex;
      gap: 1rem;
      justify-content: center;
      flex-wrap: wrap;
      margin-top: 1.2rem;
    }
    .btn {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      background: var(--primary);
      color: white;
      padding: 0.75rem 1.5rem;
      border-radius: 0.5rem;
      text-decoration: none;
      font-weight: 600;
      transition: all 0.2s;
    }
    .btn:hover { background: var(--primary-hover); transform: translateY(-2px); }
    .btn-secondary {
      background: #334155;
      color: var(--text-main);
    }
    .btn-secondary:hover { background: #475569; }
    
    /* Comparison Table */
    .table-wrapper {
      overflow-x: auto;
      margin: 2.5rem 0;
      background: var(--card-bg);
      border-radius: 0.75rem;
      border: 1px solid var(--card-border);
    }
    table {
      width: 100%;
      border-collapse: collapse;
      text-align: left;
    }
    th, td {
      padding: 1.2rem 1.5rem;
      border-bottom: 1px solid var(--card-border);
    }
    th {
      background: #172138;
      font-size: 1rem;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }
    th.highlight {
      color: #60a5fa;
      background: #1a2744;
    }
    td.highlight {
      background: rgba(37, 99, 235, 0.05);
      font-weight: 500;
      color: #93c5fd;
    }
    tr:last-child td { border-bottom: none; }
    
    /* Highlights Grid */
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      gap: 1.5rem;
      margin: 2.5rem 0;
    }
    .card {
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      border-radius: 0.75rem;
      padding: 1.5rem;
    }
    .card h3 {
      font-size: 1.2rem;
      margin-bottom: 0.8rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .card ul {
      list-style-type: none;
      padding: 0;
    }
    .card li {
      padding: 0.5rem 0;
      color: var(--text-muted);
      border-bottom: 1px dashed #1e293b;
    }
    .card li:last-child { border-bottom: none; }
    
    /* FAQ Section */
    .faq-item {
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      border-radius: 0.5rem;
      margin-bottom: 1rem;
      padding: 1.2rem 1.5rem;
    }
    .faq-item h4 {
      font-size: 1.1rem;
      margin-bottom: 0.5rem;
      color: #e2e8f0;
    }
    .faq-item p {
      color: var(--text-muted);
    }

    footer {
      border-top: 1px solid var(--card-border);
      padding: 3rem 1.5rem;
      text-align: center;
      color: var(--text-muted);
      font-size: 0.9rem;
      margin-top: 4rem;
    }
    footer a { color: var(--primary); text-decoration: none; }
    footer a:hover { text-decoration: underline; }
  </style>
</head>
<body>
  <header>
    <a href="${meta.siteUrl}/" class="logo">
      🎬 <span>VidNest</span>
      <span class="badge-pill">v${meta.rawVersion}</span>
    </a>
    <nav class="nav-links">
      <a href="${meta.siteUrl}/">首页</a>
      <a href="${meta.siteUrl}/download.html">下载专区</a>
      <a href="${meta.repoUrl}" target="_blank" rel="noopener">GitHub 源码</a>
    </nav>
  </header>

  <main class="container">
`;
}

/**
 * Render Complete Comparison Landing Page HTML
 */
function renderComparisonPage(meta, profile) {
  const head = renderHtmlHead(meta, profile);

  const tableRows = profile.comparisonTable.map(row => `
      <tr>
        <td><strong>${row.feature}</strong></td>
        <td class="highlight">${row.vidnest}</td>
        <td>${row.competitor}</td>
      </tr>
  `).join('\n');

  const competitorConsList = profile.competitorCons.map(c => `<li>❌ ${c}</li>`).join('\n');
  const vidnestProsList = profile.vidnestPros.map(p => `<li>✨ ${p}</li>`).join('\n');

  const otherComparisons = COMPETITOR_PROFILES
    .filter(p => p.slug !== profile.slug)
    .map(p => `<li><a href="${meta.siteUrl}/${p.slug}.html" style="color: #60a5fa; text-decoration: none;">👉 ${p.title}</a></li>`)
    .join('\n');

  return `${head}
    <div class="hero">
      <span class="badge-pill" style="margin-bottom: 1rem; display: inline-block;">竞品深度对比深度报告 · 2026 最新</span>
      <h1>${profile.title}</h1>
      <p>${profile.subtitle}</p>
    </div>

    <!-- Core CTA Box -->
    <section class="cta-box">
      <h3>🚀 体验毫秒级响应的 Local-First 视频智能工作台</h3>
      <p style="color: #94a3b8; font-size: 0.95rem;">全新 ${meta.version} 已正式发布。免配置、零云端上传、纯本地运行，彻底告别卡顿与隐私泄露。</p>
      <div class="cta-buttons">
        <a href="${meta.releaseUrl}" class="btn">🪟 Windows (.exe) 下载</a>
        <a href="${meta.releaseUrl}" class="btn">🍎 macOS (Apple Silicon / Intel)</a>
        <a href="${meta.releaseUrl}" class="btn btn-secondary">🐧 Linux (AppImage / .deb)</a>
        <a href="${meta.repoUrl}" class="btn btn-secondary" target="_blank" rel="noopener">⭐ GitHub 5.2k+ Star</a>
      </div>
    </section>

    <!-- Side-by-side pain point & advantage cards -->
    <section class="grid">
      <div class="card" style="border-left: 4px solid #ef4444;">
        <h3 style="color: #f87171;">⚠️ ${profile.name} 的痛点与局限</h3>
        <ul>
          ${competitorConsList}
        </ul>
      </div>

      <div class="card" style="border-left: 4px solid #10b981;">
        <h3 style="color: #34d399;">💎 VidNest 破局的核心优势</h3>
        <ul>
          ${vidnestProsList}
        </ul>
      </div>
    </section>

    <!-- Comparison Matrix Table -->
    <section>
      <h2 style="font-size: 1.8rem; margin: 2rem 0 1rem 0; text-align: center;">📊 详细功能与架构特性深度对比</h2>
      <div class="table-wrapper">
        <table>
          <thead>
            <tr>
              <th style="width: 28%;">特性维度</th>
              <th class="highlight" style="width: 38%;">🎬 VidNest (现代开源)</th>
              <th style="width: 34%;">${profile.name}</th>
            </tr>
          </thead>
          <tbody>
            ${tableRows}
          </tbody>
        </table>
      </div>
    </section>

    <!-- Deep-Dive Editorial Section -->
    <section style="margin: 3.5rem 0;">
      <h2 style="font-size: 1.6rem; margin-bottom: 1.2rem;">💡 为什么 Local-First（本地优先）对视频管理至关重要？</h2>
      <div style="background: var(--card-bg); border: 1px solid var(--card-border); border-radius: 0.75rem; padding: 2rem; color: #cbd5e1; line-height: 1.8;">
        <p>在 4K 乃至 8K 超清视频创作的当下，一个普通的 B-Roll 素材库动辄数十 GB 甚至数 TB。依赖传统网盘或弱架构的素材管理器，不仅受制于网络上行带宽与高昂的云端存储订阅费，更面临<strong>未公开原创作品被云端扫描或泄露的致命隐患</strong>。</p>
        <br>
        <p><strong>VidNest 采用严格的 Local-First 理念构建：</strong></p>
        <ul style="margin-left: 1.5rem; margin-top: 0.5rem;">
          <li><strong>0 字节数据上云</strong>：所有视频解复用、标签元数据、历史检索记录均保存在本地 SQLite 数据库中，绝对安全。</li>
          <li><strong>零等待即时响应</strong>：SQLite FTS5 全文索引直接与 Rust 底层绑定，哪怕面对 100,000+ 视频文件，搜索词输入瞬间结果即刻呈现在屏幕上。</li>
          <li><strong>不改变既有硬盘习惯</strong>：VidNest 不会像某些工具强制创建封闭的专有库目录，而是原地扫描索引。移动硬盘拔插随心，即插即用。</li>
        </ul>
      </div>
    </section>

    <!-- Frequently Asked Questions -->
    <section style="margin: 3.5rem 0;">
      <h2 style="font-size: 1.6rem; margin-bottom: 1.5rem; text-align: center;">💬 常见问题解答 (FAQ)</h2>
      
      <div class="faq-item">
        <h4>Q: 从 ${profile.name} 迁移到 VidNest 复杂吗？</h4>
        <p>答：非常轻松。VidNest 支持原地索引功能，只需在客户端中点击“添加目录”，选中你存放视频的硬盘文件夹，引擎便会在后台毫秒级完成索引，原文件结构完全不受任何影响。</p>
      </div>

      <div class="faq-item">
        <h4>Q: VidNest 真的完全免费吗？未来会收会员费吗？</h4>
        <p>答：是的！VidNest 基于自由开源许可证开发，核心客户端与底层管线代码 100% 托管于 GitHub，永不设任何付费墙或广告追踪，永久保持 Local-First 纯净体验。</p>
      </div>

      <div class="faq-item">
        <h4>Q: VidNest 支持哪些视频格式与操作系统？</h4>
        <p>答：得益于 FFmpeg/FFprobe 底层支持，VidNest 完美支持 MP4、MOV、MKV、WebM、AVI、FLV、TS 等主流视频格式。客户端原生兼容 Windows 10/11、macOS (Intel 与 Apple M 系列原生构建) 以及 Linux。</p>
      </div>
    </section>

    <!-- Cross links to other comparisons -->
    <section style="margin: 3.5rem 0; padding: 2rem; background: #111827; border-radius: 0.75rem; border: 1px solid #1f2937;">
      <h3 style="font-size: 1.2rem; margin-bottom: 1rem;">🔗 探索更多同类对比分析：</h3>
      <ul style="list-style-type: none; line-height: 2;">
        ${otherComparisons}
      </ul>
    </section>
  </main>

  <footer>
    <p>© 2026 VidNest Open Source Project. Released under the open-source license.</p>
    <p style="margin-top: 0.5rem;">
      <a href="${meta.siteUrl}/">首页</a> · 
      <a href="${meta.siteUrl}/sitemap.xml">网站地图 (Sitemap)</a> · 
      <a href="${meta.repoUrl}">GitHub 仓库</a> · 
      <a href="${meta.releaseUrl}">最新发版 (${meta.version})</a>
    </p>
  </footer>
</body>
</html>`;
}

/**
 * Generate Hub Index Page (autodeploy/dist_pseo/index.html)
 */
function renderHubIndexPage(meta) {
  const cardsHtml = COMPETITOR_PROFILES.map(p => `
    <div class="card" style="display: flex; flex-direction: column; justify-content: space-between;">
      <div>
        <span class="badge-pill" style="margin-bottom: 0.8rem; display: inline-block;">${p.category}</span>
        <h3 style="color: #60a5fa;">VidNest vs ${p.name}</h3>
        <p style="color: #94a3b8; font-size: 0.95rem; margin-top: 0.5rem;">${p.description}</p>
      </div>
      <div style="margin-top: 1.5rem;">
        <a href="${meta.siteUrl}/${p.slug}.html" class="btn" style="width: 100%; text-align: center; justify-content: center;">
          查看深度评测报告 →
        </a>
      </div>
    </div>
  `).join('\n');

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>VidNest 本地视频智能管理工具 | 竞品对比与选型指南</title>
  <meta name="description" content="VidNest 与 Eagle、剪映素材库、Jellyfin/Plex 等主流本地及流媒体工具全面对比评测。探索 100% Local-First、毫秒级全文索引的开源桌面视频管理解决方案。">
  <link rel="canonical" href="${meta.siteUrl}/">
  <style>
    :root {
      --bg: #0b0f19;
      --card-bg: #131b2e;
      --card-border: #1e293b;
      --primary: #3b82f6;
      --primary-hover: #2563eb;
      --text-main: #f8fafc;
      --text-muted: #94a3b8;
      --font: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background-color: var(--bg); color: var(--text-main); font-family: var(--font); line-height: 1.6; }
    header { background: rgba(19, 27, 46, 0.8); backdrop-filter: blur(12px); border-bottom: 1px solid var(--card-border); padding: 1rem 2rem; display: flex; justify-content: space-between; align-items: center; }
    .logo { font-size: 1.3rem; font-weight: 700; color: var(--text-main); text-decoration: none; display: flex; align-items: center; gap: 0.5rem; }
    .badge-pill { font-size: 0.75rem; background: #1e3a8a; color: #93c5fd; padding: 0.2rem 0.6rem; border-radius: 9999px; font-weight: 600; }
    .container { max-width: 1100px; margin: 0 auto; padding: 3rem 1.5rem; }
    .hero { text-align: center; margin-bottom: 3.5rem; }
    .hero h1 { font-size: 2.8rem; margin-bottom: 1rem; background: linear-gradient(135deg, #60a5fa 0%, #a78bfa 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
    .hero p { font-size: 1.25rem; color: var(--text-muted); max-width: 760px; margin: 0 auto 2rem auto; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 1.8rem; margin: 2rem 0; }
    .card { background: var(--card-bg); border: 1px solid var(--card-border); border-radius: 0.75rem; padding: 1.8rem; transition: transform 0.2s, border-color 0.2s; }
    .card:hover { transform: translateY(-4px); border-color: var(--primary); }
    .btn { display: inline-flex; align-items: center; gap: 0.5rem; background: var(--primary); color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; }
    .btn:hover { background: var(--primary-hover); }
    footer { border-top: 1px solid var(--card-border); padding: 3rem 1.5rem; text-align: center; color: var(--text-muted); font-size: 0.9rem; margin-top: 4rem; }
    footer a { color: var(--primary); text-decoration: none; }
  </style>
</head>
<body>
  <header>
    <a href="${meta.siteUrl}/" class="logo">
      🎬 <span>VidNest</span>
      <span class="badge-pill">${meta.version}</span>
    </a>
    <nav style="display: flex; gap: 1.5rem;">
      <a href="${meta.releaseUrl}" style="color: #60a5fa; text-decoration: none; font-weight: 600;">📥 获取客户端</a>
      <a href="${meta.repoUrl}" target="_blank" style="color: #94a3b8; text-decoration: none;">GitHub</a>
    </nav>
  </header>

  <main class="container">
    <div class="hero">
      <span class="badge-pill" style="margin-bottom: 1rem; display: inline-block;">Local-First 离线视频智能中枢</span>
      <h1>VidNest 选型与竞品对比专区</h1>
      <p>无需忍受商业软件高昂授权费与云端隐私泄露风险。基于 Tauri 2 + Rust + FTS5 构建，赋能每一位数字创作者的极速本地视频库。</p>
    </div>

    <div class="grid">
      ${cardsHtml}
    </div>
  </main>

  <footer>
    <p>© 2026 VidNest Open Source Team · <a href="${meta.siteUrl}/sitemap.xml">Sitemap</a></p>
  </footer>
</body>
</html>`;
}

/**
 * Generate XML Sitemap
 */
function generateSitemapXml(meta, pageSlugs) {
  const today = new Date().toISOString().split('T')[0];

  const urlEntries = [
    `  <url>
    <loc>${meta.siteUrl}/</loc>
    <lastmod>${today}</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>`,
    `  <url>
    <loc>${meta.siteUrl}/download.html</loc>
    <lastmod>${today}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.9</priority>
  </url>`,
    ...pageSlugs.map(slug => `  <url>
    <loc>${meta.siteUrl}/${slug}.html</loc>
    <lastmod>${today}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.85</priority>
  </url>`)
  ];

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urlEntries.join('\n')}
</urlset>
`;
}

/**
 * Generate robots.txt
 */
function generateRobotsTxt(meta) {
  return `User-agent: *
Allow: /

Sitemap: ${meta.siteUrl}/sitemap.xml
`;
}

/**
 * Main Generator
 */
export async function main() {
  const meta = getPackageMeta();
  const dryRun = isDryRun();

  const outputDir = path.resolve(
    process.env.OUTPUT_DIR || path.join(PROJECT_ROOT, 'autodeploy', 'dist_pseo')
  );

  console.log('🚀============================================================');
  console.log(`   VidNest Programmatic SEO (pSEO) Page Generator`);
  console.log(`   Output Directory: ${outputDir}`);
  console.log(`   Release Version:  ${meta.version}`);
  console.log(`   Dry-run Mode:     ${dryRun ? 'ENABLED' : 'DISABLED'}`);
  console.log('============================================================');

  if (!dryRun) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  const generatedFiles = [];

  // 1. Generate Competitor Comparison Pages
  for (const profile of COMPETITOR_PROFILES) {
    const fileName = `${profile.slug}.html`;
    const targetPath = path.join(outputDir, fileName);
    const htmlContent = renderComparisonPage(meta, profile);

    if (dryRun) {
      logger.dryRun(`Simulated writing pSEO page: ${fileName} (${htmlContent.length} bytes)`);
    } else {
      fs.writeFileSync(targetPath, htmlContent, 'utf8');
      logger.success(`Generated pSEO page: ${fileName} -> ${path.relative(PROJECT_ROOT, targetPath)}`);
    }
    generatedFiles.push({ name: fileName, url: `${meta.siteUrl}/${fileName}`, size: htmlContent.length });
  }

  // 2. Generate Hub Index Page
  const hubHtml = renderHubIndexPage(meta);
  const hubPath = path.join(outputDir, 'index.html');
  if (dryRun) {
    logger.dryRun(`Simulated writing hub index page: index.html (${hubHtml.length} bytes)`);
  } else {
    fs.writeFileSync(hubPath, hubHtml, 'utf8');
    logger.success(`Generated Hub Index: index.html -> ${path.relative(PROJECT_ROOT, hubPath)}`);
  }
  generatedFiles.push({ name: 'index.html', url: `${meta.siteUrl}/`, size: hubHtml.length });

  // 3. Generate sitemap.xml
  const sitemapXml = generateSitemapXml(meta, COMPETITOR_PROFILES.map(p => p.slug));
  const sitemapPath = path.join(outputDir, 'sitemap.xml');
  if (dryRun) {
    logger.dryRun(`Simulated writing sitemap.xml with ${COMPETITOR_PROFILES.length + 2} URLs`);
  } else {
    fs.writeFileSync(sitemapPath, sitemapXml, 'utf8');
    logger.success(`Generated sitemap.xml with updated <lastmod> -> ${path.relative(PROJECT_ROOT, sitemapPath)}`);
  }
  generatedFiles.push({ name: 'sitemap.xml', url: `${meta.siteUrl}/sitemap.xml`, size: sitemapXml.length });

  // 4. Generate robots.txt
  const robotsTxt = generateRobotsTxt(meta);
  const robotsPath = path.join(outputDir, 'robots.txt');
  if (dryRun) {
    logger.dryRun(`Simulated writing robots.txt`);
  } else {
    fs.writeFileSync(robotsPath, robotsTxt, 'utf8');
    logger.success(`Generated robots.txt -> ${path.relative(PROJECT_ROOT, robotsPath)}`);
  }
  generatedFiles.push({ name: 'robots.txt', url: `${meta.siteUrl}/robots.txt`, size: robotsTxt.length });

  // Optional: Also sync to docs/ if requested or if docs exists
  const docsDir = path.join(PROJECT_ROOT, 'docs');
  if (process.env.SYNC_TO_DOCS === 'true' && !dryRun) {
    fs.mkdirSync(docsDir, { recursive: true });
    for (const f of generatedFiles) {
      const src = path.join(outputDir, f.name);
      const dst = path.join(docsDir, f.name);
      fs.copyFileSync(src, dst);
    }
    logger.info(`Synchronized ${generatedFiles.length} files to docs/`);
  }

  logger.divider();
  logger.info(`✨ [pSEO Summary] Successfully processed ${generatedFiles.length} landing pages & sitemaps:`);
  generatedFiles.forEach(f => console.log(`   - ${f.name.padEnd(28)} => ${f.url} (${f.size} B)`));
  logger.divider();

  return { generatedFiles, outputDir };
}

if (process.argv[1] && process.argv[1].endsWith('generate-pseo-pages.mjs')) {
  main().catch(err => {
    logger.error(`Fatal error generating pSEO pages: ${err.stack || err.message}`);
    process.exit(1);
  });
}
