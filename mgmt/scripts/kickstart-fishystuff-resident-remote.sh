#!/usr/bin/env bash
set -euo pipefail

graph_dir="${1:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
ssh_target="${2:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
host_name="${3:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
timeout_secs="${4:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
remote_mgmt_bin="${5:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
bootstrap_environment_dir="/etc/fishystuff"
bootstrap_environment_file="$bootstrap_environment_dir/fishystuff-mgmt.env"
bootstrap_environment_file_content=""
bootstrap_client_urls="${FISHYSTUFF_MGMT_BOOTSTRAP_CLIENT_URLS:-}"
bootstrap_server_urls="${FISHYSTUFF_MGMT_BOOTSTRAP_SERVER_URLS:-}"
bootstrap_advertise_client_urls="${FISHYSTUFF_MGMT_BOOTSTRAP_ADVERTISE_CLIENT_URLS:-}"
bootstrap_advertise_server_urls="${FISHYSTUFF_MGMT_BOOTSTRAP_ADVERTISE_SERVER_URLS:-}"
bootstrap_seeds="${FISHYSTUFF_MGMT_BOOTSTRAP_SEEDS:-}"
bootstrap_ssh_url="${FISHYSTUFF_MGMT_BOOTSTRAP_SSH_URL:-}"
bootstrap_ssh_hostkey="${FISHYSTUFF_MGMT_BOOTSTRAP_SSH_HOSTKEY:-}"
bootstrap_ssh_id="${FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID:-}"
bootstrap_ssh_id_dir="${FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID_DIR:-}"
bootstrap_ssh_id_content_b64=""

encode_remote_arg() {
	if [[ -z "$1" ]]; then
		printf '%s' "-"
		return
	fi
	printf '%s' "$1" | base64 -w 0
}

append_environment_line() {
	local name="${1:?missing environment name}"
	local value="${2-}"

	[[ -n "$value" ]] || return 0
	value="${value//\\/\\\\}"
	value="${value//\"/\\\"}"
	if [[ -n "$bootstrap_environment_file_content" ]]; then
		bootstrap_environment_file_content+=$'\n'
	fi
	bootstrap_environment_file_content+="$name=\"$value\""
}

append_environment_line HETZNER_API_TOKEN "${HETZNER_API_TOKEN:-}"
append_environment_line HETZNER_SSH_KEY_NAME "${HETZNER_SSH_KEY_NAME:-}"
append_environment_line HETZNER_SSH_PUBLIC_KEY "${HETZNER_SSH_PUBLIC_KEY:-}"
append_environment_line CLOUDFLARE_API_TOKEN "${CLOUDFLARE_API_TOKEN:-}"
append_environment_line FISHYSTUFF_RESIDENT_RUNTIME_HOSTNAME "$host_name"
append_environment_line FISHYSTUFF_HETZNER_STATE "${FISHYSTUFF_HETZNER_STATE:-}"
append_environment_line FISHYSTUFF_HETZNER_ALLOW_REBUILD "${FISHYSTUFF_HETZNER_ALLOW_REBUILD:-}"
append_environment_line FISHYSTUFF_HETZNER_IMAGE "${FISHYSTUFF_HETZNER_IMAGE:-}"
append_environment_line FISHYSTUFF_HETZNER_HTTP01_HOST "${FISHYSTUFF_HETZNER_HTTP01_HOST:-}"
append_environment_line FISHYSTUFF_HETZNER_HTTPS_HOST "${FISHYSTUFF_HETZNER_HTTPS_HOST:-}"
append_environment_line FISHYSTUFF_HETZNER_MGMT_CONTROL "${FISHYSTUFF_HETZNER_MGMT_CONTROL:-}"
append_environment_line FISHYSTUFF_HETZNER_MGMT_CONTROL_KEY_DIR "${FISHYSTUFF_HETZNER_MGMT_CONTROL_KEY_DIR:-}"

if [[ -n "${FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID_CONTENT:-}" ]]; then
	bootstrap_ssh_id_content_b64="$(encode_remote_arg "$FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID_CONTENT")"
