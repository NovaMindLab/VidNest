#!/usr/bin/env node
/**
 * ============================================================================
 * VidNest - Universal Package Manifest Generator (Node.js ESM)
 * ============================================================================
 * Generates official package manager manifests for:
 *   1. Microsoft WinGet (3-file spec: installer, version, locale)
 *   2. Scoop (JSON manifest with url, hash, bin, shortcuts)
 *   3. Homebrew Cask (Ruby .rb file for macOS)
 *
 * Supports:
 *   - Live extraction from GitHub Releases API + streaming SHA-256 calculation
 *   - Offline --dry-run mode with simulated assets and deterministic hashes
 *   - Explicit CLI overrides for URLs and hashes
 * ============================================================================
 */

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '..', '..');

// ----------------------------------------------------------------------------
// 1. Defaults & Metadata Configuration
// ----------------------------------------------------------------------------
const CONFIG = {
  publisher: 'NovaMindLab',
  appName: 'VidNest',
  productTitle: 'Local Video Intelligence',
  packageId: 'NovaMindLab.VidNest',
  repo: 'NovaMindLab/VidNest',
  homepage: 'https://github.com/NovaMindLab/VidNest',
  license: 'MIT',
  licenseUrl: 'https://github.com/NovaMindLab/VidNest/blob/main/LICENSE',
  defaultLocale: 'en-US',
  shortDescription: 'Local-first video analysis, indexing, and intelligent summarization desktop app.',
  description: 'VidNest (Local Video Intelligence) is an enterprise-grade, privacy-first desktop application engineered for local video indexing, transcription, and intelligent multi-modal analysis powered by Tauri and Vue.',
  tags: ['video', 'intelligence', 'transcription', 'local-first', 'ai', 'tauri', 'vue'],
  bundleId: 'com.vidnest.localvideointelligence',
  minWindowsOS: '10.0.0.0',
  manifestVersion: '1.6.0',
};

// ----------------------------------------------------------------------------
// 2. CLI Argument Parser
// ----------------------------------------------------------------------------
function printHelp() {
  console.log(`
VidNest Manifest Generator

Usage:
  node autodeploy/scripts/generate-package-manifests.mjs [options]

Options:
  --dry-run             Run offline using mock assets and mock SHA-256 hashes
  --version <version>   Target release version (e.g. 0.1.0, 1.0.0) [default: from package.json]
  --tag <tag>           GitHub release tag (default: v<version>)
  --repo <owner/repo>   GitHub repository [default: ${CONFIG.repo}]
  --token <token>       GitHub Personal Access Token for API calls (or env GITHUB_TOKEN)
  --out-dir <dir>       Output directory for manifests [default: autodeploy/manifests]

Overrides (bypasses GitHub asset download):
  --exe-url <url>       Download URL for Windows x64 NSIS setup installer (.exe)
  --exe-hash <sha256>   SHA-256 hash for Windows x64 setup installer
  --zip-url <url>       Download URL for Scoop portable package (.zip or .exe)
  --zip-hash <sha256>   SHA-256 hash for Scoop package
  --dmg-url <url>       Download URL for macOS DMG installer (.dmg)
  --dmg-hash <sha256>   SHA-256 hash for macOS DMG installer

  -h, --help            Show this help manual
`);
}

