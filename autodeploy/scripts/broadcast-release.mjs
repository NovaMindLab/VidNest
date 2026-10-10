#!/usr/bin/env node
/**
 * VidNest - Decentralized Social & Team IM Multi-Channel Broadcast Engine
 * 
 * Channels Supported:
 * 1. Bluesky (AT Protocol via com.atproto.server.createSession & com.atproto.repo.createRecord)
 * 2. Mastodon (Fediverse /api/v1/statuses)
 * 3. Feishu / Lark (Interactive Card Webhook with action buttons)
 * 4. DingTalk / 钉钉 (Markdown Webhook with optional HMAC-SHA256 signature)
 * 5. Discord (Rich Embed Webhook)
 * 6. Telegram Bot (MarkdownV2 format release dispatch)
 * 
 * Reliability Features:
 * - Graceful degradation: Missing credentials skip cleanly with warning logs
 * - Isolated channel execution: Failure in one channel does not abort others
 * - DRY_RUN mode for pipeline preview & debugging without sending requests
 */

import crypto from 'node:crypto';
import { httpRequest, logger, isDryRun, getPackageMeta } from './utils.mjs';

/**
 * Helper to escape special characters for Telegram MarkdownV2
 * Reserved characters: _ * [ ] ( ) ~ ` > # + - = | { } . !
 */
