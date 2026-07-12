# Building & signing DJ Uploader (macOS)

This produces a **signed + notarized** `.dmg` you can hand to anyone — it installs
with a normal double-click, no "unidentified developer" warning.

## TL;DR

```bash
pnpm install
pnpm op:build:universal     # signed + notarized universal DMG (Intel + Apple Silicon)
```

Result:
```
src-tauri/target/universal-apple-darwin/release/bundle/dmg/DJ Uploader_2026.6.1_universal.dmg
```
Send that file. (Version comes from `src-tauri/Cargo.toml`.)

> Path note: a targeted build lands under `target/universal-apple-darwin/release/bundle/`.
> A plain `pnpm op:build` (your Mac's arch only) lands under `target/release/bundle/`
> as `..._aarch64.dmg` (Apple Silicon) or `..._x64.dmg` (Intel).

---

## One-time setup

1. **Xcode command line tools** — `xcode-select --install`
2. **Rust** (rustup) and **Node + pnpm**
3. Both macOS Rust targets (for a universal build):
   ```bash
   rustup target add aarch64-apple-darwin x86_64-apple-darwin
   ```
4. **Developer ID Application certificate** in your login keychain. You already have
   `Developer ID Application: Mark Blake (WA364LS8BA)` — confirm with:
   ```bash
   security find-identity -v -p codesigning
   ```
   (The free "Apple Development" cert can't be notarized — you need the Developer ID one.)
5. **App Store Connect API key** for notarization: App Store Connect → Users and Access
   → Integrations → App Store Connect API → generate a key. Note the **Issuer ID** and
   **Key ID**, and download the `AuthKey_<KeyID>.p8` (downloadable only once).
6. **Secrets in 1Password** (see below).

## Secrets

Secret **values** live in 1Password. The repo stores only *pointers* to them, in one
committed file:

> ### 👉 `.env.1password` (repo root)
> This is the file that maps each build environment variable to an
> `op://vault/item/field` reference. It contains **no secrets**. `pnpm op:build`
> reads it via `op run`. **You only edit this file if your vault isn't named
> `Private` or you use different item/field names** (see "If your setup differs").

One-time setup:

```bash
op signin
./scripts/1password-seed.sh          # creates the 3 items below, pre-filling what it can
```

Then fill in the fields. The three items and how each field maps to `.env.1password`:

### 1. `dj-uploader-apple` — code signing + notarization

| 1Password field                  | → env var in `.env.1password` | Value                                                |
| -------------------------------- | ----------------------------- | ---------------------------------------------------- |
| `signing_identity`               | `APPLE_SIGNING_IDENTITY`      | `Developer ID Application: Mark Blake (WA364LS8BA)`  |
| `api_issuer`                     | `APPLE_API_ISSUER`            | App Store Connect **Issuer ID**                      |
| `api_key_id`                     | `APPLE_API_KEY`               | App Store Connect **Key ID**                         |
| `AuthKey.p8` *(file attachment)* | *(not in `.env.1password`)*   | the downloaded `AuthKey_<KeyID>.p8` — attach as a file field named exactly `AuthKey.p8` |

> The `.p8` is fetched separately by `scripts/with-appstore-key.sh` (it materializes it
> to a temp file, sets `APPLE_API_KEY_PATH`, and deletes it after the build), which is
> why it's an attachment on the item rather than a line in `.env.1password`.

### 2. `dj-uploader-updater` — auto-updater signing key

| 1Password field | → env var in `.env.1password`         | Value                                            |
| --------------- | ------------------------------------- | ------------------------------------------------ |
| `private_key`   | `TAURI_SIGNING_PRIVATE_KEY`           | contents of `~/.tauri/dj-uploader-updater.key`   |
| `password`      | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`  | the key's password (leave empty if none)         |

### 3. `dj-uploader-api` — Mixcloud/SoundCloud app credentials (baked into the binary)

| 1Password field            | → env var in `.env.1password`   | Value                     |
| -------------------------- | ------------------------------- | ------------------------- |
| `mixcloud_client_id`       | `DJ_MIXCLOUD_CLIENT_ID`         | Mixcloud client id        |
| `mixcloud_client_secret`   | `DJ_MIXCLOUD_CLIENT_SECRET`     | Mixcloud client secret    |
| `soundcloud_client_id`     | `DJ_SOUNDCLOUD_CLIENT_ID`       | SoundCloud client id      |
| `soundcloud_client_secret` | `DJ_SOUNDCLOUD_CLIENT_SECRET`   | SoundCloud client secret  |

### If your setup differs (edit `.env.1password`)

- **Vault not named `Private`?** Replace `Private` in every `op://Private/...` line of
  `.env.1password` with your vault name (and run the seed with `OP_VAULT=YourVault`).
- **Different item or field names?** Change the matching `op://…` reference line in
  `.env.1password` so it points where your value actually lives.
- **`.p8` reference:** the default is `op://Private/dj-uploader-apple/AuthKey.p8`. To
  point elsewhere set `APPLE_API_KEY_REF="op://…"`; if it's stored as a 1Password
  *Document* (not a file field) set `APPLE_API_KEY_DOC=1`. Both are env vars you export
  before `pnpm op:build`, not entries in `.env.1password`.

> Prefer not to use 1Password? Export `APPLE_SIGNING_IDENTITY`, `APPLE_API_ISSUER`,
> `APPLE_API_KEY`, `APPLE_API_KEY_PATH` (path to your `.p8`), `TAURI_SIGNING_PRIVATE_KEY`,
> and the `DJ_*` client creds, then run `pnpm tauri build`. See `scripts/README.md`.

## Build

```bash
pnpm op:build:universal      # universal (recommended for hand-off)
# or
pnpm op:build                # just your Mac's architecture (faster)
```

`tauri build` automatically: builds the frontend + Rust release binary → assembles
`DJ Uploader.app` → **signs** it with your Developer ID (hardened runtime +
`src-tauri/entitlements.plist`) → **notarizes** with Apple and **staples** the ticket →
packages the `.dmg`. Notarization adds a few minutes (it waits on Apple).

## Keychain token storage

Mixcloud and SoundCloud tokens are stored together as one generic-password record in the
user's macOS **login Keychain**. This backend works for Developer ID and development builds
without a provisioning profile or a `keychain-access-groups` entitlement.

If the login Keychain is locked or unavailable, `TokenStorage` falls back to
`~/.config/dj-uploader/tokens.json` with owner-only (`0600`) permissions. The fallback is
promoted on a later successful Keychain write and is deleted only after that write is
confirmed, so a failed promotion cannot erase either provider's credentials.

## Verify before sending

```bash
APP="src-tauri/target/release/bundle/macos/DJ Uploader.app"
codesign -dv --verbose=4 "$APP"      # Authority → Developer ID Application: Mark Blake
xcrun stapler validate "$APP"        # → "The validate action worked!"
spctl -a -vvv "$APP"                 # → accepted, source=Notarized Developer ID
```

If all three pass, it will open cleanly on any Mac.

---

## Handing it to your dad

1. Send him **`DJ Uploader_2026.6.1_universal.dmg`** (from `.../bundle/dmg/`). The
   universal build runs on both Intel and Apple-Silicon Macs.
2. He opens the `.dmg` and drags **DJ Uploader** into **Applications**.
3. First launch: double-click it. macOS shows a one-time *"DJ Uploader was downloaded
   from the Internet — Open?"* → he clicks **Open**. That's it — no scary
   "Apple cannot check it for malicious software" block, because it's notarized.

He'll also get **automatic update prompts** in-app whenever you publish a new release
(that's what `pnpm op:release` sets up).

### If notarization is skipped

If you build without the Apple ID / password (e.g. a quick unsigned test), the app is
**not** notarized and your dad would have to right-click → Open the first time and
approve it in System Settings → Privacy & Security. Always build with the full secrets
for anything you hand off.
