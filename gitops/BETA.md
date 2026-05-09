# Beta GitOps Service Set

The next beta target is a distinct GitOps-managed service set, preferably on a new Hetzner host while the old beta host remains untouched as operational history. This page tracks the beta-specific contract as it is built out.

Hard boundary for the beta path:

- no production SSH key or host access
- no production service unit names
- no production GitOps state paths
- no production TLS credential paths
- no production public hostnames
- no Cloudflare or Hetzner mutation without an explicit separate confirmation

The first concrete artifacts are the beta GitOps handoff service bundles:

```bash
just gitops-beta-service-bundles
just gitops-beta-service-bundles-test
just gitops-beta-check-service-bundle service=api
just gitops-beta-check-service-bundle service=dolt
just gitops-beta-deploy-credentials-packet
just gitops-beta-host-provision-plan
just gitops-beta-host-provision-plan-test
just gitops-beta-host-selection-packet public_ipv4=<new-beta-public-ip>
just gitops-beta-host-selection-packet-test
FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 just gitops-beta-deploy-key-ensure
just gitops-beta-deploy-credentials-test
just secrets-check profile=beta-runtime
just gitops-beta-write-runtime-env service=api
FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env-secretspec service=api
just gitops-beta-check-runtime-env service=api
just gitops-beta-write-runtime-env service=dolt
just gitops-beta-check-runtime-env service=dolt
just gitops-beta-runtime-env-packet
just gitops-beta-runtime-env-packet-test
just gitops-beta-runtime-env-host-preflight
just gitops-beta-runtime-env-host-preflight-test
just gitops-beta-runtime-env-test
just gitops-beta-host-bootstrap-plan
just gitops-beta-host-bootstrap-plan-test
FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 just gitops-beta-host-bootstrap-apply
just gitops-beta-host-bootstrap-apply-test
just gitops-beta-service-start-plan
just gitops-beta-service-start-plan-test
just gitops-beta-service-start-packet
just gitops-beta-service-start-packet-test
FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256=<checked beta Dolt unit hash> FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=<checked beta API unit hash> just gitops-beta-start-services
just gitops-beta-start-services-test
just gitops-beta-install-service service=api
just gitops-beta-install-service service=dolt
just gitops-beta-install-service-test
just gitops-beta-edge-handoff-bundle
just gitops-beta-edge-handoff-bundle-test
just gitops-beta-current-desired
just gitops-beta-current-validate
just gitops-beta-current-desired-test
just gitops-beta-current-handoff-plan
just gitops-beta-current-handoff-plan-test
just gitops-beta-current-handoff
just gitops-beta-current-handoff-test
just gitops-beta-write-activation-admission-evidence
just gitops-beta-observe-admission
just gitops-beta-observe-admission-test
just gitops-beta-admission-packet
just gitops-beta-admission-packet-test
just gitops-beta-activation-draft-packet
just gitops-beta-activation-draft-packet-test
just gitops-beta-first-service-set-plan
just gitops-beta-first-service-set-plan-test
just gitops-beta-first-service-set-packet
just gitops-beta-first-service-set-packet-test
just gitops-beta-activation-draft
just gitops-beta-activation-draft-test
just gitops-beta-host-handoff-plan
just gitops-beta-host-handoff-plan-test
just gitops-beta-verify-activation-served
just gitops-beta-verify-activation-served-test
just gitops-beta-operator-proof
just gitops-beta-operator-proof-packet
just gitops-check-beta-operator-proof
just gitops-beta-operator-proof-packet-test
just gitops-beta-operator-proof-test
just gitops-beta-served-proof
just gitops-beta-served-proof-packet
just gitops-beta-served-proof-packet-test
just gitops-beta-served-proof-test
just gitops-beta-proof-index
just gitops-beta-proof-index-test
just gitops-beta-apply-activation-draft
just gitops-beta-apply-activation-draft-test
just gitops-beta-edge-install-packet
just gitops-beta-edge-install-packet-test
just gitops-beta-install-edge
just gitops-beta-install-edge-test
nix build .#checks.x86_64-linux.api-service-bundle-beta-gitops-handoff --no-link
nix build .#checks.x86_64-linux.dolt-service-bundle-beta-gitops-handoff --no-link
nix build .#checks.x86_64-linux.edge-service-bundle-beta-gitops-handoff --no-link
```

