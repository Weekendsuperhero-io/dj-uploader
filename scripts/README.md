# DJ Uploader — macOS build & release

DJ Uploader is a **Tauri v2** app: a React + Sistine frontend (repo root) over a
Rust backend (`src-tauri/`). Tauri's bundler builds, signs, notarizes, and
produces the `.app`, `.dmg`, and auto-updater artifacts — replacing the old
hand-rolled `build-and-sign-macos.sh`.

## Quick start

```bash
pnpm install          # frontend deps
pnpm tauri dev        # run the app (hot-reloads the UI)
pnpm tauri build      # produce a release .app + .dmg under src-tauri/target/release/bundle
```

Output:

- `.app`  → `src-tauri/target/release/bundle/macos/DJ Uploader.app`
- `.dmg`  → `src-tauri/target/release/bundle/dmg/DJ Uploader_<version>_<arch>.dmg`
- updater → `…/macos/DJ Uploader.app.tar.gz` (+ `.sig`) when `TAURI_SIGNING_PRIVATE_KEY` is set

App metadata (name, bundle id `com.djuploader.app`, category, min macOS, icons,
file associations, updater feed) lives in **`src-tauri/tauri.conf.json`**. The
version is read from `src-tauri/Cargo.toml`.

## Secrets with 1Password

All secrets are sourced from 1Password via the `op` CLI — nothing sensitive is
committed or exported by hand. The pointers live in one committed file,
**`.env.1password`** (repo root): it maps each build env var to an
`op://vault/item/field` reference (no secrets). **That is the only file to edit** if
your vault or item/field names differ from the defaults.

The exact env-var → `op://` reference → 1Password item/field mapping (the three items
`dj-uploader-apple`, `dj-uploader-updater`, `dj-uploader-api` and every field to fill)
is in **[`BUILD.md`](../BUILD.md#secrets)**.

One-time setup:

```bash
brew install 1password-cli            # if needed
op signin
./scripts/1password-seed.sh           # creates the items from your current local values
```

Then build/release with secrets injected (into the process env only, never disk):

```bash
pnpm op:dev        # op run --env-file=.env.1password -- tauri dev
pnpm op:build      # signed + notarized build
pnpm op:release    # build + draft GitHub release
```

Which secrets flow where:

- **Apple signing/notarization** and the **updater private key** → env vars read by `tauri build`.
- **Mixcloud/SoundCloud client id/secret** → `DJ_*` env vars read by `build.rs`, which
  encrypts them into the binary. `build.rs` falls back to a local git-ignored
  `config.json`, then to placeholders — so **CI builds without any secrets**, and
  `config.json` is no longer committed.

For CI that must produce signed releases, use a
[1Password service account](https://developer.1password.com/docs/service-accounts/)
(`OP_SERVICE_ACCOUNT_TOKEN` as a repo secret) with the same `op run` command, or
map the values to GitHub Actions secrets.

## Code signing & notarization

Tauri reads these environment variables at `tauri build` time:

| Variable                     | Purpose                                                          |
| ---------------------------- | --------------------------------------------------------------- |
| `APPLE_SIGNING_IDENTITY`     | Certificate name, e.g. `Developer ID Application: … (TEAMID)`    |
| `APPLE_CERTIFICATE`          | Base64 `.p12` (CI only; alternative to a keychain identity)      |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` above                                   |
| `APPLE_API_ISSUER`           | App Store Connect Issuer ID (notarization)                      |
| `APPLE_API_KEY`              | App Store Connect Key ID (notarization)                         |
| `APPLE_API_KEY_PATH`         | Path to the `AuthKey_<KeyID>.p8` file (notarization)            |

Codesigning uses the Developer ID certificate; notarization uses an **App Store
Connect API key** (Issuer ID + Key ID + `.p8`). The Apple-ID + app-specific-password
method also works (`APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID`) but the API key is
preferred. Example (signed + notarized):

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_API_ISSUER="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
export APPLE_API_KEY="XXXXXXXXXX"
export APPLE_API_KEY_PATH="$HOME/.appstoreconnect/private_keys/AuthKey_XXXXXXXXXX.p8"
pnpm tauri build
```

With 1Password, `pnpm op:build` handles all of this — `scripts/with-appstore-key.sh`
pulls the `.p8` from your vault into a temp file and deletes it after the build.

Hardened-runtime entitlements are in `src-tauri/entitlements.plist` (network
client/server for the OAuth loopback, user-selected file read/write, JIT).

Use `./scripts/cert-helper.sh {list,setup,troubleshoot,fix}` to inspect or repair
signing certificates (installs Apple WWDR intermediates when needed).

## Auto-updater

The in-app updater (`@tauri-apps/plugin-updater`) checks the **latest GitHub
release** for `latest.json` (endpoint configured in `tauri.conf.json`).

Signing keypair (generated once, kept OUT of the repo):

```bash
cargo tauri signer generate -w ~/.tauri/dj-uploader-updater.key
# → put the PUBLIC key in tauri.conf.json > plugins.updater.pubkey
```

To sign updater artifacts during a build, set:

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/dj-uploader-updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="…"   # if the key has a password
```

## Releasing

```bash
./scripts/create-release.sh
```

This runs `pnpm tauri build`, then creates a **draft** GitHub release with the
DMG, the updater `*.app.tar.gz` + `.sig`, and a generated `latest.json`. Publish
with `gh release edit v<version> --draft=false` — `latest.json` must sit on the
newest published release for the updater to find it.

## Icons

`./scripts/make_icons.sh` regenerates the iconset from a source PNG, or use
`cargo tauri icon assets/dj-uploader.png` to regenerate `src-tauri/icons/`
(source image must be square).

## Prerequisites

- macOS 10.15+, Xcode Command Line Tools
- Rust + Cargo, Node + pnpm
- (Optional) Apple Developer account for signing/notarization

## Resources

- [Tauri: macOS code signing](https://tauri.app/distribute/sign/macos/)
- [Tauri: updater plugin](https://tauri.app/plugin/updater/)
- [Apple Developer Program](https://developer.apple.com/programs/)
