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

ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- \
	"$timeout_secs" \
	"$remote_mgmt_bin" \
	"$remote_tar" \
	"${HETZNER_API_TOKEN:-}" \
	"${HETZNER_SSH_KEY_NAME:-}" \
	"${HETZNER_SSH_PUBLIC_KEY:-}" \
	"${CLOUDFLARE_API_TOKEN:-}" \
	"${FISHYSTUFF_HETZNER_STATE:-}" \
	"${FISHYSTUFF_HETZNER_ALLOW_REBUILD:-}" \
	"${FISHYSTUFF_HETZNER_IMAGE:-}" \
	"${FISHYSTUFF_HETZNER_HTTP01_HOST:-}" \
	"${FISHYSTUFF_HETZNER_HTTPS_HOST:-}" \
	"${FISHYSTUFF_HETZNER_MGMT_CONTROL:-}" \
	"${FISHYSTUFF_HETZNER_MGMT_CONTROL_KEY_DIR:-}" <<'EOF'
set -euo pipefail

timeout_secs="${1:?missing timeout seconds}"
remote_mgmt_bin="${2:?missing remote mgmt binary path}"
remote_tar="${3:?missing remote tar path}"
hetzner_api_token="${4:-}"
hetzner_ssh_key_name="${5:-}"
hetzner_ssh_public_key="${6:-}"
cloudflare_api_token="${7:-}"
fishystuff_hetzner_state="${8:-}"
fishystuff_hetzner_allow_rebuild="${9:-}"
fishystuff_hetzner_image="${10:-}"
fishystuff_hetzner_http01_host="${11:-}"
fishystuff_hetzner_https_host="${12:-}"
fishystuff_hetzner_mgmt_control="${13:-}"
fishystuff_hetzner_mgmt_control_key_dir="${14:-}"
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
as_root env \
	HETZNER_API_TOKEN="$hetzner_api_token" \
	HETZNER_SSH_KEY_NAME="$hetzner_ssh_key_name" \
	HETZNER_SSH_PUBLIC_KEY="$hetzner_ssh_public_key" \
	CLOUDFLARE_API_TOKEN="$cloudflare_api_token" \
	FISHYSTUFF_HETZNER_STATE="$fishystuff_hetzner_state" \
	FISHYSTUFF_HETZNER_ALLOW_REBUILD="$fishystuff_hetzner_allow_rebuild" \
	FISHYSTUFF_HETZNER_IMAGE="$fishystuff_hetzner_image" \
	FISHYSTUFF_HETZNER_HTTP01_HOST="$fishystuff_hetzner_http01_host" \
	FISHYSTUFF_HETZNER_HTTPS_HOST="$fishystuff_hetzner_https_host" \
	FISHYSTUFF_HETZNER_MGMT_CONTROL="$fishystuff_hetzner_mgmt_control" \
	FISHYSTUFF_HETZNER_MGMT_CONTROL_KEY_DIR="$fishystuff_hetzner_mgmt_control_key_dir" \
	"$remote_mgmt_bin" deploy --no-git --seeds=http://127.0.0.1:2379 lang --module-path "$remote_module_path" "$remote_tmp/"
EOF