function escapeTelegramV2(str) {
  if (!str) return '';
  return str.replace(/([_*\[\]()~`>#+\-=|{}.!\\])/g, '\\$1');
}

/**
 * 1. Broadcast to Bluesky (AT Protocol)
 */
async function broadcastBluesky(meta, highlights, dryRun) {
  const handle = process.env.BLUESKY_HANDLE;
  const password = process.env.BLUESKY_APP_PASSWORD;
  const pdsService = (process.env.BLUESKY_SERVICE || 'https://bsky.social').replace(/\/$/, '');

  logger.divider();
  logger.info('🌐 [Bluesky (AT Protocol)] Inspecting credentials...');

  if (!handle || !password) {
    logger.warn('⚠️ [Bluesky] BLUESKY_HANDLE or BLUESKY_APP_PASSWORD not set.');
    logger.warn('⚠️ [Bluesky] Skipping Bluesky broadcast gracefully.');
    return { channel: 'Bluesky', skipped: true, reason: 'Credentials missing' };
  }

  const postText = `🚀 VidNest ${meta.version} is officially released!

100% Local-First AI Video Intelligence & Management Tool.
• Sub-millisecond FTS5 search
• Tauri 2 + Rust native core (<80MB RAM)
• Zero cloud upload, absolute privacy

🔗 Release & Downloads: ${meta.releaseUrl}
#opensource #devtools #localfirst #tauri #rust`;

  const payload = {
    handle,
    postText,
    collection: 'app.bsky.feed.post'
  };

  if (dryRun) {
    logger.dryRun('Simulating Bluesky broadcast:');
    logger.dryRun(`Endpoint: ${pdsService}/xrpc/com.atproto.server.createSession`);
    logger.dryRun(`Post Text:\n${postText}`);
    logger.success('✅ [Bluesky DRY_RUN] Simulated post successful.');
    return { channel: 'Bluesky', success: true, simulated: true };
  }

  try {
    logger.info(`Authenticating with Bluesky PDS (${pdsService}) for @${handle}...`);
    // Step 1: Create Session
    const sessionRes = await httpRequest(`${pdsService}/xrpc/com.atproto.server.createSession`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ identifier: handle, password })
    });

    if (!sessionRes.ok) {
      throw new Error(`Session creation failed (HTTP ${sessionRes.status}): ${sessionRes.rawText}`);
    }

    const session = sessionRes.data;

    // Step 2: Create Record (Post)
    logger.info('Submitting release record to AT Protocol repo...');
    const recordRes = await httpRequest(`${pdsService}/xrpc/com.atproto.repo.createRecord`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${session.accessJwt}`
      },
      body: JSON.stringify({
        repo: session.did,
        collection: 'app.bsky.feed.post',
        record: {
          text: postText,
          createdAt: new Date().toISOString()
        }
      })
    });

    if (recordRes.ok) {
      logger.success('✅ [Bluesky] Successfully broadcasted release post!');
      return { channel: 'Bluesky', success: true, data: recordRes.data };
    } else {
      throw new Error(`Record creation failed (HTTP ${recordRes.status}): ${recordRes.rawText}`);
    }
  } catch (err) {
    logger.error(`❌ [Bluesky] Broadcast failed: ${err.message}`);
    return { channel: 'Bluesky', success: false, error: err.message };
  }
}

/**
 * 2. Broadcast to Mastodon (Fediverse)
 */
async function broadcastMastodon(meta, highlights, dryRun) {
  const instance = (process.env.MASTODON_INSTANCE || 'https://mastodon.social').replace(/\/$/, '');
  const token = process.env.MASTODON_ACCESS_TOKEN;

  logger.divider();
  logger.info(`🐘 [Mastodon] Inspecting Fediverse credentials for ${instance}...`);

  if (!token) {
    logger.warn('⚠️ [Mastodon] MASTODON_ACCESS_TOKEN is not defined.');
    logger.warn('⚠️ [Mastodon] Skipping Mastodon broadcast gracefully.');
    return { channel: 'Mastodon', skipped: true, reason: 'MASTODON_ACCESS_TOKEN missing' };
  }

  const statusText = `🚀 VidNest ${meta.version} has been released!

Local-First AI Video Intelligence & Management Tool (Tauri 2 + Rust + Vue 3).

✨ Highlights:
• 100% Local-First: Zero cloud upload, absolute privacy
• SQLite + FTS5 sub-millisecond full-text search
• Native performance: <80MB RAM footprint
• Integrated FFmpeg & yt-dlp media pipelines

📦 Release & Binaries:
${meta.releaseUrl}

#opensource #devtools #localfirst #tauri #rust #privacy #software`;

  const endpoint = `${instance}/api/v1/statuses`;

  if (dryRun) {
    logger.dryRun(`Simulating Mastodon POST request to ${endpoint}`);
    logger.dryRun(`Toot Content:\n${statusText}`);
    logger.success('✅ [Mastodon DRY_RUN] Simulated toot successful.');
    return { channel: 'Mastodon', success: true, simulated: true };
  }

  try {
    logger.info(`Sending toot to ${endpoint}...`);
    const res = await httpRequest(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`
      },
      body: JSON.stringify({
        status: statusText,
        visibility: 'public'
      })
    });

    if (res.ok) {
      logger.success(`✅ [Mastodon] Successfully posted to ${instance}!`);
      return { channel: 'Mastodon', success: true, data: res.data };
    } else {
      throw new Error(`HTTP ${res.status}: ${res.rawText}`);
    }
  } catch (err) {
    logger.error(`❌ [Mastodon] Broadcast failed: ${err.message}`);
    return { channel: 'Mastodon', success: false, error: err.message };
  }
}

/**
 * 3. Broadcast to Feishu / Lark (Interactive Card Webhook)
 */
async function broadcastFeishu(meta, highlights, dryRun) {
  const webhookUrl = process.env.FEISHU_WEBHOOK_URL;

  logger.divider();
  logger.info('📱 [Feishu / Lark] Inspecting interactive card webhook...');

  if (!webhookUrl) {
    logger.warn('⚠️ [Feishu] FEISHU_WEBHOOK_URL is not defined.');
    logger.warn('⚠️ [Feishu] Skipping Feishu card broadcast gracefully.');
    return { channel: 'Feishu', skipped: true, reason: 'FEISHU_WEBHOOK_URL missing' };
  }

  const cardPayload = {
    msg_type: 'interactive',
    card: {
      config: {
        wide_screen_mode: true,
        enable_forward: true
      },
      header: {
        title: {
          tag: 'plain_text',
          content: `🚀 VidNest ${meta.version} 正式发版上线通知`
        },
        template: 'blue'
      },
      elements: [
        {
          tag: 'div',
          text: {
            tag: 'lark_md',
            content: `**版本编号**: <font color='green'>${meta.version}</font>\n**技术栈**: Tauri 2 (Rust) + Vue 3 + SQLite FTS5\n**发布时间**: ${new Date().toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })}\n\n**🔥 核心特性与发版亮点**:\n• **100% Local-First 隐私优先**：全流程纯本地运行，零数据上云，原创与商业敏感素材绝对安全\n• **毫秒级极速检索**：SQLite + FTS5 全文索引，十万级视频资产瞬间过滤呈现\n• **极简原生资源占用**：Rust 原生内核，内存占用 <80MB，彻底告别传统工具卡顿\n• **多媒体全能管线**：内置 FFmpeg/FFprobe 与 yt-dlp 本地智能处理管线`
          }
        },
        {
          tag: 'hr'
        },
        {
          tag: 'action',
          actions: [
            {
              tag: 'button',
              text: {
                tag: 'plain_text',
                content: '📥 下载最新客户端安装包'
              },
              type: 'primary',
              url: meta.releaseUrl
            },
            {
              tag: 'button',
              text: {
                tag: 'plain_text',
                content: '🌐 访问官方网站 & 竞品对比'
              },
              type: 'default',
              url: meta.siteUrl
            },
            {
              tag: 'button',
              text: {
                tag: 'plain_text',
                content: '⭐ GitHub 源码仓库'
              },
              type: 'default',
              url: meta.repoUrl
            }
          ]
        }
      ]
    }
  };

  if (dryRun) {
    logger.dryRun('Simulating Feishu Interactive Card Webhook:');
    logger.dryRun(JSON.stringify(cardPayload, null, 2));
    logger.success('✅ [Feishu DRY_RUN] Simulated card push successful.');
    return { channel: 'Feishu', success: true, simulated: true };
  }

  try {
    logger.info('Pushing interactive card to Feishu webhook...');
    const res = await httpRequest(webhookUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(cardPayload)
    });

    if (res.ok && (!res.data?.code || res.data.code === 0)) {
      logger.success('✅ [Feishu] Successfully dispatched interactive card!');
      return { channel: 'Feishu', success: true, data: res.data };
    } else {
      throw new Error(`Feishu response code ${res.data?.code}: ${res.data?.msg || res.rawText}`);
    }
  } catch (err) {
    logger.error(`❌ [Feishu] Broadcast failed: ${err.message}`);
    return { channel: 'Feishu', success: false, error: err.message };
  }
}

/**
 * 4. Broadcast to DingTalk / 钉钉 (Markdown Webhook with HMAC signature)
 */
async function broadcastDingTalk(meta, highlights, dryRun) {
  let webhookUrl = process.env.DINGTALK_WEBHOOK_URL;
  const secret = process.env.DINGTALK_SECRET;

  logger.divider();
  logger.info('🔔 [DingTalk] Inspecting DingTalk robot webhook...');

  if (!webhookUrl) {
    logger.warn('⚠️ [DingTalk] DINGTALK_WEBHOOK_URL is not defined.');
    logger.warn('⚠️ [DingTalk] Skipping DingTalk broadcast gracefully.');
    return { channel: 'DingTalk', skipped: true, reason: 'DINGTALK_WEBHOOK_URL missing' };
  }

  // Calculate HMAC-SHA256 signature if secret is provided
  if (secret) {
    const timestamp = Date.now();
    const stringToSign = `${timestamp}\n${secret}`;
    const sign = crypto
      .createHmac('sha256', secret)
      .update(stringToSign, 'utf8')
      .digest('base64');
    const encodedSign = encodeURIComponent(sign);
    webhookUrl += (webhookUrl.includes('?') ? '&' : '?') + `timestamp=${timestamp}&sign=${encodedSign}`;
  }

  const markdownText = `### 🚀 VidNest ${meta.version} 正式发版通知

**Local-First 本地 AI 视频智能管理工具 (Tauri 2 + Rust)**

---

#### 🔥 核心亮点与更新特性:
- **100% Local-First**：完全离线运行，零云端上传，商业隐私绝对安全
- **毫秒级极速检索**：SQLite + FTS5 全文引擎，百万视频瞬间响应
- **原生极速性能**：Rust 原生内核，内存占用低于 80MB
- **智能媒体管线**：内置 FFmpeg/FFprobe 与 yt-dlp 本地智能处理

---

[📥 前往 GitHub 下载安装包](${meta.releaseUrl}) | [🌐 访问官方主页](${meta.siteUrl}) | [⭐ GitHub Star](${meta.repoUrl})`;

  const payload = {
    msgtype: 'markdown',
    markdown: {
      title: `🚀 VidNest ${meta.version} 发版通知`,
      text: markdownText
    }
  };

  if (dryRun) {
    logger.dryRun('Simulating DingTalk Markdown Webhook:');
    logger.dryRun(JSON.stringify(payload, null, 2));
    logger.success('✅ [DingTalk DRY_RUN] Simulated DingTalk notification successful.');
    return { channel: 'DingTalk', success: true, simulated: true };
  }

  try {
    logger.info('Pushing Markdown notification to DingTalk...');
    const res = await httpRequest(webhookUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    if (res.ok && (!res.data?.errcode || res.data.errcode === 0)) {
      logger.success('✅ [DingTalk] Successfully broadcasted markdown card!');
      return { channel: 'DingTalk', success: true, data: res.data };
    } else {
      throw new Error(`DingTalk errcode ${res.data?.errcode}: ${res.data?.errmsg || res.rawText}`);
    }
  } catch (err) {
    logger.error(`❌ [DingTalk] Broadcast failed: ${err.message}`);
    return { channel: 'DingTalk', success: false, error: err.message };
  }
}

/**
 * 5. Broadcast to Discord Webhook (Rich Embed)
 */
async function broadcastDiscord(meta, highlights, dryRun) {
  const webhookUrl = process.env.DISCORD_WEBHOOK_URL;

  logger.divider();
  logger.info('👾 [Discord] Inspecting Discord webhook...');

  if (!webhookUrl) {
    logger.warn('⚠️ [Discord] DISCORD_WEBHOOK_URL is not defined.');
    logger.warn('⚠️ [Discord] Skipping Discord broadcast gracefully.');
    return { channel: 'Discord', skipped: true, reason: 'DISCORD_WEBHOOK_URL missing' };
  }

  const embedPayload = {
    username: 'VidNest Release Dispatcher',
    embeds: [
      {
        title: `🚀 VidNest ${meta.version} is now available!`,
        url: meta.releaseUrl,
        description: `A brand-new version of **VidNest** (Local-First AI Video Intelligence & Management Tool) has just been released!`,
        color: 0x3b82f6, // Royal Blue
        fields: [
          {
            name: '✨ Highlights',
            value: [
              '• **Local-First & Private**: 100% offline, zero cloud uploads.',
              '• **Sub-millisecond Search**: SQLite + FTS5 full-text engine.',
              '• **Lightweight Native Core**: Tauri 2 + Rust (<80MB RAM).',
              '• **Built-in Media Pipeline**: FFmpeg & yt-dlp integrated.'
            ].join('\n'),
            inline: false
          },
          {
            name: '📦 Download Links',
            value: `[GitHub Releases](${meta.releaseUrl}) · [Official Site](${meta.siteUrl})`,
            inline: false
          }
        ],
        footer: {
          text: 'VidNest Open Source Ecosystem'
        },
        timestamp: new Date().toISOString()
      }
    ]
  };

  if (dryRun) {
    logger.dryRun('Simulating Discord Embed Webhook:');
    logger.dryRun(JSON.stringify(embedPayload, null, 2));
    logger.success('✅ [Discord DRY_RUN] Simulated Discord embed successful.');
    return { channel: 'Discord', success: true, simulated: true };
  }

  try {
    logger.info('Dispatching embed to Discord webhook...');
    const res = await httpRequest(webhookUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(embedPayload)
    });

    if (res.ok) {
      logger.success('✅ [Discord] Successfully dispatched embed notification!');
      return { channel: 'Discord', success: true };
    } else {
      throw new Error(`HTTP ${res.status}: ${res.rawText}`);
    }
  } catch (err) {
    logger.error(`❌ [Discord] Broadcast failed: ${err.message}`);
    return { channel: 'Discord', success: false, error: err.message };
  }
}

/**
 * 6. Broadcast to Telegram Bot (MarkdownV2)
 */
async function broadcastTelegram(meta, highlights, dryRun) {
  const botToken = process.env.TELEGRAM_BOT_TOKEN;
  const chatId = process.env.TELEGRAM_CHAT_ID;

  logger.divider();
  logger.info('✈️ [Telegram] Inspecting Telegram Bot credentials...');

  if (!botToken || !chatId) {
    logger.warn('⚠️ [Telegram] TELEGRAM_BOT_TOKEN or TELEGRAM_CHAT_ID is not defined.');
    logger.warn('⚠️ [Telegram] Skipping Telegram broadcast gracefully.');
    return { channel: 'Telegram', skipped: true, reason: 'Credentials missing' };
  }

  const endpoint = `https://api.telegram.org/bot${botToken}/sendMessage`;
  const escapedVersion = escapeTelegramV2(meta.version);

  const telegramText = `🚀 *VidNest ${escapedVersion} 正式发版上线\\!*

*Local\\-First 本地 AI 视频智能管理工具*

🔥 *核心亮点*:
• 100% Local\\-First 隐私安全，零数据上云
• SQLite \\+ FTS5 毫秒级全文检索
• Tauri 2 \\+ Rust 原生轻量架构 \\(<80MB 内存\\)
• 内置 FFmpeg 与 yt\\-dlp 本地多媒体管线

🔗 [点击查看 GitHub Releases 下载安装包](${meta.releaseUrl})
🌐 [访问官网与落地页](${meta.siteUrl})`;

  const payload = {
    chat_id: chatId,
    text: telegramText,
    parse_mode: 'MarkdownV2',
    disable_web_page_preview: false
  };

  if (dryRun) {
    logger.dryRun(`Simulating Telegram Bot POST request to https://api.telegram.org/bot***/sendMessage`);
    logger.dryRun(`Chat ID: ${chatId}`);
    logger.dryRun(`Message Content:\n${telegramText}`);
    logger.success('✅ [Telegram DRY_RUN] Simulated Telegram broadcast successful.');
    return { channel: 'Telegram', success: true, simulated: true };
  }

  try {
    logger.info(`Sending MarkdownV2 notification to Telegram chat ${chatId}...`);
    const res = await httpRequest(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    if (res.ok && res.data?.ok) {
      logger.success('✅ [Telegram] Successfully dispatched Telegram message!');
      return { channel: 'Telegram', success: true, data: res.data };
    } else {
      throw new Error(`Telegram API Error: ${res.data?.description || res.rawText}`);
    }
  } catch (err) {
    logger.error(`❌ [Telegram] Broadcast failed: ${err.message}`);
    return { channel: 'Telegram', success: false, error: err.message };
  }
}

/**
 * Main Broadcast Runner
 */
export async function main() {
  const meta = getPackageMeta();
  const dryRun = isDryRun();

  console.log('🚀============================================================');
  console.log(`   VidNest Omnichannel Release Broadcast Engine (${meta.version})`);
  console.log(`   Dry-run Mode: ${dryRun ? 'ENABLED' : 'DISABLED'}`);
  console.log(`   Release URL:  ${meta.releaseUrl}`);
  console.log('============================================================');

  const highlights = [
    '100% Local-First 隐私安全，数据绝对不出本地',
    'SQLite + FTS5 毫秒级全文索引，百万视频即刻响应',
    'Tauri 2 + Rust 原生极速内核，超低内存占用 (<80MB)',
    '内置 FFmpeg/FFprobe 与 yt-dlp 本地下载与媒体管线'
  ];

  // Execute all channels in parallel / isolated sequence
  const results = await Promise.all([
    broadcastBluesky(meta, highlights, dryRun),
    broadcastMastodon(meta, highlights, dryRun),
    broadcastFeishu(meta, highlights, dryRun),
    broadcastDingTalk(meta, highlights, dryRun),
    broadcastDiscord(meta, highlights, dryRun),
    broadcastTelegram(meta, highlights, dryRun)
  ]);

  logger.divider();
  logger.info('📊 [Omnichannel Broadcast Summary]');
  let successCount = 0;
  let skippedCount = 0;
  let failedCount = 0;

  for (const r of results) {
    const status = r.skipped ? `SKIPPED (${r.reason})` : r.success ? 'SUCCESS' : `FAILED (${r.error})`;
    const icon = r.skipped ? '⚪' : r.success ? '🟢' : '🔴';
    console.log(`   ${icon} ${r.channel.padEnd(12)}: ${status}`);

    if (r.skipped) skippedCount++;
    else if (r.success) successCount++;
    else failedCount++;
  }

  logger.divider();
  logger.info(`Done: ${successCount} sent/simulated, ${skippedCount} skipped, ${failedCount} failed.`);

  return { results, successCount, skippedCount, failedCount };
}

if (process.argv[1] && process.argv[1].endsWith('broadcast-release.mjs')) {
  main().catch(err => {
    logger.error(`Fatal error in broadcast engine: ${err.stack || err.message}`);
    process.exit(1);
  });
}
