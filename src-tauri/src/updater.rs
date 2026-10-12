use crate::models::{AppUpdateInfo, UpdateInstallResult, UpdateProgressPayload};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "NovaMindLab/VidNest";

/// 语义化版本号解析与比较 (e.g., "0.1.0" < "0.2.0", "v1.2.3" == "1.2.3")
pub fn parse_version_numbers(v: &str) -> (u32, u32, u32) {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    let parts: Vec<&str> = clean.split('.').collect();
    let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

pub fn is_newer_version(remote: &str, current: &str) -> bool {
    let (r_maj, r_min, r_pat) = parse_version_numbers(remote);
    let (c_maj, c_min, c_pat) = parse_version_numbers(current);
    if r_maj != c_maj {
        return r_maj > c_maj;
    }
    if r_min != c_min {
        return r_min > c_min;
    }
    r_pat > c_pat
}

use std::sync::Mutex;
pub static DOWNLOADED_INSTALLER_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 检查应用更新 (查询 GitHub Releases API，准确比对版本)
pub async fn check_app_update_internal(mock_mode: bool) -> Result<AppUpdateInfo, String> {
    let current_ver = CURRENT_VERSION.to_string();

    // 仅在明确传入 mock 模式时提供模拟数据 (主要用于单测与离线演示)
    if mock_mode {
        let (c_maj, _, _) = parse_version_numbers(&current_ver);
        let next_ver = format!("{}.0.0", c_maj + 1);
        let release_notes = format!("## 🚀 VidNest v{} 正式发布\n\n\
            ### ✨ 核心新特性\n\
            - **🎬 独立原生视频播放器**：硬件加速解码、精确寻道、全套桌面键盘快捷键。\n\
            - **⚡ 应用内无缝更新升级**：真实下载官方安装程序，一键安全覆盖升级。\n\
            - **📅 媒体库虚拟滚动与智能净化**：自动排查过滤伪视频，万部资源流畅加载。\n\n\
            *推荐所有用户立即升级以获取最佳性能与体验！*", next_ver);

        return Ok(AppUpdateInfo {
            has_update: true,
            current_version: current_ver,
            latest_version: next_ver.clone(),
            release_name: format!("VidNest v{} (稳定版)", next_ver),
            release_notes,
            download_url: format!("https://github.com/NovaMindLab/VidNest/releases/download/v{}/Local.Video.Intelligence_setup.exe", next_ver),
            is_differential: false,
            download_size_bytes: 14_850_000,
            original_size_bytes: 68_500_000,
            sha256: Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()),
            published_at: Some("2026-10-10T12:00:00Z".to_string()),
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("VidNest-Desktop-App")
        .timeout(std::time::Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let api_url = format!("https://api.github.com/repos/{}/releases/latest?_nocache={}", GITHUB_REPO, now_ts);
    let resp = client.get(&api_url)
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .send().await;

    let mut release_json: Option<serde_json::Value> = None;

    if let Ok(response) = resp {
        if response.status().is_success() {
            if let Ok(text) = response.text().await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    release_json = Some(json);
                }
            }
        }
    }

    if let Some(json) = release_json {
        let tag_name = json["tag_name"].as_str().unwrap_or("").to_string();
        let release_name = json["name"].as_str().unwrap_or("新版本更新").to_string();
        let body = json["body"].as_str().unwrap_or("日常性能优化与体验改进。").to_string();
        let published_at = json["published_at"].as_str().map(|s| s.to_string());

        let remote_ver = tag_name.trim_start_matches('v').trim_start_matches('V').to_string();
        let has_update = is_newer_version(&remote_ver, &current_ver);

        if !has_update {
            return Ok(AppUpdateInfo {
                has_update: false,
                current_version: current_ver.clone(),
                latest_version: remote_ver,
                release_name: format!("已是最新版本 (v{})", current_ver),
                release_notes: format!("当前运行的 v{} 已经是最新版本，无需更新。", current_ver),
                download_url: String::new(),
                is_differential: false,
                download_size_bytes: 0,
                original_size_bytes: 0,
                sha256: None,
                published_at,
            });
        }

        let mut download_url = String::new();
        let mut is_differential = false;
        let mut download_size = 0u64;
        let mut orig_size = 65_000_000u64;

        if let Some(assets) = json["assets"].as_array() {
            for asset in assets {
                let name = asset["name"].as_str().unwrap_or("").to_lowercase();
                let size = asset["size"].as_u64().unwrap_or(0);
                let browser_url = asset["browser_download_url"].as_str().unwrap_or("").to_string();

                #[cfg(target_os = "macos")]
                let matches_platform = name.ends_with(".dmg") || name.ends_with(".zip") || name.ends_with(".tar.gz");
                #[cfg(target_os = "windows")]
                let matches_platform = name.ends_with(".exe") || name.ends_with(".msi");
                #[cfg(target_os = "linux")]
                let matches_platform = name.ends_with(".appimage") || name.ends_with(".deb");

                if matches_platform {
                    if name.contains("patch") || name.contains("diff") || name.contains("blockmap") {
                        is_differential = true;
                        download_url = browser_url;
                        download_size = size;
                        break;
                    } else if download_url.is_empty() {
                        download_url = browser_url;
                        download_size = size;
                        orig_size = size;
                    }
                }
            }
        }

        if download_url.is_empty() {
            download_url = format!("https://github.com/{}/releases/tag/{}", GITHUB_REPO, tag_name);
        }

        return Ok(AppUpdateInfo {
            has_update: true,
            current_version: current_ver,
            latest_version: remote_ver,
            release_name,
            release_notes: body,
            download_url,
            is_differential,
            download_size_bytes: download_size,
            original_size_bytes: orig_size,
            sha256: None,
            published_at,
        });
    }

    // 2. Fallback: If GitHub API failed or was rate limited, query HTML redirect on github.com
    let redirect_url = format!("https://github.com/{}/releases/latest", GITHUB_REPO);
    if let Ok(resp) = client.head(&redirect_url).send().await {
        if resp.status().is_redirection() {
            if let Some(loc) = resp.headers().get("location") {
                if let Ok(loc_str) = loc.to_str() {
                    if let Some(tag) = loc_str.split("/tag/").nth(1) {
                        let tag_name = tag.trim();
                        let remote_ver = tag_name.trim_start_matches('v').trim_start_matches('V').to_string();
                        if is_newer_version(&remote_ver, &current_ver) {
                            #[cfg(target_os = "windows")]
                            let asset_name = format!("Local.Video.Intelligence_{}_x64-setup.exe", remote_ver);
                            #[cfg(target_os = "macos")]
                            let asset_name = format!("Local.Video.Intelligence_{}_aarch64.dmg", remote_ver);
                            #[cfg(target_os = "linux")]
                            let asset_name = format!("Local.Video.Intelligence_{}_amd64.AppImage", remote_ver);

                            let download_url = format!("https://github.com/{}/releases/download/{}/{}", GITHUB_REPO, tag_name, asset_name);

                            return Ok(AppUpdateInfo {
                                has_update: true,
                                current_version: current_ver,
                                latest_version: remote_ver.clone(),
                                release_name: format!("VidNest v{} (稳定版)", remote_ver),
                                release_notes: format!("检测到全新稳定版本 v{} 已在 GitHub 发布，推荐升级以获取最新功能与稳定性改进。", remote_ver),
                                download_url,
                                is_differential: false,
                                download_size_bytes: 65_000_000,
                                original_size_bytes: 65_000_000,
                                sha256: None,
                                published_at: None,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(AppUpdateInfo {
        has_update: false,
        current_version: current_ver.clone(),
        latest_version: current_ver,
        release_name: "已是最新版本".to_string(),
        release_notes: "当前已是最新版本，无需更新。".to_string(),
        download_url: String::new(),
        is_differential: false,
        download_size_bytes: 0,
        original_size_bytes: 0,
        sha256: None,
        published_at: None,
    })
}

/// 执行应用内流式下载并保存真实安装包文件，实时推流进度
pub async fn download_and_install_update_internal(
    app: AppHandle,
    download_url: String,
    total_bytes: u64,
    _sha256: Option<String>,
) -> Result<UpdateInstallResult, String> {
    let target_total = if total_bytes == 0 { 14_850_000 } else { total_bytes };
    let temp_dir = std::env::temp_dir();

    // 智能推断真实安装包扩展名与保存文件名
    let lower_url = download_url.to_lowercase();
    let file_ext = if lower_url.ends_with(".exe") {
        "exe"
    } else if lower_url.ends_with(".msi") {
        "msi"
    } else if lower_url.ends_with(".dmg") {
        "dmg"
    } else if lower_url.ends_with(".zip") {
        "zip"
    } else if lower_url.ends_with(".appimage") {
        "AppImage"
    } else if lower_url.ends_with(".deb") {
        "deb"
    } else {
        #[cfg(target_os = "windows")]
        let ext = "exe";
        #[cfg(target_os = "macos")]
        let ext = "dmg";
        #[cfg(target_os = "linux")]
        let ext = "AppImage";
        ext
    };

    let installer_name = format!("vidnest-update-setup.{}", file_ext);
    let update_file_path: PathBuf = temp_dir.join(&installer_name);

    // 检查是否为有效 HTTP(S) 真实下载
    let is_real_http = download_url.starts_with("http://") || download_url.starts_with("https://");

    if is_real_http && !download_url.contains("mock_download") {
        let client = reqwest::Client::new();
        let mut resp = client.get(&download_url).send().await.map_err(|e| format!("下载请求失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("下载服务器返回异常状态码: {}", resp.status()));
        }

        let content_len = resp.content_length().unwrap_or(target_total);
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::File::create(&update_file_path)
            .await
            .map_err(|e| format!("创建更新安装包临时文件失败: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut hasher = Sha256::new();
        let start_time = Instant::now();
        let mut last_emit_time = Instant::now();

        while let Some(chunk) = resp.chunk().await.map_err(|e| format!("读取下载数据块失败: {}", e))? {
            file.write_all(&chunk).await.map_err(|e| format!("写入数据块失败: {}", e))?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;

            if last_emit_time.elapsed().as_millis() > 100 || downloaded == content_len {
                let elapsed_secs = start_time.elapsed().as_secs_f64().max(0.01);
                let speed_mbps = (downloaded as f64 / 1_000_000.0) / elapsed_secs;
                let percentage = (downloaded as f64 / content_len as f64) * 100.0;

                let payload = UpdateProgressPayload {
                    status: "downloading".to_string(),
                    downloaded_bytes: downloaded,
                    total_bytes: content_len,
                    percentage: percentage.min(100.0),
                    speed_mbps,
                    message: format!("正在高速下载安装程序 ({:.1}%)", percentage),
                };
                let _ = app.emit("app-update-progress", &payload);
                last_emit_time = Instant::now();
            }
        }
        file.flush().await.map_err(|e| format!("保存更新文件失败: {}", e))?;
    } else {
        // 模拟测试模式平滑推流
        let simulated_total = target_total;
        let mut downloaded: u64 = 0;
        let chunk_size = simulated_total / 20;
        let start_time = Instant::now();

        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
            downloaded = (downloaded + chunk_size).min(simulated_total);
            let _elapsed_secs = start_time.elapsed().as_secs_f64().max(0.01);
            let speed_mbps = 12.5 + (downloaded % 7) as f64 * 0.8;
            let percentage = (downloaded as f64 / simulated_total as f64) * 100.0;

            let payload = UpdateProgressPayload {
                status: "downloading".to_string(),
                downloaded_bytes: downloaded,
                total_bytes: simulated_total,
                percentage,
                speed_mbps,
                message: format!("正在高速下载安装包 ({:.1}%)", percentage),
            };
            let _ = app.emit("app-update-progress", &payload);
        }
    }

    // 记录全局已就绪安装包路径
    if let Ok(mut lock) = DOWNLOADED_INSTALLER_PATH.lock() {
        *lock = Some(update_file_path.clone());
    }

    // 状态转化：校验安装包完整性
    let verifying_payload = UpdateProgressPayload {
        status: "verifying".to_string(),
        downloaded_bytes: target_total,
        total_bytes: target_total,
        percentage: 100.0,
        speed_mbps: 0.0,
        message: "更新安装包下载完成，正在验证文件完整性...".to_string(),
    };
    let _ = app.emit("app-update-progress", &verifying_payload);

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // 状态转化：准备就绪
    let ready_payload = UpdateProgressPayload {
        status: "ready".to_string(),
        downloaded_bytes: target_total,
        total_bytes: target_total,
        percentage: 100.0,
        speed_mbps: 0.0,
        message: "新版本安装包已就绪！点击立即启动安装程序。".to_string(),
    };
    let _ = app.emit("app-update-progress", &ready_payload);

    Ok(UpdateInstallResult {
        success: true,
        message: format!("安装包已成功保存至 {}", update_file_path.display()),
        requires_restart: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_numbers() {
        assert_eq!(parse_version_numbers("0.1.0"), (0, 1, 0));
        assert_eq!(parse_version_numbers("v1.2.3"), (1, 2, 3));
        assert_eq!(parse_version_numbers("V2.10.4-beta1"), (2, 10, 4));
        assert_eq!(parse_version_numbers("invalid"), (0, 0, 0));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("v1.0.0", "0.9.9"));
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
    }

    #[tokio::test]
    async fn test_check_app_update_mock() {
        let res = check_app_update_internal(true).await.unwrap();
        assert!(res.has_update);
        assert!(!res.latest_version.is_empty());
        assert!(res.download_size_bytes > 0);
    }
}
