//! Tauri IPC command wrappers around the platform-agnostic upload/auth logic.
//!
//! These mirror the callbacks the old Slint GUI (`gui.rs`) exposed:
//! file selection now happens in the frontend via the dialog plugin, while
//! connecting, disconnecting, checking auth status, and uploading run here.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State, Window};

use crate::config::{TokenInfo, TokenStorage};

/// Event carrying a human-readable stage message ("Uploading to Mixcloud…").
const UPLOAD_STAGE_EVENT: &str = "upload-stage";
/// Event carrying byte-level upload progress, for the progress bar.
const UPLOAD_PROGRESS_EVENT: &str = "upload-progress";
/// Event fired when a transient failure triggers an automatic retry.
const UPLOAD_RETRY_EVENT: &str = "upload-retry";

/// Shared "please cancel the upload" flag. Only one upload runs at a time (the
/// UI disables the button while uploading), so a single flag suffices. Held as
/// Tauri managed state so the `cancel_upload` command and the running upload
/// share it.
#[derive(Clone, Default)]
pub struct UploadCancel(pub Arc<AtomicBool>);

/// Byte progress payload for the `upload-progress` event.
#[derive(Serialize, Clone)]
struct UploadProgress {
    platform: String,
    sent: u64,
    total: u64,
}

/// Payload for the `upload-retry` event, so the UI can show a "reconnecting…"
/// state (which attempt, how long until the next try) instead of failing.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UploadRetry {
    platform: String,
    attempt: u32,
    max_attempts: u32,
    delay_secs: u64,
    reason: String,
}

