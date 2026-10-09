#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgressUpdate {
    pub progress: Option<f64>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed: Option<f64>,
    pub speed_str: Option<String>,
    pub eta: Option<String>,
    pub file_path: Option<String>,
}

/// Splits a token (like "10.23MiB" or "1.20MiB/s") or pair of tokens ("10.23", "MiB") into (number, unit).
pub fn split_num_and_unit(token: &str, next_token: Option<&str>) -> (Option<String>, Option<String>) {
    let clean = token.trim().trim_end_matches("/s");
    if clean.is_empty() {
        return (None, None);
    }

    // Check if the entire clean token is numeric
    if clean.parse::<f64>().is_ok() {
        if let Some(next) = next_token {
            let next_clean = next.trim().trim_end_matches("/s");
            if !next_clean.is_empty() && !next_clean.starts_with(|c: char| c.is_ascii_digit()) {
                return (Some(clean.to_string()), Some(next_clean.to_string()));
            }
        }
        return (Some(clean.to_string()), None);
    }

    // Token has digits followed by letters, e.g. "10.23MiB"
    let mut num_end = 0;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || (c == '-' && i == 0) {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }

    if num_end > 0 && num_end < clean.len() {
        let num_part = &clean[..num_end];
        let unit_part = &clean[num_end..];
        if num_part.parse::<f64>().is_ok() {
            return (Some(num_part.to_string()), Some(unit_part.to_string()));
        }
    }

    (None, None)
}

/// Parses size string into bytes.
pub fn parse_size_bytes(num: &str, unit: &str) -> Option<u64> {
    let val: f64 = num.parse().ok()?;
    let unit_lower = unit.to_lowercase();
    let multiplier: f64 = match unit_lower.as_str() {
        "b" | "bytes" => 1.0,
        "kib" => 1024.0,
        "kb" => 1000.0,
        "mib" => 1024.0 * 1024.0,
        "mb" => 1000.0 * 1000.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        "gb" => 1000.0 * 1000.0 * 1000.0,
        "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "tb" => 1000.0 * 1000.0 * 1000.0 * 1000.0,
        _ => return None,
    };

    Some((val * multiplier).round() as u64)
}

/// Parses speed string into bytes per second.
pub fn parse_speed_bytes(num: &str, unit: &str) -> Option<f64> {
    let val: f64 = num.parse().ok()?;
    let unit_lower = unit.to_lowercase();
    let multiplier: f64 = match unit_lower.as_str() {
        "b" | "bytes" => 1.0,
        "kib" => 1024.0,
        "kb" => 1000.0,
        "mib" => 1024.0 * 1024.0,
        "mb" => 1000.0 * 1000.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        "gb" => 1000.0 * 1000.0 * 1000.0,
        _ => return None,
    };

    Some(val * multiplier)
}

/// Parses a line of output from yt-dlp to extract progress, total size, speed, ETA, or destination.
pub fn parse_progress_line(line: &str) -> Option<ProgressUpdate> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut update = ProgressUpdate::default();
    let mut has_useful_info = false;

    // 1. Destination / filepath lines
    if let Some(dest) = trimmed.strip_prefix("[download] Destination: ") {
        let clean = dest.trim().trim_matches('"').to_string();
        update.file_path = Some(clean);
        return Some(update);
    }
    if let Some(merger) = trimmed.strip_prefix("[Merger] Merging formats into ") {
        let clean = merger.trim().trim_matches('"').to_string();
        update.file_path = Some(clean);
        return Some(update);
    }
    if let Some(ffmpeg) = trimmed.strip_prefix("[ffmpeg] Destination: ") {
        let clean = ffmpeg.trim().trim_matches('"').to_string();
        update.file_path = Some(clean);
        return Some(update);
    }
    if trimmed.starts_with("[download] ") && trimmed.ends_with(" has already been downloaded") {
        let path = trimmed["[download] ".len()..trimmed.len() - " has already been downloaded".len()].trim();
        update.file_path = Some(path.trim_matches('"').to_string());
        update.progress = Some(100.0);
        return Some(update);
    }

    // 2. Download progress lines
    if !trimmed.starts_with("[download]") {
        return None;
    }

    let content = trimmed["[download]".len()..].trim();
    let parts: Vec<&str> = content.split_whitespace().collect();

    // Look for percentage, e.g. "45.0%"
    if let Some(pct_idx) = parts.iter().position(|p| p.ends_with('%')) {
        let pct_str = parts[pct_idx].trim_end_matches('%');
        if let Ok(val) = pct_str.parse::<f64>() {
            update.progress = Some(val);
            has_useful_info = true;
        }
    }

    // Look for total bytes: "of [~] <num><unit>"
    if let Some(of_idx) = parts.iter().position(|&p| p == "of") {
        let mut idx = of_idx + 1;
        if idx < parts.len() && parts[idx] == "~" {
            idx += 1;
        }
        if idx < parts.len() {
            let num_candidate = parts[idx];
            if num_candidate != "Unknown" {
                let next_token = parts.get(idx + 1).copied();
                let (val_str, unit_str) = split_num_and_unit(num_candidate, next_token);
                if let (Some(num), Some(unit)) = (val_str, unit_str) {
                    if let Some(bytes) = parse_size_bytes(&num, &unit) {
                        update.total_bytes = Some(bytes);
                        has_useful_info = true;
                        if let Some(pct) = update.progress {
                            update.downloaded_bytes = Some(((bytes as f64) * (pct / 100.0)).round() as u64);
                        }
                    }
                }
            }
        }
    }

    // Look for speed: "at <num><unit>/s"
    if let Some(at_idx) = parts.iter().position(|&p| p == "at") {
        let idx = at_idx + 1;
        if idx < parts.len() {
            let speed_candidate = parts[idx];
            if speed_candidate != "Unknown" {
                let next_token = parts.get(idx + 1).copied();
                let (val_str, unit_str) = split_num_and_unit(speed_candidate, next_token);
                if let (Some(num), Some(unit)) = (val_str, unit_str) {
                    if let Some(bytes_sec) = parse_speed_bytes(&num, &unit) {
                        update.speed = Some(bytes_sec);
                        update.speed_str = Some(format!("{}/s", speed_candidate.trim_end_matches("/s")));
                        has_useful_info = true;
                    }
                }
            }
        }
    }

    // Look for ETA: "ETA <time>" or "in <time>"
    if let Some(eta_idx) = parts.iter().position(|&p| p == "ETA") {
        let idx = eta_idx + 1;
        if idx < parts.len() {
            let eta_candidate = parts[idx];
            if eta_candidate != "Unknown" {
                update.eta = Some(eta_candidate.to_string());
                has_useful_info = true;
            }
        }
    } else if parts.iter().any(|&p| p == "in") && update.progress == Some(100.0) {
        update.eta = Some("00:00".to_string());
        has_useful_info = true;
    }

    if has_useful_info {
        Some(update)
    } else {
        None
    }
}

