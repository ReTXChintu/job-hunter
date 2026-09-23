#!/usr/bin/env bash
# Start the desktop app in development mode. Pass --mock to use fixture data.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "${1:-}" == "--mock" ]]; then export MOCK_MODE=true; fi
pnpm tauri dev
