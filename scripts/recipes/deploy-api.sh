#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

skopeo --insecure-policy --debug copy \
  docker-archive:"$(nix build .#api-container --no-link --print-out-paths)" \
  docker://registry.fly.io/api-fishystuff-fish:latest \
  --dest-creds x:"$(fly -a api-fishystuff-fish tokens create deploy --expiry 10m)" \
  --format v2s2
flyctl deploy --remote-only --smoke-checks=false --wait-timeout 10m -c api/fly.toml
