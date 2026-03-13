#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$SITE_DIR/public}"

mkdir -p "$OUT_DIR"
rm -rf "$OUT_DIR/images"
cp -R "$SITE_DIR/assets/images" "$OUT_DIR/images"
