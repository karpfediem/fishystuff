#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$SITE_DIR/zine.ziggy"
BACKUP_PATH="$(mktemp "$SITE_DIR/.zine.ziggy.backup.XXXXXX")"
GENERATED_PATH="$(mktemp "$SITE_DIR/.zine.ziggy.generated.XXXXXX")"
GENERATED_CONTENT_ROOT=".generated/content"
OUTPUT_DIR=""
EXPECT_OUTPUT_DIR="0"

cleanup() {
  if [ -f "$BACKUP_PATH" ]; then
    mv "$BACKUP_PATH" "$CONFIG_PATH"
  fi
  rm -f "$GENERATED_PATH"
}

cp "$CONFIG_PATH" "$BACKUP_PATH"
trap cleanup EXIT

for arg in "$@"; do
  if [ "$EXPECT_OUTPUT_DIR" = "1" ]; then
    OUTPUT_DIR="$arg"
    EXPECT_OUTPUT_DIR="0"
    continue
  fi
  case "$arg" in
    --output)
      EXPECT_OUTPUT_DIR="1"
      ;;
    --output=*)
      OUTPUT_DIR="${arg#--output=}"
      ;;
  esac
done

node "$SITE_DIR/scripts/build-shell-pages.mjs" \
  --out-root "$GENERATED_CONTENT_ROOT"

node "$SITE_DIR/scripts/write-zine-config.mjs" \
  --template "$BACKUP_PATH" \
  --out "$GENERATED_PATH" \
  --generated-content-root "$GENERATED_CONTENT_ROOT"
cp "$GENERATED_PATH" "$CONFIG_PATH"

cd "$SITE_DIR"
zine release "$@"

if [ -n "$OUTPUT_DIR" ]; then
  node "$SITE_DIR/scripts/build-sitemap.mjs" \
    --root-dir "$SITE_DIR" \
    --out "$OUTPUT_DIR/sitemap.xml"
  node "$SITE_DIR/scripts/build-robots.mjs" \
    --out "$OUTPUT_DIR/robots.txt"
fi
