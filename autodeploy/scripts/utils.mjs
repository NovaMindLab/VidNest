import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import https from 'node:https';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
export const PROJECT_ROOT = path.resolve(__dirname, '../../');

/**
 * Check if current execution is in DRY_RUN mode
 */
export function isDryRun() {
  const envVal = (process.env.DRY_RUN || '').toLowerCase();
  return envVal === 'true' || envVal === '1' || process.argv.includes('--dry-run');
}

/**
 * ANSI Color Logger
 */
export const logger = {
  info: (msg, ...args) => console.log(`\x1b[34m[INFO]\x1b[0m ${msg}`, ...args),
  success: (msg, ...args) => console.log(`\x1b[32m[SUCCESS]\x1b[0m ${msg}`, ...args),
  warn: (msg, ...args) => console.warn(`\x1b[33m[WARN]\x1b[0m ${msg}`, ...args),
  error: (msg, ...args) => console.error(`\x1b[31m[ERROR]\x1b[0m ${msg}`, ...args),
  dryRun: (msg, ...args) => console.log(`\x1b[35m[DRY_RUN]\x1b[0m ${msg}`, ...args),
  divider: () => console.log('\x1b[90m------------------------------------------------------------\x1b[0m')
};

/**
 * Universal HTTP Request Helper
 * Works on modern Node (native fetch) and falls back to node:https/http on older runtimes.
 */
export async function httpRequest(urlStr, options = {}) {
  const method = options.method || 'GET';
  const headers = options.headers || {};
  const body = options.body;
  const timeoutMs = options.timeout || 15000;

  if (typeof globalThis.fetch === 'function') {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetch(urlStr, {
        method,
        headers,
        body,
        signal: controller.signal
      });
      clearTimeout(timer);
      const text = await response.text();
      let data = text;
      try {
        data = JSON.parse(text);
      } catch {
        // preserve text if not JSON
      }
      return {
        ok: response.ok,
        status: response.status,
        statusText: response.statusText,
        data,
        rawText: text
      };
    } catch (err) {
      clearTimeout(timer);
      throw err;
    }
  }

  // Fallback to node:https / node:http
  return new Promise((resolve, reject) => {
    const parsedUrl = new URL(urlStr);
    const client = parsedUrl.protocol === 'https:' ? https : http;
    const reqOptions = {
      method,
      hostname: parsedUrl.hostname,
      port: parsedUrl.port || (parsedUrl.protocol === 'https:' ? 443 : 80),
      path: `${parsedUrl.pathname}${parsedUrl.search}`,
      headers,
      timeout: timeoutMs
    };

    const req = client.request(reqOptions, (res) => {
      let chunks = '';
      res.setEncoding('utf8');
      res.on('data', (chunk) => {
        chunks += chunk;
      });
      res.on('end', () => {
        let data = chunks;
        try {
          data = JSON.parse(chunks);
        } catch {
          // not JSON
        }
        resolve({
          ok: res.statusCode >= 200 && res.statusCode < 300,
          status: res.statusCode,
          statusText: res.statusMessage,
          data,
          rawText: chunks
        });
      });
    });

    req.on('error', (err) => reject(err));
    req.on('timeout', () => {
      req.destroy();
      reject(new Error(`Request timeout after ${timeoutMs}ms`));
    });

    if (body) {
      req.write(typeof body === 'string' ? body : JSON.stringify(body));
    }
    req.end();
  });
}

/**
 * Retrieve metadata from package.json and Git info
 */
export function getPackageMeta() {
  const pkgPath = path.join(PROJECT_ROOT, 'package.json');
  let pkg = { name: 'VidNest', version: '0.1.0' };
  try {
    if (fs.existsSync(pkgPath)) {
      pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
    }
  } catch (err) {
    logger.warn(`Could not parse package.json: ${err.message}`);
  }

  const version = process.env.RELEASE_VERSION || process.env.RELEASE_TAG || `v${pkg.version || '0.1.0'}`;
  const normalizedVersion = version.startsWith('v') ? version : `v${version}`;
  const repo = process.env.GITHUB_REPOSITORY || 'NovaMindLab/VidNest';
  const siteUrl = (process.env.SITE_URL || 'https://novamindlab.github.io/VidNest').replace(/\/$/, '');
  const siteHost = (process.env.SITE_HOST || new URL(siteUrl).hostname);

  return {
    name: 'VidNest',
    description: 'Local-First AI Video Intelligence & Management Tool',
    version: normalizedVersion,
    rawVersion: normalizedVersion.replace(/^v/, ''),
    repo,
    repoUrl: `https://github.com/${repo}`,
    releaseUrl: `https://github.com/${repo}/releases/tag/${normalizedVersion}`,
    siteUrl,
    siteHost
  };
}