/// Identifies the platform name from URL.
pub fn extract_platform_from_url(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.contains("youtube.com") || lower.contains("youtu.be") {
        "youtube".to_string()
    } else if lower.contains("bilibili.com") {
        "bilibili".to_string()
    } else if lower.contains("twitter.com") || lower.contains("x.com") {
        "twitter".to_string()
    } else if lower.contains("tiktok.com") {
        "tiktok".to_string()
    } else if lower.contains("vimeo.com") {
        "vimeo".to_string()
    } else if lower.contains("douyin.com") {
        "douyin".to_string()
    } else if lower.contains("weibo.com") {
        "weibo".to_string()
    } else if let Some(domain) = extract_domain(&lower) {
        domain
    } else {
        "web".to_string()
    }
}

fn extract_domain(url: &str) -> Option<String> {
    let without_scheme = if let Some(pos) = url.find("://") {
        &url[pos + 3..]
    } else {
        url
    };
    let host = without_scheme.split('/').next()?.split(':').next()?;
    let trimmed = host.trim_start_matches("www.");
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_progress_line() {
        let line = "[download]   0.5% of   10.23MiB at  1.20MiB/s ETA 00:08";
        let res = parse_progress_line(line).expect("parse progress");
        assert_eq!(res.progress, Some(0.5));
        assert_eq!(res.total_bytes, Some(10726932)); // 10.23 * 1024 * 1024
        assert!(res.downloaded_bytes.is_some());
        assert_eq!(res.speed_str, Some("1.20MiB/s".to_string()));
        assert_eq!(res.eta, Some("00:08".to_string()));
    }

    #[test]
    fn test_parse_approximate_size_and_kib() {
        let line = "[download]  45.0% of ~ 15.00MiB at  2.50MiB/s ETA 00:03";
        let res = parse_progress_line(line).expect("parse progress");
        assert_eq!(res.progress, Some(45.0));
        assert_eq!(res.total_bytes, Some(15728640)); // 15.00 * 1024 * 1024
        assert_eq!(res.eta, Some("00:03".to_string()));
    }

    #[test]
    fn test_parse_finished_download() {
        let line = "[download] 100% of 10.23MiB in 00:03 at 3.12MiB/s";
        let res = parse_progress_line(line).expect("parse progress");
        assert_eq!(res.progress, Some(100.0));
        assert_eq!(res.eta, Some("00:00".to_string()));
        assert_eq!(res.downloaded_bytes, res.total_bytes);
    }

    #[test]
    fn test_parse_destination_and_merger() {
        let line1 = r#"[download] Destination: D:\downloads\video.mp4"#;
        let res1 = parse_progress_line(line1).expect("parse dest");
        assert_eq!(res1.file_path, Some(r#"D:\downloads\video.mp4"#.to_string()));

        let line2 = r#"[Merger] Merging formats into "D:\downloads\merged.mp4""#;
        let res2 = parse_progress_line(line2).expect("parse merger");
        assert_eq!(res2.file_path, Some(r#"D:\downloads\merged.mp4"#.to_string()));

        let line3 = r#"[download] D:\downloads\exists.mp4 has already been downloaded"#;
        let res3 = parse_progress_line(line3).expect("parse already");
        assert_eq!(res3.file_path, Some(r#"D:\downloads\exists.mp4"#.to_string()));
        assert_eq!(res3.progress, Some(100.0));
    }

    #[test]
    fn test_parse_unknown_size() {
        let line = "[download]  10.0% of Unknown size at 500.00KiB/s ETA Unknown";
        let res = parse_progress_line(line).expect("parse unknown");
        assert_eq!(res.progress, Some(10.0));
        assert_eq!(res.total_bytes, None);
        assert_eq!(res.eta, None);
        assert_eq!(res.speed_str, Some("500.00KiB/s".to_string()));
    }

    #[test]
    fn test_extract_platform_from_url() {
        assert_eq!(extract_platform_from_url("https://www.youtube.com/watch?v=123"), "youtube");
        assert_eq!(extract_platform_from_url("https://youtu.be/123"), "youtube");
        assert_eq!(extract_platform_from_url("https://www.bilibili.com/video/BV123"), "bilibili");
        assert_eq!(extract_platform_from_url("https://x.com/user/status/123"), "twitter");
        assert_eq!(extract_platform_from_url("https://vimeo.com/12345"), "vimeo");
        assert_eq!(extract_platform_from_url("https://myvideo.net/play"), "myvideo.net");
    }
}
