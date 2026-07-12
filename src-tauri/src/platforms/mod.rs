pub mod mixcloud;
pub mod soundcloud;

use anyhow::{Context, Result, bail};
use log::debug;
use std::io::Read;
use std::path::Path;

use crate::cli::Platform;
use crate::config::TokenStorage;

/// A `Read` wrapper that reports cumulative bytes read to a callback, throttled
/// so it fires at most ~once per 1% (and never more often than every 256 KiB) to
/// avoid flooding the IPC event channel. Used to drive the upload progress bar.
pub(crate) struct ProgressReader<R, F> {
    inner: R,
    total: u64,
    read: u64,
    last_emitted: u64,
    step: u64,
    callback: F,
}

impl<R: Read, F: Fn(u64, u64)> ProgressReader<R, F> {
    pub(crate) fn new(inner: R, total: u64, callback: F) -> Self {
        let step = (total / 100).max(256 * 1024);
        Self {
            inner,
            total,
            read: 0,
            last_emitted: 0,
            step,
            callback,
        }
    }
}

impl<R: Read, F: Fn(u64, u64)> Read for ProgressReader<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.read += n as u64;
            if self.read - self.last_emitted >= self.step || self.read >= self.total {
                self.last_emitted = self.read;
                (self.callback)(self.read, self.total);
            }
        }
        Ok(n)
    }
}

/// Bring the app back to the foreground after an OAuth callback.
/// On macOS, this activates the app using AppleScript.
/// On other platforms, this is a no-op.
pub fn activate_app() {
    #[cfg(target_os = "macos")]
    {
        let pid = std::process::id();
        let script = format!(
            r#"tell application "System Events"
    set targetProcess to first application process whose unix id is {}
    set frontmost of targetProcess to true
end tell"#,
            pid
        );
        let result = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output();

        match result {
            Ok(output) if output.status.success() => {
                debug!("Activated app via AppleScript");
            }
            _ => {
                // Fallback: try to activate by bundle ID for .app builds
                let _ = std::process::Command::new("open")
                    .args(["-b", "com.djuploader.app"])
                    .output();
            }
        }
    }
}

/// Documented upload size limits.
pub(crate) const MAX_AUDIO_BYTES: u64 = 4_294_967_296; // 4 GiB — SoundCloud & Mixcloud
pub(crate) const MAX_MIXCLOUD_PICTURE_BYTES: u64 = 10_485_760; // 10 MiB — Mixcloud `picture`

/// Human-readable byte size for error messages.
pub(crate) fn human_bytes(bytes: u64) -> String {
    const GIB: f64 = 1_073_741_824.0;
    const MIB: f64 = 1_048_576.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MB", b / MIB)
    } else {
        format!("{bytes} bytes")
    }
}

/// Pre-flight: fail (before reading the file into memory) if it exceeds a limit.
pub(crate) fn ensure_within_limit(path: &Path, max_bytes: u64, label: &str) -> Result<()> {
    let size = std::fs::metadata(path)
        .with_context(|| format!("Failed to read {label} metadata"))?
        .len();
    if size > max_bytes {
        bail!(
            "{label} is {} — exceeds the {} limit",
            human_bytes(size),
            human_bytes(max_bytes)
        );
    }
    Ok(())
}

/// Best-effort MIME type for an audio file from its extension. The uploaders
/// accept mp3/m4a/wav/flac, so a hardcoded `audio/mpeg` would mislabel the rest.
pub(crate) fn audio_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("m4a" | "aac") => "audio/mp4",
        Some("wav") => "audio/wav",
        Some("flac") => "audio/flac",
        Some("aif" | "aiff") => "audio/aiff",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    }
}

pub fn handle_auth(platform: Platform) -> Result<()> {
    match platform {
        Platform::Mixcloud => {
            mixcloud::MixcloudClient::authorize()?;
        }
        Platform::Soundcloud => {
            soundcloud::SoundcloudClient::authorize()?;
        }
    }
    Ok(())
}

/// Simple upload progress for the CLI: overwrites one stderr line with a percent.
fn cli_progress(sent: u64, total: u64) {
    use std::io::Write;
    if let Some(pct) = (sent * 100).checked_div(total) {
        eprint!("\r  Uploading: {pct}%   ");
        let _ = std::io::stderr().flush();
    }
}

