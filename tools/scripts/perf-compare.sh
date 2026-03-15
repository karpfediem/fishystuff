#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
  echo "usage: $0 <baseline-json> <candidate-json> [limit]" >&2
  exit 2
fi

baseline="$1"
candidate="$2"
limit="${3:-12}"

if command -v jq >/dev/null 2>&1; then
  jq_cmd=(jq)
else
  jq_cmd=(devenv shell -- jq)
fi

"${jq_cmd[@]}" -n --slurpfile base "$baseline" --slurpfile cand "$candidate" --argjson limit "$limit" '
  def r(v): ((v * 1000.0) | round / 1000.0);
  def spans(doc): (doc.named_spans // {});
  def span(doc; key): (spans(doc)[key] // { total_ms: 0, avg_ms: 0, p95_ms: 0, count: 0 });
  def allkeys(base; cand): (((spans(base) | keys) + (spans(cand) | keys)) | unique);
  ($base[0]) as $b |
  ($cand[0]) as $c |
  "baseline=\($b.scenario) candidate=\($c.scenario)",
  "frame_avg_ms delta=\(r($c.frame_time_ms.avg - $b.frame_time_ms.avg)) baseline=\(r($b.frame_time_ms.avg)) candidate=\(r($c.frame_time_ms.avg))",
  "frame_p95_ms delta=\(r($c.frame_time_ms.p95 - $b.frame_time_ms.p95)) baseline=\(r($b.frame_time_ms.p95)) candidate=\(r($c.frame_time_ms.p95))",
  (
    allkeys($b; $c)
    | map({
        name: .,
        delta_total_ms: (span($c; .).total_ms - span($b; .).total_ms),
        baseline_total_ms: span($b; .).total_ms,
        candidate_total_ms: span($c; .).total_ms
      })
    | sort_by(-(.delta_total_ms | if . < 0 then -. else . end))
    | .[:$limit][]
    | "\(.name) delta_total_ms=\(r(.delta_total_ms)) baseline_total_ms=\(r(.baseline_total_ms)) candidate_total_ms=\(r(.candidate_total_ms))"
  )
'
