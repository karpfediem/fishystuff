#!/usr/bin/env bash
set -euo pipefail

if (( $# < 2 )); then
	echo "usage: push-fishystuff-bundles-remote.sh SSH_TARGET PATH [PATH ...]" >&2
	exit 1
fi

ssh_target="${1:?missing ssh target}"
shift

ssh_opts=()
if [[ -n "${SSH_OPTS:-}" ]]; then
	# shellcheck disable=SC2206
	ssh_opts=(${SSH_OPTS})
fi

remote_nix_max_jobs="${FISHYSTUFF_REMOTE_NIX_MAX_JOBS:-0}"
if [[ ! "$remote_nix_max_jobs" =~ ^[0-9]+$ ]]; then
	echo "FISHYSTUFF_REMOTE_NIX_MAX_JOBS must be a non-negative integer" >&2
	exit 1
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
copy_paths=()
substitute_roots=()
realise_inputs=()
declare -A seen_substitute=()
declare -A seen_realise=()
while (( $# > 0 )); do
	input_path="${1:?missing path}"
	shift

	input_path="$(readlink -f "$input_path")"
	if [[ -f "${input_path}/bundle.json" ]]; then
		if [[ ! -f "${input_path}/store-paths" ]]; then
			echo "missing store-paths under ${input_path}" >&2
			exit 1
		fi

		if [[ -f "${input_path}/mode-substitute.txt" ]]; then
			while IFS= read -r root; do
				[[ -n "$root" ]] || continue
				if [[ -z "${seen_substitute[$root]+x}" ]]; then
					seen_substitute["$root"]=1
					substitute_roots+=("$root")
				fi
			done < "${input_path}/mode-substitute.txt"
		fi

		if [[ -f "${input_path}/mode-realise.txt" ]]; then
			while IFS= read -r input; do
				[[ -n "$input" ]] || continue
				if [[ -z "${seen_realise[$input]+x}" ]]; then
					seen_realise["$input"]=1
					realise_inputs+=("$input")
				fi
			done < "${input_path}/mode-realise.txt"
		fi

		bundle_paths+=("$input_path")
		copy_paths+=("$input_path")
		continue
	fi

	if [[ "${input_path}" != /nix/store/* ]]; then
		echo "path is neither a bundle directory nor a Nix store path: ${input_path}" >&2
		exit 1
	fi

	copy_paths+=("$input_path")
done

remote_preamble=(
	"${#substitute_roots[@]}"
	"${#realise_inputs[@]}"
	"$remote_nix_max_jobs"
)

if (( ${#realise_inputs[@]} > 0 )); then
	echo "[bundle-push] seeding ${#realise_inputs[@]} realise input(s) on ${ssh_target}"
	nix copy --no-check-sigs --substitute-on-destination --to "$nix_copy_target" "${realise_inputs[@]}"
fi

echo "[bundle-push] pre-materializing ${#substitute_roots[@]} substitute root(s) and ${#realise_inputs[@]} realise input(s) on ${ssh_target}"
ssh "${ssh_opts[@]}" "$ssh_target" /bin/bash -s -- \
	"${remote_preamble[@]}" \
	"${substitute_roots[@]}" \
	"${realise_inputs[@]}" <<'EOF'
set -euo pipefail

substitute_count="${1:?missing substitute count}"
realise_count="${2:?missing realise count}"
remote_nix_max_jobs="${3:?missing remote nix max jobs}"
shift 3

substitute_roots=()
for (( idx = 0; idx < substitute_count; idx++ )); do
	substitute_roots+=("${1:?missing substitute root}")
	shift
done

realise_inputs=()
for (( idx = 0; idx < realise_count; idx++ )); do
	realise_inputs+=("${1:?missing realise input}")
	shift
done

if test -x /nix/var/nix/profiles/default/bin/nix-store; then
	nix_store=/nix/var/nix/profiles/default/bin/nix-store
elif command -v nix-store >/dev/null 2>&1; then
	nix_store="$(command -v nix-store)"
else
	echo "could not detect nix-store on remote host" >&2
	exit 1
fi

if (( ${#substitute_roots[@]} > 0 )); then
	if ! sudo "$nix_store" --realise --keep-going --max-jobs 0 "${substitute_roots[@]}"; then
		echo "[bundle-push] remote substitute pre-materialization was incomplete; continuing with nix copy fallback" >&2
	fi
fi

if (( ${#realise_inputs[@]} > 0 )); then
	if ! sudo "$nix_store" --realise --keep-going --max-jobs "$remote_nix_max_jobs" "${realise_inputs[@]}"; then
		echo "[bundle-push] remote realise pre-materialization was incomplete; continuing with nix copy fallback" >&2
	fi
fi
EOF

echo "[bundle-push] copying ${#copy_paths[@]} path closure(s) to ${ssh_target}"
nix copy --no-check-sigs --substitute-on-destination --to "$nix_copy_target" "${copy_paths[@]}"
