# Primitive Reconnaissance

Scope inspected:

- FishyStuff `mgmt/` tree in this repository
- Adjacent local mgmt checkouts under `/home/carp/code/mgmt`, `/home/carp/code/mgmt-nix-resources`, and `/home/carp/code/mgmt-fishystuff-beta`

The old FishyStuff beta/resident graphs were used only to discover names and signatures.

## JSON And File Input

- `encoding.decode_json(data str, type str) <typed value>`
- `encoding.decode_json_flexible(data str, type str) <typed value>`
- `encoding.decode_json_flexible(data str) <inferred typed value>`

Implementation: `/home/carp/code/mgmt/lang/core/encoding/decode_json.go`.

The flexible variant supplies zero values for missing struct fields. `gitops/main.mcl` uses this with an explicit desired-state struct type.

- `os.readfile(filename str) str`
- `os.readfilewait(filename str) str`
- `deploy.readfile(filename str) str`
- `deploy.readfileabs(filename str) str`

`os.readfile` reads a local path and watches it. `deploy.readfile` reads a file packaged in a mgmt deploy. The new GitOps module uses `os.readfile` because the state file is operator/VM-local during this milestone.

## Nix

`nix:closure` was not present in the plain `/home/carp/code/mgmt` checkout, but is implemented in `/home/carp/code/mgmt-nix-resources` and `/home/carp/code/mgmt-fishystuff-beta`.

Signature from `/home/carp/code/mgmt-nix-resources/engine/resources/nix_closure.go`:

```text
nix:closure "<name>" {
  state => "present"
  paths => []str
  drvs => []str
  mode => "verify" | "substitute" | "realise" | "realize"
  realise_inputs => []str
  keep_going => bool
  ignore_unknown => bool
  check_contents => bool
  max_jobs => int
  cores => int
  build_timeout => uint64
  max_silent_time => uint64
  command_timeout => uint64
  nix_store => str
  store_dir => str
  nix_options => map{str: str}
  env => map{str: str}
}
```

Defaults: `state = "present"`, `mode = "verify"`, `nix_store = "nix-store"`, `store_dir = "/nix/store"`.

`nix:gcroot` was likewise present in the adjacent Nix-resource checkouts, not in plain `/home/carp/code/mgmt`.

Signature from `/home/carp/code/mgmt-nix-resources/engine/resources/nix_gcroot.go`:

```text
nix:gcroot "<path>" {
  path => str
  target => str
  state => "exists" | "absent"
  force => bool
  gc_roots_dir => str
  store_dir => str
}
```

Defaults: `state = "exists"`, `gc_roots_dir = "/nix/var/nix/gcroots"`, `store_dir = "/nix/store"`.

The new graph currently emits `nix:closure` in `vm-test-closures` and future `local-apply` mode, then uses ordinary `file` symlink resources under `/nix/var/nix/gcroots/fishystuff/...` for GC roots. This is deliberate: VM testing showed the pinned mgmt build could create `nix:gcroot`/symlink roots but did not unblock dependent status/active/route publication when those resources were used as publication gates. `validate` and plain `vm-test` intentionally no-op closure realization.

## Local Files

Signature from `/home/carp/code/mgmt/engine/resources/file.go`:

```text
file "<path>" {
  path => str
  dirname => str
  basename => str
  state => "exists" | "absent"
  content => str
  source => str
  fragments => []str
  owner => str
  group => str
  mode => str
  recurse => bool
  force => bool
  purge => bool
  symlink => bool
  selinux => str
}
```

Used by the VM-test instance/admission/status modules.

## Exec

Signature from `/home/carp/code/mgmt/engine/resources/exec.go` includes:

```text
exec "<name>" {
  cmd => str
  args => []str
  cwd => str
  shell => str
  timeout => uint64
  env => map{str: str}
  watchcmd => str
  watchfiles => []str
  ifcmd => str
  nifcmd => str
  creates => str
  mtimes => []str
  donecmd => str
  user => str
  group => str
}
```

The GitOps graph uses `exec` only as narrow host-local bridges for behavior not yet covered by dedicated mgmt resources: Dolt `fetch_pin`/SQL admission in VM tests and loopback HTTP admission probes in local writing modes. These execs invoke the packaged `fishystuff_deploy` helper with structured request/status files and `needs-*` freshness checks; they are not deployment controllers.

## Services

Signature from `/home/carp/code/mgmt/engine/resources/svc.go`:

```text
svc "<unit>" {
  state => "running" | "stopped" | ""
  startup => "enabled" | "disabled" | ""
  session => bool
}
```

Additional signature from the pinned GitOps mgmt input at `/home/carp/code/mgmt-fishystuff-beta/engine/resources/svc.go`:

```text
svc "<unit>" {
  refresh_action => "reload-or-try-restart" | "try-restart" | ""
}
```

The GitOps graph uses `svc` only for an optional local-writing-mode candidate API service. Desired state must provide the bare mgmt service name, not a `.service` unit filename, because mgmt appends `.service` internally. HTTP admission waits for this local candidate service when `api_service` is set. The graph writes a candidate API env file with `FISHYSTUFF_RELEASE_ID`, `FISHYSTUFF_RELEASE_IDENTITY`, `FISHYSTUFF_DOLT_COMMIT`, and `FISHYSTUFF_DEPLOYMENT_ENVIRONMENT`, sets `Notify => Svc[...]` on that env file, and sets `refresh_action => "try-restart"` so an env-file change restarts a running candidate instead of leaving admission stuck on the old release identity.

