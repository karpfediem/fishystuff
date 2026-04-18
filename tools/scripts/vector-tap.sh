#!/usr/bin/env bash
set -euo pipefail

default_api_url="${FISHYSTUFF_VECTOR_TAP_URL:-${VECTOR_TAP_URL:-http://127.0.0.1:8686/graphql}}"
default_duration_ms="${FISHYSTUFF_VECTOR_TAP_DURATION_MS:-10000}"
default_interval_ms="${FISHYSTUFF_VECTOR_TAP_INTERVAL_MS:-500}"
default_limit="${FISHYSTUFF_VECTOR_TAP_LIMIT:-100}"
default_format="${FISHYSTUFF_VECTOR_TAP_FORMAT:-json}"

print_help() {
  cat <<'EOF'
usage: tools/scripts/vector-tap.sh [preset] [options] [-- <extra vector tap args>]

Repo-native entrypoint for live local Vector inspection.
It defaults to:
  - the local Vector GraphQL API at http://127.0.0.1:8686/graphql
  - JSON output with event metadata
  - quiet output
  - a 10 second bounded sample window

Presets:
  all                   Tap all source/transform outputs.
  process-logs          Tap normalized local process logs before Loki.
  raw-process-logs      Tap raw process log lines from data/vector/process/*.log.
  browser-logs          Tap normalized browser OTLP log events before Loki.
  raw-browser-logs      Tap raw OTLP log ingress events.
  raw-traces            Tap raw OTLP trace ingress events.
  raw-metrics           Tap raw OTLP metric ingress events.
  to-loki               Tap the events entering the Loki sink.
  to-collector-traces   Tap the traces entering the OTEL collector trace sink.
  to-collector-metrics  Tap the metrics entering the OTEL collector metric sink.

Options:
  --follow              Stream until interrupted instead of auto-exiting.
  --duration-ms <ms>    Override the bounded sample window.
  --interval <ms>       Override Vector tap sampling interval.
  --limit <count>       Override Vector tap sample limit per interval.
  --format <fmt>        json, yaml, or logfmt.
  --url <url>           Override the Vector GraphQL API endpoint.
  --no-meta             Hide Vector tap metadata.
  --show-notices        Include Vector tap notices instead of quiet mode.
  --list-presets        Print the preset catalog and exit.
  -h, --help            Print this help and exit.

Examples:
  tools/scripts/vector-tap.sh browser-logs
  tools/scripts/vector-tap.sh process-logs --follow
  tools/scripts/vector-tap.sh raw-traces --duration-ms 3000
  tools/scripts/vector-tap.sh to-loki -- --format logfmt
EOF
}

print_presets() {
  cat <<'EOF'
all                  outputs-of *
process-logs         outputs-of normalized_process_logs
raw-process-logs     outputs-of devenv_process_logs
browser-logs         outputs-of normalized_telemetry_logs
raw-browser-logs     outputs-of telemetry_logs_ingress.logs
raw-traces           outputs-of telemetry_otlp_ingress.traces
raw-metrics          outputs-of telemetry_otlp_ingress.metrics
to-loki              inputs-of logs_loki
to-collector-traces  inputs-of telemetry_ingress_traces_to_collector
to-collector-metrics inputs-of telemetry_ingress_metrics_to_collector
EOF
}

api_url="$default_api_url"
duration_ms="$default_duration_ms"
interval_ms="$default_interval_ms"
limit="$default_limit"
format="$default_format"
follow=false
include_meta=true
quiet=true
preset="all"
preset_explicit=false
extra_args=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help)
      print_help
      exit 0
      ;;
    --list-presets)
      print_presets
      exit 0
      ;;
    --follow)
      follow=true
      ;;
    --duration-ms)
      shift
      duration_ms="${1:-}"
      ;;
    --duration-ms=*)
      duration_ms="${1#*=}"
      ;;
    --interval)
      shift
      interval_ms="${1:-}"
      ;;
    --interval=*)
      interval_ms="${1#*=}"
      ;;
    --limit)
      shift
      limit="${1:-}"
      ;;
    --limit=*)
      limit="${1#*=}"
      ;;
    --format)
      shift
      format="${1:-}"
      ;;
    --format=*)
      format="${1#*=}"
      ;;
    --url)
      shift
      api_url="${1:-}"
      ;;
    --url=*)
      api_url="${1#*=}"
      ;;
    --no-meta)
      include_meta=false
      ;;
    --show-notices)
      quiet=false
      ;;
    --)
      shift
      extra_args=("$@")
      break
      ;;
    -*)
      echo "unknown option: $1" >&2
      print_help >&2
      exit 2
      ;;
    *)
      if [ "$preset_explicit" = true ]; then
        echo "expected at most one preset before --, got: $1" >&2
        print_help >&2
        exit 2
      fi
      preset="$1"
      preset_explicit=true
      ;;
  esac
  shift
done

tap_scope=()
case "$preset" in
  all)
    tap_scope=(--outputs-of "*")
    ;;
  process-logs)
    tap_scope=(--outputs-of "normalized_process_logs")
    ;;
  raw-process-logs)
    tap_scope=(--outputs-of "devenv_process_logs")
    ;;
  browser-logs)
    tap_scope=(--outputs-of "normalized_telemetry_logs")
    ;;
  raw-browser-logs)
    tap_scope=(--outputs-of "telemetry_logs_ingress.logs")
    ;;
  raw-traces)
    tap_scope=(--outputs-of "telemetry_otlp_ingress.traces")
    ;;
  raw-metrics)
    tap_scope=(--outputs-of "telemetry_otlp_ingress.metrics")
    ;;
  to-loki)
    tap_scope=(--inputs-of "logs_loki")
    ;;
  to-collector-traces)
    tap_scope=(--inputs-of "telemetry_ingress_traces_to_collector")
    ;;
  to-collector-metrics)
    tap_scope=(--inputs-of "telemetry_ingress_metrics_to_collector")
    ;;
  *)
    echo "unknown preset: $preset" >&2
    print_presets >&2
    exit 2
    ;;
esac

tap_cmd=(
  vector tap
  --url "$api_url"
  --format "$format"
  --interval "$interval_ms"
  --limit "$limit"
)

if [ "$include_meta" = true ]; then
  tap_cmd+=(--meta)
fi
if [ "$quiet" = true ]; then
  tap_cmd+=(--quiet)
fi
if [ "$follow" = false ]; then
  tap_cmd+=(--duration-ms "$duration_ms")
fi

tap_cmd+=("${tap_scope[@]}")
tap_cmd+=("${extra_args[@]}")

if ! command -v vector >/dev/null 2>&1; then
  echo "vector is not on PATH." >&2
  echo "Run this from an active devenv shell or wrap it with:" >&2
  echo "  devenv shell -- tools/scripts/vector-tap.sh $preset" >&2
  exit 1
fi

exec "${tap_cmd[@]}"
