use crate::models::{YtDlpUpdateCheck, YtDlpUpdateResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Returns the path where the app manages its own portable yt-dlp binary.
pub fn get_managed_binary_path(app_data_dir: &Path) -> PathBuf {
    let filename = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    app_data_dir.join("bin").join(filename)
}

/// Resolves the active yt-dlp binary:
/// 1. App-managed binary (if present and executable)
/// 2. System PATH binary
pub fn resolve_active_binary(app_data_dir: Option<&Path>) -> (Option<PathBuf>, bool) {
    if let Some(dir) = app_data_dir {
        let managed = get_managed_binary_path(dir);
        if managed.exists() && get_binary_version(&managed).is_some() {
            return (Some(managed), true);
        }
    }

    let system_bin = PathBuf::from(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
    if get_binary_version(&system_bin).is_some() {
        return (Some(system_bin), false);
    }

    (None, false)
}

/// Executes `--version` on the specified binary to query its version string.
pub fn get_binary_version(bin_path: &Path) -> Option<String> {
    let mut cmd = Command::new(bin_path);
    cmd.arg("--version");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout
                .lines()
                .find(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_string());
            first_line
        }
        _ => None,
    }
}

/// Extracts the release version tag from a redirect URL or release payload.
pub fn extract_tag_from_url(url: &str) -> Option<String> {
    // E.g. https://github.com/yt-dlp/yt-dlp/releases/download/2026.08.19/yt-dlp.exe
    let marker = "/releases/download/";
    if let Some(pos) = url.find(marker) {
        let remainder = &url[pos + marker.len()..];
        if let Some(slash_pos) = remainder.find('/') {
            return Some(remainder[..slash_pos].trim().to_string());
        }
    }
    None
}

/// Parses version strings from `yt-dlp -U` output if available.
pub fn parse_ytdlp_update_output(output_text: &str) -> (Option<String>, Option<String>) {
    let mut current = None;
    let mut latest = None;

    for line in output_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Current version:") {
            // e.g. "Current version: stable@2025.10.14 from yt-dlp/yt-dlp"
            current = trimmed
                .split('@')
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .map(|s| s.to_string());
        } else if trimmed.starts_with("Latest version:") {
            // e.g. "Latest version: stable@2026.08.19 from yt-dlp/yt-dlp"
            latest = trimmed
                .split('@')
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .map(|s| s.to_string());
        }
    }

    (current, latest)
}

/// Checks the latest available version from GitHub releases.
pub async fn fetch_latest_release_version() -> Result<String, String> {
    // 1. Try HEAD request on the latest download redirect URL (fastest, no API rate limits)
    let asset_name = if cfg!(windows) {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp_linux"
    };
    let download_url = format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{asset_name}");

    let client = reqwest::Client::builder()
        .user_agent("LocalVideoIntelligence/1.4.0")
        .timeout(std::time::Duration::from_millis(3500))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    if let Ok(resp) = client.head(download_url).send().await {
        if resp.status().is_redirection() {
            if let Some(location) = resp.headers().get("location") {
                if let Ok(loc_str) = location.to_str() {
                    if let Some(tag) = extract_tag_from_url(loc_str) {
                        return Ok(tag);
                    }
                }
            }
        }
    }

    // 2. Fallback: Query GitHub API
    let api_url = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
    let api_client = reqwest::Client::builder()
        .user_agent("LocalVideoIntelligence/1.4.0")
        .timeout(std::time::Duration::from_millis(3500))
        .build()
        .map_err(|e| format!("Failed to build HTTP client for API: {}", e))?;

    let res = api_client
        .get(api_url)
        .send()
        .await
        .map_err(|e| format!("Network request to GitHub failed: {}", e))?;

    if res.status().is_success() {
        let text = res
            .text()
            .await
            .map_err(|e| format!("Failed to read release response: {}", e))?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(tag) = json.get("tag_name").and_then(|t| t.as_str()) {
                return Ok(tag.to_string());
            }
        }
    }

    Err("Could not retrieve latest yt-dlp release version from GitHub".to_string())
}