pub fn handle_upload(
    platform: Platform,
    file_path: &Path,
    title: &str,
    description: Option<&str>,
    image_path: Option<&Path>,
    tags: Option<Vec<String>>,
    publish_date: Option<&str>,
) -> Result<()> {
    match platform {
        Platform::Mixcloud => {
            let mut client = mixcloud::MixcloudClient::new()?;
            let response = client.upload(
                file_path,
                title,
                description,
                image_path,
                tags,
                publish_date,
                cli_progress,
            )?;

            println!("\n✓ Upload successful!");
            println!("  Message: {}", response.result.message);
            println!("  Key: {}", response.result.key);
            println!("  URL: https://www.mixcloud.com{}", response.result.key);
            if publish_date.is_some() {
                println!("  Scheduled: Yes (check Mixcloud for publish time)");
            }
        }
        Platform::Soundcloud => {
            let mut client = soundcloud::SoundcloudClient::new()?;
            let response = client.upload(
                file_path,
                title,
                description,
                image_path,
                tags,
                cli_progress,
            )?;

            println!("\n✓ Upload successful!");
            println!("  ID: {}", response.id);
            println!("  Title: {}", response.title);
            if let Some(url) = response.permalink_url {
                println!("  URL: {}", url);
            }
            if let Some(desc) = response.description {
                println!("  Description: {}", desc);
            }
        }
    }
    Ok(())
}

pub fn show_status() -> Result<()> {
    let token_storage = TokenStorage::load()?;

    println!("\n=== DJ Uploader Status ===\n");

    // Mixcloud status
    match &token_storage.mixcloud {
        Some(token_info) => {
            println!("Mixcloud: ✓ Authorized");
            println!(
                "  Token created: {}",
                token_info.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );

            if let Some(remaining) = token_info.time_until_expiry() {
                let days = remaining.num_days();
                let hours = remaining.num_hours() % 24;
                let minutes = remaining.num_minutes() % 60;

                if days > 0 {
                    println!("  Expires in: {} days, {} hours", days, hours);
                } else if hours > 0 {
                    println!("  Expires in: {} hours, {} minutes", hours, minutes);
                } else if minutes > 0 {
                    println!("  Expires in: {} minutes", minutes);
                } else {
                    println!("  Expires in: <1 minute (needs refresh)");
                }

                if token_info.is_expired() {
                    println!("  Status: ⚠️  Expired or expiring soon");
                    if token_info.refresh_token.is_some() {
                        println!("  Will auto-refresh on next upload");
                    } else {
                        println!("  Run 'dj-uploader auth mixcloud' to re-authorize");
                    }
                }
            } else {
                println!("  Expires: Unknown (no expiry info)");
            }
        }
        None => {
            println!("Mixcloud: ✗ Not authorized");
            println!("  Run 'dj-uploader auth mixcloud' to authorize");
        }
    }

    println!();

    // SoundCloud status
    match &token_storage.soundcloud {
        Some(token_info) => {
            println!("SoundCloud: ✓ Authorized");
            println!(
                "  Token created: {}",
                token_info.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );

            if let Some(remaining) = token_info.time_until_expiry() {
                let hours = remaining.num_hours();
                let minutes = remaining.num_minutes() % 60;

                if hours > 0 {
                    println!("  Expires in: {} hours, {} minutes", hours, minutes);
                } else if minutes > 0 {
                    println!("  Expires in: {} minutes", minutes);
                } else {
                    println!("  Expires in: <1 minute (needs refresh)");
                }

                if token_info.is_expired() {
                    println!("  Status: ⚠️  Expired or expiring soon");
                    if token_info.refresh_token.is_some() {
                        println!("  Will auto-refresh on next upload");
                    } else {
                        println!("  Run 'dj-uploader auth soundcloud' to re-authorize");
                    }
                }
            } else {
                println!("  Expires: Unknown (no expiry info)");
            }
        }
        None => {
            println!("SoundCloud: ✗ Not authorized");
            println!("  Run 'dj-uploader auth soundcloud' to authorize");
        }
    }

    println!("\nToken storage: {}", TokenStorage::location());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(500), "500 bytes");
        assert_eq!(human_bytes(MAX_MIXCLOUD_PICTURE_BYTES), "10.0 MB");
        assert_eq!(human_bytes(MAX_AUDIO_BYTES), "4.00 GB");
    }

    #[test]
    fn ensure_within_limit_checks_size() {
        let path = std::env::temp_dir().join("dj-size-test.bin");
        std::fs::write(&path, b"hello world").unwrap(); // 11 bytes
        assert!(ensure_within_limit(&path, 100, "test").is_ok());
        assert!(ensure_within_limit(&path, 5, "test").is_err());
        let _ = std::fs::remove_file(&path);
    }
}
