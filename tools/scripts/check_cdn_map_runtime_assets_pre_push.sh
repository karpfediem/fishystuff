#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_REMOTE_BRANCH="${FISHYSTUFF_PRE_PUSH_TARGET_BRANCH_REF:-refs/heads/main}"
REMOTE_BRANCH="${PRE_COMMIT_REMOTE_BRANCH:-}"

# pre-commit exposes the destination ref for pre-push hooks. Skip the expensive
# CDN check unless this push targets main.
if [ -n "$REMOTE_BRANCH" ] && [ "$REMOTE_BRANCH" != "$TARGET_REMOTE_BRANCH" ]; then
  echo "Skipping CDN map runtime hook for ${REMOTE_BRANCH}; only runs when pushing to ${TARGET_REMOTE_BRANCH}." >&2
  exit 0
fi

exec "$ROOT_DIR/tools/scripts/check_cdn_map_runtime_assets.sh"
