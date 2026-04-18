#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_PATH="$SITE_DIR/zine.ziggy"
BACKUP_PATH="$(mktemp "$SITE_DIR/.zine.ziggy.backup.XXXXXX")"
GENERATED_PATH="$(mktemp "$SITE_DIR/.zine.ziggy.generated.XXXXXX")"

cleanup() {
  if [ -f "$BACKUP_PATH" ]; then
    mv "$BACKUP_PATH" "$CONFIG_PATH"
  fi
  rm -f "$GENERATED_PATH"
}

cp "$CONFIG_PATH" "$BACKUP_PATH"
trap cleanup EXIT

node "$SITE_DIR/scripts/write-zine-config.mjs" \
  --template "$BACKUP_PATH" \
  --out "$GENERATED_PATH"
cp "$GENERATED_PATH" "$CONFIG_PATH"

cd "$SITE_DIR"
zine release "$@"
