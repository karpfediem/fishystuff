#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

skopeo --insecure-policy --debug copy \
  docker-archive:"$(nix build .#bot-container --no-link --print-out-paths)" \
  docker://registry.fly.io/criobot:latest \
  --dest-creds x:"$(fly -a criobot tokens create deploy --expiry 10m)" \
  --format v2s2
flyctl deploy --remote-only -c bot/fly.toml
