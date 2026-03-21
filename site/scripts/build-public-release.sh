#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$(mktemp -d "$SITE_DIR/.public-build.XXXXXX")"
PREV_DIR="$SITE_DIR/public.prev"

if [ -z "${FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY:-}" ]; then
  if git -C "$SITE_DIR/.." rev-parse HEAD >/dev/null 2>&1; then
    FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY="$(git -C "$SITE_DIR/.." rev-parse --short=16 HEAD)"
  else
    FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY="$(date -u +%Y%m%dT%H%M%SZ)"
  fi
fi
export FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY

cleanup() {
  rm -rf "$BUILD_DIR" "$PREV_DIR"
}

trap cleanup EXIT

cd "$SITE_DIR"

bun run pre-build
zine release --output "$BUILD_DIR"
bun run ./scripts/write-runtime-config.mjs --out "$BUILD_DIR/runtime-config.js"
bun run tailwind:scan
bunx @tailwindcss/cli -i tailwind.input.css -o "$BUILD_DIR/css/site.css" --minify

rm -rf "$PREV_DIR"
if [ -e "$SITE_DIR/public" ]; then
  mv "$SITE_DIR/public" "$PREV_DIR"
fi
mv "$BUILD_DIR" "$SITE_DIR/public"

trap - EXIT
cleanup