fi

ssh_opts=()
if [[ -n "${SSH_OPTS:-}" ]]; then
	# shellcheck disable=SC2206
	ssh_opts=(${SSH_OPTS})
fi

local_tar="$(mktemp /tmp/fishystuff-mgmt-bootstrap.XXXXXX.tar)"
trap 'rm -f "$local_tar"' EXIT
tar -C "$graph_dir" -cf "$local_tar" .

remote_tar="$(ssh "${ssh_opts[@]}" "$ssh_target" 'mktemp /tmp/fishystuff-mgmt-bootstrap.XXXXXX.tar')"
remote_tar="${remote_tar//$'\r'/}"
remote_tar="${remote_tar//$'\n'/}"

cat "$local_tar" | ssh "${ssh_opts[@]}" "$ssh_target" "cat > '$remote_tar'"

ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- \
	"$host_name" \
	"$timeout_secs" \
	"$remote_mgmt_bin" \
	"$remote_tar" \
	"$bootstrap_environment_dir" \
	"$bootstrap_environment_file" \
	"$(encode_remote_arg "$bootstrap_environment_file_content")" \
	"$(encode_remote_arg "$bootstrap_client_urls")" \
	"$(encode_remote_arg "$bootstrap_server_urls")" \
	"$(encode_remote_arg "$bootstrap_advertise_client_urls")" \
	"$(encode_remote_arg "$bootstrap_advertise_server_urls")" \
	"$(encode_remote_arg "$bootstrap_seeds")" \
	"$(encode_remote_arg "$bootstrap_ssh_url")" \
	"$(encode_remote_arg "$bootstrap_ssh_hostkey")" \
	"$(encode_remote_arg "$bootstrap_ssh_id")" \
	"$(encode_remote_arg "$bootstrap_ssh_id_dir")" \
	"${bootstrap_ssh_id_content_b64:--}" <<'EOF'
set -euo pipefail

host_name="${1:?missing host name}"
timeout_secs="${2:?missing timeout seconds}"
remote_mgmt_bin="${3:?missing remote mgmt binary path}"
remote_tar="${4:?missing remote tar path}"
bootstrap_environment_dir="${5:-}"
bootstrap_environment_file="${6:-}"
bootstrap_environment_file_content_b64="${7:-}"
bootstrap_client_urls_b64="${8:-}"
bootstrap_server_urls_b64="${9:-}"
bootstrap_advertise_client_urls_b64="${10:-}"
bootstrap_advertise_server_urls_b64="${11:-}"
bootstrap_seeds_b64="${12:-}"
bootstrap_ssh_url_b64="${13:-}"
bootstrap_ssh_hostkey_b64="${14:-}"
bootstrap_ssh_id_b64="${15:-}"
bootstrap_ssh_id_dir_b64="${16:-}"
bootstrap_ssh_id_content_b64="${17:-}"
remote_tmp="$(mktemp -d /tmp/fishystuff-mgmt-bootstrap.XXXXXX)"
bootstrap_client_url="http://127.0.0.1:32379"
bootstrap_server_url="http://127.0.0.1:32380"
bootstrap_module_path="/var/lib/fishystuff/modules/"
trap 'rm -rf "$remote_tmp"; rm -f "$remote_tar"' EXIT

decode_remote_arg() {
	if [[ -z "$1" || "$1" == "-" ]]; then
		printf ''
		return
	fi
	printf '%s' "$1" | base64 -d
}

as_root() {
	if [[ "$(id -u)" == "0" ]]; then
		"$@"
		return
	fi
	sudo "$@"
}