The API bundle validates:

- service ID `fishystuff-beta-api`
- systemd unit `fishystuff-beta-api.service`
- operator-owned beta runtime env file:
  - `/var/lib/fishystuff/gitops-beta/api/runtime.env`
- GitOps-owned release identity env file:
  - `/var/lib/fishystuff/gitops-beta/api/beta.env`
- beta API listener:
  - `127.0.0.1:18192`
- beta deployment and OTEL environment labels
- no shared `/run/fishystuff/api/env`
- no production API user/group lines

The Dolt bundle validates:

- service ID `fishystuff-beta-dolt`
- systemd unit `fishystuff-beta-dolt.service`
- beta runtime env file:
  - `/var/lib/fishystuff/gitops-beta/dolt/beta.env`
- beta data directory:
  - `/var/lib/fishystuff/beta-dolt`
- beta SQL listener:
  - `127.0.0.1:3316`
- beta runtime user/group:
  - `fishystuff-beta-dolt`
- no shared `/run/fishystuff/api/env`
- no production Dolt user/group/state-directory lines

The edge bundle validates:

- service ID `fishystuff-beta-edge`
- systemd unit `fishystuff-beta-edge.service`
- beta hostnames only:
  - `beta.fishystuff.fish`
  - `api.beta.fishystuff.fish`
  - `cdn.beta.fishystuff.fish`
  - `telemetry.beta.fishystuff.fish`
- beta served roots:
  - `/var/lib/fishystuff/gitops-beta/served/beta/site`
  - `/var/lib/fishystuff/gitops-beta/served/beta/cdn`
- beta TLS credential directory:
  - `/run/fishystuff/beta-edge/tls`
- beta loopback API upstream:
  - `127.0.0.1:18192`

The validator refuses production hostnames, production served roots, production TLS paths, and production edge dependencies in the beta edge bundle.

`just gitops-beta-current-desired` writes `data/gitops/beta-current.desired.json` as a validate-mode desired-state snapshot from exact local outputs. It is parameterized from the production-current generator but pins the beta service bundle attrs, `site-content-beta`, Dolt branch context `beta`, beta gcroot/cache roots, and the beta release-ref prefix `fishystuff/gitops-beta`. The default CDN runtime attr is `cdn-serving-root`, so the recipe requires `FISHYSTUFF_OPERATOR_ROOT` for operator-local CDN data unless `FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE` supplies an exact existing closure. When it must build `cdn-serving-root` from `FISHYSTUFF_OPERATOR_ROOT`, it uses a narrowly scoped `nix build --impure` only for that attr because the local operator CDN payload is intentionally supplied through the environment. It does not apply or serve anything.

`just gitops-beta-current-validate` generates that same snapshot and type-checks it through `gitops/main.mcl`. It is still local-only: no SSH, no Hetzner, no Cloudflare, no systemd changes.

`just gitops-beta-current-handoff-plan` is the read-only input check for `just gitops-beta-current-handoff`. It reports whether exact closure paths are already provided, which attrs would be built, whether the CDN runtime build is blocked by missing `FISHYSTUFF_OPERATOR_ROOT`, whether Dolt commit/remote discovery is available, and whether mgmt would be built from the pinned local mgmt flake. It does not run Nix builds or write handoff artifacts.

`just gitops-beta-check-service-bundle` validates one beta API or Dolt service bundle outside Nix test derivations. It accepts `service=api` or `service=dolt`, builds the matching beta handoff bundle by default, and checks the beta service ID, beta systemd unit name, beta runtime env path, beta loopback listener or Dolt state directory, and absence of production unit names or production state paths.

`just gitops-beta-deploy-credentials-packet` is the read-only beta deploy credential check. It verifies the local `beta-deploy` SecretSpec profile, reports whether Hetzner/Cloudflare deploy tokens and the beta SSH key material are present, derives the public key from the private key, and prints the public key fingerprint without printing secret values. If key material is missing, `FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 just gitops-beta-deploy-key-ensure` can generate an ed25519 beta deploy key and store only the local SecretSpec values. It does not upload keys, open SSH, mutate Hetzner, mutate Cloudflare, or touch host services.

