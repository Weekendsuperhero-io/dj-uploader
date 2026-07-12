use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngExt;
use std::fs;

fn encrypt_string(plaintext: &str, key: &[u8; 32], nonce: &[u8; 12]) -> String {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from(*nonce);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .expect("encryption failure");

    hex::encode(ciphertext)
}

fn main() {
    // Generate the Tauri context (reads tauri.conf.json, capabilities, icons).
    tauri_build::build();

    // API client credentials are resolved with this precedence:
    //   1. environment variables (e.g. injected by `op run` from 1Password)
    //   2. a local, git-ignored config.json (optional convenience for dev)
    //   3. a placeholder (so CI can build/test without any secrets)
    // The chosen values are AES-256-GCM encrypted and baked into the binary
    // (obfuscation, not real protection — the key ships in the same binary).
    let config: Option<serde_json::Value> = fs::read_to_string("config.json")
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok());

    let resolve = |env_key: &str, section: &str, field: &str, placeholder: &str| -> String {
        if let Ok(value) = std::env::var(env_key)
            && !value.is_empty()
        {
            return value;
        }
        config
            .as_ref()
            .and_then(|c| c.get(section))
            .and_then(|s| s.get(field))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| placeholder.to_string())
    };

    let mc_id = resolve(
        "DJ_MIXCLOUD_CLIENT_ID",
        "mixcloud",
        "client_id",
        "MIXCLOUD_CLIENT_ID_PLACEHOLDER",
    );
    let mc_secret = resolve(
        "DJ_MIXCLOUD_CLIENT_SECRET",
        "mixcloud",
        "client_secret",
        "MIXCLOUD_CLIENT_SECRET_PLACEHOLDER",
    );
    let sc_id = resolve(
        "DJ_SOUNDCLOUD_CLIENT_ID",
        "soundcloud",
        "client_id",
        "SOUNDCLOUD_CLIENT_ID_PLACEHOLDER",
    );
    let sc_secret = resolve(
        "DJ_SOUNDCLOUD_CLIENT_SECRET",
        "soundcloud",
        "client_secret",
        "SOUNDCLOUD_CLIENT_SECRET_PLACEHOLDER",
    );

    // Generate a random 256-bit AES key and 96-bit nonce for this build.
    let mut rng = rand::rng();
    let key: [u8; 32] = rng.random();
    let nonce: [u8; 12] = rng.random();

    println!(
        "cargo:rustc-env=MIXCLOUD_CLIENT_ID={}",
        encrypt_string(&mc_id, &key, &nonce)
    );
    println!(
        "cargo:rustc-env=MIXCLOUD_CLIENT_SECRET={}",
        encrypt_string(&mc_secret, &key, &nonce)
    );
    println!(
        "cargo:rustc-env=SOUNDCLOUD_CLIENT_ID={}",
        encrypt_string(&sc_id, &key, &nonce)
    );
    println!(
        "cargo:rustc-env=SOUNDCLOUD_CLIENT_SECRET={}",
        encrypt_string(&sc_secret, &key, &nonce)
    );

    // Store the encryption key and nonce as hex strings.
    println!("cargo:rustc-env=ENCRYPTION_KEY={}", hex::encode(key));
    println!("cargo:rustc-env=ENCRYPTION_NONCE={}", hex::encode(nonce));

    let using_real_creds =
        mc_id != "MIXCLOUD_CLIENT_ID_PLACEHOLDER" || sc_id != "SOUNDCLOUD_CLIENT_ID_PLACEHOLDER";
    if using_real_creds {
        println!("cargo:warning=Building with embedded API credentials");
    } else {
        println!(
            "cargo:warning=Building with PLACEHOLDER API credentials — set the DJ_* env vars (e.g. via `op run`) or provide config.json for working uploads"
        );
    }

    // Rebuild when the credential source changes.
    println!("cargo:rerun-if-changed=config.json");
    for env_key in [
        "DJ_MIXCLOUD_CLIENT_ID",
        "DJ_MIXCLOUD_CLIENT_SECRET",
        "DJ_SOUNDCLOUD_CLIENT_ID",
        "DJ_SOUNDCLOUD_CLIENT_SECRET",
    ] {
        println!("cargo:rerun-if-env-changed={env_key}");
    }

    println!("cargo:warning=Building with AES-256-GCM encrypted credentials");
}