bootstrap_client_urls="$(decode_remote_arg "$bootstrap_client_urls_b64")"
bootstrap_server_urls="$(decode_remote_arg "$bootstrap_server_urls_b64")"
bootstrap_advertise_client_urls="$(decode_remote_arg "$bootstrap_advertise_client_urls_b64")"
bootstrap_advertise_server_urls="$(decode_remote_arg "$bootstrap_advertise_server_urls_b64")"
bootstrap_seeds="$(decode_remote_arg "$bootstrap_seeds_b64")"
bootstrap_ssh_url="$(decode_remote_arg "$bootstrap_ssh_url_b64")"
bootstrap_ssh_hostkey="$(decode_remote_arg "$bootstrap_ssh_hostkey_b64")"
bootstrap_ssh_id="$(decode_remote_arg "$bootstrap_ssh_id_b64")"
bootstrap_ssh_id_dir="$(decode_remote_arg "$bootstrap_ssh_id_dir_b64")"
bootstrap_ssh_id_content="$(decode_remote_arg "$bootstrap_ssh_id_content_b64")"
bootstrap_environment_file_content="$(decode_remote_arg "$bootstrap_environment_file_content_b64")"

tar -C "$remote_tmp" -xf "$remote_tar"
as_root install -d -m 0755 /usr/local/bin
as_root install -d -m 0755 "$bootstrap_module_path"
run_status=0
as_root env \
	FISHYSTUFF_MGMT_BOOTSTRAP_HOSTNAME="$host_name" \
	FISHYSTUFF_MGMT_BOOTSTRAP_MGMT_EXEC="/usr/local/bin/mgmt" \
	FISHYSTUFF_MGMT_BOOTSTRAP_MGMT_EXEC_SOURCE="$remote_mgmt_bin" \
	FISHYSTUFF_MGMT_BOOTSTRAP_ENVIRONMENT_DIR="$bootstrap_environment_dir" \
	FISHYSTUFF_MGMT_BOOTSTRAP_ENVIRONMENT_FILE="$bootstrap_environment_file" \
	FISHYSTUFF_MGMT_BOOTSTRAP_ENVIRONMENT_FILE_CONTENT="$bootstrap_environment_file_content" \
	FISHYSTUFF_MGMT_BOOTSTRAP_CLIENT_URLS="$bootstrap_client_urls" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SERVER_URLS="$bootstrap_server_urls" \
	FISHYSTUFF_MGMT_BOOTSTRAP_ADVERTISE_CLIENT_URLS="$bootstrap_advertise_client_urls" \
	FISHYSTUFF_MGMT_BOOTSTRAP_ADVERTISE_SERVER_URLS="$bootstrap_advertise_server_urls" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SEEDS="$bootstrap_seeds" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SSH_URL="$bootstrap_ssh_url" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SSH_HOSTKEY="$bootstrap_ssh_hostkey" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID="$bootstrap_ssh_id" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID_DIR="$bootstrap_ssh_id_dir" \
	FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID_CONTENT="$bootstrap_ssh_id_content" \
	timeout "${timeout_secs}s" "$remote_mgmt_bin" run \
		--hostname "$host_name" \
		--tmp-prefix \
		--no-watch \
		--client-urls="$bootstrap_client_url" \
		--server-urls="$bootstrap_server_url" \
		--advertise-client-urls="$bootstrap_client_url" \
		--advertise-server-urls="$bootstrap_server_url" \
		--no-pgp \
		--converged-timeout 15 \
		lang --module-path "$bootstrap_module_path" "$remote_tmp/" || run_status=$?
if [[ "$run_status" != "0" && "$run_status" != "3" ]]; then
	exit "$run_status"
fi
desired_mgmt_bin="$(readlink -f /usr/local/bin/mgmt || true)"
main_pid="$(as_root systemctl show -p MainPID --value fishystuff-mgmt.service || true)"
running_mgmt_bin=""
if [[ -n "$main_pid" && "$main_pid" != "0" && -e "/proc/$main_pid/exe" ]]; then
	running_mgmt_bin="$(readlink -f "/proc/$main_pid/exe" || true)"
fi
if [[ -n "$desired_mgmt_bin" && "$running_mgmt_bin" != "$desired_mgmt_bin" ]]; then
	as_root systemctl restart fishystuff-mgmt.service
fi
as_root systemctl is-enabled fishystuff-mgmt.service >/dev/null
as_root systemctl is-active fishystuff-mgmt.service >/dev/null
EOF
