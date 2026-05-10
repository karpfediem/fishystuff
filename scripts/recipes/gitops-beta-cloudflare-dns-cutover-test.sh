#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-cloudflare-dns-cutover-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
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
    printf '[gitops-beta-cloudflare-dns-cutover-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-cloudflare-dns-cutover-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_curl="${root}/curl"

cat >"$fake_curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail

method=GET
data=""
url=""
while (($# > 0)); do
  case "$1" in
    -X)
      method="$2"
      shift 2
      ;;
    --data)
      data="$2"
      shift 2
      ;;
    -H | --max-time | --connect-timeout | --dump-header | --output | --write-out)
      shift 2
      ;;
    -fsS | -sS)
      shift
      ;;
    https://*)
      url="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

printf '%s %s %s\n' "$method" "$url" "$data" >>"${FISHYSTUFF_FAKE_CLOUDFLARE_LOG:?}"

case "$url" in
  'https://api.cloudflare.com/client/v4/zones?name=fishystuff.fish')
    jq -cn '{success: true, result: [{id: "zone-test", name: "fishystuff.fish"}]}'
    ;;
  'https://api.cloudflare.com/client/v4/zones/zone-test/dns_records?type=A&name='*)
    name="${url##*name=}"
    jq -cn --arg name "$name" '{
      success: true,
      result: [
        {
          id: ("id-" + ($name | gsub("[.]"; "-"))),
          type: "A",
          name: $name,
          content: "178.104.230.121",
          ttl: 1,
          proxied: false
        }
      ]
    }'
    ;;
  'https://api.cloudflare.com/client/v4/zones/zone-test/dns_records/'*)
    name="$(jq -er '.name' <<<"$data")"
    content="$(jq -er '.content' <<<"$data")"
    jq -cn --arg name "$name" --arg content "$content" '{
      success: true,
      result: {
        id: "updated",
        type: "A",
        name: $name,
        content: $content,
        ttl: 1,
        proxied: false
      }
    }'
    ;;
  *)
    printf 'unexpected fake Cloudflare URL: %s\n' "$url" >&2
    exit 2
    ;;
esac
CURL
chmod +x "$fake_curl"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DNS_CUTOVER=1 \
  FISHYSTUFF_GITOPS_BETA_DNS_TARGET_IPV4=49.13.192.24 \
  CLOUDFLARE_API_TOKEN=fixture-token \
  FISHYSTUFF_FAKE_CLOUDFLARE_LOG="${root}/cloudflare.log" \
  bash scripts/recipes/gitops-beta-cloudflare-dns-cutover.sh 49.13.192.24 fishystuff.fish "$fake_curl" >"${root}/dns.out"
grep -F "gitops_beta_cloudflare_dns_cutover_checked=true" "${root}/dns.out" >/dev/null
grep -F "gitops_beta_cloudflare_dns_cutover_ok=true" "${root}/dns.out" >/dev/null
grep -F "cloudflare_dns_mutation_performed=true" "${root}/dns.out" >/dev/null
grep -F "production_mutation_performed=false" "${root}/dns.out" >/dev/null
grep -F "record_beta_fishystuff_fish_before=178.104.230.121" "${root}/dns.out" >/dev/null
grep -F "record_api_beta_fishystuff_fish_after=49.13.192.24" "${root}/dns.out" >/dev/null
grep -F "PATCH https://api.cloudflare.com/client/v4/zones/zone-test/dns_records/id-beta-fishystuff-fish" "${root}/cloudflare.log" >/dev/null
grep -F "PATCH https://api.cloudflare.com/client/v4/zones/zone-test/dns_records/id-api-beta-fishystuff-fish" "${root}/cloudflare.log" >/dev/null
grep -F "PATCH https://api.cloudflare.com/client/v4/zones/zone-test/dns_records/id-cdn-beta-fishystuff-fish" "${root}/cloudflare.log" >/dev/null
grep -F "PATCH https://api.cloudflare.com/client/v4/zones/zone-test/dns_records/id-telemetry-beta-fishystuff-fish" "${root}/cloudflare.log" >/dev/null
if grep -E '(^|[[:space:]])fishystuff\.fish([[:space:]]|$)' "${root}/cloudflare.log" >/dev/null; then
  printf '[gitops-beta-cloudflare-dns-cutover-test] root production hostname was patched\n' >&2
  exit 1
fi
pass "updates only beta A records"

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_DNS_CUTOVER=1
  FISHYSTUFF_GITOPS_BETA_DNS_TARGET_IPV4=49.13.192.24
  CLOUDFLARE_API_TOKEN=fixture-token
  FISHYSTUFF_FAKE_CLOUDFLARE_LOG="${root}/cloudflare-fail.log"
)

expect_fail_contains \
  "requires cutover opt-in" \
  "gitops-beta-cloudflare-dns-cutover requires FISHYSTUFF_GITOPS_ENABLE_BETA_DNS_CUTOVER=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    CLOUDFLARE_API_TOKEN=fixture-token \
    bash scripts/recipes/gitops-beta-cloudflare-dns-cutover.sh 49.13.192.24 fishystuff.fish "$fake_curl"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-cloudflare-dns-cutover requires FISHYSTUFF_GITOPS_BETA_DNS_TARGET_IPV4=49.13.192.24" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_DNS_TARGET_IPV4=49.13.192.25 \
    bash scripts/recipes/gitops-beta-cloudflare-dns-cutover.sh 49.13.192.24 fishystuff.fish "$fake_curl"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-cloudflare-dns-cutover.sh 49.13.192.24 fishystuff.fish "$fake_curl"

expect_fail_contains \
  "rejects non-fishystuff zone" \
  "only the fishystuff.fish Cloudflare zone is supported" \
  env \
    "${base_env[@]}" \
    bash scripts/recipes/gitops-beta-cloudflare-dns-cutover.sh 49.13.192.24 example.com "$fake_curl"

printf '[gitops-beta-cloudflare-dns-cutover-test] %s checks passed\n' "$pass_count"
