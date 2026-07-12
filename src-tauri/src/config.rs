use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use keyring_core::Entry;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// AES-256-GCM encrypted compile-time credentials (read from config.json during build)
const ENCRYPTED_MIXCLOUD_CLIENT_ID: &str = env!("MIXCLOUD_CLIENT_ID");
const ENCRYPTED_MIXCLOUD_CLIENT_SECRET: &str = env!("MIXCLOUD_CLIENT_SECRET");

// AES-256-GCM encrypted SoundCloud credentials
const ENCRYPTED_SOUNDCLOUD_CLIENT_ID: &str = env!("SOUNDCLOUD_CLIENT_ID");
const ENCRYPTED_SOUNDCLOUD_CLIENT_SECRET: &str = env!("SOUNDCLOUD_CLIENT_SECRET");

// Encryption key and nonce
const ENCRYPTION_KEY: &str = env!("ENCRYPTION_KEY");
const ENCRYPTION_NONCE: &str = env!("ENCRYPTION_NONCE");

fn decrypt_string(ciphertext_hex: &str) -> String {
    // Parse the encryption key and nonce from hex
    let key_bytes = hex::decode(ENCRYPTION_KEY).expect("Invalid encryption key");
    let nonce_bytes = hex::decode(ENCRYPTION_NONCE).expect("Invalid encryption nonce");

    // Parse ciphertext from hex
    let ciphertext = hex::decode(ciphertext_hex).expect("Invalid ciphertext hex");

    // Create cipher
    let key: [u8; 32] = key_bytes.try_into().expect("Key must be 32 bytes");
    let cipher = Aes256Gcm::new(&key.into());
    let nonce_arr: [u8; 12] = nonce_bytes.try_into().expect("Nonce must be 12 bytes");
    let nonce = Nonce::from(nonce_arr);

    // Decrypt
    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .expect("Decryption failed");

    String::from_utf8(plaintext).expect("Invalid UTF-8 after decryption")
}

#[derive(Debug, Clone)]
pub struct MixcloudCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl MixcloudCredentials {
    pub fn new() -> Self {
        Self {
            client_id: decrypt_string(ENCRYPTED_MIXCLOUD_CLIENT_ID),
            client_secret: decrypt_string(ENCRYPTED_MIXCLOUD_CLIENT_SECRET),
        }
    }
}

impl Default for MixcloudCredentials {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SoundcloudCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl SoundcloudCredentials {
    pub fn new() -> Self {
        Self {
            client_id: decrypt_string(ENCRYPTED_SOUNDCLOUD_CLIENT_ID),
            client_secret: decrypt_string(ENCRYPTED_SOUNDCLOUD_CLIENT_SECRET),
        }
    }
}

impl Default for SoundcloudCredentials {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Seconds until token expires (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

impl TokenInfo {
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            created_at: Utc::now(),
            expires_in,
        }
    }

    /// Check if token is expired or will expire soon (within 5 minutes)
    pub fn is_expired(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let expiry_time = self.created_at + Duration::seconds(expires_in);
            let now = Utc::now();
            let buffer = Duration::minutes(5);

            now >= (expiry_time - buffer)
        } else {
            // If no expiry info, assume it's still valid
            false
        }
    }