/// Checks whether yt-dlp has an available update.
pub async fn check_ytdlp_update(app_data_dir: Option<&Path>) -> Result<YtDlpUpdateCheck, String> {
    let (active_bin, is_managed) = resolve_active_binary(app_data_dir);
    let current_version = active_bin.as_deref().and_then(get_binary_version);

    let latest_version = match fetch_latest_release_version().await {
        Ok(v) => Some(v),
        Err(_) => {
            // If network lookup failed, try `yt-dlp -U` dry inspection
            if let Some(bin) = &active_bin {
                let mut cmd = Command::new(bin);
                cmd.arg("-U");
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    cmd.creation_flags(CREATE_NO_WINDOW);
                }
                if let Ok(output) = cmd.output() {
                    let out_text = format!(
                        "{}\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                    let (_, latest) = parse_ytdlp_update_output(&out_text);
                    latest
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    let has_update = match (&current_version, &latest_version) {
        (Some(curr), Some(lat)) => curr != lat,
        (None, Some(_)) => true, // Missing tool needs installation
        _ => false,
    };

    let message = match (&current_version, &latest_version) {
        (Some(curr), Some(lat)) if curr == lat => {
            format!("yt-dlp 已是最新版本 ({})", curr)
        }
        (Some(curr), Some(lat)) => {
            format!("发现新版本：当前版本 {}，最新版本 {}", curr, lat)
        }
        (None, Some(lat)) => {
            format!("未检测到本地 yt-dlp，可安装最新版本 {}", lat)
        }
        (Some(curr), None) => {
            format!("当前版本 {}，未能获取远端最新版本信息", curr)
        }
        (None, None) => "未检测到本地 yt-dlp 且无法连接远端服务器".to_string(),
    };

    Ok(YtDlpUpdateCheck {
        current_version,
        latest_version,
        has_update,
        executable_path: active_bin.map(|p| p.to_string_lossy().to_string()),
        is_managed,
        message,
    })
}

/// Downloads and installs the latest official yt-dlp binary into the app-managed bin folder.
pub async fn download_managed_binary(app_data_dir: &Path) -> Result<String, String> {
    let bin_dir = app_data_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create managed bin directory: {}", e))?;

    let target_file = get_managed_binary_path(app_data_dir);
    let temp_file = bin_dir.join(format!(".yt-dlp_download_{}.tmp", std::process::id()));

    let asset_name = if cfg!(windows) {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp_linux"
    };

    let download_candidates = [
        format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{asset_name}"),
        format!("https://ghfast.top/https://github.com/yt-dlp/yt-dlp/releases/latest/download/{asset_name}"),
        format!("https://mirror.ghproxy.com/https://github.com/yt-dlp/yt-dlp/releases/latest/download/{asset_name}"),
    ];

    let client = reqwest::Client::builder()
        .user_agent("LocalVideoIntelligence/0.1.0")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to build download client: {}", e))?;

    let mut last_err = String::new();
    let mut downloaded_bytes = None;

    for url in &download_candidates {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.bytes().await {
                    Ok(b) if !b.is_empty() => {
                        downloaded_bytes = Some(b);
                        break;
                    }
                    Ok(_) => {
                        last_err = format!("Empty response from {}", url);
                    }
                    Err(e) => {
                        last_err = format!("Failed reading bytes from {}: {}", url, e);
                    }
                }
            }
            Ok(resp) => {
                last_err = format!("HTTP error {} from {}", resp.status(), url);
            }
            Err(e) => {
                last_err = format!("Connection error for {}: {}", url, e);
            }
        }
    }

    let bytes = downloaded_bytes.ok_or_else(|| {
        format!("yt-dlp 下载失败（已尝试官方与国内加速节点）: {}", last_err)
    })?;

    fs::write(&temp_file, &bytes)
        .map_err(|e| format!("Failed to write temporary binary: {}", e))?;

    // On Unix systems, make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_file)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_file, perms).map_err(|e| e.to_string())?;
    }

    // Verify downloaded binary is functional before replacing
    let downloaded_version = get_binary_version(&temp_file).ok_or_else(|| {
        let _ = fs::remove_file(&temp_file);
        "Downloaded binary failed verification test (--version)".to_string()
    })?;

    // Atomically replace target file
    if target_file.exists() {
        let _ = fs::remove_file(&target_file);
    }
    fs::rename(&temp_file, &target_file)
        .map_err(|e| format!("Failed to install managed yt-dlp binary: {}", e))?;

    Ok(downloaded_version)
}

/// Executes auto-update for yt-dlp.
/// 1. If currently using an app-managed binary, first tries `yt-dlp -U`.
/// 2. If `yt-dlp -U` fails (or if system version cannot self-update), downloads and installs the latest binary to the managed directory.
pub async fn update_ytdlp(app_data_dir: &Path) -> Result<YtDlpUpdateResult, String> {
    let (active_bin, is_managed) = resolve_active_binary(Some(app_data_dir));
    let previous_version = active_bin.as_deref().and_then(get_binary_version);

    // If already managed, try native self-update first
    if is_managed {
        if let Some(managed_path) = &active_bin {
            let mut cmd = Command::new(managed_path);
            cmd.arg("-U");
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let new_ver = get_binary_version(managed_path);
                    return Ok(YtDlpUpdateResult {
                        success: true,
                        previous_version,
                        current_version: new_ver.clone(),
                        executable_path: managed_path.to_string_lossy().to_string(),
                        method: "native_self_update".to_string(),
                        message: format!("通过 yt-dlp 原生更新成功，当前版本: {}", new_ver.unwrap_or_default()),
                    });
                }
            }
        }
    }

    // Otherwise, perform direct GitHub release download into app-managed bin
    match download_managed_binary(app_data_dir).await {
        Ok(new_ver) => {
            let managed_path = get_managed_binary_path(app_data_dir);
            Ok(YtDlpUpdateResult {
                success: true,
                previous_version,
                current_version: Some(new_ver.clone()),
                executable_path: managed_path.to_string_lossy().to_string(),
                method: "github_download".to_string(),
                message: format!(
                    "已下载并更新为应用内置独立托管版本，当前版本: {}",
                    new_ver
                ),
            })
        }
        Err(err) => Err(format!("yt-dlp 自动更新失败: {}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag_from_url() {
        let url = "https://github.com/yt-dlp/yt-dlp/releases/download/2026.08.19/yt-dlp.exe";
        assert_eq!(extract_tag_from_url(url), Some("2026.08.19".to_string()));

        let url2 = "https://github.com/yt-dlp/yt-dlp/releases/download/2025.10.14/yt-dlp";
        assert_eq!(extract_tag_from_url(url2), Some("2025.10.14".to_string()));

        let invalid = "https://github.com/yt-dlp/yt-dlp";
        assert_eq!(extract_tag_from_url(invalid), None);
    }

    #[test]
    fn test_parse_ytdlp_update_output() {
        let text = r#"
Current version: stable@2025.10.14 from yt-dlp/yt-dlp
Latest version: stable@2026.08.19 from yt-dlp/yt-dlp
ERROR: You installed yt-dlp from a manual build or with a package manager; Use that to update
        "#;
        let (current, latest) = parse_ytdlp_update_output(text);
        assert_eq!(current, Some("2025.10.14".to_string()));
        assert_eq!(latest, Some("2026.08.19".to_string()));
    }

    #[test]
    fn test_get_managed_binary_path() {
        let dummy = PathBuf::from("D:\\TestAppData");
        let path = get_managed_binary_path(&dummy);
        assert!(path.starts_with(&dummy));
        assert!(path.ends_with(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" }));
    }

    #[tokio::test]
    async fn test_fetch_latest_release_version() {
        let res = fetch_latest_release_version().await;
        assert!(res.is_ok(), "Failed to fetch latest release: {:?}", res.err());
        let ver = res.unwrap();
        assert!(!ver.is_empty());
        println!("Confirmed real remote yt-dlp release version: {}", ver);
    }
}
