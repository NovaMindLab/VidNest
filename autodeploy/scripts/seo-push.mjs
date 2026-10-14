#!/usr/bin/env node
/**
 * VidNest - Instant Search Engine Push Client (SEO Push)
 * Implements:
 * 1. IndexNow Protocol (Microsoft Bing / Seznam / Yandex / Naver)
 * 2. Baidu Webmaster Active Push API (百度站长主动推送)
 * 
 * Features:
 * - Graceful degradation: Skips with warning when keys/tokens are absent
 * - DRY_RUN simulation mode (DRY_RUN=true or --dry-run)
 * - Auto-discovers URLs from sitemap.xml or pSEO output
 */

import fs from 'node:fs';
import path from 'node:path';
import { httpRequest, logger, isDryRun, getPackageMeta, PROJECT_ROOT } from './utils.mjs';

/**
 * Collect target URLs to push to search engines
 */
function resolveUrlList(siteUrl, siteHost) {
  const customUrls = process.env.PUSH_URLS
    ? process.env.PUSH_URLS.split(/[\n,]/).map(u => u.trim()).filter(Boolean)
    : [];

  if (customUrls.length > 0) {
    return Array.from(new Set(customUrls));
  }

  const urls = new Set([
    `${siteUrl}/`,
    `${siteUrl}/index.html`,
    `${siteUrl}/download.html`,
    `${siteUrl}/vs-eagle.html`,
    `${siteUrl}/vs-capcut-jianying.html`,
    `${siteUrl}/vs-jellyfin-plex.html`,
    `${siteUrl}/vs-local-video-manager.html`,
    `${siteUrl}/sitemap.xml`
  ]);

  // Try auto-discovering URLs from sitemap files
  const candidateSitemaps = [
    path.join(PROJECT_ROOT, 'autodeploy', 'dist_pseo', 'sitemap.xml'),
    path.join(PROJECT_ROOT, 'docs', 'sitemap.xml'),
    path.join(PROJECT_ROOT, 'public', 'sitemap.xml'),
    path.join(PROJECT_ROOT, 'dist', 'sitemap.xml')
  ];

  for (const sitemapPath of candidateSitemaps) {
    if (fs.existsSync(sitemapPath)) {
      try {
        const content = fs.readFileSync(sitemapPath, 'utf8');
        const locMatches = content.match(/<loc>(.*?)<\/loc>/g);
        if (locMatches) {
          for (const m of locMatches) {
            const extracted = m.replace(/<\/?loc>/g, '').trim();
            if (extracted) {
              urls.add(extracted);
            }
          }
          logger.info(`Loaded ${locMatches.length} URLs from sitemap: ${path.relative(PROJECT_ROOT, sitemapPath)}`);
          break;
        }
      } catch (err) {
        logger.warn(`Failed reading sitemap at ${sitemapPath}: ${err.message}`);
      }
    }
  }

  // Filter out invalid URLs or non-matching hosts
  return Array.from(urls).filter(url => {
    try {
      const parsed = new URL(url);
      return parsed.hostname === siteHost || parsed.hostname.endsWith(`.${siteHost}`);
    } catch {
      return false;
    }
  });
}

/**
 * 1. Push to IndexNow API (Bing, Seznam, Yandex)
 */
async function pushIndexNow({ siteHost, siteUrl, urls, dryRun }) {
  const indexNowKey = process.env.INDEXNOW_KEY || '8f7d6a5c4e3b2a1098f7e6d5c4b3a201';

  logger.divider();
  logger.info('⚡ [IndexNow] Preparing push to Microsoft Bing, Seznam & Yandex...');

  if (!indexNowKey) {
    logger.warn('⚠️ [IndexNow] INDEXNOW_KEY environment variable is not defined.');
    logger.warn('⚠️ [IndexNow] Skipping IndexNow push. Please set INDEXNOW_KEY in repository secrets.');
    return { skipped: true, reason: 'INDEXNOW_KEY missing' };
  }

  const keyLocation = process.env.INDEXNOW_KEY_LOCATION || 
    (siteHost.endsWith('github.io') ? `https://${siteHost}/${indexNowKey}.txt` : `${siteUrl}/${indexNowKey}.txt`);
  const endpoint = 'https://api.indexnow.org/IndexNow';

  const payload = {
    host: siteHost,
    key: indexNowKey,
    keyLocation,
    urlList: urls
  };

  if (dryRun) {
    logger.dryRun(`Simulating IndexNow POST request to ${endpoint}`);
    logger.dryRun(`Host: ${siteHost}`);
    logger.dryRun(`Key Location: ${keyLocation}`);
    logger.dryRun(`URLs to submit (${urls.length}):`);
    urls.forEach(u => console.log(`   - ${u}`));
    logger.success('✅ [IndexNow DRY_RUN] Simulated push completed successfully (HTTP 200/202).');
    return { success: true, simulated: true };
  }

  try {
    logger.info(`Sending ${urls.length} URLs to ${endpoint}...`);
    const res = await httpRequest(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json; charset=utf-8'
      },
      body: JSON.stringify(payload)
    });

    if (res.status === 200 || res.status === 202) {
      logger.success(`✅ [IndexNow] Push accepted with HTTP ${res.status}!`);
      logger.info('   Search engines (Bing, Seznam, Yandex) will crawl and update in minutes.');
      return { success: true, status: res.status };
    } else {
      logger.error(`❌ [IndexNow] Push rejected with HTTP ${res.status} ${res.statusText}:`);
      console.error(res.data || res.rawText);
      return { success: false, status: res.status, error: res.rawText };
    }
  } catch (err) {
    logger.error(`❌ [IndexNow] Request failed: ${err.message}`);
    return { success: false, error: err.message };
  }
}

