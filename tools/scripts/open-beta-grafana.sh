#!/usr/bin/env bash
set -euo pipefail

view="${1:-dashboard}"
ssh_target="${FS_BETA_SSH_TARGET:-root@beta.fishystuff.fish}"
local_port="${FS_BETA_LOCAL_PORT:-3300}"
remote_port="${FS_BETA_REMOTE_PORT:-3000}"
socket_path="/tmp/fishystuff-beta-grafana-${local_port}.sock"

case "$view" in
  dashboard)
    path="/d/fishystuff-operator-overview/fishystuff-operator-overview"
    ;;
  grafana | explore | logs | loki)
    path="/d/fishystuff-operator-overview/fishystuff-operator-overview?orgId=1&var-env=beta&viewPanel=17"
    ;;
  *)
    echo "unknown beta grafana view: $view" >&2
    echo "available views: dashboard grafana" >&2
    exit 2
    ;;
esac

tmp_key="$(mktemp /tmp/fishystuff-beta-grafana-ssh.XXXXXX)"
cleanup() {
  rm -f "$tmp_key"
}
trap cleanup EXIT

umask 077
printf '%s\n' "${HETZNER_SSH_PRIVATE_KEY:?missing HETZNER_SSH_PRIVATE_KEY}" > "$tmp_key"
chmod 600 "$tmp_key"

ssh_base=(
  ssh
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o ExitOnForwardFailure=yes
  -o ServerAliveInterval=30
  -o ServerAliveCountMax=3
  -S "$socket_path"
)

if ! "${ssh_base[@]}" -O check "$ssh_target" >/dev/null 2>&1; then
  rm -f "$socket_path"
  if ! "${ssh_base[@]}" -fN -M -L "127.0.0.1:${local_port}:127.0.0.1:${remote_port}" "$ssh_target"; then
    echo "failed to establish beta Grafana tunnel on 127.0.0.1:${local_port}" >&2
    echo "override the local port with: just open ${view}-beta local_port=<port>" >&2
    exit 1
  fi
fi

url="http://127.0.0.1:${local_port}${path}"
printf 'beta Grafana tunnel ready: %s -> %s:127.0.0.1:%s\n' "127.0.0.1:${local_port}" "$ssh_target" "$remote_port"
printf 'open %s\n' "$url"

if [[ "${FS_SKIP_OPEN:-0}" == "1" ]]; then
  exit 0
fi

if command -v xdg-open >/dev/null 2>&1; then
  exec xdg-open "$url"
fi

echo "xdg-open not found; open the URL above manually" >&2
