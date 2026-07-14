use anyhow::{Context, Result, bail};
use log::{debug, info};
use reqwest::blocking::{Client, multipart};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use url::Url;

use crate::config::{MixcloudCredentials, TokenInfo, TokenStorage};

const OAUTH_AUTHORIZE_URL: &str = "https://www.mixcloud.com/oauth/authorize";
const OAUTH_TOKEN_URL: &str = "https://www.mixcloud.com/oauth/access_token";
const UPLOAD_URL: &str = "https://api.mixcloud.com/upload/";
const REDIRECT_URI: &str = "http://localhost:8888/callback";

/// Extract the access token from a Mixcloud token response, accepting either a
/// JSON body (`{"access_token": "..."}`) or a form-encoded one (`access_token=...`).
fn extract_access_token(body: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(token) = value.get("access_token").and_then(|t| t.as_str())
    {
        return Some(token.to_string());
    }
    url::form_urlencoded::parse(body.as_bytes())
        .find(|(key, _)| key == "access_token")
        .map(|(_, value)| value.into_owned())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub result: UploadResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub success: bool,
    pub message: String,
    pub key: String,
}

pub struct MixcloudClient {
    client: Client,
    token_storage: TokenStorage,
}

impl MixcloudClient {
    pub fn new() -> Result<Self> {
        // No overall request timeout: a large mix on a slow uplink must not be
        // cut off while it's still making progress. Instead, bound the initial
        // connect and use TCP keepalive so a genuinely dead/stalled connection
        // still errors out during a long upload.
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        let token_storage = TokenStorage::load()?;

        Ok(Self {
            client,
            token_storage,
        })
    }

    pub fn authorize() -> Result<()> {
        info!("Starting Mixcloud OAuth2 authorization...");

        let credentials = MixcloudCredentials::new();

        // Build authorization URL
        let mut auth_url = Url::parse(OAUTH_AUTHORIZE_URL)?;
        auth_url
            .query_pairs_mut()
            .append_pair("client_id", &credentials.client_id)
            .append_pair("redirect_uri", REDIRECT_URI);

        println!("\nOpening browser for authorization...");
        println!("If the browser doesn't open, visit this URL:\n");
        println!("{}\n", auth_url);

        // Open browser
        if let Err(e) = webbrowser::open(auth_url.as_str()) {
            eprintln!("Failed to open browser: {}", e);
        }

        // Start local server to receive callback
        let listener = TcpListener::bind("127.0.0.1:8888")
            .context("Failed to start callback server. Is port 8888 already in use?")?;

        println!("Waiting for authorization...");

        let (mut stream, _) = listener.accept()?;
        let buf_reader = BufReader::new(&stream);
        let request_line = buf_reader
            .lines()
            .next()
            .context("Failed to read request")?
            .context("Empty request")?;

        // Parse the authorization code from the request
        let code = Self::extract_code_from_request(&request_line)?;

        // Send success response to browser with auto-close script
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
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
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
        // Auto-close after 2 seconds
        setTimeout(function() {
            window.close();
        }, 2000);
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

        // Exchange the code for an access token. Per the Mixcloud docs the
        // parameters go in the query string; the response carries the token.
        let client = Client::new();
        let params = [
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("code", code.as_str()),
        ];

        let response = client
            .get(OAUTH_TOKEN_URL)
            .query(&params)
            .send()
            .context("Failed to exchange authorization code")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Token exchange failed with status {}: {}", status, body);
        }

        let body = response.text().context("Failed to read token response")?;
        let access_token =
            extract_access_token(&body).context("Mixcloud did not return an access token")?;

        // Mixcloud tokens are long-lived: no refresh token, no expiry.
        let token_info = TokenInfo::new(access_token, None, None);

        TokenStorage::update(|storage| storage.set_mixcloud_tokens(token_info))
            .context("Failed to preserve OAuth tokens in the Keychain")?;

        // Bring the app back to the foreground
        super::activate_app();

        println!("\n✓ Authorization successful!");
        println!("Token saved to: {}", TokenStorage::location());

        println!("\nYou can now upload mixes with:");
        println!("  dj-uploader upload mixcloud --file <path> --title \"Your Mix\"");

