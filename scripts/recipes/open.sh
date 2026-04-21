#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

target="$(normalize_named_arg target "${1-}")"
ssh_target="$(normalize_named_arg ssh_target "${2-root@beta.fishystuff.fish}")"
local_port="$(normalize_named_arg local_port "${3-3300}")"

case "$target" in
  site) url="http://127.0.0.1:1990/" ;;
  map) url="http://127.0.0.1:1990/map/" ;;
  api) url="http://127.0.0.1:8080/api/v1/meta" ;;
  cdn) url="http://127.0.0.1:4040/" ;;
  jaeger) url="http://127.0.0.1:16686/" ;;
  grafana|logs|loki) url="http://127.0.0.1:3000/explore" ;;
  dashboard|grafana-dashboard)
    url="http://127.0.0.1:3000/d/fishystuff-operator-overview/fishystuff-operator-overview"
    ;;
  dashboard-local|grafana-dashboard-local)
    url="http://127.0.0.1:3000/d/fishystuff-local-observability/fishystuff-local-observability"
    ;;
  grafana-beta|logs-beta|loki-beta)
    exec secretspec run --profile beta-deploy -- \
      env FS_BETA_SSH_TARGET="$ssh_target" FS_BETA_LOCAL_PORT="$local_port" \
      bash tools/scripts/open-beta-grafana.sh grafana
    ;;
  dashboard-beta|grafana-dashboard-beta)
    exec secretspec run --profile beta-deploy -- \
      env FS_BETA_SSH_TARGET="$ssh_target" FS_BETA_LOCAL_PORT="$local_port" \
      bash tools/scripts/open-beta-grafana.sh dashboard
    ;;
  loki-status) url="http://127.0.0.1:3100/services" ;;
  prometheus) url="http://127.0.0.1:9090/" ;;
  vector) url="http://127.0.0.1:8686/playground" ;;
  *)
    echo "unknown open target: $target" >&2
    echo "available targets: site map api cdn jaeger grafana dashboard dashboard-local grafana-beta dashboard-beta logs loki logs-beta loki-beta loki-status prometheus vector" >&2
    exit 2
    ;;
esac

exec xdg-open "$url"