/// Where a track landed, returned to the frontend so it can show a link.
#[derive(Serialize)]
pub struct UploadOutcome {
    platform: String,
    title: String,
    url: String,
    success: bool,
    error: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PlatformStatus {
    /// A stored token exists (may be expired but auto-refreshable on upload).
    pub connected: bool,
    /// Seconds until the access token expires, if known.
    pub expires_in_seconds: Option<i64>,
    /// The token is expired or expiring within the refresh buffer.
    pub needs_refresh: bool,
}

#[derive(Serialize, Clone)]
pub struct AuthStatus {
    pub mixcloud: PlatformStatus,
    pub soundcloud: PlatformStatus,
}

fn platform_status(token: Option<&TokenInfo>) -> PlatformStatus {
    match token {
        Some(t) => PlatformStatus {
            // An expired token without a refresh token is not upload-ready and
            // should show the Connect action again in the UI.
            connected: !t.is_expired() || t.refresh_token.is_some(),
            expires_in_seconds: t.time_until_expiry().map(|d| d.num_seconds()),
            needs_refresh: t.is_expired(),
        },
        None => PlatformStatus {
            connected: false,
            expires_in_seconds: None,
            needs_refresh: false,
        },
    }
}

/// Report per-platform authorization status from the on-disk token store.
#[tauri::command]
pub async fn get_auth_status() -> Result<AuthStatus, String> {
    let storage = TokenStorage::load().map_err(|e| e.to_string())?;
    Ok(AuthStatus {
        mixcloud: platform_status(storage.mixcloud.as_ref()),
        soundcloud: platform_status(storage.soundcloud.as_ref()),
    })
}

/// Run the browser OAuth flow for a platform (loopback callback server), then
/// bring the app window back to the foreground.
#[tauri::command]
pub async fn connect_platform(window: Window, platform: String) -> Result<(), String> {
    let plat = platform.to_lowercase();

    // The authorize flows are blocking (reqwest::blocking + a TcpListener that
    // waits for the browser redirect), so run them off the async runtime.
    let result = tauri::async_runtime::spawn_blocking(move || match plat.as_str() {
        "mixcloud" => crate::platforms::mixcloud::MixcloudClient::authorize(),
        "soundcloud" => crate::platforms::soundcloud::SoundcloudClient::authorize(),
        other => Err(anyhow::anyhow!("Unknown platform: {other}")),
    })
    .await
    .map_err(|e| e.to_string())?;

    // Refocus the window after the browser round-trip (in addition to the
    // AppleScript re-activation the OAuth flow already performs on macOS).
    let _ = window.set_focus();

    result.map_err(|e| e.to_string())
}

/// Forget the stored token for a platform.
#[tauri::command]
pub async fn disconnect_platform(platform: String) -> Result<(), String> {
    match platform.to_lowercase().as_str() {
        "mixcloud" => TokenStorage::update(|storage| storage.mixcloud = None),
        "soundcloud" => TokenStorage::update(|storage| storage.soundcloud = None),
        other => return Err(format!("Unknown platform: {other}")),
    }
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadParams {
    pub file_path: String,
    pub title: String,
    pub description: String,
    pub image_path: String,
    pub tags: String,
    pub mixcloud: bool,
    pub soundcloud: bool,
    pub schedule_enabled: bool,
    pub schedule_date: String,
    pub schedule_time: String,
    pub generate_previews: bool,
}

/// Optionally generate preview snippets, then upload to the selected platforms.
/// Emits `upload-stage` (message) and `upload-progress` (bytes) events while it
/// works. Returns one `UploadOutcome` per platform (with the public track URL).
#[tauri::command]
pub async fn upload(
    app: AppHandle,
    cancel: State<'_, UploadCancel>,
    params: UploadParams,
) -> Result<Vec<UploadOutcome>, String> {
    if params.file_path.is_empty() || params.title.is_empty() {
        return Err("File and title are required".into());
    }
    if !params.mixcloud && !params.soundcloud {
        return Err("Select at least one platform".into());
    }

    // Clear any leftover cancel request from a previous upload before starting.
    let cancel = cancel.0.clone();
    cancel.store(false, Ordering::Relaxed);

    tauri::async_runtime::spawn_blocking(move || perform_upload(&app, &cancel, params))
        .await
        .map_err(|e| e.to_string())?
}

/// Signal the in-flight upload to stop. The `ProgressReader` polls this flag and
/// aborts the current request; the retry loop then stops instead of retrying.
#[tauri::command]
pub async fn cancel_upload(cancel: State<'_, UploadCancel>) -> Result<(), String> {
    cancel.0.store(true, Ordering::Relaxed);
    Ok(())
}

fn emit_stage(app: &AppHandle, message: &str) {
    let _ = app.emit(UPLOAD_STAGE_EVENT, message.to_string());
}

/// Build a `Fn(sent, total)` that emits byte-progress events for one platform.
fn progress_emitter(app: &AppHandle, platform: &str) -> impl Fn(u64, u64) + Send + Clone + 'static {
    let app = app.clone();
    let platform = platform.to_string();
    move |sent, total| {
        let _ = app.emit(
            UPLOAD_PROGRESS_EVENT,
            UploadProgress {
                platform: platform.clone(),
                sent,
                total,
            },
        );
    }
}

/// Build an `FnMut(attempt, max, delay_secs, reason)` that emits retry events for
/// one platform, so the UI can show a "reconnecting…" state between attempts.
fn retry_emitter(app: &AppHandle, platform: &str) -> impl FnMut(u32, u32, u64, &str) {
    let app = app.clone();
    let platform = platform.to_string();
    move |attempt, max_attempts, delay_secs, reason| {
        let _ = app.emit(
            UPLOAD_RETRY_EVENT,
            UploadRetry {
                platform: platform.clone(),
                attempt,
                max_attempts,
                delay_secs,
                reason: reason.to_string(),
            },
        );
    }
}

fn perform_upload(
    app: &AppHandle,
    cancel: &Arc<AtomicBool>,
    p: UploadParams,
) -> Result<Vec<UploadOutcome>, String> {
    use crate::platforms::{mixcloud, soundcloud as sc};
    use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

    let file = PathBuf::from(&p.file_path);

    // OAuth belongs to the explicit Connect action. Upload must never open a
    // browser or wait for an OAuth callback behind the UI's progress state.
    let storage = TokenStorage::load().map_err(|e| e.to_string())?;
    let mut disconnected = Vec::new();
    if p.mixcloud && storage.mixcloud.is_none() {
        disconnected.push("Mixcloud");
    }
    if p.soundcloud && storage.soundcloud.is_none() {
        disconnected.push("SoundCloud");
    }
    if !disconnected.is_empty() {
        return Err(format!(
            "Connect {} before uploading",
            disconnected.join(" and ")
        ));
    }

    if p.generate_previews {
        emit_stage(app, "Generating preview snippets…");
        match crate::audio::create_preview_snippets(&file) {
            Ok(snippets) => emit_stage(
                app,
                &format!("Generated {} preview snippets", snippets.len()),
            ),
            Err(e) => emit_stage(app, &format!("Warning: preview generation failed: {e}")),
        }
    }

    let image = if p.image_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(&p.image_path))
    };
    let desc = if p.description.is_empty() {
        None
    } else {
        Some(p.description.as_str())
    };
    let tag_list: Option<Vec<String>> = if p.tags.is_empty() {
        None
    } else {
        Some(
            p.tags
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        )
    };

    // Convert a scheduled local publish time to UTC (Mixcloud Pro only).
    let publish_date =
        if p.schedule_enabled && !p.schedule_date.is_empty() && !p.schedule_time.is_empty() {
            let date = NaiveDate::parse_from_str(&p.schedule_date, "%Y-%m-%d")
                .map_err(|e| format!("Invalid date format. Use YYYY-MM-DD: {e}"))?;
            let time = NaiveTime::parse_from_str(&p.schedule_time, "%H:%M")
                .map_err(|e| format!("Invalid time format. Use HH:MM: {e}"))?;
            let naive = NaiveDateTime::new(date, time);
            let local = Local
                .from_local_datetime(&naive)
                .single()
                .ok_or_else(|| "Ambiguous local time".to_string())?;
            Some(
                local
                    .with_timezone(&chrono::Utc)
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            )
        } else {
            None
        };

    let mut results = Vec::new();

    if p.mixcloud {
        emit_stage(app, "Uploading to Mixcloud…");
        let result = mixcloud::MixcloudClient::new().and_then(|mut client| {
            client.upload(
                &file,
                &p.title,
                desc,
                image.as_deref(),
                tag_list.clone(),
                publish_date.as_deref(),
                cancel.clone(),
                progress_emitter(app, "mixcloud"),
                retry_emitter(app, "mixcloud"),
            )
        });
        match result {
            Ok(response) => results.push(UploadOutcome {
                platform: "Mixcloud".into(),
                title: p.title.clone(),
                url: format!("https://www.mixcloud.com{}", response.result.key),
                success: true,
                error: None,
            }),
            Err(error) => results.push(UploadOutcome {
                platform: "Mixcloud".into(),
                title: p.title.clone(),
                url: String::new(),
                success: false,
                error: Some(error.to_string()),
            }),
        }
    }

    if p.soundcloud {
        emit_stage(app, "Uploading to SoundCloud…");
        let result = sc::SoundcloudClient::new().and_then(|mut client| {
            client.upload(
                &file,
                &p.title,
                desc,
                image.as_deref(),
                tag_list,
                cancel.clone(),
                progress_emitter(app, "soundcloud"),
                retry_emitter(app, "soundcloud"),
            )
        });
        match result {
            Ok(response) => results.push(UploadOutcome {
                platform: "SoundCloud".into(),
                title: response.title,
                url: response.permalink_url.unwrap_or_default(),
                success: true,
                error: None,
            }),
            Err(error) => results.push(UploadOutcome {
                platform: "SoundCloud".into(),
                title: p.title.clone(),
                url: String::new(),
                success: false,
                error: Some(error.to_string()),
            }),
        }
    }

    Ok(results)
}
