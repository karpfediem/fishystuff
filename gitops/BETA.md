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
just gitops-beta-edge-handoff-bundle
just gitops-beta-edge-handoff-bundle-test
just gitops-beta-current-desired
just gitops-beta-current-validate
just gitops-beta-current-desired-test
just gitops-beta-current-handoff
just gitops-beta-current-handoff-test
just gitops-beta-write-activation-admission-evidence
just gitops-beta-activation-draft
just gitops-beta-activation-draft-test
just gitops-beta-host-handoff-plan
just gitops-beta-host-handoff-plan-test
just gitops-beta-verify-activation-served
just gitops-beta-verify-activation-served-test
nix build .#checks.x86_64-linux.api-service-bundle-beta-gitops-handoff --no-link
nix build .#checks.x86_64-linux.dolt-service-bundle-beta-gitops-handoff --no-link
nix build .#checks.x86_64-linux.edge-service-bundle-beta-gitops-handoff --no-link
```

The API bundle validates:

- service ID `fishystuff-beta-api`
- systemd unit `fishystuff-beta-api.service`
- beta runtime env file:
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

`just gitops-beta-current-desired` writes `data/gitops/beta-current.desired.json` as a validate-mode desired-state snapshot from exact local outputs. It is parameterized from the production-current generator but pins the beta service bundle attrs, `site-content-beta`, Dolt branch context `beta`, beta gcroot/cache roots, and the beta release-ref prefix `fishystuff/gitops-beta`. The default CDN runtime attr is `cdn-serving-root`, so the recipe requires `FISHYSTUFF_OPERATOR_ROOT` for operator-local CDN data unless `FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE` supplies an exact existing closure. It does not apply or serve anything.

`just gitops-beta-current-validate` generates that same snapshot and type-checks it through `gitops/main.mcl`. It is still local-only: no SSH, no Hetzner, no Cloudflare, no systemd changes.

`just gitops-beta-current-handoff` adds the first beta handoff proof around that snapshot. It generates the beta desired-state file, verifies local closure paths, verifies the active CDN serving manifest, runs GitOps unify, writes a handoff summary, and verifies that summary. Unlike production-current handoff it does not require retained rollback releases yet, because this is the first clean beta service-set candidate rather than a live production upgrade. It records that serving readiness was intentionally skipped.

`just gitops-beta-write-activation-admission-evidence` and `just gitops-beta-activation-draft` are the beta-shaped admission and activation wrappers. They require a beta handoff summary and refuse production summaries. The shared activation checker now reads the environment from the handoff summary, so the same invariant applies to beta: a serving draft must include explicit admission evidence and a retained rollback release. The current `gitops-beta-current-handoff` output is therefore candidate-only until a retained beta release is added.

`just gitops-beta-host-handoff-plan` is a dry-run host-local handoff review for a checked beta activation draft and beta edge bundle. It validates the beta edge bundle, beta served roots, beta TLS paths, and beta API upstream. It intentionally reports `beta_apply_gate_available=false`; consuming the draft on a host still requires the next beta operator-proof/apply gate slice.

`just gitops-beta-verify-activation-served` is the read-only served-state check for the beta path. It refuses non-beta handoff summaries, then verifies that the local beta served documents under `/var/lib/fishystuff/gitops-beta` and `/run/fishystuff/gitops-beta` still match the checked beta activation draft, admission evidence, selected host, selected release, route, admission, and roots-ready state.

Next pieces to add:

1. beta operator-proof and local apply gate wrappers parameterized from the production path
2. beta served proof/proof-index wrappers around the served verifier
3. only then, a separate manually confirmed host bootstrap path
