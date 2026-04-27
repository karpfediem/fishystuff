#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

target=""
host=""
timeout="120"
mgmt_flake="/home/carp/code/mgmt-fishystuff-beta"
mgmt_package="minimal"
bootstrap_client_urls=""
bootstrap_server_urls=""
bootstrap_advertise_client_urls=""
bootstrap_advertise_server_urls=""
bootstrap_seeds=""
bootstrap_ssh_url=""
bootstrap_ssh_hostkey=""
bootstrap_ssh_id=""
bootstrap_ssh_id_dir="/root/.ssh"

positional_index=0
for arg in "$@"; do
  case "$arg" in
    target=*) target="${arg#target=}" ;;
    host=*) host="${arg#host=}" ;;
    timeout=*) timeout="${arg#timeout=}" ;;
    mgmt_flake=*) mgmt_flake="${arg#mgmt_flake=}" ;;
    mgmt_package=*) mgmt_package="${arg#mgmt_package=}" ;;
    bootstrap_client_urls=*) bootstrap_client_urls="${arg#bootstrap_client_urls=}" ;;
    bootstrap_server_urls=*) bootstrap_server_urls="${arg#bootstrap_server_urls=}" ;;
    bootstrap_advertise_client_urls=*) bootstrap_advertise_client_urls="${arg#bootstrap_advertise_client_urls=}" ;;
    bootstrap_advertise_server_urls=*) bootstrap_advertise_server_urls="${arg#bootstrap_advertise_server_urls=}" ;;
    bootstrap_seeds=*) bootstrap_seeds="${arg#bootstrap_seeds=}" ;;
    bootstrap_ssh_url=*) bootstrap_ssh_url="${arg#bootstrap_ssh_url=}" ;;
    bootstrap_ssh_hostkey=*) bootstrap_ssh_hostkey="${arg#bootstrap_ssh_hostkey=}" ;;
    bootstrap_ssh_id=*) bootstrap_ssh_id="${arg#bootstrap_ssh_id=}" ;;
    bootstrap_ssh_id_dir=*) bootstrap_ssh_id_dir="${arg#bootstrap_ssh_id_dir=}" ;;
    *=*)
      echo "unknown override for mgmt-resident-kickstart-remote: $arg" >&2
      exit 2
      ;;
    *)
      positional_index=$((positional_index + 1))
      case "$positional_index" in
        1) target="$arg" ;;
        2) host="$arg" ;;
        3) timeout="$arg" ;;
        4) mgmt_flake="$arg" ;;
        5) mgmt_package="$arg" ;;
        *)
          echo "unexpected positional argument for mgmt-resident-kickstart-remote: $arg" >&2
          exit 2
          ;;
      esac
      ;;
  esac
done

require_value "$target" "missing target=... for mgmt-resident-kickstart-remote"
require_value "$host" "missing host=... for mgmt-resident-kickstart-remote"

case "$host" in
  mgmt-root)
    bootstrap_client_urls="${bootstrap_client_urls:-http://127.0.0.1:2379}"
    bootstrap_server_urls="${bootstrap_server_urls:-http://127.0.0.1:2380}"
    bootstrap_advertise_client_urls="${bootstrap_advertise_client_urls:-http://127.0.0.1:2379}"
    bootstrap_advertise_server_urls="${bootstrap_advertise_server_urls:-http://127.0.0.1:2380}"
    ;;
  site-nbg1-beta | telemetry-nbg1 | site-nbg1-prod)
    bootstrap_seeds="${bootstrap_seeds:-http://127.0.0.1:2379}"
    bootstrap_ssh_url="${bootstrap_ssh_url:-root@204.168.223.57}"
    bootstrap_ssh_id="${bootstrap_ssh_id:-/root/.ssh/fishystuff-mgmt-control}"
    ;;
esac

if [[ -n "$bootstrap_ssh_url" && -z "$bootstrap_ssh_hostkey" ]]; then
  ssh_scan_target="${bootstrap_ssh_url#ssh://}"
  ssh_scan_target="${ssh_scan_target#*@}"
  ssh_scan_target="${ssh_scan_target%%:*}"
  if ! command -v ssh-keyscan >/dev/null; then
    echo "ssh-keyscan is required to derive bootstrap_ssh_hostkey for $bootstrap_ssh_url" >&2
    exit 2
  fi
  bootstrap_ssh_hostkey="$(ssh-keyscan -t ed25519 "$ssh_scan_target" 2>/dev/null | awk '$2 == "ssh-ed25519" { print $3; exit }')"
  require_value "$bootstrap_ssh_hostkey" "could not derive bootstrap_ssh_hostkey for $bootstrap_ssh_url"
fi

