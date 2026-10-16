#!/usr/bin/env node
/**
 * scripts/setup-whisper-resources.mjs
 * 
 * Automates downloading & preparing bundled Whisper resources for Tauri packaging:
 * 1. ggml-base.bin (142MB offline speech recognition model) into src-tauri/resources/models/
 * 2. whisper-cli (+ required runtime DLLs/libraries) into src-tauri/resources/bin/
 * 
 * Supports cross-compilation targets:
 *   --target x86_64-pc-windows-msvc
 *   --target aarch64-apple-darwin
 *   --target x86_64-apple-darwin
 *   --target x86_64-unknown-linux-gnu
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';
import os from 'node:os';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '..');

const RESOURCES_DIR = path.join(PROJECT_ROOT, 'src-tauri', 'resources');
const BIN_DIR = path.join(RESOURCES_DIR, 'bin');
const MODELS_DIR = path.join(RESOURCES_DIR, 'models');

// Whisper.cpp release tag with verified multi-platform binaries
const WHISPER_RELEASE_TAG = 'b5454';

// ANSI terminal colors
const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  blue: '\x1b[34m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  bold: '\x1b[1m'
};

const log = {
  info: (msg, ...args) => console.log(`${colors.blue}[WHISPER-SETUP]${colors.reset} ${msg}`, ...args),
  success: (msg, ...args) => console.log(`${colors.green}${colors.bold}[SUCCESS]${colors.reset} ${msg}`, ...args),
  warn: (msg, ...args) => console.warn(`${colors.yellow}[WARN]${colors.reset} ${msg}`, ...args),
  error: (msg, ...args) => console.error(`${colors.red}[ERROR]${colors.reset} ${msg}`, ...args),
};

// Parse command line arguments
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    target: null,
    modelOnly: false,
    binOnly: false,
    skipModel: false,
    force: false,
    dryRun: false
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--target' && i + 1 < args.length) {
      options.target = args[++i];
    } else if (arg.startsWith('--target=')) {
      options.target = arg.split('=')[1];
    } else if (arg === '--model-only') {
      options.modelOnly = true;
    } else if (arg === '--bin-only') {
      options.binOnly = true;
    } else if (arg === '--skip-model') {
      options.skipModel = true;
    } else if (arg === '--force') {
      options.force = true;
    } else if (arg === '--dry-run') {
      options.dryRun = true;
    }
  }

  return options;
}

// Determine target platform & architecture
function resolvePlatform(targetArg) {
  if (targetArg) {
    if (targetArg.includes('windows')) {
      return { platform: 'win32', arch: 'x64' };
    }
    if (targetArg.includes('apple-darwin')) {
      return { platform: 'darwin', arch: targetArg.includes('aarch64') ? 'arm64' : 'x64' };
    }
    if (targetArg.includes('linux')) {
      return { platform: 'linux', arch: targetArg.includes('aarch64') ? 'arm64' : 'x64' };
    }
  }

  return {
    platform: process.platform,
    arch: process.arch === 'x64' ? 'x64' : 'arm64'
  };
}

// Download file with progress reporting and mirror fallback
async function downloadFileWithFallback(urls, destinationPath, minExpectedSize = 0) {
  for (let i = 0; i < urls.length; i++) {
    const url = urls[i];
    const isLast = i === urls.length - 1;
    log.info(`Downloading: ${url} -> ${destinationPath}`);

    try {
      const tempPath = `${destinationPath}.tmp_${Date.now()}`;
      const res = await fetch(url, { redirect: 'follow' });
      if (!res.ok) {
        throw new Error(`HTTP status ${res.status} ${res.statusText}`);
      }

      const totalBytes = Number(res.headers.get('content-length')) || 0;
      let downloadedBytes = 0;
      let lastPercent = 0;

      const fileStream = fs.createWriteStream(tempPath);
      const reader = res.body.getReader();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        downloadedBytes += value.length;
        fileStream.write(Buffer.from(value));

        if (totalBytes > 0) {
          const percent = Math.floor((downloadedBytes / totalBytes) * 100);
          if (percent >= lastPercent + 10) {
            lastPercent = percent;
            process.stdout.write(`\r${colors.cyan}[DOWNLOADING]${colors.reset} ${percent}% (${(downloadedBytes / 1024 / 1024).toFixed(1)}MB / ${(totalBytes / 1024 / 1024).toFixed(1)}MB)`);
          }
        }
      }

      await new Promise((resolve, reject) => {
        fileStream.end(err => err ? reject(err) : resolve());
      });

      console.log(''); // newline after progress

      const stats = fs.statSync(tempPath);
      if (minExpectedSize > 0 && stats.size < minExpectedSize) {
        fs.unlinkSync(tempPath);
        throw new Error(`Downloaded file too small (${stats.size} bytes < ${minExpectedSize} bytes)`);
      }

      if (fs.existsSync(destinationPath)) {
        fs.unlinkSync(destinationPath);
      }
      fs.renameSync(tempPath, destinationPath);
      log.success(`Downloaded successfully: ${path.basename(destinationPath)} (${(stats.size / 1024 / 1024).toFixed(2)} MB)`);
      return true;
    } catch (err) {
      log.warn(`Download failed from ${url}: ${err.message}`);
      if (isLast) {
        throw new Error(`All download mirrors failed for ${destinationPath}: ${err.message}`);
      }
      log.info(`Attempting next mirror...`);
    }
  }
}

// 1. Setup ggml-base.bin offline model
async function setupModel(options) {
  const modelFile = path.join(MODELS_DIR, 'ggml-base.bin');
  const MIN_SIZE = 140 * 1024 * 1024; // ~140MB

  if (!options.force && fs.existsSync(modelFile)) {
    const stats = fs.statSync(modelFile);
    if (stats.size >= MIN_SIZE) {
      log.info(`Model 'ggml-base.bin' already present (${(stats.size / 1024 / 1024).toFixed(2)} MB), skipping download.`);
      return;
    }
  }

  log.info(`Preparing Whisper offline model 'ggml-base.bin'...`);
  if (options.dryRun) {
    log.info(`[DRY-RUN] Would download ggml-base.bin to ${modelFile}`);
    return;
  }

  const modelUrls = [
    'https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-base.bin',
    'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin'
  ];

  await downloadFileWithFallback(modelUrls, modelFile, MIN_SIZE);
}

// 2. Setup whisper-cli and companion binaries
async function setupBinary(options, targetInfo) {
  const { platform, arch } = targetInfo;
  log.info(`Resolving whisper-cli binary for target: platform=${platform}, arch=${arch}`);

  const exeName = platform === 'win32' ? 'whisper-cli.exe' : 'whisper-cli';
  const targetBinary = path.join(BIN_DIR, exeName);

  if (!options.force && fs.existsSync(targetBinary)) {
    log.info(`Binary '${exeName}' already exists in ${BIN_DIR}, skipping.`);
    return;
  }

  if (options.dryRun) {
    log.info(`[DRY-RUN] Would configure ${exeName} for ${platform}-${arch} in ${BIN_DIR}`);
    return;
  }

  const tempDir = path.join(os.tmpdir(), `vidnest-whisper-${Date.now()}`);
  fs.mkdirSync(tempDir, { recursive: true });

  try {
    if (platform === 'win32') {
      const zipUrl = `https://github.com/ggml-org/whisper.cpp/releases/download/${WHISPER_RELEASE_TAG}/whisper-bin-x64.zip`;
      const zipFile = path.join(tempDir, 'whisper-win.zip');
      await downloadFileWithFallback([zipUrl], zipFile, 5 * 1024 * 1024);

      log.info(`Extracting Windows whisper-cli and runtime DLLs...`);
      // Use powershell or tar for cross-platform extraction
      try {
        if (process.platform === 'win32') {
          execSync(`powershell -NoProfile -Command "Expand-Archive -Path '${zipFile}' -DestinationPath '${tempDir}/extracted' -Force"`, { stdio: 'inherit' });
        } else {
          execSync(`unzip -q "${zipFile}" -d "${tempDir}/extracted"`, { stdio: 'inherit' });
        }
      } catch (e) {
        execSync(`tar -xf "${zipFile}" -C "${tempDir}"`, { stdio: 'inherit' });
      }

      const extractedDir = path.join(tempDir, 'extracted');
      const releaseSubdir = path.join(extractedDir, 'Release');
      const sourceDir = fs.existsSync(releaseSubdir) ? releaseSubdir : extractedDir;

      // Copy whisper-cli.exe and all DLLs into BIN_DIR
      const files = fs.readdirSync(sourceDir);
      for (const file of files) {
        if (file.toLowerCase().endsWith('.dll') || file.toLowerCase() === 'whisper-cli.exe') {
          fs.copyFileSync(path.join(sourceDir, file), path.join(BIN_DIR, file));
        }
      }
      log.success(`Installed Windows whisper-cli.exe and runtime DLLs into ${BIN_DIR}`);

    } else if (platform === 'linux') {
      const tarUrl = `https://github.com/ggml-org/whisper.cpp/releases/download/${WHISPER_RELEASE_TAG}/whisper-bin-ubuntu-x64.tar.gz`;
      const tarFile = path.join(tempDir, 'whisper-linux.tar.gz');
      await downloadFileWithFallback([tarUrl], tarFile, 5 * 1024 * 1024);

      log.info(`Extracting Linux whisper-cli and companion shared libraries...`);
      execSync(`tar -zxvf "${tarFile}" -C "${tempDir}"`, { stdio: 'pipe' });

      const extractedDir = path.join(tempDir, 'whisper-bin-ubuntu-x64');
      if (fs.existsSync(extractedDir)) {
        const files = fs.readdirSync(extractedDir);
        for (const file of files) {
          const srcPath = path.join(extractedDir, file);
          const stat = fs.lstatSync(srcPath);
          const dstPath = path.join(BIN_DIR, file);
          if (stat.isFile() || stat.isSymbolicLink()) {
            if (fs.existsSync(dstPath)) fs.unlinkSync(dstPath);
            fs.copyFileSync(srcPath, dstPath);
            fs.chmodSync(dstPath, 0o755);
          }
        }
      }
      log.success(`Installed Linux whisper-cli and libraries into ${BIN_DIR}`);

    } else if (platform === 'darwin') {
      log.info(`Configuring whisper-cli for macOS (${arch})...`);

      // Check if cmake is available
      let hasCmake = false;
      try {
        execSync('cmake --version', { stdio: 'pipe' });
        hasCmake = true;
      } catch {
        hasCmake = false;
      }

      if (hasCmake) {
        log.info(`Building whisper-cli with native Apple Metal GPU acceleration...`);
        const srcDir = path.join(tempDir, 'whisper-src');
        execSync(`git clone --depth 1 https://github.com/ggml-org/whisper.cpp.git "${srcDir}"`, { stdio: 'inherit' });
        
        const buildDir = path.join(srcDir, 'build');
        execSync(`cmake -B "${buildDir}" -S "${srcDir}" -DCMAKE_BUILD_TYPE=Release -DWHISPER_BUILD_EXAMPLES=ON -DGGML_METAL=ON`, { stdio: 'inherit' });
        execSync(`cmake --build "${buildDir}" --config Release --target whisper-cli -j 4`, { stdio: 'inherit' });

        const compiledCli = path.join(buildDir, 'bin', 'whisper-cli');
        if (fs.existsSync(compiledCli)) {
          fs.copyFileSync(compiledCli, targetBinary);
          fs.chmodSync(targetBinary, 0o755);
          log.success(`Compiled & installed native Metal-accelerated whisper-cli into ${BIN_DIR}`);
        } else {
          throw new Error(`Compiled whisper-cli not found at ${compiledCli}`);
        }
      } else {
        log.warn(`cmake not found on macOS. Checking Homebrew or local binary...`);
        let installedViaBrew = false;
        try {
          execSync('brew install whisper-cpp', { stdio: 'inherit' });
          const brewPrefix = execSync('brew --prefix whisper-cpp', { encoding: 'utf8' }).trim();
          const brewCli = path.join(brewPrefix, 'bin', 'whisper-cli');
          if (fs.existsSync(brewCli)) {
            fs.copyFileSync(brewCli, targetBinary);
            fs.chmodSync(targetBinary, 0o755);
            installedViaBrew = true;
            log.success(`Installed whisper-cli from Homebrew into ${BIN_DIR}`);
          }
        } catch {
          installedViaBrew = false;
        }

        if (!installedViaBrew) {
          log.warn(`Could not build whisper-cli automatically on this host.`);
          log.warn(`Please run: 'brew install cmake' and re-run this script, or place whisper-cli into: ${BIN_DIR}`);
        }
      }
    }
  } finally {
    try {
      fs.rmSync(tempDir, { recursive: true, force: true });
    } catch {}
  }
}

async function main() {
  const options = parseArgs();
  const targetInfo = resolvePlatform(options.target);

  log.info(`Target resolved: platform=${targetInfo.platform}, arch=${targetInfo.arch}`);
  fs.mkdirSync(BIN_DIR, { recursive: true });
  fs.mkdirSync(MODELS_DIR, { recursive: true });

  if (!options.binOnly && !options.skipModel) {
    await setupModel(options);
  }

  if (!options.modelOnly) {
    await setupBinary(options, targetInfo);
  }

  log.success('Whisper offline resources setup check completed!');
}

main().catch(err => {
  log.error(`Setup failed: ${err.message}`);
  process.exit(1);
});
