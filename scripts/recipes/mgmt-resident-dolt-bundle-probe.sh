#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

target="$(normalize_named_arg target "${1-}")"
timeout="$(normalize_named_arg timeout "${2-120}")"
bundle_path="$(normalize_named_arg bundle_path "${3-}")"
gcroot_path="$(normalize_named_arg gcroot_path "${4-/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current}")"
remote_mgmt_bin="$(normalize_named_arg remote_mgmt_bin "${5-/usr/local/bin/mgmt}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${6-}")"
mgmt_flake="$(normalize_named_arg mgmt_flake "${7-/home/carp/code/mgmt-fishystuff-beta}")"
mgmt_package="$(normalize_named_arg mgmt_package "${8-minimal}")"
mgmt_modules_dir="$(normalize_named_arg mgmt_modules_dir "${9-/home/carp/code/mgmt-fishystuff-beta/modules}")"

require_value "$target" "missing target=... for mgmt-resident-dolt-bundle-probe"

if [[ -z "$bundle_path" ]]; then
  bundle_path="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
fi
if [[ -z "$mgmt_bin" ]]; then
  mgmt_installable="$mgmt_flake"
  if [[ -n "$mgmt_package" && "$mgmt_package" != "default" ]]; then
    mgmt_installable="$mgmt_flake#$mgmt_package"
  fi
  mgmt_store="$(nix build "$mgmt_installable" --no-link --print-out-paths)"
  mgmt_bin="$mgmt_store/bin/mgmt"
fi

probe_dir="$(mktemp -d /tmp/fishystuff-resident-bundle-probe.XXXXXX)"
trap 'rm -rf "$probe_dir"' EXIT
mkdir -p "$probe_dir/modules/lib" "$probe_dir/modules/github.com/purpleidea/mgmt/modules"
cp -a mgmt/resident-beta/modules/lib/fishystuff-systemd "$probe_dir/modules/lib/"
cp -a mgmt/resident-beta/modules/lib/fishystuff-bundle-nix "$probe_dir/modules/lib/"
cp -a mgmt/resident-beta/modules/lib/fishystuff-bundle-systemd "$probe_dir/modules/lib/"
cp -a mgmt/modules/lib/systemd-daemon-reload "$probe_dir/modules/lib/"
cp -a "$mgmt_modules_dir/misc" "$probe_dir/modules/github.com/purpleidea/mgmt/modules/"
printf '%s\n' \
  'import "modules/lib/fishystuff-bundle-systemd/" as fishystuff_bundle_systemd' \
  '' \
  'include fishystuff_bundle_systemd.unit(struct {' \
  "	bundle_path => \"${bundle_path}\"," \
  "	gcroot_path => \"${gcroot_path}\"," \
  '	startup_mode => "enabled",' \
  '})' \
  > "$probe_dir/main.mcl"
printf 'main: main.mcl\npath: modules/\n' > "$probe_dir/metadata.yaml"
"$mgmt_bin" run lang --module-path "$probe_dir/modules/" --only-unify "$probe_dir/main.mcl"

secretspec run --profile beta-deploy -- \
  env FS_SSH_TARGET="$target" FS_BUNDLE_PATH="$bundle_path" \
  bash -lc '
    set -euo pipefail
    source "$1/scripts/recipes/lib/common.sh"
    ssh_target="${FS_SSH_TARGET:?}"
    bundle_path="${FS_BUNDLE_PATH:?}"
    tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-mgmt-ssh.XXXXXX)"
    trap '\''rm -f "$tmp_key"'\'' EXIT
    remote_nix_daemon_path="$(detect_remote_nix_daemon_path "$ssh_target" "$tmp_key")"
    if [[ -z "$remote_nix_daemon_path" ]]; then
      echo "could not detect remote nix-daemon path on $ssh_target" >&2
      exit 1
    fi
    SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
    NIX_SSH_KEY_PATH="$tmp_key" \
    NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
    bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
        "$ssh_target" \
        "$bundle_path"
  ' \
  -- "$RECIPE_REPO_ROOT"

secretspec run --profile beta-deploy -- \
  bash -lc '
    set -euo pipefail
    source "$1/scripts/recipes/lib/common.sh"
    tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-mgmt-ssh.XXXXXX)"
    trap '\''rm -f "$tmp_key"'\'' EXIT
    SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
    bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
        "$2" \
        "$3" \
        "$4" \
        "$5"
  ' \
  -- "$RECIPE_REPO_ROOT" "$probe_dir" "$target" "$timeout" "$remote_mgmt_bin"
