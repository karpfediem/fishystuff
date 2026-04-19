#!/usr/bin/env bash
set -euo pipefail

if (( $# < 3 )) || (( $# % 2 == 0 )); then
	echo "usage: push-fishystuff-bundles-remote.sh SSH_TARGET BUNDLE_PATH GCROOT_PATH [BUNDLE_PATH GCROOT_PATH ...]" >&2
	exit 1
fi

ssh_target="${1:?missing ssh target}"
shift

ssh_opts=()
if [[ -n "${SSH_OPTS:-}" ]]; then
	# shellcheck disable=SC2206
	ssh_opts=(${SSH_OPTS})
fi

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

bundle_paths=()
remote_args=()
while (( $# > 0 )); do
	bundle_path="${1:?missing bundle path}"
	gcroot_path="${2:?missing gcroot path}"
	shift 2

	bundle_path="$(readlink -f "$bundle_path")"
	if [[ ! -f "${bundle_path}/bundle.json" ]]; then
		echo "missing bundle.json under ${bundle_path}" >&2
		exit 1
	fi
	if [[ ! -f "${bundle_path}/store-paths" ]]; then
		echo "missing store-paths under ${bundle_path}" >&2
		exit 1
	fi

	bundle_paths+=("$bundle_path")
	remote_args+=("$bundle_path" "$gcroot_path")
done

echo "[bundle-push] copying ${#bundle_paths[@]} bundle closure(s) to ${ssh_target}"
nix copy --no-check-sigs --to "$nix_copy_target" "${bundle_paths[@]}"

ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- "${remote_args[@]}" <<'EOF'
set -euo pipefail

while (( $# > 0 )); do
	bundle_path="${1:?missing bundle path}"
	gcroot_path="${2:?missing gcroot path}"
	shift 2

	sudo install -d -m 0755 "$(dirname "$gcroot_path")"
	sudo ln -sfnT "$bundle_path" "$gcroot_path"
	test -f "$gcroot_path/bundle.json"
	test -f "$gcroot_path/store-paths"
done
EOF
