#!/usr/bin/env bash
set -euo pipefail

if (( $# < 2 )); then
	echo "usage: push-fishystuff-bundles-remote.sh SSH_TARGET PATH [PATH ...]" >&2
	exit 1
fi

ssh_target="${1:?missing ssh target}"
shift

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

copy_paths=()
declare -A seen_copy=()

add_copy_path() {
	local path="${1:?missing path}"
	if [[ -n "${seen_copy[$path]+x}" ]]; then
		return
	fi
	seen_copy["$path"]=1
	copy_paths+=("$path")
}

while (( $# > 0 )); do
	input_path="${1:?missing path}"
	shift

	input_path="$(readlink -f "$input_path")"
	if [[ -f "${input_path}/bundle.json" ]]; then
		if [[ ! -f "${input_path}/store-paths" ]]; then
			echo "missing store-paths under ${input_path}" >&2
			exit 1
		fi

		add_copy_path "$input_path"
		continue
	fi

	if [[ "${input_path}" != /nix/store/* ]]; then
		echo "path is neither a bundle directory nor a Nix store path: ${input_path}" >&2
		exit 1
	fi

	add_copy_path "$input_path"
done

echo "[bundle-push] copying ${#copy_paths[@]} store path/drv closure(s) to ${ssh_target}"
nix copy --no-check-sigs --substitute-on-destination --to "$nix_copy_target" "${copy_paths[@]}"
