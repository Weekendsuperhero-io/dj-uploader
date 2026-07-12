#!/bin/bash
# Materialize the App Store Connect API key (.p8) from 1Password into a private
# temp file, run the given command with APPLE_API_KEY_PATH pointing at it, then
# delete the file when the command finishes — even on failure or Ctrl-C.
#
#   ./scripts/with-appstore-key.sh op run --env-file=.env.1password -- tauri build
#
# The .p8 never lands on disk permanently: op writes it with mode 0600 into a
# temp dir that a trap removes on exit.
#
# Config:
#   APPLE_API_KEY_REF  1Password reference to the .p8 (default below).
#   APPLE_API_KEY_DOC  set to 1 if the key is a 1Password *Document* (uses
#                      `op document get` instead of `op read`).

set -euo pipefail

command -v op >/dev/null 2>&1 || { echo "1Password CLI (op) not installed" >&2; exit 1; }
[ "$#" -gt 0 ] || { echo "usage: $0 <command...>" >&2; exit 1; }

P8_REF="${APPLE_API_KEY_REF:-op://Private/Apple Dev Auth Key/AuthKey_8AK388AUVY.p8}"

TMP_DIR="$(mktemp -d -t dj-asc-key)"
trap 'rm -rf "$TMP_DIR"' EXIT
export APPLE_API_KEY_PATH="$TMP_DIR/AuthKey.p8"

if [ "${APPLE_API_KEY_DOC:-0}" = "1" ]; then
    op document get "$P8_REF" --out-file "$APPLE_API_KEY_PATH" >/dev/null
else
    op read --out-file "$APPLE_API_KEY_PATH" "$P8_REF" >/dev/null
fi

# Run the wrapped command with APPLE_API_KEY_PATH exported; the trap deletes the
# key afterwards regardless of how the command exits.
"$@"