function parseArgs(args) {
  const envDryRun = (process.env.DRY_RUN || '').toLowerCase();
  const options = {
    dryRun: envDryRun === 'true' || envDryRun === '1',
    version: null,
    tag: null,
    repo: CONFIG.repo,
    token: process.env.GITHUB_TOKEN || process.env.GH_TOKEN || null,
    outDir: path.join(REPO_ROOT, 'autodeploy', 'manifests'),
    exeUrl: null,
    exeHash: null,
    zipUrl: null,
    zipHash: null,
    dmgUrl: null,
    dmgHash: null,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--dry-run') {
      options.dryRun = true;
    } else if (arg === '-h' || arg === '--help') {
      printHelp();
      process.exit(0);
    } else if (arg === '--version' && args[i + 1]) {
      options.version = args[++i].replace(/^v/, '');
    } else if (arg === '--tag' && args[i + 1]) {
      options.tag = args[++i];
    } else if (arg === '--repo' && args[i + 1]) {
      options.repo = args[++i];
    } else if (arg === '--token' && args[i + 1]) {
      options.token = args[++i];
    } else if (arg === '--out-dir' && args[i + 1]) {
      options.outDir = path.resolve(process.cwd(), args[++i]);
    } else if (arg === '--exe-url' && args[i + 1]) {
      options.exeUrl = args[++i];
    } else if (arg === '--exe-hash' && args[i + 1]) {
      options.exeHash = args[++i].trim();
    } else if (arg === '--zip-url' && args[i + 1]) {
      options.zipUrl = args[++i];
    } else if (arg === '--zip-hash' && args[i + 1]) {
      options.zipHash = args[++i].trim();
    } else if (arg === '--dmg-url' && args[i + 1]) {
      options.dmgUrl = args[++i];
    } else if (arg === '--dmg-hash' && args[i + 1]) {
      options.dmgHash = args[++i].trim();
    }
  }

  // Auto-detect version if not supplied
  if (!options.version) {
    try {
      const pkgPath = path.join(REPO_ROOT, 'package.json');
      if (fs.existsSync(pkgPath)) {
        const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
        if (pkg.version) options.version = pkg.version;
      }
    } catch {
      // fallback
    }
  }

  if (!options.version) {
    options.version = '0.1.0';
  }

  if (!options.tag) {
    options.tag = `v${options.version}`;
  }

  return options;
}

// ----------------------------------------------------------------------------
// 3. Network & Hash Helpers
// ----------------------------------------------------------------------------
function fetchUrlBuffer(targetUrl, headers = {}) {
  return new Promise((resolve, reject) => {
    const defaultHeaders = {
      'User-Agent': 'VidNest-Manifest-Bot/1.0',
      ...headers,
    };

    const parsed = new URL(targetUrl);
    const client = parsed.protocol === 'http:' ? http : https;

    const req = client.get(
      targetUrl,
      { headers: defaultHeaders },
      (res) => {
        // Handle HTTP redirects (301, 302, 307, 308)
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return resolve(fetchUrlBuffer(res.headers.location, headers));
        }

        if (res.statusCode < 200 || res.statusCode >= 300) {
          return reject(
            new Error(`HTTP ${res.statusCode} ${res.statusMessage} while fetching ${targetUrl}`)
          );
        }

        const chunks = [];
        res.on('data', (chunk) => chunks.push(chunk));
        res.on('end', () => resolve(Buffer.concat(chunks)));
        res.on('error', reject);
      }
    );

    req.on('error', reject);
  });
}

async function fetchJson(targetUrl, token) {
  const headers = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  const buf = await fetchUrlBuffer(targetUrl, headers);
  return JSON.parse(buf.toString('utf8'));
}

async function computeSha256FromUrl(url) {
  console.log(`  ⏳ Downloading and hashing asset: ${url}`);
  const buf = await fetchUrlBuffer(url);
  const hash = crypto.createHash('sha256').update(buf).digest('hex');
  console.log(`  ✓ Computed SHA-256: ${hash}`);
  return hash;
}

function mockDeterministicHash(seed) {
  return crypto.createHash('sha256').update(seed).digest('hex');
}

