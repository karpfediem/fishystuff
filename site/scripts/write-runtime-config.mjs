import fs from "node:fs/promises";
import path from "node:path";

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

function normalizeBaseUrl(value) {
  const normalized = String(value ?? "").trim();
  if (!normalized) {
    return "";
  }
  return normalized.replace(/\/+$/, "");
}

function normalizeFlag(value, fallback = false) {
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

function normalizeFloat(value, fallback) {
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

function normalizeCacheKey(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    return;
  }

  const runtimeConfig = {
    siteBaseUrl:
      normalizeBaseUrl(process.env.FISHYSTUFF_RUNTIME_SITE_BASE_URL) || "https://fishystuff.fish",
    apiBaseUrl:
      normalizeBaseUrl(process.env.FISHYSTUFF_RUNTIME_API_BASE_URL) || "https://api.fishystuff.fish",
    cdnBaseUrl:
      normalizeBaseUrl(process.env.FISHYSTUFF_RUNTIME_CDN_BASE_URL) || "https://cdn.fishystuff.fish",
    mapAssetCacheKey: normalizeCacheKey(process.env.FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY),
    tracing: {
      enabled: normalizeFlag(process.env.FISHYSTUFF_RUNTIME_OTEL_ENABLED, false),
      debug: normalizeFlag(process.env.FISHYSTUFF_RUNTIME_OTEL_DEBUG, false),
      serviceName:
        String(process.env.FISHYSTUFF_RUNTIME_OTEL_SERVICE_NAME ?? "").trim() || "fishystuff-site",
      deploymentEnvironment:
        String(process.env.FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT ?? "").trim()
        || "production",
      serviceVersion:
        String(process.env.FISHYSTUFF_RUNTIME_OTEL_SERVICE_VERSION ?? "").trim(),
      exporterEndpoint:
        String(process.env.FISHYSTUFF_RUNTIME_OTEL_EXPORTER_ENDPOINT ?? "").trim(),
      jaegerUiUrl: normalizeBaseUrl(process.env.FISHYSTUFF_RUNTIME_OTEL_JAEGER_UI_URL),
      sampleRatio: normalizeFloat(process.env.FISHYSTUFF_RUNTIME_OTEL_SAMPLE_RATIO, 0.25),
    },
  };

  const outPath = path.resolve(process.cwd(), args.out);
  await fs.mkdir(path.dirname(outPath), { recursive: true });
  await fs.writeFile(
    outPath,
    `window.__fishystuffRuntimeConfig = Object.freeze(${JSON.stringify(runtimeConfig, null, 2)});\n`,
    "utf8",
  );
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