`just gitops-beta-host-provision-plan` is the read-only packet for selecting or provisioning the first clean beta resident host. It records the intended Hetzner shape (`site-nbg1-beta`, `nbg1`, `nbg1-dc3`, `cx33`, `debian-13`, beta deploy SSH key name), carries the beta deploy credential status, warns not to use public beta DNS for a new host until the operator confirms it points at that host, and emits only manual confirmation steps. It deliberately emits no `hcloud`, SSH, DNS, deploy, or local-host mutation command.

`just gitops-beta-host-selection-packet public_ipv4=<new-beta-public-ip>` binds an operator-confirmed fresh beta host address to the next read-only and guarded commands. It prints `FISHYSTUFF_BETA_RESIDENT_TARGET=root@<ip>` command prefixes for the host preflight, first-service packet, and guarded host bootstrap without probing SSH, changing DNS, or mutating any host.

`just gitops-beta-write-runtime-env` and `just gitops-beta-check-runtime-env` materialize and validate the host-local beta runtime env boundary. API runtime config lives in `/var/lib/fishystuff/gitops-beta/api/runtime.env`; the GitOps graph owns `/var/lib/fishystuff/gitops-beta/api/beta.env` for release identity and selected Dolt ref, so reconciliation cannot erase the operator-owned database/CORS/CDN settings. API writing requires `FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1` and `FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL=mysql://...@127.0.0.1:3316/fishystuff`; `just gitops-beta-write-runtime-env-secretspec profile=beta-runtime` provides the same write path through the narrow SecretSpec `beta-runtime` profile instead of broad deploy/cloud credentials. Public site/CDN defaults are beta-only and production hostnames are refused. Dolt writing requires `FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1` and currently writes an intentionally empty beta env file, because the beta Dolt unit carries its deployment identity statically. Real beta runtime env writes under `/var/lib/fishystuff/gitops-beta` also require the current hostname to match the expected beta resident hostname; `/tmp` fixture writes remain available for local tests. The checker is read-only and rejects the shared `/run/fishystuff/api/env`, production GitOps state, production origins, and non-beta Dolt branch overrides. `just gitops-beta-runtime-env-packet` is the operator short form: missing files are reported as pending write commands along with beta-runtime SecretSpec readiness and embedded host preflight status/action, but invalid existing files still fail hard; once both files are ready, it prints the exact `gitops-beta-service-start-packet` command for the next step. `just gitops-beta-runtime-env-host-preflight` is the read-only host-context check to run before those guarded writes; it reports the current hostname, expected beta resident hostname, target parent directories, path writability, host/path readiness, active operator SecretSpec profile, the next safe action, and no-mutation flags without creating directories or files.

`just gitops-beta-service-start-plan` is the read-only review step before starting the distinct beta API/Dolt service set. It checks the beta API runtime env, beta Dolt env, beta API bundle, and beta Dolt bundle, then prints the reviewed unit hashes and guarded `gitops-beta-install-service` commands. `just gitops-beta-service-start-packet` is the short form that prints the checked bundle/env tuple, current host context, unit hashes, Dolt-before-API order, and exact `gitops-beta-start-services` command; when runtime env files are missing, it reports `pending_runtime_env` instead of failing. When `api_bundle=auto` or `dolt_bundle=auto`, the plan first tries to resolve those bundle paths from `data/gitops/beta-current.handoff-summary.json`, so the reviewed start plan follows the exact handoff tuple instead of rebuilding a possibly different attr. It keeps Dolt before API and refuses fixture env-path mismatches outside the test-only override, so the real plan proves the env files being reviewed are the env files the generated units will read.

`just gitops-beta-host-bootstrap-plan` is the read-only fresh-host contract. It reuses the deploy safety assertions, refuses production SecretSpec scope, checks the beta path constants, reports the current hostname versus the expected beta resident hostname, and prints the required beta directories, `fishystuff-beta-dolt` user/group, env files, loopback ports, service unit names, closure materialization order, handoff into `just gitops-beta-service-start-packet`, and later handoff into `just gitops-beta-admission-packet`. It does not create users, directories, VMs, DNS records, or remote sessions.