// ----------------------------------------------------------------------------
// 4. Asset Resolution (Live GitHub Release vs. Dry Run)
// ----------------------------------------------------------------------------
async function resolveAssets(options) {
  const { version, tag, repo, token, dryRun } = options;

  if (dryRun) {
    console.log(`\n⚙️  [Dry-Run Mode] Generating mock release assets for ${repo}@${tag}...`);
    const mockExeUrl = `https://github.com/${repo}/releases/download/${tag}/Local.Video.Intelligence_${version}_x64-setup.exe`;
    const mockZipUrl = `https://github.com/${repo}/releases/download/${tag}/Local.Video.Intelligence_${version}_x64_portable.zip`;
    const mockDmgUrl = `https://github.com/${repo}/releases/download/${tag}/Local.Video.Intelligence_${version}_universal.dmg`;

    return {
      exe: {
        url: options.exeUrl || mockExeUrl,
        hash: options.exeHash || mockDeterministicHash(`vidnest-exe-${version}`),
      },
      scoop: {
        url: options.zipUrl || mockZipUrl,
        hash: options.zipHash || mockDeterministicHash(`vidnest-scoop-${version}`),
      },
      dmg: {
        url: options.dmgUrl || mockDmgUrl,
        hash: options.dmgHash || mockDeterministicHash(`vidnest-dmg-${version}`),
      },
    };
  }

  // If explicit URLs and hashes are provided, skip API lookup
  if (options.exeUrl && options.exeHash && options.zipUrl && options.zipHash && options.dmgUrl && options.dmgHash) {
    console.log('\n⚙️  Using provided asset URLs and SHA-256 hashes from CLI flags.');
    return {
      exe: { url: options.exeUrl, hash: options.exeHash },
      scoop: { url: options.zipUrl, hash: options.zipHash },
      dmg: { url: options.dmgUrl, hash: options.dmgHash },
    };
  }

  console.log(`\n🌐 Querying GitHub Releases API: ${repo} (tag: ${tag})...`);
  const apiUrl = `https://api.github.com/repos/${repo}/releases/tags/${tag}`;
  let releaseData;
  try {
    releaseData = await fetchJson(apiUrl, token);
  } catch (err) {
    console.warn(`  ⚠️  Failed to query release tag '${tag}': ${err.message}`);
    console.log('  🔄 Trying latest release endpoint...');
    releaseData = await fetchJson(`https://api.github.com/repos/${repo}/releases/latest`, token);
  }

  const assets = releaseData.assets || [];
  if (assets.length === 0) {
    throw new Error(`No assets found in GitHub release '${releaseData.tag_name || tag}'.`);
  }

  console.log(`  ✓ Found ${assets.length} release assets on GitHub.`);

  // Find assets
  const exeAsset = assets.find((a) => a.name.endsWith('.exe'));
  const zipAsset = assets.find((a) => a.name.endsWith('.zip'));
  const dmgAsset = assets.find((a) => a.name.endsWith('.dmg'));

  // Look for checksums asset (e.g. SHA256SUMS.txt)
  const checksumAsset = assets.find(
    (a) =>
      a.name.toLowerCase().includes('sha256') ||
      a.name.toLowerCase().includes('checksum')
  );

  let checksumMap = new Map();
  if (checksumAsset) {
    try {
      console.log(`  📄 Reading checksums file: ${checksumAsset.name}`);
      const rawBuf = await fetchUrlBuffer(checksumAsset.browser_download_url);
      const lines = rawBuf.toString('utf8').split(/\r?\n/);
      for (const line of lines) {
        const parts = line.trim().split(/\s+/);
        if (parts.length >= 2) {
          const hash = parts[0].toLowerCase();
          const filename = path.basename(parts[1].replace(/^\*/, ''));
          checksumMap.set(filename, hash);
        }
      }
    } catch (e) {
      console.warn(`  ⚠️  Could not parse checksum file: ${e.message}`);
    }
  }

  async function resolveAssetHash(asset, cliHash, cliUrl) {
    if (cliHash) return cliHash;
    if (!asset && !cliUrl) return null;
    const assetName = asset ? asset.name : path.basename(cliUrl);
    if (checksumMap.has(assetName)) {
      const h = checksumMap.get(assetName);
      console.log(`  ✓ Found SHA-256 for ${assetName} in checksum manifest: ${h}`);
      return h;
    }
    const downloadUrl = cliUrl || asset.browser_download_url;
    return await computeSha256FromUrl(downloadUrl);
  }

  const exeUrl = options.exeUrl || exeAsset?.browser_download_url;
  const exeHash = await resolveAssetHash(exeAsset, options.exeHash, exeUrl);

  const scoopUrl = options.zipUrl || zipAsset?.browser_download_url || exeUrl;
  const scoopHash = await resolveAssetHash(zipAsset || exeAsset, options.zipHash, scoopUrl);

  const dmgUrl = options.dmgUrl || dmgAsset?.browser_download_url;
  const dmgHash = await resolveAssetHash(dmgAsset, options.dmgHash, dmgUrl);

  if (!exeUrl || !exeHash) throw new Error('Could not resolve Windows installer (.exe) and hash.');
  if (!scoopUrl || !scoopHash) throw new Error('Could not resolve Scoop installer/zip and hash.');
  if (!dmgUrl || !dmgHash) throw new Error('Could not resolve macOS installer (.dmg) and hash.');

  return {
    exe: { url: exeUrl, hash: exeHash },
    scoop: { url: scoopUrl, hash: scoopHash },
    dmg: { url: dmgUrl, hash: dmgHash },
  };
}

