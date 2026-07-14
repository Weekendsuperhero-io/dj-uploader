use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use log::{debug, info, warn};
use rand::RngExt;
use reqwest::blocking::{Client, multipart};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use url::Url;

use crate::config::{SoundcloudCredentials, TokenInfo, TokenStorage};

const OAUTH_AUTHORIZE_URL: &str = "https://secure.soundcloud.com/authorize";
const OAUTH_TOKEN_URL: &str = "https://secure.soundcloud.com/oauth/token";
const UPLOAD_URL: &str = "https://api.soundcloud.com/tracks";
const REDIRECT_URI: &str = "http://localhost:8889/callback";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub id: i64,
    #[serde(default)]
    pub permalink_url: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Generate PKCE code verifier (random string)
fn generate_code_verifier() -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    URL_SAFE_NO_PAD.encode(&random_bytes)
}

/// Generate PKCE code challenge from verifier (SHA256)
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

pub struct SoundcloudClient {
    client: Client,
    credentials: SoundcloudCredentials,
    token_storage: TokenStorage,
}

impl SoundcloudClient {
    /// `force_http1` pins uploads to HTTP/1.1. A large streaming upload over
    /// HTTP/2 can be cut off at a fixed byte offset (h2 flow-control stalls,
    /// proxy body-window limits) — the "always fails at the same percentage"
    /// symptom — so the UI exposes this as a compatibility toggle.
    pub fn new(force_http1: bool) -> Result<Self> {
        // No overall request timeout: a large mix on a slow uplink must not be
        // cut off while it's still making progress. Instead, bound the initial
        // connect and use TCP keepalive so a genuinely dead/stalled connection
        // still errors out during a long upload.
        let mut builder = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .tcp_keepalive(std::time::Duration::from_secs(60));
        if force_http1 {
            builder = builder.http1_only();
        }
        let client = builder.build().context("Failed to create HTTP client")?;

        let credentials = SoundcloudCredentials::new();
        let token_storage = TokenStorage::load()?;

        Ok(Self {
            client,
            credentials,
            token_storage,
        })
    }

    pub fn authorize() -> Result<()> {
        info!("Starting SoundCloud OAuth2 authorization with PKCE...");

        let credentials = SoundcloudCredentials::new();

        // Generate PKCE values
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        // Generate random state for CSRF protection
        let state: String = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        // Build authorization URL
        let mut auth_url = Url::parse(OAUTH_AUTHORIZE_URL)?;
        auth_url
            .query_pairs_mut()
            .append_pair("client_id", &credentials.client_id)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("response_type", "code")
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state);

        println!("\nOpening browser for authorization...");
        println!("If the browser doesn't open, visit this URL:\n");
        println!("{}\n", auth_url);

        // Open browser
        if let Err(e) = webbrowser::open(auth_url.as_str()) {
            eprintln!("Failed to open browser: {}", e);
        }

        // Start local server to receive callback
        let listener = TcpListener::bind("127.0.0.1:8889")
            .context("Failed to start callback server. Is port 8889 already in use?")?;

        println!("Waiting for authorization...");

        let (mut stream, _) = listener.accept()?;
        let buf_reader = BufReader::new(&stream);
        let request_line = buf_reader
            .lines()
            .next()
            .context("Failed to read request")?
            .context("Empty request")?;

        // Parse the authorization code from the request
        let (code, returned_state) = Self::extract_code_from_request(&request_line)?;

        // Validate state
        if returned_state != state {
            bail!("State mismatch - possible CSRF attack");
        }

        // Send success response to browser
        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Authorization Successful</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #ff8800 0%, #ff3300 100%);
            color: white;
        }
        .container {
            text-align: center;
            padding: 2rem;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 10px;
            backdrop-filter: blur(10px);
        }
        h1 { margin: 0 0 1rem 0; }
        p { margin: 0; opacity: 0.9; }
    </style>
</head>
<body>
    <div class="container">
        <h1>✓ Authorization Successful!</h1>
        <p>You can close this window and return to the terminal.</p>
        <p style="margin-top: 1rem; font-size: 0.9em;">This window will close automatically...</p>
    </div>
    <script>
        setTimeout(function() { window.close(); }, 2000);
    </script>