    pub fn time_until_expiry(&self) -> Option<Duration> {
        if let Some(expires_in) = self.expires_in {
            let expiry_time = self.created_at + Duration::seconds(expires_in);
            let now = Utc::now();
            let remaining = expiry_time - now;

            if remaining.num_seconds() > 0 {
                Some(remaining)
            } else {
                Some(Duration::zero())
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixcloud: Option<TokenInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soundcloud: Option<TokenInfo>,
}

/// Keychain service + account under which both providers' OAuth tokens are
/// stored as one JSON record in the user's macOS login Keychain.
const KEYCHAIN_SERVICE: &str = "com.djuploader.app";
const KEYCHAIN_ACCOUNT: &str = "oauth-tokens";

/// Serialize in-process read/modify/write operations so two OAuth callbacks or
/// a token refresh cannot overwrite the other provider's freshly saved token.
static TOKEN_STORAGE_UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// Register the login Keychain as keyring-core's default store, once, before
/// any `Entry` is created. Unlike the protected-data store, this backend works
/// for Developer ID and unsigned development builds without a provisioning profile.
fn ensure_keychain_store() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(
        || match apple_native_keyring_store::keychain::Store::new() {
            Ok(store) => keyring_core::set_default_store(store),
            Err(e) => eprintln!("Warning: failed to initialize keychain store: {e}"),
        },
    );
}

impl TokenStorage {
    fn keychain_entry() -> Result<Entry> {
        ensure_keychain_store();
        Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .context("Failed to access the system keychain")
    }

    /// Read the token blob from the keychain. `Ok(Some(json))` if present,
    /// `Ok(None)` if there's no item yet, or `Err` if the keychain itself is
    /// unavailable — e.g. while the login Keychain is locked.
    fn keychain_get() -> Result<Option<String>> {
        let entry = Self::keychain_entry()?;
        match entry.get_password() {
            Ok(json) => Ok(Some(json)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
    }

    pub fn load() -> Result<Self> {
        match Self::keychain_get() {
            Ok(Some(json)) => {
                serde_json::from_str(&json).context("Failed to parse tokens from keychain")
            }
            // Keychain reachable but empty: import a token file if one exists, else start fresh.
            Ok(None) => Ok(Self::migrate_legacy_tokens()?.unwrap_or_default()),
            // Keychain unavailable: preserve access through the local fallback.
            Err(_) => Self::load_from_file(),
        }
    }

    fn keychain_set(contents: &str) -> Result<()> {
        Self::keychain_entry()?
            .set_password(contents)
            .context("Failed to write tokens to the macOS Keychain")
    }

    pub fn save(&self) -> Result<()> {
        let contents = serde_json::to_string(self).context("Failed to serialize tokens")?;
        match Self::keychain_set(&contents) {
            Ok(()) => {
                // A confirmed Keychain write makes any old fallback stale.
                if let Ok(path) = Self::token_file_path() {
                    let _ = fs::remove_file(path);
                }
                Ok(())
            }
            Err(error) => {
                eprintln!("Warning: {error:#}; using the local token fallback");
                self.save_to_file(&contents)
            }
        }
    }

    /// Atomically load the latest two-provider token record, mutate it, and
    /// persist it back to the macOS Keychain (or the development file fallback).
    pub fn update(mutator: impl FnOnce(&mut Self)) -> Result<Self> {
        let _guard = TOKEN_STORAGE_UPDATE_LOCK
            .lock()
            .map_err(|_| anyhow::anyhow!("Token storage update lock was poisoned"))?;
        let mut storage = Self::load()?;
        mutator(&mut storage);
        storage.save()?;
        Ok(storage)
    }

    /// Fallback store for builds where the keychain isn't available (dev).
    fn load_from_file() -> Result<Self> {
        let path = Self::token_file_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path).context("Failed to read token file")?;
        serde_json::from_str(&contents).context("Failed to parse token file")
    }

    fn save_to_file(&self, contents: &str) -> Result<()> {
        let path = Self::token_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create token directory")?;
        }
        let mut options = fs::OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&path).context("Failed to open token file")?;
        file.write_all(contents.as_bytes())
            .context("Failed to write token file")?;
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .context("Failed to secure token file permissions")?;
        Ok(())
    }

    /// When the Keychain is empty, import an existing fallback file. The file is
    /// removed only after a confirmed Keychain write; a failed promotion leaves
    /// it intact so the next load retains both providers.
    fn migrate_legacy_tokens() -> Result<Option<Self>> {
        let Ok(path) = Self::token_file_path() else {
            return Ok(None);
        };
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path).context("Failed to read legacy token file")?;
        Self::promote_fallback(&path, &contents, Self::keychain_set).map(Some)
    }

    fn promote_fallback(
        path: &Path,
        contents: &str,
        keychain_set: impl FnOnce(&str) -> Result<()>,
    ) -> Result<Self> {
        let storage =
            serde_json::from_str(contents).context("Failed to parse legacy token file")?;
        match keychain_set(contents) {
            Ok(()) => {
                let _ = fs::remove_file(path);
            }
            Err(error) => {
                eprintln!("Warning: could not promote OAuth tokens to Keychain: {error:#}");
            }
        }
        Ok(storage)
    }