// ----------------------------------------------------------------------------
// 5. Manifest Generators
// ----------------------------------------------------------------------------

/**
 * Generates WinGet 3-file package manifests matching Microsoft official schema:
 * 1. <PackageIdentifier>.installer.yaml
 * 2. <PackageIdentifier>.yaml
 * 3. <PackageIdentifier>.locale.<Locale>.yaml
 */
export function generateWinGetManifests(outDir, version, exeData) {
  const wingetDir = path.join(
    outDir,
    'winget',
    'NovaMindLab',
    'VidNest',
    version
  );
  fs.mkdirSync(wingetDir, { recursive: true });

  const installerYaml = `# Created using generate-package-manifests.mjs
# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.${CONFIG.manifestVersion}.schema.json

PackageIdentifier: ${CONFIG.packageId}
PackageVersion: ${version}
MinimumOSVersion: ${CONFIG.minWindowsOS}
InstallerType: nullsoft
InstallModes:
  - interactive
  - silent
InstallerSwitches:
  Silent: /S
  SilentWithProgress: /S
UpgradeBehavior: install
Installers:
  - Architecture: x64
    InstallerType: nullsoft
    InstallerUrl: ${exeData.url}
    InstallerSha256: ${exeData.hash.toUpperCase()}
ManifestType: installer
ManifestVersion: ${CONFIG.manifestVersion}
`;

  const versionYaml = `# Created using generate-package-manifests.mjs
# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.${CONFIG.manifestVersion}.schema.json

PackageIdentifier: ${CONFIG.packageId}
PackageVersion: ${version}
DefaultLocale: ${CONFIG.defaultLocale}
ManifestType: version
ManifestVersion: ${CONFIG.manifestVersion}
`;

  const localeYaml = `# Created using generate-package-manifests.mjs
# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.${CONFIG.manifestVersion}.schema.json

PackageIdentifier: ${CONFIG.packageId}
PackageVersion: ${version}
PackageLocale: ${CONFIG.defaultLocale}
Publisher: ${CONFIG.publisher}
PublisherUrl: ${CONFIG.homepage}
PublisherSupportUrl: ${CONFIG.homepage}/issues
PackageName: ${CONFIG.productTitle}
PackageUrl: ${CONFIG.homepage}
License: ${CONFIG.license}
LicenseUrl: ${CONFIG.licenseUrl}
Copyright: Copyright (c) 2026 ${CONFIG.publisher}
ShortDescription: ${CONFIG.shortDescription}
Description: |-
  ${CONFIG.description}
Moniker: vidnest
Tags:
${CONFIG.tags.map((t) => `  - ${t}`).join('\n')}
ReleaseNotesUrl: ${CONFIG.homepage}/releases/tag/v${version}
ManifestType: defaultLocale
ManifestVersion: ${CONFIG.manifestVersion}
`;

  const localeZhYaml = `# Created using generate-package-manifests.mjs
# yaml-language-server: $schema=https://aka.ms/winget-manifest.locale.${CONFIG.manifestVersion}.schema.json

PackageIdentifier: ${CONFIG.packageId}
PackageVersion: ${version}
PackageLocale: zh-CN
Publisher: ${CONFIG.publisher}
PublisherUrl: ${CONFIG.homepage}
PublisherSupportUrl: ${CONFIG.homepage}/issues
PackageName: VidNest (巢视) 本地视频智能中枢
PackageUrl: ${CONFIG.homepage}
License: ${CONFIG.license}
LicenseUrl: ${CONFIG.licenseUrl}
Copyright: Copyright (c) 2026 ${CONFIG.publisher}
ShortDescription: 极速本地优先多媒体资产管理中心与 yt-dlp 4K 视频下载引擎。
Description: |-
  VidNest (巢视) 是一款企业级纯本地优先的智能视频管理、字幕转写与音视频裁剪工作台。基于 Tauri 2 + Rust + Vue 构建，提供 100% 隐私安全、FTS5 毫秒检索与离线 Whisper AI 理解。
Moniker: vidnest
Tags:
  - 视频管理
  - 视频下载
  - 视频裁剪
  - yt-dlp
  - 本地优先
  - tauri
ReleaseNotesUrl: ${CONFIG.homepage}/releases/tag/v${version}
ManifestType: locale
ManifestVersion: ${CONFIG.manifestVersion}
`;

  const fileInstaller = path.join(wingetDir, `${CONFIG.packageId}.installer.yaml`);
  const fileVersion = path.join(wingetDir, `${CONFIG.packageId}.yaml`);
  const fileLocale = path.join(wingetDir, `${CONFIG.packageId}.locale.${CONFIG.defaultLocale}.yaml`);
  const fileLocaleZh = path.join(wingetDir, `${CONFIG.packageId}.locale.zh-CN.yaml`);

  fs.writeFileSync(fileInstaller, installerYaml, 'utf8');
  fs.writeFileSync(fileVersion, versionYaml, 'utf8');
  fs.writeFileSync(fileLocale, localeYaml, 'utf8');
  fs.writeFileSync(fileLocaleZh, localeZhYaml, 'utf8');

  return {
    dir: wingetDir,
    files: [fileInstaller, fileVersion, fileLocale, fileLocaleZh],
  };
}

