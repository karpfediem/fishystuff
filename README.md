# fishystuff

A very fishy website

## Development

### Prerequisites

This project uses [devenv](https://devenv.sh/) for the local development environment.
Runtime secrets are declared in [secretspec.toml](/home/carp/code/fishystuff/secretspec.toml)
and loaded with [SecretSpec](https://secretspec.dev/).

To install them you can follow this guide: https://devenv.sh/getting-started/

Once installed, enter the development environment with:

```bash
devenv shell
```

If you use `direnv`, run `direnv allow` once at the repo root and the environment
will activate automatically on entry.

To run the local development servers:

```bash
just up
```

`just up` runs `devenv up --no-tui` with `process-compose` and starts the
long-lived local services:

- `db` must become ready before `api`
- `jaeger` serves the local Jaeger v2 trace UI, including the Monitor tab, on `127.0.0.1:16686`
- `loki` serves local log ingestion and queries on `127.0.0.1:3100`
- `otel-collector` receives traces and metrics from Vector on `127.0.0.1:4818`, forwards traces to Jaeger, and exports spanmetrics on `127.0.0.1:8889`
- `vector` tails repo-local process logs under `data/vector/process/*.log`, accepts local OTLP on `127.0.0.1:4820`, writes newline-delimited JSON archives under `data/vector/archive/`, ships normalized logs to Loki, and forwards traces plus metrics downstream to the collector
- `prometheus` scrapes the collector spanmetrics endpoint and serves the local Prometheus UI on `127.0.0.1:9090`
- `grafana` is the local human-facing observability UI on `127.0.0.1:3000`, with repo-provisioned Loki, Prometheus, and Jaeger datasources
- `caddy` serves `site/.out/` on `127.0.0.1:1990` and `data/cdn/public/` on `127.0.0.1:4040`

If you want the same stack plus rebuild/restart watchers, use:

```bash
just watch
```

`just watch` runs `devenv up --profile watch --no-tui` and adds:

- API auto-restart on backend changes
- map runtime rebuild plus CDN restaging on map/lib changes
- CDN host asset restaging on `site/assets/map` changes
- site output rebuilds on site source changes

Use the manual build commands when you want a one-shot rebuild without starting
the stack:

- `just build`
  - one-shot build of the map runtime, staged CDN payload, and site output
- `just build-map`
  - build the wasm runtime and refresh the staged CDN payload
- `just build-site`
  - rebuild `site/.out`

The site build still emits `.out/runtime-config.js` from the current
environment. That file is the single local-development source of truth for the
site/API/CDN base URLs consumed by the browser host and Bevy runtime.
Public/static deployments can set `FISHYSTUFF_PUBLIC_SITE_BASE_URL` and let the
site build derive sibling defaults like `api.<site-host>`, `cdn.<site-host>`,
and `telemetry.<site-host>`, or override any of them explicitly with:

- `FISHYSTUFF_PUBLIC_API_BASE_URL`
- `FISHYSTUFF_PUBLIC_CDN_BASE_URL`
- `FISHYSTUFF_PUBLIC_TELEMETRY_BASE_URL`
- `FISHYSTUFF_PUBLIC_TELEMETRY_TRACES_ENDPOINT`

The legacy `FISHYSTUFF_PUBLIC_OTEL_BASE_URL` and
`FISHYSTUFF_PUBLIC_OTEL_TRACES_ENDPOINT` names are still accepted as
compatibility aliases.

Local development still uses the explicit `FISHYSTUFF_RUNTIME_*` overrides from
`devenv.nix`, which take precedence over the public-origin layer.
The same public-origin layer also drives the repo-managed static site build
metadata and shell tooling defaults, so beta deploys can switch the site/API/
CDN/OTEL family together without editing hard-coded URLs in scripts.

For browser request tracing in local development, the supported path is:

```bash
devenv shell
just up
```

Then open `http://127.0.0.1:1990/`, `http://127.0.0.1:3000/explore`, and
`http://127.0.0.1:16686/`. Use Grafana as the primary frontend for local logs,
metrics, and trace correlation, and keep Jaeger open when you want the native
Jaeger trace UI or the Monitor tab. The site
runtime emits browser fetch spans through the JS OpenTelemetry Web SDK and the
API emits server/store spans directly from Rust. The browser uses same-origin
`/telemetry/v1/*` endpoints from `site/.out/runtime-config.js`; Caddy proxies
those requests to Vector, and Vector forwards traces and metrics downstream to
the collector. Local API CORS still has to allow the site origin, but browser
OTLP CORS no longer depends on the collector.

Jaeger Service Performance Monitoring now uses Prometheus-backed RED metrics
derived from the collector's `spanmetrics` connector. Expect the Monitor tab to
remain empty until spans have been emitted and Prometheus has completed at least
one scrape cycle.

Vector now owns the local OTLP ingress as well as the log/archive layer, and
Loki is the local query surface for normalized logs. The local flow is:

- browser telemetry -> Caddy `/telemetry/v1/*` -> Vector
- API traces -> Vector OTLP HTTP
- Vector traces/metrics -> collector -> Jaeger + Prometheus
- Vector logs -> Loki + local NDJSON archives

The correlation flow is:

- frontend fetch spans record `fishystuff.response.request_id`, `fishystuff.response.trace_id`, and `fishystuff.response.span_id`
- the API now emits structured request-completion/error logs with matching `request.id`, `trace.id`, and `span.id` fields
- Vector normalizes those into `request_id`, `trace_id`, and `span_id` for Loki structured metadata and for the local NDJSON archives

Grafana is provisioned from repo files and comes up with:

- `Loki` as the default Explore datasource
- `Prometheus` for spanmetrics and runtime metrics
- `Jaeger` for trace search and trace-detail views
- a log-derived `trace_id` link from Loki log lines back into Jaeger traces

The current local archive/query paths are:

- `data/vector/process/*.log`
  - per-process timestamped stdout/stderr captures from the local supervised stack
- `data/vector/archive/logs/*.ndjson`
  - Vector-normalized log events with process/service/correlation fields
- `data/vector/archive/traces/*.ndjson`
  - Vector-serialized trace events captured at the local OTLP ingress

For live LLM- or shell-driven inspection, the preferred entrypoint is:

```bash
tools/scripts/vector-tap.sh browser-logs
```

That wrapper follows the official Vector `tap` flow, targets the repo's local
Vector API, emits bounded JSON samples by default, and exposes stable presets
for the current pipeline graph. Use `tools/scripts/vector-tap.sh --list-presets`
to see the current tap surface.

Example live flows:

```bash
tools/scripts/vector-tap.sh process-logs
tools/scripts/vector-tap.sh browser-logs --follow
tools/scripts/vector-tap.sh raw-traces --duration-ms 3000
tools/scripts/vector-tap.sh to-loki -- --format logfmt
```

If you are not already inside the active `devenv` shell, wrap the same entrypoint
with:

```bash
devenv shell -- tools/scripts/vector-tap.sh browser-logs
```

Use Loki or the NDJSON archives when you need historical queries or raw files
instead of a live stream. Loki is useful when you already know a stable
low-cardinality stream selector, while `data/vector/archive/*.ndjson` is easier
when you want direct `rg`/`jq` access to the raw normalized events.

For browser use, the convenience entrypoints are:

```bash
just open dashboard
just open grafana
just open loki
just open jaeger
just open prometheus
just open loki-status
```

The first provisioned dashboard is `Fishystuff Local Observability` at:

```text
http://127.0.0.1:3000/d/fishystuff-local-observability/fishystuff-local-observability
```

Example Loki query flow:

```bash
curl -G -s http://127.0.0.1:3100/loki/api/v1/query_range \
  --data-urlencode 'query={app="fishystuff",process="api"} | json | trace_id="YOUR_TRACE_ID"' \
  | jq
```

Example archive flow:

```bash
rg '"trace_id":"YOUR_TRACE_ID"' data/vector/archive/logs
rg '"fishystuff.response.trace_id":"YOUR_TRACE_ID"' data/vector/archive/traces
```

If you want to add downstream routing later, extend
`tools/telemetry/vector.local.yaml` with new sinks from
`normalized_process_logs` for logs or `telemetry_ingress.traces` for OTLP trace
copies.

This tracing path remains request-scoped. The local browser runtime now also
exports a small Prometheus-facing OTLP metrics surface through the same
collector, covering live Bevy FPS/frame-time gauges plus selected map/terrain
runtime state. Use Prometheus for continuous local runtime dashboards, and keep
using the existing browser/native profiling harnesses under `tools/scripts/`
when you need scenario reports, traces, or deeper hotspot analysis.

The API uses a strict explicit CORS allowlist. Production origins are declared
in [api/config.toml](/home/carp/code/fishystuff/api/config.toml), and `devenv`
adds the local site origins through `FISHYSTUFF_CORS_ALLOWED_ORIGINS`, so the
same CORS model is exercised in both dev and prod.

The API and other DB-backed Rust tooling use the repo's `secretspec.toml`
through repo-owned defaults, so local builds and runs do not require
`secretspec config init`, `FISHYSTUFF_DATABASE_URL`, or SecretSpec selector
environment variables.

Only the `cdn` and `bot` profiles still need an explicit provider setup when
you work on those paths. Check them with:

```bash
just secrets-check cdn
just secrets-check bot
```

To update the pinned `devenv` inputs after intentional environment changes:

```bash
devenv update
```

### Commands

List commands

```bash
just -l
```
