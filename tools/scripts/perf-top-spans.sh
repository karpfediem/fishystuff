#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <report-json> [limit]" >&2
  exit 2
fi

report="$1"
limit="${2:-8}"

if command -v jq >/dev/null 2>&1; then
  jq_cmd=(jq)
else
  jq_cmd=(devenv shell -- jq)
fi

"${jq_cmd[@]}" -r --argjson limit "$limit" '
  def r: ((. * 1000.0) | round / 1000.0);
  "scenario=\(.scenario) frames=\(.frames) warmup=\(.warmup_frames) frame_avg_ms=\(.frame_time_ms.avg | r) p95_ms=\(.frame_time_ms.p95 | r)",
  (
    .named_spans
    | to_entries
    | sort_by(-.value.total_ms)
    | .[:$limit][]
    | "\(.key) total_ms=\(.value.total_ms | r) avg_ms=\(.value.avg_ms | r) p95_ms=\(.value.p95_ms | r) count=\(.value.count)"
  )
' "$report"