mgmt_installable="$mgmt_flake"
if [[ -n "$mgmt_package" && "$mgmt_package" != "default" ]]; then
  mgmt_installable="$mgmt_flake#$mgmt_package"
fi
mgmt_store="$(nix build "$mgmt_installable" --no-link --print-out-paths)"

secretspec run --profile beta-deploy -- \
  bash -lc '
    set -euo pipefail
    source "$1/scripts/recipes/lib/common.sh"
    tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-mgmt-ssh.XXXXXX)"
    trap '\''rm -f "$tmp_key"'\'' EXIT
    remote_nix_probe="$(read_remote_nix_paths "$2" "$tmp_key")"
    remote_nix_path=""
    remote_nix_daemon_path=""
    if [[ -n "$remote_nix_probe" ]]; then
      IFS=$'\''\t'\'' read -r remote_nix_path remote_nix_daemon_path <<< "$remote_nix_probe"
    fi
    nix_copy_target="$(build_nix_copy_target "$2" "$tmp_key" "$remote_nix_daemon_path")"
    remote_mgmt_bin="$5/bin/mgmt"
    if [[ -n "$remote_nix_path" ]]; then
      nix copy --no-check-sigs --to "$nix_copy_target" "$5"
    else
      (
        cd "$6"
        devenv shell -- bash -lc '\''MGMT_NOCGO=true MGMT_NOGOLANGRACE=true GOTAGS="noaugeas novirt nodocker" make -B build/mgmt-linux-amd64'\''
      )
      cat "$6/build/mgmt-linux-amd64" | ssh -i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$2" "sudo install -d -m 0755 /usr/local/bin && sudo tee /usr/local/bin/fishystuff-mgmt-bootstrap >/dev/null && sudo chmod 0755 /usr/local/bin/fishystuff-mgmt-bootstrap"
      remote_mgmt_bin="/usr/local/bin/fishystuff-mgmt-bootstrap"
    fi
    SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
    FISHYSTUFF_MGMT_BOOTSTRAP_CLIENT_URLS="$7" \
    FISHYSTUFF_MGMT_BOOTSTRAP_SERVER_URLS="$8" \
    FISHYSTUFF_MGMT_BOOTSTRAP_ADVERTISE_CLIENT_URLS="$9" \
    FISHYSTUFF_MGMT_BOOTSTRAP_ADVERTISE_SERVER_URLS="${10}" \
    FISHYSTUFF_MGMT_BOOTSTRAP_SEEDS="${11}" \
    FISHYSTUFF_MGMT_BOOTSTRAP_SSH_URL="${12}" \
    FISHYSTUFF_MGMT_BOOTSTRAP_SSH_HOSTKEY="${13}" \
    FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID="${14}" \
    FISHYSTUFF_MGMT_BOOTSTRAP_SSH_ID_DIR="${15}" \
      bash mgmt/scripts/kickstart-fishystuff-resident-remote.sh \
        mgmt/resident-bootstrap \
        "$2" \
        "$3" \
        "$4" \
        "$remote_mgmt_bin"
    if [[ "$remote_mgmt_bin" != "$5/bin/mgmt" ]]; then
      remote_nix_probe="$(read_remote_nix_paths "$2" "$tmp_key")"
      remote_nix_daemon_path=""
      if [[ -n "$remote_nix_probe" ]]; then
        IFS=$'\''\t'\'' read -r _remote_nix_path remote_nix_daemon_path <<< "$remote_nix_probe"
      fi
      if [[ -z "$remote_nix_daemon_path" ]]; then
        echo "could not detect remote nix-daemon path on $2 after bootstrap" >&2
        exit 1
      fi
      nix_copy_target="$(build_nix_copy_target "$2" "$tmp_key" "$remote_nix_daemon_path")"
      nix copy --no-check-sigs --to "$nix_copy_target" "$5"
      ssh -i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$2" "sudo ln -sfn '\''$5/bin/mgmt'\'' /usr/local/bin/mgmt && sudo systemctl daemon-reload && sudo systemctl restart fishystuff-mgmt.service && sudo systemctl is-enabled fishystuff-mgmt.service >/dev/null && sudo systemctl is-active fishystuff-mgmt.service >/dev/null"
    fi
  ' \
  -- "$RECIPE_REPO_ROOT" "$target" "$host" "$timeout" "$mgmt_store" "$mgmt_flake" "$bootstrap_client_urls" "$bootstrap_server_urls" "$bootstrap_advertise_client_urls" "$bootstrap_advertise_server_urls" "$bootstrap_seeds" "$bootstrap_ssh_url" "$bootstrap_ssh_hostkey" "$bootstrap_ssh_id" "$bootstrap_ssh_id_dir"