    fn token_file_path() -> Result<PathBuf> {
        // Use XDG_CONFIG_HOME if set, otherwise ~/.config
        let config_dir = if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else {
            dirs::home_dir()
                .context("Failed to determine home directory")?
                .join(".config")
        };

        Ok(config_dir.join("dj-uploader").join("tokens.json"))
    }

    /// Human-readable description of where tokens live (for status output).
    pub fn location() -> String {
        format!("macOS Keychain (service: {KEYCHAIN_SERVICE})")
    }

    pub fn set_mixcloud_tokens(&mut self, token_info: TokenInfo) {
        self.mixcloud = Some(token_info);
    }

    pub fn set_soundcloud_tokens(&mut self, token_info: TokenInfo) {
        self.soundcloud = Some(token_info);
    }

    pub fn get_mixcloud_token(&self) -> Result<&TokenInfo> {
        self.mixcloud
            .as_ref()
            .context("Not authorized with Mixcloud. Run 'dj-uploader auth mixcloud' first")
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenInfo, TokenStorage};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn token(value: &str) -> TokenInfo {
        TokenInfo::new(
            value.to_string(),
            Some(format!("{value}-refresh")),
            Some(3600),
        )
    }

    fn fallback_path(test_name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dj-uploader-{test_name}-{}-{nonce}.json",
            std::process::id()
        ))
    }

    #[test]
    fn setting_mixcloud_token_preserves_soundcloud_token() {
        let mut storage = TokenStorage {
            mixcloud: None,
            soundcloud: Some(token("soundcloud")),
        };

        storage.set_mixcloud_tokens(token("mixcloud"));

        assert_eq!(
            storage
                .soundcloud
                .as_ref()
                .map(|token| token.access_token.as_str()),
            Some("soundcloud")
        );
    }

    #[test]
    fn setting_soundcloud_token_preserves_mixcloud_token() {
        let mut storage = TokenStorage {
            mixcloud: Some(token("mixcloud")),
            soundcloud: None,
        };

        storage.set_soundcloud_tokens(token("soundcloud"));

        assert_eq!(
            storage
                .mixcloud
                .as_ref()
                .map(|token| token.access_token.as_str()),
            Some("mixcloud")
        );
    }

    #[test]
    fn two_provider_token_record_round_trips() {
        let storage = TokenStorage {
            mixcloud: Some(token("mixcloud")),
            soundcloud: Some(token("soundcloud")),
        };

        let json = serde_json::to_string(&storage).unwrap();
        let restored: TokenStorage = serde_json::from_str(&json).unwrap();

        assert_eq!(
            (
                restored
                    .mixcloud
                    .as_ref()
                    .map(|token| token.access_token.as_str()),
                restored
                    .soundcloud
                    .as_ref()
                    .map(|token| token.access_token.as_str()),
            ),
            (Some("mixcloud"), Some("soundcloud"))
        );
    }

    #[test]
    fn failed_keychain_promotion_preserves_fallback_file() {
        let path = fallback_path("failed-promotion");
        let contents = serde_json::to_string(&TokenStorage {
            mixcloud: Some(token("mixcloud")),
            soundcloud: Some(token("soundcloud")),
        })
        .unwrap();
        fs::write(&path, &contents).unwrap();

        TokenStorage::promote_fallback(&path, &contents, |_| {
            Err(anyhow::anyhow!("simulated Keychain failure"))
        })
        .unwrap();

        assert!(
            path.exists(),
            "fallback file was deleted after a failed promotion"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn successful_keychain_promotion_removes_fallback_file() {
        let path = fallback_path("successful-promotion");
        let contents = serde_json::to_string(&TokenStorage {
            mixcloud: Some(token("mixcloud")),
            soundcloud: Some(token("soundcloud")),
        })
        .unwrap();
        fs::write(&path, &contents).unwrap();

        TokenStorage::promote_fallback(&path, &contents, |_| Ok(())).unwrap();

        assert!(
            !path.exists(),
            "fallback file remained after a successful promotion"
        );
    }
}
