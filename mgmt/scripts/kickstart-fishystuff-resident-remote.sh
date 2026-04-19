#!/usr/bin/env bash
set -euo pipefail

graph_dir="${1:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
ssh_target="${2:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
host_name="${3:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
timeout_secs="${4:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"
remote_mgmt_bin="${5:?usage: kickstart-fishystuff-resident-remote.sh GRAPH_DIR SSH_TARGET HOSTNAME TIMEOUT_SECS REMOTE_MGMT_BIN}"

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

ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- "$host_name" "$timeout_secs" "$remote_mgmt_bin" "$remote_tar" <<'EOF'
set -euo pipefail

host_name="${1:?missing host name}"
timeout_secs="${2:?missing timeout seconds}"
remote_mgmt_bin="${3:?missing remote mgmt binary path}"
remote_tar="${4:?missing remote tar path}"
remote_tmp="$(mktemp -d /tmp/fishystuff-mgmt-bootstrap.XXXXXX)"
bootstrap_client_url="http://127.0.0.1:32379"
bootstrap_server_url="http://127.0.0.1:32380"
trap 'rm -rf "$remote_tmp"; rm -f "$remote_tar"' EXIT

tar -C "$remote_tmp" -xf "$remote_tar"
sudo install -d -m 0755 /usr/local/bin
sudo ln -sfn "$remote_mgmt_bin" /usr/local/bin/mgmt
run_status=0
sudo env \
	FISHYSTUFF_MGMT_BOOTSTRAP_HOSTNAME="$host_name" \
	FISHYSTUFF_MGMT_BOOTSTRAP_MGMT_EXEC="/usr/local/bin/mgmt" \
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
		lang "$remote_tmp/" || run_status=$?
if [[ "$run_status" != "0" && "$run_status" != "3" ]]; then
	exit "$run_status"
fi
sudo systemctl is-enabled fishystuff-mgmt.service >/dev/null
sudo systemctl is-active fishystuff-mgmt.service >/dev/null
EOF
