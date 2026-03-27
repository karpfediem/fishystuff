#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <meta-or-paz-or-archive-dir> [output-dir] [-- extra pazifista args]" >&2
  exit 64
fi

INPUT_PATH="$1"
shift

OUTPUT_DIR="$ROOT_DIR/data/data/original-excel"
if [[ $# -gt 0 && "${1:0:1}" != "-" ]]; then
  OUTPUT_DIR="$1"
  shift
fi

cd "$ROOT_DIR"
devenv shell -- cargo run -q -p pazifista -- archive extract-fishing-workbooks \
  "$INPUT_PATH" \
  -o "$OUTPUT_DIR" \
  "$@"