/**
 * Generates Scoop manifest (JSON) containing url, hash, bin, shortcuts
 */
export function generateScoopManifest(outDir, version, scoopData) {
  const scoopDir = path.join(outDir, 'scoop');
  fs.mkdirSync(scoopDir, { recursive: true });

  const manifest = {
    version: version,
    description: `${CONFIG.productTitle} - Local video analysis, indexing and AI summarization tool`,
    homepage: CONFIG.homepage,
    license: CONFIG.license,
    url: scoopData.url,
    hash: scoopData.hash.toLowerCase(),
    bin: `${CONFIG.productTitle}.exe`,
    shortcuts: [
      [
        `${CONFIG.productTitle}.exe`,
        CONFIG.appName
      ]
    ],
    checkver: {
      github: CONFIG.homepage
    },
    autoupdate: {
      url: `https://github.com/${CONFIG.repo}/releases/download/v$version/Local.Video.Intelligence_$version_x64-setup.exe`
    }
  };

  const filePath = path.join(scoopDir, 'vidnest.json');
  fs.writeFileSync(filePath, JSON.stringify(manifest, null, 2) + '\n', 'utf8');

  return {
    file: filePath,
    manifest,
  };
}

/**
 * Generates Homebrew Cask ruby script (.rb) containing url, sha256, app "VidNest.app"
 */