</body>
</html>
"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
            html.len(),
            html
        );
        stream.write_all(response.as_bytes())?;

        info!("Received authorization code, exchanging for access token...");

        // Exchange code for access token
        let http_client = Client::new();
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code".to_string());
        params.insert("client_id", credentials.client_id.clone());
        params.insert("client_secret", credentials.client_secret.clone());
        params.insert("redirect_uri", REDIRECT_URI.to_string());
        params.insert("code", code);
        params.insert("code_verifier", code_verifier);

        let response = http_client
            .post(OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .context("Failed to exchange authorization code")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Token exchange failed with status {}: {}", status, body);
        }

        let token_response: TokenResponse =
            response.json().context("Failed to parse token response")?;

        // Save tokens to storage
        let token_info = TokenInfo::new(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in,
        );

        TokenStorage::update(|storage| storage.set_soundcloud_tokens(token_info))
            .context("Failed to preserve OAuth tokens in the Keychain")?;

        // Bring the app back to the foreground
        super::activate_app();

        println!("\n✓ Authorization successful!");
        println!("Token saved to: {}", TokenStorage::location());

        if let Some(expires_in) = token_response.expires_in {
            let hours = expires_in / 3600;
            println!("Token expires in {} hours", hours);
        }

        println!("\nYou can now upload tracks with:");
        println!("  dj-uploader upload soundcloud --file <path> --title \"Your Track\"");

        Ok(())
    }

    fn extract_code_from_request(request_line: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            bail!("Invalid request format");
        }

        let path = parts[1];
        let url = Url::parse(&format!("http://localhost{}", path))?;

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.to_string())
            .context("Authorization code not found in callback")?;

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.to_string())
            .context("State not found in callback")?;

        Ok((code, state))
    }

    fn refresh_token_if_needed(&mut self) -> Result<()> {
        let token_info = self
            .token_storage
            .soundcloud
            .as_ref()
            .context("No SoundCloud token available")?;

        if token_info.is_expired() {
            warn!("Access token is expired or expiring soon, refreshing...");

            let refresh_token = token_info.refresh_token.as_ref().context(
                "No refresh token available. Please re-authorize with 'dj-uploader auth soundcloud'",
            )?;

            let mut params = HashMap::new();
            params.insert("grant_type", "refresh_token".to_string());
            params.insert("client_id", self.credentials.client_id.clone());
            params.insert("client_secret", self.credentials.client_secret.clone());
            params.insert("refresh_token", refresh_token.clone());

            let response = self
                .client
                .post(OAUTH_TOKEN_URL)
                .form(&params)
                .send()
                .context("Failed to refresh token")?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().unwrap_or_default();
                bail!(
                    "Token refresh failed with status {}: {}. Please re-authorize.",
                    status,
                    body
                );
            }

            let token_response: TokenResponse = response
                .json()
                .context("Failed to parse token refresh response")?;

            // SoundCloud refresh tokens are single-use, so persisting the token
            // that was just consumed would guarantee the next refresh fails.
            let replacement_refresh_token = token_response
                .refresh_token
                .context("SoundCloud did not return a replacement refresh token")?;
            let new_token_info = TokenInfo::new(
                token_response.access_token,
                Some(replacement_refresh_token),
                token_response.expires_in,
            );

            self.token_storage = TokenStorage::update(|storage| {
                storage.set_soundcloud_tokens(new_token_info);
            })?;

            info!("Token refreshed successfully");
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upload(
        &mut self,
        file_path: &Path,
        title: &str,
        description: Option<&str>,
        image_path: Option<&Path>,
        tags: Option<Vec<String>>,
        cancel: Arc<AtomicBool>,
        progress: impl Fn(u64, u64) + Send + Clone + 'static,
        on_retry: impl FnMut(u32, u32, u64, &str),
    ) -> Result<UploadResponse> {
        // Pre-flight — fail fast, before auth or reading the file into memory.
        if !file_path.exists() {
            bail!("File not found: {}", file_path.display());
        }
        super::ensure_within_limit(file_path, super::MAX_AUDIO_BYTES, "Audio file")?;

        // Check if we have a token, if not, authorize first
        if self.token_storage.soundcloud.is_none() {
            println!("\nNo authorization found. Starting OAuth2 flow...\n");
            Self::authorize()?;
            // Reload token storage after authorization
            self.token_storage = TokenStorage::load()?;
        }

        // Refresh token if needed
        self.refresh_token_if_needed()?;

        let access_token = self
            .token_storage
            .soundcloud
            .as_ref()
            .context("No SoundCloud token")?
            .access_token
            .clone();

        info!("Uploading {} to SoundCloud...", file_path.display());

        // Compute the invariant parts once; each retry only rebuilds the form.
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid file name")?
            .to_string();

        // Stream large DJ mixes directly from disk instead of allocating the
        // entire file in memory before the request starts.
        let audio_len = file_path
            .metadata()
            .context("Failed to read audio file metadata")?
            .len();

        // Prepare the cover art once. A bad image is a permanent error, so
        // surface it here before the retry loop.
        let artwork = match image_path.filter(|p| p.exists()) {
            Some(img_path) => Some(crate::artwork::prepare(img_path)?),
            None => None,
        };

        // SoundCloud's tag_list is space-separated, so multi-word tags must be
        // wrapped in double quotes to stay intact.
        let tags_string = tags.map(|tag_list| {
            tag_list
                .iter()
                .filter(|t| !t.is_empty())
                .map(|t| {
                    if t.contains(char::is_whitespace) {
                        format!("\"{t}\"")
                    } else {
                        t.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        });

        // One upload attempt: reopen the file, rebuild the multipart form (the
        // streaming `ProgressReader` is single-use), send, and parse. Wrapped in
        // `send_with_retry` so a dropped connection retries with backoff instead
        // of failing outright.
        let send_once = || -> Result<UploadResponse, super::AttemptError> {
            let audio_file = File::open(file_path).map_err(|e| {
                super::AttemptError::Permanent(
                    anyhow::Error::new(e).context("Failed to open audio file"),
                )
            })?;
            let reader = super::ProgressReader::new(
                audio_file,
                audio_len,
                cancel.clone(),
                progress.clone(),
            );
            let file_part = multipart::Part::reader_with_length(reader, audio_len)
                .file_name(file_name.clone())
                .mime_str(super::audio_mime(file_path))
                .map_err(|e| super::AttemptError::Permanent(anyhow::Error::new(e)))?;

            let mut form = multipart::Form::new().part("track[asset_data]", file_part);
            form = form.text("track[title]", title.to_string());
            if let Some(desc) = description {
                form = form.text("track[description]", desc.to_string());
            }
            if let Some(art) = &artwork {
                let img_part = multipart::Part::bytes(art.bytes.clone())
                    .file_name(art.file_name.clone())
                    .mime_str(art.mime)
                    .map_err(|e| super::AttemptError::Permanent(anyhow::Error::new(e)))?;
                form = form.part("track[artwork_data]", img_part);
            }
            if let Some(tags_string) = &tags_string
                && !tags_string.is_empty()
            {
                form = form.text("track[tag_list]", tags_string.clone());
            }
            form = form.text("track[sharing]", "public");

            debug!("Sending upload request...");
            let started = std::time::Instant::now();
            let response = match self
                .client
                .post(UPLOAD_URL)
                .header("Authorization", format!("OAuth {access_token}"))
                .multipart(form)
                .send()
            {
                Ok(r) => r,
                Err(e) => {
                    // A cancel surfaces here as a body/stream error; don't retry it.
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        return Err(super::AttemptError::Cancelled);
                    }
                    // Note the elapsed time: a *constant* time-to-failure points at
                    // a proxy duration timeout; a constant *byte offset* points at
                    // an h2/body-size cutoff.
                    return Err(super::AttemptError::Retryable(anyhow::Error::new(e).context(
                        format!(
                            "Upload connection failed after {:.1}s",
                            started.elapsed().as_secs_f64()
                        ),
                    )));
                }
            };

            let status = response.status();
            if !status.is_success() {
                let body = response.text().unwrap_or_default();
                return Err(super::classify_status(
                    status,
                    anyhow::anyhow!(
                        "Upload rejected with status {status} after {:.1}s: {body}",
                        started.elapsed().as_secs_f64()
                    ),
                ));
            }

            let response_text = response.text().map_err(|e| {
                super::AttemptError::Permanent(
                    anyhow::Error::new(e).context("Failed to read response body"),
                )
            })?;

            println!("\nSoundCloud API Response:");
            println!("{response_text}");
            println!();

            serde_json::from_str(&response_text).map_err(|e| {
                super::AttemptError::Permanent(
                    anyhow::Error::new(e).context("Failed to parse upload response"),
                )
            })
        };

        let upload_response = super::send_with_retry(&cancel, on_retry, send_once)?;
        info!("Upload successful!");
        Ok(upload_response)
    }
}
