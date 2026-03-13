#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

until (echo > /dev/tcp/127.0.0.1/3306) >/dev/null 2>&1; do
  echo "waiting for Dolt SQL server on 127.0.0.1:3306..."
  sleep 1
done

exec cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_server -- \
  --config "$ROOT_DIR/api/config.toml" \
  --bind 127.0.0.1:8080
