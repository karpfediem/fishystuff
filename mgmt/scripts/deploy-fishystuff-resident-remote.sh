#!/usr/bin/env bash
set -euo pipefail

graph_dir="${1:?usage: deploy-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET TIMEOUT_SECS [REMOTE_MGMT_BIN]}"
ssh_target="${2:?usage: deploy-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET TIMEOUT_SECS [REMOTE_MGMT_BIN]}"
timeout_secs="${3:?usage: deploy-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET TIMEOUT_SECS [REMOTE_MGMT_BIN]}"
remote_mgmt_bin="${4:-/usr/local/bin/mgmt}"

ssh_opts=()
if [[ -n "${SSH_OPTS:-}" ]]; then
	# shellcheck disable=SC2206
	ssh_opts=(${SSH_OPTS})
fi

local_tar="$(mktemp /tmp/fishystuff-mgmt-deploy.XXXXXX.tar)"
trap 'rm -f "$local_tar"' EXIT
tar -C "$graph_dir" -cf "$local_tar" .

remote_tar="$(ssh "${ssh_opts[@]}" "$ssh_target" 'mktemp /tmp/fishystuff-mgmt-deploy.XXXXXX.tar')"
remote_tar="${remote_tar//$'\r'/}"
remote_tar="${remote_tar//$'\n'/}"

cat "$local_tar" | ssh "${ssh_opts[@]}" "$ssh_target" "cat > '$remote_tar'"

ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- "$timeout_secs" "$remote_mgmt_bin" "$remote_tar" "${HETZNER_API_TOKEN:-}" <<'EOF'
set -euo pipefail

timeout_secs="${1:?missing timeout seconds}"
remote_mgmt_bin="${2:?missing remote mgmt binary path}"
remote_tar="${3:?missing remote tar path}"
hetzner_api_token="${4:-}"
shift 4
remote_tmp="$(mktemp -d /tmp/fishystuff-mgmt-deploy.XXXXXX)"
remote_module_path="$remote_tmp/modules/"
trap 'rm -rf "$remote_tmp"; rm -f "$remote_tar"' EXIT

as_root() {
	if [[ "$(id -u)" == "0" ]]; then
		"$@"
		return
	fi
	sudo "$@"
}

tar -C "$remote_tmp" -xf "$remote_tar"
as_root env HETZNER_API_TOKEN="$hetzner_api_token" "$remote_mgmt_bin" deploy --no-git --seeds=http://127.0.0.1:2379 lang --module-path "$remote_module_path" "$remote_tmp/"
EOF