The FishyStuff flake applies `nix/patches/mgmt-recwatch-bound-watch-path-index.patch` to the pinned GitOps mgmt package. This is a local backport of the adjacent mgmt `util: Bound watch path index` change, needed because nested GitOps state directory creation can otherwise panic in `recwatch` before service/admission resources reconcile.

## KV And Schedule

`kv` resource signature from `/home/carp/code/mgmt/engine/resources/kv.go`:

```text
kv "<name>" {
  key => str
  value => str
  mapped => bool
  skiplessthan => bool
  skipcmpstyle => int
}
```

`world.kvlookup(namespace str) map{str: str}` is available from `/home/carp/code/mgmt/lang/core/world/kvlookup.go`.

`schedule` resource signature from `/home/carp/code/mgmt/engine/resources/schedule.go`:

```text
schedule "<name>" {
  namespace => str
  strategy => str
  max => int
  persist => bool
  ttl => int
  withdraw => bool
}
```

`world.schedule(namespace str) []str` is available from `/home/carp/code/mgmt/lang/core/world/schedule.go`.

The first milestone avoids KV because local status files are enough and avoid etcd/world coupling in the VM test.

## HTTP And Probes

Available HTTP server resources:

- `http:server`
- `http:server:file`
- `http:server:flag`
- `http:server:ui`
- `http:server:ui:input`
- `http:server:proxy`

No dedicated HTTP client probe/status resource was found. Existing FishyStuff health checks use `exec` with `curl` in the old resident graph. The new graph does not run real admission in `validate` mode, uses deterministic fixtures in plain VM tests, and uses the structured `fishystuff_deploy` HTTP helper for loopback-only admission in local writing modes.

The local `fishystuff_deploy` helper now includes a narrow HTTP probe bridge:

```text
fishystuff_deploy http probe-status --request <json> --status <json>
fishystuff_deploy http needs-probe-status --request <json> --status <json>
fishystuff_deploy http probe-json-scalar --request <json> --status <json>
fishystuff_deploy http needs-probe-json-scalar --request <json> --status <json>
fishystuff_deploy http probe-json-scalars --request <json> --status <json>
fishystuff_deploy http needs-probe-json-scalars --request <json> --status <json>
```

The request tuple includes environment, host, release ID, release identity, probe name, URL, expected status, and optional timeout. JSON scalar probes add `json_pointer` and `expected_scalar`; JSON scalars probes add an `expected_scalars` object keyed by JSON pointer. The helper only probes loopback HTTP targets and rejects credential-bearing URLs. This keeps the bridge host-local and reusable for API readiness/meta checks without becoming a deployment controller.

## Git/Dolt Signals

No `git/ref` mgmt resource or function was found in the inspected checkouts. Future Git/Dolt watching should either compose existing primitives or introduce a small Unix-like primitive if needed.

Dolt's own CLI and SQL server procedures are sufficient for the first bandwidth-saving materialization path:

- `dolt clone --branch <branch> --single-branch <remote-url> <dir>` bootstraps a persistent local cache.
- `dolt fetch <remote> <refspec>` incrementally fetches changed objects into that cache.
- `dolt remote -v`, `dolt remote remove <name>`, and `dolt remote add <name> <url>` are enough to reconcile the cache's `origin` URL when desired state switches to another mirror.
- `dolt branch -f <release-ref> <commit>` pins an exact desired commit under a local ref so rollback commits remain reachable.
- `dolt sql -r csv -q "select dolt_hashof('<ref>') as hash"` or `dolt log -n 1 <ref> --oneline` can verify the pinned ref resolves to the exact desired commit.
- SQL procedures `DOLT_FETCH()` and `DOLT_RESET()` mirror the CLI shape for a running SQL server, but the old branch-tip reset pattern is not exact enough for GitOps serving without a commit-hash verification gate.
- `dolt backup sync-url <url>` and `dolt backup restore <url> <name>` can move point-in-time database contents through `file`, `http(s)`, `aws`, or `gs` backup URLs. This is useful for bootstrap/disaster recovery, but it is not the normal deploy path because restoring a backup is a larger state replacement than pinning an already-warm cache.
- `dolt sql-server` supports `remotesapi.port`, `remotesapi.read_only`, and a documented `cluster` configuration section for replication. This should become a separate `replica_pin` path only after a local test proves exact commit pinning and read-only serving semantics against a running replica.

`gitops/modules/fishy/dolt.mcl` uses `exec` only in VM test mode to invoke the narrow `fishystuff_deploy dolt fetch-pin` Rust helper against a file-backed Dolt remote. The helper is a host-local bridge over Dolt's CLI, not a deployment controller. A future upstreamable primitive may still replace the bridge if the shape proves generally useful.

Signal/debounce/stable-related pieces found:

- `history(value, index_ms)` can model simple time hysteresis.
- `schedule` and `world.schedule` can coordinate host selection through mgmt world state.
- No dedicated debounce/stable health primitive was found.

## Hetzner

`hetzner:vm` exists in `/home/carp/code/mgmt/engine/resources/hetzner_vm.go` with fields including:

```text
apitoken => str
state => "" | "absent" | "exists" | "off" | "running"
allowrebuild => "" | "ifneeded" | "ignore"
servertype => str
datacenter => str
image => str
userdata => str
serverrescuemode => "" | "linux32" | "linux64" | "freebsd64"
serverrescuekeys => []str
waitinterval => uint32
waittimeout => uint32
```

The FishyStuff beta branch also has observed metadata sends such as `publicipv4`. The new GitOps graph does not import or use Hetzner resources in this milestone.
