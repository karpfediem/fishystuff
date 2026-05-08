#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-production-edge-handoff-bundle-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

make_bundle() {
  local bundle="$1"
  mkdir -p "${bundle}/artifacts/exe" "${bundle}/artifacts/config"
  cat >"${bundle}/artifacts/exe/main" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "${bundle}/artifacts/exe/main"
  cat >"${bundle}/artifacts/config/base" <<'EOF'
{
  auto_https off
}

https://fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  root * /var/lib/fishystuff/gitops/served/production/site
  header Cache-Control "no-store"
  header Cache-Control "public, max-age=31536000, immutable"
}

https://api.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  reverse_proxy 127.0.0.1:18092
}

https://cdn.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  root * /var/lib/fishystuff/gitops/served/production/cdn
  @runtime_manifest path /map/runtime-manifest.json
  header Cache-Control "no-store"
  header Cache-Control "public, max-age=31536000, immutable"
}

https://telemetry.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  reverse_proxy 127.0.0.1:4820
}
EOF
}

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local root=""
  local stderr=""

  root="$(mktemp -d)"
  stderr="${root}/stderr"
  if "$@" >"${root}/stdout" 2>"$stderr"; then
    printf '[gitops-production-edge-handoff-bundle-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-edge-handoff-bundle-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
valid="${root}/valid"
make_bundle "$valid"

bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$valid" >"${root}/valid.stdout"
grep -F "gitops_edge_handoff_bundle_ok=${valid}" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_site_root=/var/lib/fishystuff/gitops/served/production/site" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_cdn_root=/var/lib/fishystuff/gitops/served/production/cdn" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_api_upstream=127.0.0.1:18092" "${root}/valid.stdout" >/dev/null
pass "valid bundle"

beta="${root}/beta"
cp -R "$valid" "$beta"
printf '\n# beta.fishystuff.fish must never be present here\n' >>"${beta}/artifacts/config/base"
expect_fail_contains \
  "reject beta hostname" \
  "must not contain beta hostname" \
  bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$beta"

legacy="${root}/legacy"
cp -R "$valid" "$legacy"
printf '\n# /srv/fishystuff must never be present here\n' >>"${legacy}/artifacts/config/base"
expect_fail_contains \
  "reject legacy serving root" \
  "must not contain legacy serving root" \
  bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$legacy"

store_root="${root}/store-root"
cp -R "$valid" "$store_root"
printf '\nroot * /nix/store/example-site\n' >>"${store_root}/artifacts/config/base"
expect_fail_contains \
  "reject fixed store serving root" \
  "must not contain fixed store serving root" \
  bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$store_root"

wrong_api="${root}/wrong-api"
cp -R "$valid" "$wrong_api"
perl -0pi -e 's/reverse_proxy 127\.0\.0\.1:18092/reverse_proxy 127.0.0.1:18091/' "${wrong_api}/artifacts/config/base"
expect_fail_contains \
  "reject wrong API upstream" \
  "missing loopback candidate API upstream" \
  bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$wrong_api"

missing_exe="${root}/missing-exe"
cp -R "$valid" "$missing_exe"
rm -f "${missing_exe}/artifacts/exe/main"
expect_fail_contains \
  "reject missing executable" \
  "Caddy executable is missing or not executable" \
  bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$missing_exe"

printf '[gitops-production-edge-handoff-bundle-test] %s checks passed\n' "$pass_count"
