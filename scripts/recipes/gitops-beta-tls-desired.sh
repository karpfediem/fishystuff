#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-tls.desired.json}")"
ca="$(normalize_named_arg ca "${2-staging}")"
contact_email="$(normalize_named_arg contact_email "${3-${FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL:-}}")"
if [[ -z "$contact_email" ]]; then
  contact_email="${FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL:-}"
fi

cd "$RECIPE_REPO_ROOT"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name is required" >&2
    exit 127
  fi
}

require_positive_int() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "$name must be a positive integer, got: ${value:-<empty>}" >&2
    exit 2
  fi
}

acme_directory_url() {
  case "$1" in
    staging)
      printf '%s' "https://acme-staging-v02.api.letsencrypt.org/directory"
      ;;
    production)
      printf '%s' "https://acme-v02.api.letsencrypt.org/directory"
      ;;
    *)
      echo "ca must be staging or production, got: $1" >&2
      exit 2
      ;;
  esac
}

require_command jq
require_positive_int FISHYSTUFF_GITOPS_GENERATION "${FISHYSTUFF_GITOPS_GENERATION:-1}"

if [[ "$ca" == "production" && "${FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_PRODUCTION_DESIRED:-}" != "1" ]]; then
  echo "gitops-beta-tls-desired requires FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_PRODUCTION_DESIRED=1 for production ACME desired state" >&2
  exit 2
fi
if [[ -z "$contact_email" || "$contact_email" != *@* ]]; then
  echo "gitops-beta-tls-desired requires contact_email or FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL with an email address" >&2
  exit 2
fi

generation="${FISHYSTUFF_GITOPS_GENERATION:-1}"
directory_url="$(acme_directory_url "$ca")"

json="$(
  jq -n \
    --argjson generation "$generation" \
    --arg directory_url "$directory_url" \
    --arg contact_email "$contact_email" \
    '{
      cluster: "beta",
      generation: $generation,
      mode: "local-apply",
      tls: {
        "beta-edge": {
          enabled: true,
          materialize: true,
          solve: true,
          present_dns: true,
          certificate_name: "fishystuff-beta-edge",
          account_name: "fishystuff-beta-edge-account",
          directory_url: $directory_url,
          contact_email: $contact_email,
          challenge: "dns-01",
          dns_provider: "cloudflare",
          dns_zone: "fishystuff.fish",
          domains: [
            "beta.fishystuff.fish",
            "api.beta.fishystuff.fish",
            "cdn.beta.fishystuff.fish",
            "telemetry.beta.fishystuff.fish"
          ],
          request_namespace: "acme/cert-requests/fishystuff-beta",
          account_key_path: "/var/lib/fishystuff/gitops-beta/acme/fishystuff-beta-edge-account/account.key",
          account_cache_dir: "/var/lib/fishystuff/gitops-beta/acme/fishystuff-beta-edge-account",
          key_algorithm: "ecdsa-p256",
          renew_before: 2592000,
          tls_dir: "/var/lib/fishystuff/gitops-beta/tls/live",
          key_path: "/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem",
          cert_path: "/var/lib/fishystuff/gitops-beta/tls/live/cert.pem",
          chain_path: "/var/lib/fishystuff/gitops-beta/tls/live/chain.pem",
          fullchain_path: "/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem",
          cloudflare_token_env: "CLOUDFLARE_API_TOKEN",
          reload_service: "fishystuff-beta-edge",
          reload_service_action: "reload-or-try-restart",
          attempt_ttl: 2400,
          presentation_timeout: 1800,
          poll_interval: 10,
          presentation_settle: 60,
          cooldown: 600
        }
      },
      hosts: {},
      releases: {},
      environments: {}
    }'
)"

if [[ "$output" == "-" ]]; then
  printf '%s\n' "$json"
else
  if [[ "$output" != /* ]]; then
    output="${RECIPE_REPO_ROOT}/${output}"
  fi
  mkdir -p "$(dirname "$output")"
  tmp="$(mktemp "$(dirname "$output")/.${output##*/}.XXXXXX")"
  printf '%s\n' "$json" >"$tmp"
  mv "$tmp" "$output"
  printf 'wrote %s\n' "$output" >&2
fi

printf 'gitops_beta_tls_desired_ok=true\n' >&2
printf 'beta_tls_ca=%s\n' "$ca" >&2
printf 'beta_tls_directory_url=%s\n' "$directory_url" >&2
printf 'beta_tls_contact_email=%s\n' "$contact_email" >&2
printf 'remote_deploy_performed=false\n' >&2
printf 'infrastructure_mutation_performed=false\n' >&2
printf 'local_host_mutation_performed=false\n' >&2
