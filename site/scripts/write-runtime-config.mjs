import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

function parseArgs(argv) {
  const args = {
    out: ".out/runtime-config.js",
    help: false,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--out" && i + 1 < argv.length) {
      args.out = argv[++i];
      continue;
    }
    throw new Error(`unknown arg: ${arg}`);
  }

  return args;
}

function printHelp() {
  console.log(`write-runtime-config.mjs

Emit the browser runtime config consumed by the site shell.

Options:
  --out <path>   Output file (default: .out/runtime-config.js)
  --help         Show this message
`);
}

export function normalizeBaseUrl(value) {
  const normalized = String(value ?? "").trim();
  if (!normalized) {
    return "";
  }
  return normalized.replace(/\/+$/, "");
}

export function normalizeEndpointUrl(value) {
  const normalized = String(value ?? "").trim();
  if (!normalized) {
    return "";
  }
  try {
    return new URL(normalized).toString();
  } catch {
    return normalized;
  }
}

export function isLoopbackHost(hostname) {
  return hostname === "127.0.0.1" || hostname === "localhost";
}

export function deriveSiblingBaseUrl(baseUrl, subdomain) {
  const normalizedBaseUrl = normalizeBaseUrl(baseUrl);
  const normalizedSubdomain = String(subdomain ?? "").trim().replace(/\.+$/, "");
  if (!normalizedBaseUrl || !normalizedSubdomain) {
    return "";
  }
  try {
    const url = new URL(normalizedBaseUrl);
    if (!url.hostname || isLoopbackHost(url.hostname)) {
      return "";
    }
    url.hostname = `${normalizedSubdomain}.${url.hostname}`;
    url.pathname = "";
    url.search = "";
    url.hash = "";
    return normalizeBaseUrl(url.toString());
  } catch {
    return "";
  }
}

export function joinUrl(baseUrl, pathname) {
  const normalizedBaseUrl = normalizeBaseUrl(baseUrl);
  const normalizedPath = String(pathname ?? "").trim();
  if (!normalizedBaseUrl || !normalizedPath) {
    return "";
  }
  try {
    return new URL(
      normalizedPath.startsWith("/") ? normalizedPath : `/${normalizedPath}`,
      `${normalizedBaseUrl}/`,
    ).toString();
  } catch {
    return `${normalizedBaseUrl}${normalizedPath.startsWith("/") ? "" : "/"}${normalizedPath}`;
  }
}

export function siblingEndpointUrl(endpointUrl, pathname) {
  const normalizedEndpointUrl = normalizeEndpointUrl(endpointUrl);
  const normalizedPath = String(pathname ?? "").trim();
  if (!normalizedEndpointUrl || !normalizedPath) {
    return "";
  }
  try {
    return new URL(
      normalizedPath.startsWith("/") ? normalizedPath : `/${normalizedPath}`,
      normalizedEndpointUrl,
    ).toString();
  } catch {
    return "";
  }
}

export function normalizeFlag(value, fallback = false) {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (!normalized) {
    return fallback;
  }
  if (["1", "true", "yes", "on"].includes(normalized)) {
    return true;
  }
  if (["0", "false", "no", "off"].includes(normalized)) {
    return false;
  }
  return fallback;
}

export function normalizeFloat(value, fallback) {
  const numeric = Number.parseFloat(String(value ?? "").trim());
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  if (numeric <= 0) {
    return 0;
  }
  if (numeric >= 1) {
    return 1;
  }
  return numeric;
}

