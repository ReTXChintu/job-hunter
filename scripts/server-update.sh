#!/usr/bin/env bash
# Install, build and (re)start the server from the current checkout. Used by
# `pnpm server:update` and by CI over SSH after `git pull`.
#
# CI's SSH command is a non-interactive shell: it doesn't read ~/.bashrc, which
# is where nvm, fnm, Volta or pnpm's installer usually put node/pnpm/pm2 on
# PATH. So look for them the way those tools install them.
set -euo pipefail
cd "$(dirname "$0")/.."

export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
if [ -s "$NVM_DIR/nvm.sh" ]; then
  # shellcheck disable=SC1091
  . "$NVM_DIR/nvm.sh" >/dev/null
  nvm use --silent default >/dev/null 2>&1 || true
fi
if command -v fnm >/dev/null 2>&1; then
  eval "$(fnm env)"
fi
for dir in "${PNPM_HOME:-$HOME/.local/share/pnpm}" "$HOME/.volta/bin" "$HOME/.local/bin" "$HOME/.npm-global/bin" "$HOME/bin" /usr/local/bin; do
  if [ -d "$dir" ]; then PATH="$dir:$PATH"; fi
done
export PATH

if ! command -v node >/dev/null 2>&1; then
  echo "server-update: node isn't on PATH for non-interactive shells. Install it system-wide or with nvm/fnm/Volta in the default location." >&2
  exit 127
fi
if ! command -v pnpm >/dev/null 2>&1; then
  if command -v corepack >/dev/null 2>&1; then
    # Node's own corepack provides the pnpm version pinned in package.json.
    mkdir -p "$HOME/.local/bin"
    corepack enable --install-directory "$HOME/.local/bin" pnpm
    PATH="$HOME/.local/bin:$PATH"
  else
    echo "server-update: pnpm not found and corepack isn't available. Install pnpm (npm install -g pnpm)." >&2
    exit 127
  fi
fi
if ! command -v pm2 >/dev/null 2>&1; then
  echo "server-update: pm2 not found. Install it with: npm install -g pm2" >&2
  exit 127
fi

echo "server-update: node $(node --version), pnpm $(pnpm --version), pm2 $(pm2 --version)"
pnpm install --frozen-lockfile
pnpm server:build
pnpm server:start
pnpm server:status