`just gitops-beta-host-bootstrap-apply` is the guarded local executor for that contract. It requires `FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1`, `FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1`, and `FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1`, re-runs the read-only plan, requires the local hostname to match the expected beta resident hostname, creates only the beta Dolt user/group and the beta directories named by the plan, and reports that no remote deploy or infrastructure mutation occurred. It does not write runtime env files, install systemd units, restart services, provision VMs, or mutate DNS. Its regression intercepts `install`, `hostname`, `getent`, `groupadd`, and `useradd` with fake commands.

`just gitops-beta-start-services` is the guarded local sequence gate for starting the beta API/Dolt pair. It requires the explicit sequence opt-in, both service install/restart opt-ins, both reviewed unit hashes from `just gitops-beta-service-start-plan`, and the expected beta resident hostname. It re-runs the start plan against the same handoff summary, refuses stale hashes, then invokes `just gitops-beta-install-service` behavior for Dolt before API. Its regression intercepts `install`, `hostname`, and `systemctl` and checks the order, so the API cannot be started by this path before the beta Dolt unit.

`just gitops-beta-install-service` is the guarded local install/restart gate for the beta API and Dolt units. API requires `FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1`, `FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1`, `FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=<checked beta API unit hash>`, and the expected beta resident hostname. Dolt requires the corresponding `FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1`, `FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1`, `FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256=<checked beta Dolt unit hash>`, and the same hostname match. The gate re-runs the beta service-bundle check, installs only the beta unit, reloads systemd, restarts that beta unit, and verifies it is active. Its regression intercepts `install`, `hostname`, and `systemctl` with fake commands.

`just gitops-beta-current-handoff` adds the first beta handoff proof around that snapshot. It generates the beta desired-state file, verifies local closure paths, verifies the active CDN serving manifest, runs GitOps unify, writes a handoff summary, and verifies that summary. Unlike production-current handoff it does not require retained rollback releases yet, because this is the first clean beta service-set candidate rather than a live production upgrade. It records that serving readiness was intentionally skipped.

`just gitops-beta-observe-admission` captures the practical beta admission inputs from a running host-local beta candidate. It only accepts a loopback API upstream, fetches `/api/v1/meta`, probes `/api/v1/fish?lang=en` as a DB-backed route that exercises localized Dolt data, checks the active site closure's `runtime-config.js` points at the beta CDN base, checks the active CDN runtime manifest and referenced JS/WASM files exist, then writes checked activation evidence through `just gitops-beta-write-activation-admission-evidence`. `just gitops-beta-admission-packet` is the read-only short form: if evidence is missing it prints the exact observe command; if evidence exists it verifies it still matches the beta handoff summary, release, Dolt commit, and loopback upstream before handing off to the activation-draft packet. `just gitops-beta-activation-draft-packet` is the next read-only short form: it requires checked admission evidence, checks any existing activation draft against that same tuple, and then prints either the draft-generation command or the operator-proof packet. Observation writes local evidence files only; these packet commands do not apply, install, restart, SSH, or mutate DNS/cloud state.

`just gitops-beta-first-service-set-plan` is the lightweight runbook view for the first clean beta service set. It refuses non-loopback API admission targets, checks any present beta handoff/admission/draft artifacts are beta-shaped, reports proof-index completeness if proofs exist, runs the local deploy-authority and beta deploy-credential checks, and prints both the full guarded sequence and a compact `operator_packet_*` section with the immediate next command plus current host and credential context. When runtime env files are the blocker, that packet folds in beta-runtime SecretSpec readiness and host-preflight status, then uses the preflight decision as the effective next action: run the preflight on the expected beta host, bootstrap host paths, or write runtime env files. Later proof/apply/edge phases now route through the read-only packet chain first, so operators see reviewed hashes, deploy credential readiness, credential-authority state, and current host context before any guarded apply or install command. `just gitops-beta-first-service-set-packet` is the read-only short form that prints only that packet plus safety flags. Once runtime env files are ready, the next action becomes `start_or_verify_beta_services`; admission evidence is only the follow-up after the guarded beta Dolt/API start command has run or the operator has verified those beta services are already active. Neither command invokes mgmt, installs units, restarts services, uses SSH, mutates DNS, or provisions hosts.