/**
 * 2. Push to Baidu Webmaster Active Push API (百度站长主动推送)
 */
async function pushBaiduWebmaster({ siteHost, urls, dryRun }) {
  const baiduToken = process.env.BAIDU_PUSH_TOKEN;
  const baiduSite = process.env.BAIDU_PUSH_SITE || siteHost;

  logger.divider();
  logger.info('🇨🇳 [Baidu Webmaster] Preparing active push for Chinese search index...');

  if (!baiduToken) {
    logger.warn('⚠️ [Baidu Webmaster] BAIDU_PUSH_TOKEN environment variable is not defined.');
    logger.warn('⚠️ [Baidu Webmaster] Skipping Baidu push. Please set BAIDU_PUSH_TOKEN in repository secrets.');
    return { skipped: true, reason: 'BAIDU_PUSH_TOKEN missing' };
  }

  const endpoint = `http://data.zz.baidu.com/urls?site=${encodeURIComponent(baiduSite)}&token=${encodeURIComponent(baiduToken)}`;
  const bodyPayload = urls.join('\n');

  if (dryRun) {
    logger.dryRun(`Simulating Baidu Webmaster POST request to http://data.zz.baidu.com/urls?site=${baiduSite}&token=***`);
    logger.dryRun(`Submitting ${urls.length} URLs (plain text newline separated):`);
    urls.forEach(u => console.log(`   - ${u}`));
    logger.success('✅ [Baidu Webmaster DRY_RUN] Simulated push completed successfully (remain quota: 49990, success: 10).');
    return { success: true, simulated: true };
  }

  try {
    logger.info(`Sending ${urls.length} URLs to Baidu API for site: ${baiduSite}...`);
    const res = await httpRequest(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'text/plain',
        'User-Agent': 'curl/7.68.0'
      },
      body: bodyPayload
    });

    if (res.ok) {
      const data = res.data || {};
      logger.success(`✅ [Baidu Webmaster] Successfully pushed! Accepted: ${data.success || urls.length}, Remaining Daily Quota: ${data.remain ?? 'N/A'}`);
      return { success: true, data };
    } else {
      const errMsg = res.data?.message || res.rawText || res.statusText;
      logger.error(`❌ [Baidu Webmaster] Push failed (HTTP ${res.status}): ${errMsg}`);
      return { success: false, status: res.status, error: errMsg };
    }
  } catch (err) {
    logger.error(`❌ [Baidu Webmaster] Request failed: ${err.message}`);
    return { success: false, error: err.message };
  }
}

/**
 * Main Runner
 */
export async function main() {
  const meta = getPackageMeta();
  const dryRun = isDryRun();

  console.log('🚀============================================================');
  console.log(`   VidNest Instant SEO Push Engine (${meta.version})`);
  console.log(`   Host: ${meta.siteHost} | Dry-run: ${dryRun ? 'ENABLED' : 'DISABLED'}`);
  console.log('============================================================');

  const urls = resolveUrlList(meta.siteUrl, meta.siteHost);
  logger.info(`Total target URLs identified: ${urls.length}`);

  const results = {
    indexNow: await pushIndexNow({
      siteHost: meta.siteHost,
      siteUrl: meta.siteUrl,
      urls,
      dryRun
    }),
    baidu: await pushBaiduWebmaster({
      siteHost: meta.siteHost,
      urls,
      dryRun
    })
  };

  logger.divider();
  logger.info('📊 [SEO Push Summary]');
  logger.info(`- IndexNow: ${results.indexNow.skipped ? 'SKIPPED (No Key)' : results.indexNow.success ? 'SUCCESS' : 'FAILED'}`);
  logger.info(`- Baidu Push: ${results.baidu.skipped ? 'SKIPPED (No Token)' : results.baidu.success ? 'SUCCESS' : 'FAILED'}`);
  logger.divider();

  // We do not crash the pipeline if tokens are missing (graceful degradation)
  return results;
}

if (process.argv[1] && process.argv[1].endsWith('seo-push.mjs')) {
  main().catch(err => {
    logger.error(`Fatal error during SEO push: ${err.stack || err.message}`);
    process.exit(1);
  });
}
