import {
  diag,
  DiagConsoleLogger,
  DiagLogLevel,
  metrics as otelMetrics,
  SpanStatusCode,
} from "@opentelemetry/api";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { registerInstrumentations } from "@opentelemetry/instrumentation";
import { FetchInstrumentation } from "@opentelemetry/instrumentation-fetch";
import { JsonMetricsSerializer } from "@opentelemetry/otlp-transformer";
import { resourceFromAttributes } from "@opentelemetry/resources";
import {
  AggregationTemporality,
  MeterProvider,
  PeriodicExportingMetricReader,
} from "@opentelemetry/sdk-metrics";
import { ParentBasedSampler, TraceIdRatioBasedSampler } from "@opentelemetry/sdk-trace-base";
import { BatchSpanProcessor, WebTracerProvider } from "@opentelemetry/sdk-trace-web";
import { ATTR_SERVICE_NAME, ATTR_SERVICE_VERSION } from "@opentelemetry/semantic-conventions";

const OTEL_GLOBAL_KEY = "__fishystuffOtel";
const OTEL_FLUSH_HOOK_KEY = "__fishystuffOtelFlushHooksInstalled";
const TRACE_QUERY_KEY = "trace";
const TRACE_SAMPLE_QUERY_KEY = "trace_sample";
const REQUEST_ID_HEADER = "x-request-id";
const TRACE_ID_HEADER = "x-trace-id";
const SPAN_ID_HEADER = "x-span-id";
const DEFAULT_METRIC_EXPORT_TIMEOUT_MS = 4000;

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

