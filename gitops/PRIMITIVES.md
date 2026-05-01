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

The new graph only emits these in `local-apply` mode. `validate` and `vm-test` intentionally no-op closure realization.

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

The first milestone does not use `exec` in the GitOps graph. It remains available for a future narrow local admission probe bridge if no better HTTP probe primitive exists.

## Services

Signature from `/home/carp/code/mgmt/engine/resources/svc.go`:

```text
svc "<unit>" {
  state => "running" | "stopped" | ""
  startup => "enabled" | "disabled" | ""
  session => bool
}
```

Some adjacent branches add `refresh_action`, but the plain checkout has only the fields above. The first milestone does not manage services.

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

No dedicated HTTP client probe/status resource was found. Existing FishyStuff health checks use `exec` with `curl` in the old resident graph. The new graph does not run real admission in `validate` mode and uses a deterministic VM fixture in `vm-test`.

## Git/Dolt Signals

No `git/ref` mgmt resource or function was found in the inspected checkouts. Future Git/Dolt watching should either compose existing primitives or introduce a small Unix-like primitive if needed.

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
