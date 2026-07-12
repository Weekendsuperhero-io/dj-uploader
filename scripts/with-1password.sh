#!/bin/bash
# Run any command with DJ Uploader's secrets injected from 1Password.
#
#   ./scripts/with-1password.sh pnpm tauri build
#   ./scripts/with-1password.sh ./scripts/create-release.sh
#
# All `op://` references in .env.1password are resolved into environment
# variables for the duration of the command only (never written to disk).

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

command -v op >/dev/null 2>&1 || {
    echo "1Password CLI (op) not installed — brew install 1password-cli" >&2
    exit 1
}

exec op run --env-file="$PROJECT_ROOT/.env.1password" -- "$@"