function parsePositiveInteger(value, fallback) {
  const numeric = Number.parseInt(String(value ?? "").trim(), 10);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return fallback;
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

function resolveAbsoluteUrl(value, baseUrl) {
  const normalized = normalizeString(value);
  if (!normalized) {
    return "";
  }
  try {
    return new URL(normalized, baseUrl || globalThis.location?.href || "http://localhost/").toString();
  } catch {
    return normalized;
  }
}

function resolveBaseUrl(value, baseUrl) {
  return normalizeUrl(resolveAbsoluteUrl(value, baseUrl));
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
  const metricsConfig = runtimeConfig.metrics || {};
  const query = readQueryOverrides();
  const siteBaseUrl =
    resolveBaseUrl(runtimeConfig.siteBaseUrl, globalThis.location?.href)
    || normalizeUrl(globalThis.location?.origin);
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
    siteBaseUrl,
    exporterEndpoint: resolveAbsoluteUrl(tracingConfig.exporterEndpoint, siteBaseUrl),
    apiBaseUrl: resolveBaseUrl(runtimeConfig.apiBaseUrl, siteBaseUrl),
    cdnBaseUrl: resolveBaseUrl(runtimeConfig.cdnBaseUrl, siteBaseUrl),
    jaegerUiUrl: resolveBaseUrl(tracingConfig.jaegerUiUrl, siteBaseUrl),
    metricsEnabled: parseBoolean(metricsConfig.enabled, enabled),
    metricsExporterEndpoint: resolveAbsoluteUrl(metricsConfig.exporterEndpoint, siteBaseUrl),
    metricsExportIntervalMs: parsePositiveInteger(metricsConfig.exportIntervalMs, 5000),
    sampleRatio,
  };
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function buildBaseUrlPrefixPattern(baseUrl) {
  const normalized = normalizeUrl(baseUrl);
  if (!normalized) {
    return null;
  }
  return new RegExp(`^${escapeRegExp(normalized)}(?:$|[/?#])`);
}

function buildIgnorePatterns(config) {
  const patterns = [];
  if (config.exporterEndpoint) {
    patterns.push(new RegExp(`^${escapeRegExp(config.exporterEndpoint)}(?:$|[?#])`));
  }
  if (config.metricsExporterEndpoint) {
    patterns.push(new RegExp(`^${escapeRegExp(config.metricsExporterEndpoint)}(?:$|[?#])`));
  }
  if (config.cdnBaseUrl) {
    patterns.push(new RegExp(`^${escapeRegExp(config.cdnBaseUrl)}/`));
  }
  return patterns;
}

function buildPropagationTargets(config) {
  const targets = [];
  const apiBasePattern = buildBaseUrlPrefixPattern(config.apiBaseUrl);
  if (apiBasePattern) {
    targets.push(apiBasePattern);
  }
  return targets;
}

function normalizeAttributes(attributes) {
  const source = attributes && typeof attributes === "object" ? attributes : {};
  const normalized = {};
  for (const [key, value] of Object.entries(source)) {
    const normalizedKey = normalizeString(key);
    if (!normalizedKey) {
      continue;
    }
    if (typeof value === "string") {
      const normalizedValue = normalizeString(value);
      if (normalizedValue) {
        normalized[normalizedKey] = normalizedValue;
      }
      continue;
    }
    if (typeof value === "number" && Number.isFinite(value)) {
      normalized[normalizedKey] = value;
      continue;
    }
    if (typeof value === "boolean") {
      normalized[normalizedKey] = value;
    }
  }
  return normalized;
}

function requestUrlFromFetchRequest(request) {
  if (typeof request === "string") {
    return request;
  }
  if (request instanceof URL) {
    return request.toString();
  }
  if (request && typeof request === "object") {
    if (typeof request.url === "string") {
      return request.url;
    }
    if (typeof request.href === "string") {
      return request.href;
    }
  }
  return "";
}

function classifyFetchTarget(request, config) {
  const requestUrl = resolveAbsoluteUrl(
    requestUrlFromFetchRequest(request),
    config.siteBaseUrl,
  );
  if (!requestUrl) {
    return "other";
  }
  const apiBasePattern = buildBaseUrlPrefixPattern(config.apiBaseUrl);
  if (apiBasePattern?.test(requestUrl)) {
    return "api";
  }
  const cdnBasePattern = buildBaseUrlPrefixPattern(config.cdnBaseUrl);
  if (cdnBasePattern?.test(requestUrl)) {
    return "cdn";
  }
  const siteBasePattern = buildBaseUrlPrefixPattern(config.siteBaseUrl);
  if (siteBasePattern?.test(requestUrl)) {
    return "site";
  }
  return "other";
}

function responseHeaderValue(result, name) {
  const headers = result && typeof result === "object" ? result.headers : null;
  if (!headers || typeof headers.get !== "function") {
    return "";
  }
  return normalizeString(headers.get(name));
}

function extractFishystuffResponseContext(result) {
  const rawStatusCode =
    result && typeof result === "object" ? Number.parseInt(String(result.status ?? ""), 10) : NaN;
  return {
    statusCode: Number.isFinite(rawStatusCode) ? rawStatusCode : null,
    requestId: responseHeaderValue(result, REQUEST_ID_HEADER),
    traceId: responseHeaderValue(result, TRACE_ID_HEADER),
    spanId: responseHeaderValue(result, SPAN_ID_HEADER),
  };
}

function applyFishystuffRequestAttributes(span, request, config) {
  if (!span || typeof span.setAttribute !== "function") {
    return;
  }
  span.setAttribute("fishystuff.request.target", classifyFetchTarget(request, config));
}

function applyFishystuffResponseAttributes(span, result) {
  if (!span || typeof span.setAttribute !== "function") {
    return;
  }
  const context = extractFishystuffResponseContext(result);
  if (context.requestId) {
    span.setAttribute("fishystuff.response.request_id", context.requestId);
  }
  if (context.traceId) {
    span.setAttribute("fishystuff.response.trace_id", context.traceId);
  }
  if (context.spanId) {
    span.setAttribute("fishystuff.response.span_id", context.spanId);
  }
}

function errorName(error) {
  const name =
    normalizeString(error?.name)
    || normalizeString(error?.constructor?.name)
    || typeof error;
  return name || "Error";
}

function errorMessage(error) {
  return normalizeString(error?.message) || normalizeString(error) || "operation failed";
}

function recordSpanError(span, error, attributes = {}) {
  if (!span) {
    return;
  }
  const message = errorMessage(error);
  if (typeof span.recordException === "function") {
    span.recordException(error instanceof Error ? error : new Error(message));
  }
  if (typeof span.setStatus === "function") {
    span.setStatus({
      code: SpanStatusCode.ERROR,
      message,
    });
  }
  if (typeof span.setAttribute === "function") {
    span.setAttribute("error.type", errorName(error));
    for (const [key, value] of Object.entries(normalizeAttributes(attributes))) {
      span.setAttribute(key, value);
    }
  }
}

function createHttpError(result, messagePrefix = "request failed") {
  const prefix = normalizeString(messagePrefix) || "request failed";
  const context = extractFishystuffResponseContext(result);
  const parts = [];
  if (context.statusCode != null) {
    parts.push(`HTTP ${context.statusCode}`);
  }
  if (context.requestId) {
    parts.push(`request_id=${context.requestId}`);
  }
  if (context.traceId) {
    parts.push(`trace_id=${context.traceId}`);
  }
  if (context.spanId) {
    parts.push(`span_id=${context.spanId}`);
  }
  const error = new Error(parts.length ? `${prefix} (${parts.join(" ")})` : prefix);
  error.statusCode = context.statusCode;
  error.requestId = context.requestId;
  error.traceId = context.traceId;
  error.spanId = context.spanId;
  return error;
}

function normalizeSpanInvocation(options, callback) {
  if (typeof options === "function") {
    return [{}, options];
  }
  return [options || {}, callback];
}

function installFlushHooks(providers) {
  const flushables = (Array.isArray(providers) ? providers : [providers]).filter(Boolean);
  if (!flushables.length || globalThis[OTEL_FLUSH_HOOK_KEY]) {
    return;
  }

  const flush = () => {
    for (const provider of flushables) {
      Promise.resolve(provider.forceFlush?.()).catch(() => {});
    }
  };
  globalThis.addEventListener?.("pagehide", flush);
  globalThis.document?.addEventListener?.("visibilitychange", () => {
    if (globalThis.document?.visibilityState === "hidden") {
      flush();
    }
  });
  globalThis[OTEL_FLUSH_HOOK_KEY] = true;
}

function createOtlpHttpMetricExporter({
  url,
  concurrencyLimit = 1,
  timeoutMillis = DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
} = {}) {
  let shutdown = false;
  const inFlight = new Set();

  function finish(resultCallback, result) {
    queueMicrotask(() => resultCallback(result));
  }

  return {
    export(metrics, resultCallback) {
      if (shutdown || !url || typeof globalThis.fetch !== "function") {
        finish(resultCallback, {
          code: 1,
          error: new Error("OTLP metric exporter is unavailable"),
        });
        return;
      }
      if (inFlight.size >= Math.max(1, concurrencyLimit)) {
        finish(resultCallback, {
          code: 1,
          error: new Error("OTLP metric exporter is busy"),
        });
        return;
      }

      const controller =
        typeof AbortController === "function" ? new AbortController() : null;
      const timeoutId =
        controller && timeoutMillis > 0
          ? globalThis.setTimeout(() => controller.abort(), timeoutMillis)
          : 0;
      const body = JsonMetricsSerializer.serializeRequest(metrics);
      const request = Promise.resolve(
        globalThis.fetch(url, {
          method: "POST",
          headers: {
            "content-type": "application/json",
          },
          body,
          keepalive: true,
          signal: controller?.signal,
        }),
      )
        .then((response) => {
          if (!response?.ok) {
            throw new Error(`OTLP metrics export failed with HTTP ${response?.status ?? "unknown"}`);
          }
          finish(resultCallback, { code: 0 });
        })
        .catch((error) => {
          finish(resultCallback, {
            code: 1,
            error: error instanceof Error ? error : new Error(String(error)),
          });
        })
        .finally(() => {
          if (timeoutId) {
            globalThis.clearTimeout(timeoutId);
          }
          inFlight.delete(request);
        });

      inFlight.add(request);
    },
    forceFlush() {
      return Promise.allSettled(Array.from(inFlight)).then(() => {});
    },
    selectAggregationTemporality() {
      return AggregationTemporality.CUMULATIVE;
    },
    shutdown() {
      shutdown = true;
      return Promise.allSettled(Array.from(inFlight)).then(() => {});
    },
  };
}

function createTelemetryBridge(config, tracerProvider, meterProvider) {
  const tracer = tracerProvider ? tracerProvider.getTracer(config.serviceName) : null;

  return Object.freeze({
    initialized: Boolean(tracerProvider || meterProvider),
    enabled: Boolean(tracerProvider || meterProvider),
    tracingEnabled: Boolean(tracerProvider),
    metricsEnabled: Boolean(meterProvider),
    serviceName: config.serviceName,
    deploymentEnvironment: config.deploymentEnvironment,
    serviceVersion: config.serviceVersion,
    sampleRatio: config.sampleRatio,
    exporterEndpoint: config.exporterEndpoint,
    metricsExporterEndpoint: config.metricsExporterEndpoint,
    jaegerUiUrl: config.jaegerUiUrl,
    getMeter(name, version, options) {
      if (!meterProvider) {
        return null;
      }
      return meterProvider.getMeter(
        normalizeString(name) || config.serviceName,
        normalizeString(version) || undefined,
        options,
      );
    },
    responseContext(result) {
      return extractFishystuffResponseContext(result);
    },
    httpError(result, messagePrefix) {
      return createHttpError(result, messagePrefix);
    },
    withSpan(name, options, callback) {
      const [spanOptions, spanCallback] = normalizeSpanInvocation(options, callback);
      if (typeof spanCallback !== "function") {
        return undefined;
      }
      if (!tracer) {
        return spanCallback(null);
      }
      const spanName = normalizeString(name) || "browser.operation";
      return tracer.startActiveSpan(
        spanName,
        {
          attributes: normalizeAttributes(spanOptions.attributes),
        },
        (span) => {
          try {
            return spanCallback(span);
          } catch (error) {
            recordSpanError(span, error, spanOptions.errorAttributes);
            throw error;
          } finally {
            span.end();
          }
        },
      );
    },
    async withSpanAsync(name, options, callback) {
      const [spanOptions, spanCallback] = normalizeSpanInvocation(options, callback);
      if (typeof spanCallback !== "function") {
        return undefined;
      }
      if (!tracer) {
        return spanCallback(null);
      }
      const spanName = normalizeString(name) || "browser.operation";
      return tracer.startActiveSpan(
        spanName,
        {
          attributes: normalizeAttributes(spanOptions.attributes),
        },
        async (span) => {
          try {
            return await spanCallback(span);
          } catch (error) {
            recordSpanError(span, error, spanOptions.errorAttributes);
            throw error;
          } finally {
            span.end();
          }
        },
      );
    },
    recordError(span, error, attributes) {
      recordSpanError(span, error, attributes);
      return error;
    },
  });
}

function installBrowserTelemetry(config) {
  const traceEnabled = config.enabled && Boolean(config.exporterEndpoint);
  const metricsEnabled =
    config.metricsEnabled && Boolean(config.metricsExporterEndpoint);
  const disabledBridge = createTelemetryBridge(config, null, null);
  if (!traceEnabled && !metricsEnabled) {
    globalThis[OTEL_GLOBAL_KEY] = Object.freeze({
      ...disabledBridge,
      reason:
        config.enabled || config.metricsEnabled
          ? "missing-exporter-endpoint"
          : "disabled",
    });
    return;
  }

  if (globalThis[OTEL_GLOBAL_KEY]?.initialized) {
    return;
  }

  if (config.debug) {
    diag.setLogger(new DiagConsoleLogger(), DiagLogLevel.INFO);
  }

  const resource = resourceFromAttributes({
    [ATTR_SERVICE_NAME]: config.serviceName,
    "deployment.environment": config.deploymentEnvironment,
    ...(config.serviceVersion ? { [ATTR_SERVICE_VERSION]: config.serviceVersion } : {}),
  });

  let tracerProvider = null;
  if (traceEnabled) {
    const traceExporter = new OTLPTraceExporter({
      url: config.exporterEndpoint,
      concurrencyLimit: 4,
      timeoutMillis: DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
    });

    tracerProvider = new WebTracerProvider({
      resource,
      sampler: new ParentBasedSampler({
        root: new TraceIdRatioBasedSampler(config.sampleRatio),
      }),
      spanProcessors: [
        new BatchSpanProcessor(traceExporter, {
          maxQueueSize: 128,
          maxExportBatchSize: 16,
          scheduledDelayMillis: 500,
          exportTimeoutMillis: DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
        }),
      ],
      spanLimits: {
        attributeCountLimit: 16,
        attributeValueLengthLimit: 256,
        eventCountLimit: 8,
        linkCountLimit: 4,
      },
    });
    tracerProvider.register();

    registerInstrumentations({
      instrumentations: [
        new FetchInstrumentation({
          clearTimingResources: true,
          ignoreUrls: buildIgnorePatterns(config),
          propagateTraceHeaderCorsUrls: buildPropagationTargets(config),
          requestHook(span, request) {
            applyFishystuffRequestAttributes(span, request, config);
          },
          applyCustomAttributesOnSpan(span, request, result) {
            applyFishystuffRequestAttributes(span, request, config);
            applyFishystuffResponseAttributes(span, result);
          },
        }),
      ],
    });
  }

  let meterProvider = null;
  if (metricsEnabled) {
    meterProvider = new MeterProvider({
      resource,
      readers: [
        new PeriodicExportingMetricReader({
          exporter: createOtlpHttpMetricExporter({
            url: config.metricsExporterEndpoint,
            timeoutMillis: DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
          }),
          exportIntervalMillis: config.metricsExportIntervalMs,
          exportTimeoutMillis: DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
        }),
      ],
    });
    otelMetrics.setGlobalMeterProvider(meterProvider);
  }

  installFlushHooks([tracerProvider, meterProvider]);

  globalThis[OTEL_GLOBAL_KEY] = createTelemetryBridge(
    config,
    tracerProvider,
    meterProvider,
  );
}

if (typeof document !== "undefined") {
  installBrowserTelemetry(resolveRuntimeConfig());
}

export {
  applyFishystuffRequestAttributes,
  applyFishystuffResponseAttributes,
  buildBaseUrlPrefixPattern,
  buildIgnorePatterns,
  buildPropagationTargets,
  classifyFetchTarget,
  createHttpError,
  extractFishystuffResponseContext,
  resolveAbsoluteUrl,
  resolveBaseUrl,
  resolveRuntimeConfig,
};