        Ok(())
    }

    fn extract_code_from_request(request_line: &str) -> Result<String> {
        // Request line looks like: GET /callback?code=AUTH_CODE HTTP/1.1
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

        Ok(code)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upload(
        &mut self,
        file_path: &Path,
        title: &str,
        description: Option<&str>,
        image_path: Option<&Path>,
        tags: Option<Vec<String>>,
        publish_date: Option<&str>,
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
        if self.token_storage.mixcloud.is_none() {
            println!("\nNo authorization found. Starting OAuth2 flow...\n");
            Self::authorize()?;
            // Reload token storage after authorization
            self.token_storage = TokenStorage::load()?;
        }

        // Mixcloud tokens are long-lived and have no refresh flow; a revoked
        // token surfaces as a 401 on upload and the user re-authorizes.
        let access_token = self.token_storage.get_mixcloud_token()?.access_token.clone();

        info!("Uploading {} to Mixcloud...", file_path.display());

        // Compute the invariant parts once; each retry only rebuilds the form.
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid file name")?
            .to_string();

        // Stream large DJ mixes directly from disk. Reading a multi-gigabyte mix
        // into a Vec here made uploads memory-heavy and prone to failing before
        // reqwest could send the first byte.
        let audio_len = file_path
            .metadata()
            .context("Failed to read audio file metadata")?
            .len();

        // Prepare (and size-check) the cover art once. A bad image is a
        // permanent error, so surface it here before the retry loop.
        let artwork = if let Some(img_path) = image_path.filter(|p| p.exists()) {
            let art = crate::artwork::prepare(img_path)?;
            let picture_len = art.bytes.len() as u64;
            if picture_len > super::MAX_MIXCLOUD_PICTURE_BYTES {
                bail!(
                    "Cover art is {} — exceeds Mixcloud's {} picture limit",
                    super::human_bytes(picture_len),
                    super::human_bytes(super::MAX_MIXCLOUD_PICTURE_BYTES)
                );
            }
            Some(art)
        } else {
            None
        };

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

            let mut form = multipart::Form::new().part("mp3", file_part);
            form = form.text("name", title.to_string());
            if let Some(desc) = description {
                form = form.text("description", desc.to_string());
            }
            if let Some(art) = &artwork {
                let img_part = multipart::Part::bytes(art.bytes.clone())
                    .file_name(art.file_name.clone())
                    .mime_str(art.mime)
                    .map_err(|e| super::AttemptError::Permanent(anyhow::Error::new(e)))?;
                form = form.part("picture", img_part);
            }
            // Mixcloud expects tags-0-tag, tags-1-tag, etc.
            if let Some(tag_list) = &tags {
                for (index, tag) in tag_list.iter().enumerate() {
                    form = form.text(format!("tags-{index}-tag"), tag.to_string());
                }
            }
            // publish_date is Pro-only; harmless to include otherwise.
            if let Some(date) = publish_date {
                form = form.text("publish_date", date.to_string());
            }

            debug!("Sending upload request...");
            let response = match self
                .client
                .post(UPLOAD_URL)
                .query(&[("access_token", &access_token)])
                .multipart(form)
                .send()
            {
                Ok(r) => r,
                Err(e) => {
                    // A cancel surfaces here as a body/stream error; don't retry it.
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        return Err(super::AttemptError::Cancelled);
                    }
                    return Err(super::AttemptError::Retryable(
                        anyhow::Error::new(e).context("Failed to upload file"),
                    ));
                }
            };

            let status = response.status();
            if !status.is_success() {
                let body = response.text().unwrap_or_default();
                return Err(super::classify_status(
                    status,
                    anyhow::anyhow!("Upload failed with status {status}: {body}"),
                ));
            }

            let response_text = response.text().map_err(|e| {
                super::AttemptError::Permanent(
                    anyhow::Error::new(e).context("Failed to read response body"),
                )
            })?;

            // Always print the response so we can see what Mixcloud returns.
            println!("\nMixcloud API Response:");
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

#[cfg(test)]
mod tests {
    use super::extract_access_token;

    #[test]
    fn parses_json_token() {
        assert_eq!(
            extract_access_token(r#"{"access_token":"abc123","scope":"upload"}"#).as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn parses_form_encoded_token() {
        assert_eq!(
            extract_access_token("access_token=xyz789&scope=upload").as_deref(),
            Some("xyz789")
        );
    }

    #[test]
    fn none_when_token_absent() {
        assert!(extract_access_token(r#"{"error":"invalid_grant"}"#).is_none());
    }
}
