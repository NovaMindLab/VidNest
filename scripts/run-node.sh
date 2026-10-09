#!/usr/bin/env bash
set -e

# Detect current Node major version
CURRENT_NODE_MAJOR=$(node -v 2>/dev/null | cut -d'.' -f1 | tr -d 'v')

# Vite 8 / Rolldown requires Node >= 20.12 (for node:util styleText)
if [ -z "$CURRENT_NODE_MAJOR" ] || [ "$CURRENT_NODE_MAJOR" -lt 20 ] 2>/dev/null; then
  # Auto-locate installed Node 20+ from NVM
  CANDIDATE_NODE=$(ls -d "$HOME/.nvm/versions/node/v20"* "$HOME/.nvm/versions/node/v22"* 2>/dev/null | sort -V | tail -n 1)
  if [ -n "$CANDIDATE_NODE" ] && [ -x "$CANDIDATE_NODE/bin/node" ]; then
    export PATH="$CANDIDATE_NODE/bin:$PATH"
  fi
fi

exec npx "$@"