export function generateHomebrewCask(outDir, version, dmgData) {
  const brewDir = path.join(outDir, 'homebrew');
  fs.mkdirSync(brewDir, { recursive: true });

  const caskContent = `cask "vidnest" do
  version "${version}"
  sha256 "${dmgData.hash.toLowerCase()}"

  url "${dmgData.url}"
  name "VidNest"
  name "${CONFIG.productTitle}"
  desc "${CONFIG.shortDescription}"
  homepage "${CONFIG.homepage}"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "VidNest.app"

  postflight do
    # Remove macOS quarantine bit to prevent Sequoia Gatekeeper damage prompt
    system_command "/usr/bin/xattr",
                   args: ["-rd", "com.apple.quarantine", "#{appdir}/VidNest.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/${CONFIG.bundleId}",
    "~/Library/Preferences/${CONFIG.bundleId}.plist",
    "~/Library/Saved Application State/${CONFIG.bundleId}.savedState",
  ]
end
`;

  const filePath = path.join(brewDir, 'vidnest.rb');
  fs.writeFileSync(filePath, caskContent, 'utf8');

  return {
    file: filePath,
  };
}

// ----------------------------------------------------------------------------
// 6. Main Runner
// ----------------------------------------------------------------------------
async function main() {
  console.log('╔══════════════════════════════════════════════════════════════╗');
  console.log('║       VidNest Packaging & Manifest Automation Tool          ║');
  console.log('╚══════════════════════════════════════════════════════════════╝');

  const options = parseArgs(process.argv.slice(2));

  console.log('📋 Execution Parameters:');
  console.log(`  • Version:        ${options.version}`);
  console.log(`  • Tag:            ${options.tag}`);
  console.log(`  • Repository:     ${options.repo}`);
  console.log(`  • Dry-Run Mode:   ${options.dryRun ? 'ON (Simulated Assets)' : 'OFF (Live Remote)'}`);
  console.log(`  • Output Base:    ${options.outDir}`);

  // Resolve assets & hashes
  const assets = await resolveAssets(options);

  console.log('\n📦 Resolved Target Assets:');
  console.log(`  [Windows EXE]     ${assets.exe.url}`);
  console.log(`                    SHA-256: ${assets.exe.hash.toUpperCase()}`);
  console.log(`  [Scoop Package]   ${assets.scoop.url}`);
  console.log(`                    SHA-256: ${assets.scoop.hash.toLowerCase()}`);
  console.log(`  [macOS DMG]       ${assets.dmg.url}`);
  console.log(`                    SHA-256: ${assets.dmg.hash.toLowerCase()}`);

  console.log('\n🛠️  Generating Package Manifests...');

  // 1. WinGet 3-file manifests
  const wingetResult = generateWinGetManifests(options.outDir, options.version, assets.exe);
  console.log(`\n  ✅ [WinGet] 3-file manifest generated at:`);
  console.log(`     📁 ${wingetResult.dir}`);
  for (const f of wingetResult.files) {
    console.log(`        └─ 📄 ${path.basename(f)}`);
  }

  // 2. Scoop manifest
  const scoopResult = generateScoopManifest(options.outDir, options.version, assets.scoop);
  console.log(`\n  ✅ [Scoop] Manifest generated at:`);
  console.log(`     📄 ${scoopResult.file}`);

  // 3. Homebrew Cask
  const brewResult = generateHomebrewCask(options.outDir, options.version, assets.dmg);
  console.log(`\n  ✅ [Homebrew Cask] Formula script generated at:`);
  console.log(`     📄 ${brewResult.file}`);

  console.log('\n══════════════════════════════════════════════════════════════');
  console.log('🎉 All package manifests generated successfully!');
  console.log('══════════════════════════════════════════════════════════════');
  console.log('\nNext steps for promotion & distribution:');
  console.log('  1. WinGet: Submit PR to github.com/microsoft/winget-pkgs via wingetcreate or GitHub Action.');
  console.log('  2. Scoop:  Commit autodeploy/manifests/scoop/vidnest.json to your bucket.');
  console.log('  3. Homebrew: Copy autodeploy/manifests/homebrew/vidnest.rb to your homebrew-tap repository.');
  console.log('');
}

main().catch((err) => {
  console.error('\n❌ Fatal error generating package manifests:', err);
  process.exit(1);
});
