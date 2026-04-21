#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

target="$(normalize_named_arg target "${1-}")"
host="$(normalize_named_arg host "${2-}")"
timeout="$(normalize_named_arg timeout "${3-120}")"
mgmt_flake="$(normalize_named_arg mgmt_flake "${4-/home/carp/code/playground/mgmt-missing-features}")"
mgmt_package="$(normalize_named_arg mgmt_package "${5-minimal}")"

require_value "$target" "missing target=... for mgmt-resident-kickstart-remote"
require_value "$host" "missing host=... for mgmt-resident-kickstart-remote"

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
  -- "$RECIPE_REPO_ROOT" "$target" "$host" "$timeout" "$mgmt_store" "$mgmt_flake"
