import { diag, DiagConsoleLogger, DiagLogLevel } from "@opentelemetry/api";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { registerInstrumentations } from "@opentelemetry/instrumentation";
import { FetchInstrumentation } from "@opentelemetry/instrumentation-fetch";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { ParentBasedSampler, TraceIdRatioBasedSampler } from "@opentelemetry/sdk-trace-base";
import { BatchSpanProcessor, WebTracerProvider } from "@opentelemetry/sdk-trace-web";
import { ATTR_SERVICE_NAME, ATTR_SERVICE_VERSION } from "@opentelemetry/semantic-conventions";

const OTEL_GLOBAL_KEY = "__fishystuffOtel";
const TRACE_QUERY_KEY = "trace";
const TRACE_SAMPLE_QUERY_KEY = "trace_sample";

function parseBoolean(value, fallback = false) {
  if (typeof value === "boolean") {
    return value;
  }
  const normalized = String(value ?? "").trim().toLowerCase();
  if (!normalized) {
    return fallback;
  }
  if (normalized === "1" || normalized === "true" || normalized === "yes" || normalized === "on") {
    return true;
  }
  if (normalized === "0" || normalized === "false" || normalized === "no" || normalized === "off") {
    return false;
  }
  return fallback;
}

function clampSampleRatio(value, fallback = 0.25) {
  const numeric = Number.parseFloat(value);
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

function normalizeString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizeUrl(value) {
  const normalized = normalizeString(value);
  if (!normalized) {
    return "";
  }
  return normalized.replace(/\/+$/, "");
}

function readQueryOverrides() {
  const params = new URLSearchParams(globalThis.location?.search || "");
  const enabledOverride = params.get(TRACE_QUERY_KEY);
  const sampleOverride = params.get(TRACE_SAMPLE_QUERY_KEY);
  return {
    enabledOverride,
    sampleOverride,
  };
}

function resolveRuntimeConfig() {
  const runtimeConfig = globalThis.__fishystuffRuntimeConfig || {};
  const tracingConfig = runtimeConfig.tracing || {};
  const query = readQueryOverrides();
  const enabled = parseBoolean(
    query.enabledOverride,
    parseBoolean(tracingConfig.enabled, false),
  );
  const sampleRatio = clampSampleRatio(
    query.sampleOverride,
    clampSampleRatio(tracingConfig.sampleRatio, 0.25),
  );
  return {
    enabled,
    debug: parseBoolean(tracingConfig.debug, false),
    serviceName: normalizeString(tracingConfig.serviceName) || "fishystuff-site",
    deploymentEnvironment:
      normalizeString(tracingConfig.deploymentEnvironment) || "unknown",
    serviceVersion: normalizeString(tracingConfig.serviceVersion),
    exporterEndpoint: normalizeString(tracingConfig.exporterEndpoint),
    apiBaseUrl: normalizeUrl(runtimeConfig.apiBaseUrl),
    cdnBaseUrl: normalizeUrl(runtimeConfig.cdnBaseUrl),
    jaegerUiUrl: normalizeUrl(tracingConfig.jaegerUiUrl),
    sampleRatio,
  };
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function buildIgnorePatterns(config) {
  const patterns = [];
  if (config.exporterEndpoint) {
    patterns.push(new RegExp(`^${escapeRegExp(config.exporterEndpoint)}(?:$|[?#])`));
  }
  if (config.cdnBaseUrl) {
    patterns.push(new RegExp(`^${escapeRegExp(config.cdnBaseUrl)}/`));
  }
  return patterns;
}

function buildPropagationTargets(config) {
  const targets = [];
  if (config.apiBaseUrl) {
    targets.push(config.apiBaseUrl);
  }
  return targets;
}

function installBrowserTelemetry(config) {
  if (!config.enabled || !config.exporterEndpoint) {
    globalThis[OTEL_GLOBAL_KEY] = Object.freeze({
      enabled: false,
      reason: config.exporterEndpoint ? "disabled" : "missing-exporter-endpoint",
      sampleRatio: config.sampleRatio,
      exporterEndpoint: config.exporterEndpoint,
      jaegerUiUrl: config.jaegerUiUrl,
    });
    return;
  }

  if (globalThis[OTEL_GLOBAL_KEY]?.initialized) {
    return;
  }

  if (config.debug) {
    diag.setLogger(new DiagConsoleLogger(), DiagLogLevel.INFO);
  }

  const exporter = new OTLPTraceExporter({
    url: config.exporterEndpoint,
    concurrencyLimit: 4,
    timeoutMillis: 4000,
  });

  const provider = new WebTracerProvider({
    resource: resourceFromAttributes({
      [ATTR_SERVICE_NAME]: config.serviceName,
      "deployment.environment": config.deploymentEnvironment,
      ...(config.serviceVersion ? { [ATTR_SERVICE_VERSION]: config.serviceVersion } : {}),
    }),
    sampler: new ParentBasedSampler({
      root: new TraceIdRatioBasedSampler(config.sampleRatio),
    }),
    spanProcessors: [
      new BatchSpanProcessor(exporter, {
        maxQueueSize: 128,
        maxExportBatchSize: 16,
        scheduledDelayMillis: 500,
        exportTimeoutMillis: 4000,
      }),
    ],
    spanLimits: {
      attributeCountLimit: 16,
      attributeValueLengthLimit: 256,
      eventCountLimit: 8,
      linkCountLimit: 4,
    },
  });
  provider.register();

  registerInstrumentations({
    instrumentations: [
      new FetchInstrumentation({
        clearTimingResources: true,
        ignoreUrls: buildIgnorePatterns(config),
        propagateTraceHeaderCorsUrls: buildPropagationTargets(config),
      }),
    ],
  });

  globalThis[OTEL_GLOBAL_KEY] = Object.freeze({
    initialized: true,
    enabled: true,
    serviceName: config.serviceName,
    deploymentEnvironment: config.deploymentEnvironment,
    serviceVersion: config.serviceVersion,
    sampleRatio: config.sampleRatio,
    exporterEndpoint: config.exporterEndpoint,
    jaegerUiUrl: config.jaegerUiUrl,
  });
}

installBrowserTelemetry(resolveRuntimeConfig());