`just gitops-beta-write-activation-admission-evidence` and `just gitops-beta-activation-draft` are the beta-shaped admission and activation wrappers. They require a beta handoff summary and refuse production summaries. The shared activation checker now reads the environment from the handoff summary, so the same invariant applies to beta: a serving draft must include explicit admission evidence and a retained rollback release. The current `gitops-beta-current-handoff` output is therefore candidate-only until a retained beta release is added.

`just gitops-beta-host-handoff-plan` is a dry-run host-local handoff review for a checked beta activation draft and beta edge bundle. It validates the beta edge bundle, beta served roots, beta TLS paths, beta API upstream, guarded beta apply command, and guarded beta edge install command. It reports `beta_apply_gate_available=true`, but still does not apply, install, or restart anything by itself.

`just gitops-beta-verify-activation-served` is the read-only served-state check for the beta path. It refuses non-beta handoff summaries, then verifies that the local beta served documents under `/var/lib/fishystuff/gitops-beta` and `/run/fishystuff/gitops-beta` still match the checked beta activation draft, admission evidence, selected host, selected release, route, admission, and roots-ready state.

`just gitops-beta-operator-proof` and `just gitops-check-beta-operator-proof` are the beta operator-proof wrappers. They use beta defaults for state, run, unit, TLS, and edge bundle paths, refuse non-beta summaries, and write/check `fishystuff.gitops.beta-operator-proof.v1` artifacts. `just gitops-beta-operator-proof-packet` is the read-only short form: it requires a checked activation draft, checks the latest or selected beta proof, and prints the guarded beta apply command with the exact proof hash only when the proof is current. The proof is still local-only: it records inventory, preflight, and host-handoff-plan evidence, but it does not apply the activation draft or restart services.

`just gitops-beta-served-proof` and `just gitops-beta-proof-index` are the beta post-reconciliation audit wrappers. They link served-state verification back to a checked beta operator proof and require the latest beta served proof to point at the latest beta operator proof. `just gitops-beta-served-proof-packet` is the read-only short form: it reports whether the operator proof is ready, whether beta served state has appeared after apply, whether a served proof must be written, or whether the complete proof index can be checked before edge install. They remain read-only and are only meaningful after a beta apply gate has reconciled local served state.

`just gitops-beta-apply-activation-draft` is the guarded beta local apply gate. It refuses to run without `FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1`, `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`, `FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=<checked beta proof hash>`, and the expected beta resident hostname. It checks a beta operator proof, refuses production summaries/proofs, and runs mgmt only against the beta activation draft. In `local-apply` mode, the clean graph publishes beta state under `/var/lib/fishystuff/gitops-beta` and `/run/fishystuff/gitops-beta`.

`just gitops-beta-edge-install-packet` is the read-only edge handoff packet. It requires a complete beta proof index before checking the beta edge bundle, then prints the exact `FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256` and `FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256` values needed by the guarded install. `just gitops-beta-install-edge` is the beta-only edge install/restart gate. It refuses to run without `FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1`, `FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1`, `FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=<checked beta served proof hash>`, `FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=<checked beta edge unit hash>`, and the expected beta resident hostname. It re-checks that `just gitops-beta-proof-index require_complete=true` is complete, re-validates the beta edge bundle, installs only `/etc/systemd/system/fishystuff-beta-edge.service`, reloads systemd, restarts `fishystuff-beta-edge.service`, and checks it is active. The regression intercepts `install`, `hostname`, and `systemctl` with fake commands so this guard remains testable without mutating the developer machine.

Next pieces to add:

1. first real beta apply on the new service set, followed by served proof/index capture and edge install
2. fold the first-service-set runbook into a minimal repeated-deploy runbook after the first successful start
3. add stronger DB-backed admission routes once the exact production incident class has a cheap stable endpoint
