#!/bin/bash
# One-time helper: create the DJ Uploader items in 1Password from your current
# local values, so `.env.1password` resolves. Safe to re-run only after deleting
# the items it creates (op item create fails if the title already exists).
#
#   ./scripts/1password-seed.sh                 # uses the "Private" vault
#   OP_VAULT="Dev" ./scripts/1password-seed.sh  # or a vault of your choice
#
# After running, keep the vault name in .env.1password in sync with $OP_VAULT.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VAULT="${OP_VAULT:-Private}"
KEY_FILE="${TAURI_KEY_FILE:-$HOME/.tauri/dj-uploader-updater.key}"
CONFIG_JSON="$PROJECT_ROOT/src-tauri/config.json"

command -v op >/dev/null 2>&1 || { echo "1Password CLI (op) not installed" >&2; exit 1; }
op whoami >/dev/null 2>&1 || { echo "Not signed in — run: eval \$(op signin)" >&2; exit 1; }

echo "Creating items in vault: $VAULT"

# ── Updater signing key (from the generated key file) ───────────────────────
if [ -f "$KEY_FILE" ]; then
    op item create --vault "$VAULT" --category "API Credential" --title "dj-uploader-updater" \
        "private_key[password]=$(cat "$KEY_FILE")" \
        "password[password]=" \
        && echo "✓ dj-uploader-updater"
else
    echo "⚠ Updater key not found at $KEY_FILE — skipping (generate with: cargo tauri signer generate -w $KEY_FILE)"
fi

# ── Platform API credentials (from the local config.json) ───────────────────
if [ -f "$CONFIG_JSON" ]; then
    read -r MC_ID MC_SECRET SC_ID SC_SECRET < <(python3 - "$CONFIG_JSON" <<'PY'
import json, sys
c = json.load(open(sys.argv[1]))
mc = c.get("mixcloud", {}); sc = c.get("soundcloud", {})
print(mc.get("client_id",""), mc.get("client_secret",""), sc.get("client_id",""), sc.get("client_secret",""))
PY
)
    op item create --vault "$VAULT" --category "API Credential" --title "dj-uploader-api" \
        "mixcloud_client_id[text]=$MC_ID" \
        "mixcloud_client_secret[password]=$MC_SECRET" \
        "soundcloud_client_id[text]=$SC_ID" \
        "soundcloud_client_secret[password]=$SC_SECRET" \
        && echo "✓ dj-uploader-api"
else
    echo "⚠ $CONFIG_JSON not found — skipping API item"
fi

# ── Apple signing + notarization (identity known; fill the rest) ────────────
op item create --vault "$VAULT" --category "API Credential" --title "dj-uploader-apple" \
    "signing_identity[text]=Developer ID Application: Mark Blake (WA364LS8BA)" \
    "api_issuer[text]=" \
    "api_key_id[text]=" \
    && echo "✓ dj-uploader-apple — now, in the 1Password app: fill api_issuer + api_key_id, and" \
    && echo "  attach your App Store Connect AuthKey_XXXX.p8 as a file field named 'AuthKey.p8'"

echo ""
echo "Done. Now build with secrets injected:"
echo "  pnpm op:build      # op run --env-file=.env.1password -- tauri build"
