#!/usr/bin/env bash
set -euo pipefail

bundle_path="${1:?usage: smoke-fishystuff-dolt-target.sh BUNDLE_PATH SSH_TARGET [GCROOT] [SQL_HOST] [SQL_PORT] [QUERY_TIMEOUT_SECS]}"
ssh_target="${2:?usage: smoke-fishystuff-dolt-target.sh BUNDLE_PATH SSH_TARGET [GCROOT] [SQL_HOST] [SQL_PORT] [QUERY_TIMEOUT_SECS]}"
gcroot_path="${3:-/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current}"
sql_host="${4:-127.0.0.1}"
sql_port="${5:-3306}"
query_timeout_secs="${6:-20}"

ssh_opts=()
if [[ -n "${SSH_OPTS:-}" ]]; then
	# shellcheck disable=SC2206
	ssh_opts=(${SSH_OPTS})
fi

bundle_path="$(readlink -f "$bundle_path")"
bundle_json="${bundle_path}/bundle.json"

if [[ ! -f "$bundle_json" ]]; then
	echo "missing bundle.json under ${bundle_path}" >&2
	exit 1
fi

jq -e '
  .id == "fishystuff-dolt"
  and .backends.systemd.service_manager == "systemd"
  and (.backends.systemd.units | length) > 0
  and .artifacts["systemd/unit"].bundle_path == "artifacts/systemd/unit"
  and .artifacts["exe/main"].bundle_path == "artifacts/exe/main"
' "$bundle_json" >/dev/null

unit_name="$(jq -r '.backends.systemd.units[0].name' "$bundle_json")"
unit_install_path="$(jq -r '.backends.systemd.units[0].install_path' "$bundle_json")"
unit_bundle_rel="$(jq -r '.artifacts["systemd/unit"].bundle_path' "$bundle_json")"
exe_bundle_rel="$(jq -r '.artifacts["exe/main"].bundle_path' "$bundle_json")"

group_setup_commands="$(
	jq -r '
	  .activation.groups[]? |
	  "if ! getent group " + (.name | @sh) + " >/dev/null; then sudo groupadd --system " + (.name | @sh) + "; fi"
	' "$bundle_json"
)"

user_setup_commands="$(
	jq -r '
	  .activation.users[]? |
	  "if ! id -u " + (.name | @sh) + " >/dev/null 2>&1; then sudo useradd --system --no-create-home --shell /usr/sbin/nologin --gid " + (.group | @sh) + " " + (.name | @sh) + "; fi"
	' "$bundle_json"
)"

directory_setup_commands="$(
	jq -r '
	  .activation.directories[]? |
	  "sudo install -d -m " + (.mode | @sh)
	  + " -o " + ((.owner // "root") | @sh)
	  + " -g " + ((.group // "root") | @sh)
	  + " " + (.path | @sh)
	' "$bundle_json"
)"

nix_copy_target="ssh-ng://${ssh_target}"
if [[ -n "${NIX_SSH_KEY_PATH:-}" ]]; then
	nix_copy_target="${nix_copy_target}?ssh-key=${NIX_SSH_KEY_PATH}"
fi
if [[ -n "${NIX_REMOTE_PROGRAM_PATH:-}" ]]; then
	if [[ "$nix_copy_target" == *\?* ]]; then
		nix_copy_target="${nix_copy_target}&remote-program=${NIX_REMOTE_PROGRAM_PATH}"
	else
		nix_copy_target="${nix_copy_target}?remote-program=${NIX_REMOTE_PROGRAM_PATH}"
	fi
fi

echo "[dolt-smoke] copying bundle closure to ${ssh_target}"
nix copy --no-check-sigs --to "$nix_copy_target" "$bundle_path"

echo "[dolt-smoke] activating bundle ${bundle_path} on ${ssh_target}"
ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- \
	"$bundle_path" \
	"$gcroot_path" \
	"$unit_bundle_rel" \
	"$unit_install_path" \
	"$unit_name" \
	"$exe_bundle_rel" \
	"$sql_host" \
	"$sql_port" \
	"$query_timeout_secs" <<EOF
set -euo pipefail

bundle_path="\${1:?missing bundle path}"
gcroot_path="\${2:?missing gcroot path}"
unit_bundle_rel="\${3:?missing unit bundle path}"
unit_install_path="\${4:?missing unit install path}"
unit_name="\${5:?missing unit name}"
exe_bundle_rel="\${6:?missing executable bundle path}"
sql_host="\${7:?missing sql host}"
sql_port="\${8:?missing sql port}"
query_timeout_secs="\${9:?missing query timeout}"

sudo install -d -m 0755 "\$(dirname "\$gcroot_path")"
sudo ln -sfnT "\$bundle_path" "\$gcroot_path"

${group_setup_commands}
${user_setup_commands}
${directory_setup_commands}

test -f "\$gcroot_path/\$unit_bundle_rel"
test -x "\$gcroot_path/\$exe_bundle_rel"

sudo install -D -m 0644 "\$gcroot_path/\$unit_bundle_rel" "\$unit_install_path"
sudo systemctl daemon-reload
sudo systemctl enable "\$unit_name"
sudo systemctl restart "\$unit_name"
sudo systemctl is-enabled "\$unit_name" >/dev/null
sudo systemctl is-active "\$unit_name" >/dev/null

sudo timeout "\${query_timeout_secs}s" /bin/bash -c '
  exe="\$1"
  host="\$2"
  port="\$3"
  until "\$exe" --host "\$host" --port "\$port" --no-tls sql -q "select 1" >/dev/null 2>&1; do
    sleep 1
  done
' bash "\$gcroot_path/\$exe_bundle_rel" "\$sql_host" "\$sql_port"
EOF

echo "[dolt-smoke] ${unit_name} is active and answered a SQL health check on ${sql_host}:${sql_port}"