export function normalizeCacheKey(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

export function normalizeTelemetryDefaultMode(value, fallback = "opt-in") {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (normalized === "enabled" || normalized === "opt-in" || normalized === "disabled") {
    return normalized;
  }
  return fallback;
}

export function resolvePublicBaseUrls(env = process.env) {
  const publicSiteBaseUrl =
    normalizeBaseUrl(env.FISHYSTUFF_PUBLIC_SITE_BASE_URL) || "https://fishystuff.fish";
  const publicApiBaseUrl =
    normalizeBaseUrl(env.FISHYSTUFF_PUBLIC_API_BASE_URL)
    || deriveSiblingBaseUrl(publicSiteBaseUrl, "api")
    || "https://api.fishystuff.fish";
  const publicCdnBaseUrl =
    normalizeBaseUrl(env.FISHYSTUFF_PUBLIC_CDN_BASE_URL)
    || deriveSiblingBaseUrl(publicSiteBaseUrl, "cdn")
    || "https://cdn.fishystuff.fish";
  const publicTelemetryBaseUrl =
    normalizeBaseUrl(env.FISHYSTUFF_PUBLIC_TELEMETRY_BASE_URL)
    || normalizeBaseUrl(env.FISHYSTUFF_PUBLIC_OTEL_BASE_URL)
    || deriveSiblingBaseUrl(publicSiteBaseUrl, "telemetry")
    || "https://telemetry.fishystuff.fish";
  const publicTelemetryTracesEndpoint =
    normalizeEndpointUrl(env.FISHYSTUFF_PUBLIC_TELEMETRY_TRACES_ENDPOINT)
    || normalizeEndpointUrl(env.FISHYSTUFF_PUBLIC_OTEL_TRACES_ENDPOINT)
    || joinUrl(publicTelemetryBaseUrl, "/v1/traces");

  return {
    publicSiteBaseUrl,
    publicApiBaseUrl,
    publicCdnBaseUrl,
    publicTelemetryBaseUrl,
    publicTelemetryTracesEndpoint,
    publicOtelBaseUrl: publicTelemetryBaseUrl,
    publicOtelTracesEndpoint: publicTelemetryTracesEndpoint,
  };
}

export function buildRuntimeConfig(env = process.env) {
  const {
    publicSiteBaseUrl,
    publicApiBaseUrl,
    publicCdnBaseUrl,
    publicTelemetryBaseUrl,
    publicTelemetryTracesEndpoint,
  } = resolvePublicBaseUrls(env);
  const runtimeSiteBaseUrl =
    normalizeBaseUrl(env.FISHYSTUFF_RUNTIME_SITE_BASE_URL) || publicSiteBaseUrl;
  const runtimeTelemetryEnabledDefault = normalizeFlag(env.FISHYSTUFF_RUNTIME_OTEL_ENABLED, false);
  const runtimeDeploymentEnvironment =
    String(env.FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT ?? "").trim()
    || "production";
  let defaultTelemetryModeFallback = runtimeTelemetryEnabledDefault ? "enabled" : "opt-in";
  try {
    const runtimeSiteUrl = new URL(runtimeSiteBaseUrl);
    if (isLoopbackHost(runtimeSiteUrl.hostname) || runtimeDeploymentEnvironment === "local") {
      defaultTelemetryModeFallback = "opt-in";
    }
  } catch {
    if (runtimeDeploymentEnvironment === "local") {
      defaultTelemetryModeFallback = "opt-in";
    }
  }
  const telemetryDefaultMode = normalizeTelemetryDefaultMode(
    env.FISHYSTUFF_RUNTIME_TELEMETRY_DEFAULT_MODE,
    defaultTelemetryModeFallback,
  );
  const traceExporterEndpoint =
    normalizeEndpointUrl(env.FISHYSTUFF_RUNTIME_OTEL_EXPORTER_ENDPOINT)
    || publicTelemetryTracesEndpoint;

  return {
    siteBaseUrl: runtimeSiteBaseUrl,
    apiBaseUrl:
      normalizeBaseUrl(env.FISHYSTUFF_RUNTIME_API_BASE_URL) || publicApiBaseUrl,
    cdnBaseUrl:
      normalizeBaseUrl(env.FISHYSTUFF_RUNTIME_CDN_BASE_URL) || publicCdnBaseUrl,
    mapAssetCacheKey: normalizeCacheKey(env.FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY),
    client: {
      telemetry: {
        defaultMode: telemetryDefaultMode,
      },
    },
    tracing: {
      enabled: runtimeTelemetryEnabledDefault,
      debug: normalizeFlag(env.FISHYSTUFF_RUNTIME_OTEL_DEBUG, false),
      serviceName:
        String(env.FISHYSTUFF_RUNTIME_OTEL_SERVICE_NAME ?? "").trim() || "fishystuff-site",
      deploymentEnvironment: runtimeDeploymentEnvironment,
      serviceVersion:
        String(env.FISHYSTUFF_RUNTIME_OTEL_SERVICE_VERSION ?? "").trim(),
      exporterEndpoint: traceExporterEndpoint,
      jaegerUiUrl: normalizeBaseUrl(env.FISHYSTUFF_RUNTIME_OTEL_JAEGER_UI_URL),
      sampleRatio: normalizeFloat(env.FISHYSTUFF_RUNTIME_OTEL_SAMPLE_RATIO, 0.25),
    },
    metrics: {
      enabled: normalizeFlag(
        env.FISHYSTUFF_RUNTIME_OTEL_METRICS_ENABLED,
        normalizeFlag(env.FISHYSTUFF_RUNTIME_OTEL_ENABLED, false),
      ),
      exporterEndpoint:
        normalizeEndpointUrl(env.FISHYSTUFF_RUNTIME_OTEL_METRICS_ENDPOINT)
        || siblingEndpointUrl(traceExporterEndpoint, "/v1/metrics")
        || joinUrl(publicTelemetryBaseUrl, "/v1/metrics"),
      exportIntervalMs: Math.max(
        1000,
        Number.parseInt(String(env.FISHYSTUFF_RUNTIME_OTEL_METRIC_EXPORT_INTERVAL_MS ?? "5000"), 10)
          || 5000,
      ),
    },
    logs: {
      enabled: normalizeFlag(
        env.FISHYSTUFF_RUNTIME_OTEL_LOGS_ENABLED,
        normalizeFlag(env.FISHYSTUFF_RUNTIME_OTEL_ENABLED, false),
      ),
      exporterEndpoint:
        normalizeEndpointUrl(env.FISHYSTUFF_RUNTIME_OTEL_LOGS_ENDPOINT)
        || siblingEndpointUrl(traceExporterEndpoint, "/v1/logs")
        || joinUrl(publicTelemetryBaseUrl, "/v1/logs"),
    },
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    return;
  }

  const runtimeConfig = buildRuntimeConfig();

  const outPath = path.resolve(process.cwd(), args.out);
  await fs.mkdir(path.dirname(outPath), { recursive: true });
  await fs.writeFile(
    outPath,
    `window.__fishystuffRuntimeConfig = Object.freeze(${JSON.stringify(runtimeConfig, null, 2)});\n`,
    "utf8",
  );
}

const isMainModule =
  process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;

if (isMainModule) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
