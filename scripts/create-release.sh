#!/bin/bash

# Build DJ Uploader with Tauri and publish a GitHub release, including the
# updater artifacts (`*.app.tar.gz` + `.sig`) and a generated `latest.json`
# that the in-app updater reads from the "latest" release.
#
# Requirements:
#   - gh CLI (brew install gh) + `gh auth login`
#   - Node + pnpm, Rust toolchain
#   - For a signed/notarized DMG: APPLE_SIGNING_IDENTITY (codesign) +
#     APPLE_API_ISSUER / APPLE_API_KEY / APPLE_API_KEY_PATH (notarization).
#     Easiest via `pnpm op:release` — see scripts/README.md and BUILD.md
#   - For the auto-updater signature: TAURI_SIGNING_PRIVATE_KEY (contents of
#     ~/.tauri/dj-uploader-updater.key) and TAURI_SIGNING_PRIVATE_KEY_PASSWORD

set -euo pipefail

GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
log_info() { echo -e "${BLUE}ℹ ${1}${NC}"; }
log_success() { echo -e "${GREEN}✓ ${1}${NC}"; }
log_warn() { echo -e "${YELLOW}⚠ ${1}${NC}"; }
log_error() { echo -e "${RED}✗ ${1}${NC}"; }

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

REPO="WeekendSuperhero/dj-uploader"
VERSION=$(grep -E '^version = ' "src-tauri/Cargo.toml" | head -1 | sed -E 's/version = "(.*)"/\1/')
TAG="v${VERSION}"
BUNDLE_DIR="src-tauri/target/release/bundle"

log_info "Preparing release for DJ Uploader ${VERSION} (${TAG})"

command -v gh >/dev/null 2>&1 || { log_error "GitHub CLI (gh) not installed — brew install gh"; exit 1; }
gh auth status >/dev/null 2>&1 || { log_error "Not authenticated — run: gh auth login"; exit 1; }
if git rev-parse "$TAG" >/dev/null 2>&1; then
    log_error "Tag $TAG already exists. Bump the version in src-tauri/Cargo.toml first."
    exit 1
fi

if [ "${SKIP_BUILD:-false}" != "true" ]; then
    log_info "Building with 'pnpm tauri build' (this also builds the frontend)…"
    if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
        log_warn "TAURI_SIGNING_PRIVATE_KEY is not set — the updater artifacts will not be signed"
        log_warn "and the in-app updater will reject them. Set it to the contents of the private key."
    fi
    pnpm install --frozen-lockfile
    pnpm tauri build
fi

# Locate build artifacts (arch-specific file names differ between Apple Silicon and Intel).
DMG_FILE=$(ls -t "$BUNDLE_DIR"/dmg/*.dmg 2>/dev/null | head -1 || true)
APP_TAR=$(ls -t "$BUNDLE_DIR"/macos/*.app.tar.gz 2>/dev/null | head -1 || true)
APP_SIG=$(ls -t "$BUNDLE_DIR"/macos/*.app.tar.gz.sig 2>/dev/null | head -1 || true)

[ -n "$DMG_FILE" ] || { log_error "No DMG found under $BUNDLE_DIR/dmg"; exit 1; }
log_success "DMG:     $DMG_FILE"

# Stage assets under space-free names so GitHub download URLs are predictable.
STAGE="src-tauri/target/release/release-assets"
rm -rf "$STAGE"; mkdir -p "$STAGE"
DMG_ASSET="DJ-Uploader-${VERSION}.dmg"
cp "$DMG_FILE" "$STAGE/$DMG_ASSET"
ASSETS=("$STAGE/$DMG_ASSET")

if [ -n "$APP_TAR" ] && [ -n "$APP_SIG" ]; then
    TAR_ASSET="DJ-Uploader-${VERSION}.app.tar.gz"
    cp "$APP_TAR" "$STAGE/$TAR_ASSET"
    cp "$APP_SIG" "$STAGE/${TAR_ASSET}.sig"
    ASSETS+=("$STAGE/$TAR_ASSET" "$STAGE/${TAR_ASSET}.sig")

    # Map the current build arch to the Tauri updater platform key.
    case "$(uname -m)" in
        arm64|aarch64) PLATFORM_KEY="darwin-aarch64" ;;
        x86_64) PLATFORM_KEY="darwin-x86_64" ;;
        *) PLATFORM_KEY="darwin-$(uname -m)" ;;
    esac
    SIG_CONTENT=$(cat "$APP_SIG")
    PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    cat > "$STAGE/latest.json" <<EOF
{
  "version": "${VERSION}",
  "notes": "See the release page for details.",
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "${PLATFORM_KEY}": {
      "signature": "${SIG_CONTENT}",
      "url": "https://github.com/${REPO}/releases/download/${TAG}/${TAR_ASSET}"
    }
  }
}
EOF
    ASSETS+=("$STAGE/latest.json")
    log_success "Updater: $TAR_ASSET (+ .sig, latest.json for $PLATFORM_KEY)"
else
    log_warn "No updater artifacts found — the in-app updater will not have a feed for this release."
    log_warn "Ensure bundle.createUpdaterArtifacts is enabled and TAURI_SIGNING_PRIVATE_KEY is set."
fi

RELEASE_NOTES=$(cat <<EOF
# DJ Uploader ${VERSION}

## Install
1. Download \`${DMG_ASSET}\`
2. Open it and drag **DJ Uploader** to Applications
3. Launch from Applications or Spotlight

## Requirements
- macOS 10.15 (Catalina) or later

<!-- Add highlights here before publishing. -->
EOF
)

echo ""
log_info "Assets to upload:"
for a in "${ASSETS[@]}"; do echo "  - $a"; done
echo ""
read -p "Create draft release $TAG on $REPO? (y/n) " -n 1 -r; echo ""
[[ $REPLY =~ ^[Yy]$ ]] || { log_info "Cancelled"; exit 0; }

gh release create "$TAG" "${ASSETS[@]}" \
    --repo "$REPO" \
    --title "DJ Uploader v${VERSION}" \
    --notes "$RELEASE_NOTES" \
    --draft

log_success "Draft release created."
echo "Publish with: gh release edit $TAG --repo $REPO --draft=false"
echo "(latest.json must live on the LATEST published release for the updater to find it.)"
