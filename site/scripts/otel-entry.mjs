import {
  context as otelContext,
  diag,
  DiagConsoleLogger,
  DiagLogLevel,
  metrics as otelMetrics,
  SpanStatusCode,
} from "@opentelemetry/api";
import { logs as otelLogs, SeverityNumber } from "@opentelemetry/api-logs";
import { registerInstrumentations } from "@opentelemetry/instrumentation";
import { FetchInstrumentation } from "@opentelemetry/instrumentation-fetch";
import {
  ProtobufLogsSerializer,
  ProtobufMetricsSerializer,
  ProtobufTraceSerializer,
} from "@opentelemetry/otlp-transformer";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { BatchLogRecordProcessor, LoggerProvider } from "@opentelemetry/sdk-logs";
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
const OTEL_LOG_HOOK_KEY = "__fishystuffOtelLogHooksInstalled";
const TELEMETRY_STATUS_EVENT = "fishystuff:telemetry-status-change";
const TRACE_QUERY_KEY = "trace";
const TRACE_SAMPLE_QUERY_KEY = "trace_sample";
const REQUEST_ID_HEADER = "x-request-id";
const TRACE_ID_HEADER = "x-trace-id";
const SPAN_ID_HEADER = "x-span-id";
const DEFAULT_METRIC_EXPORT_TIMEOUT_MS = 4000;
const DEFAULT_LOG_EXPORT_DELAY_MS = 1000;
const DEFAULT_LOG_EXPORT_QUEUE_SIZE = 128;
const DEFAULT_LOG_EXPORT_BATCH_SIZE = 16;
const OTLP_PROTOBUF_CONTENT_TYPE = "application/x-protobuf";

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

function parseOptionalBoolean(value) {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized === "1" || normalized === "true" || normalized === "yes" || normalized === "on") {
    return true;
  }
  if (normalized === "0" || normalized === "false" || normalized === "no" || normalized === "off") {
    return false;
  }
  return null;
}

function normalizeTelemetryDefaultMode(value, fallback = "opt-in") {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (normalized === "enabled" || normalized === "opt-in" || normalized === "disabled") {
    return normalized;
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

function fallbackTelemetryState(defaultMode) {
  const normalizedDefaultMode = normalizeTelemetryDefaultMode(defaultMode, "opt-in");
  if (normalizedDefaultMode === "disabled") {
    return {
      defaultMode: normalizedDefaultMode,
      choice: "unset",
      effectiveEnabled: false,
      source: "runtime-policy",
      reason: "disabled-by-runtime-policy",
    };
  }
  if (normalizedDefaultMode === "enabled") {
    return {
      defaultMode: normalizedDefaultMode,
      choice: "unset",
      effectiveEnabled: true,
      source: "runtime-default",
      reason: "enabled-by-runtime-default",
    };
  }
  return {
    defaultMode: normalizedDefaultMode,
    choice: "unset",
    effectiveEnabled: false,
    source: "runtime-default",
    reason: "opt-in-required",
  };
}

function readClientSessionTelemetryState(defaultMode) {
  const fallback = fallbackTelemetryState(defaultMode);
  const helper = globalThis.__fishystuffClientSession;
  if (!helper || typeof helper.telemetryState !== "function") {
    return fallback;
  }
  try {
    const telemetry = helper.telemetryState();
    const continuous = telemetry && typeof telemetry === "object" ? telemetry.continuous : null;
    if (!continuous || typeof continuous !== "object") {
      return fallback;
    }
    return {
      defaultMode: normalizeTelemetryDefaultMode(continuous.defaultMode, fallback.defaultMode),
      choice: normalizeString(continuous.choice) || fallback.choice,
      effectiveEnabled: parseBoolean(continuous.effectiveEnabled, fallback.effectiveEnabled),
      source: normalizeString(continuous.source) || fallback.source,
      reason: normalizeString(continuous.reason) || fallback.reason,
    };
  } catch {
    return fallback;
  }
}

function resolveRuntimeConfig() {
  const runtimeConfig = globalThis.__fishystuffRuntimeConfig || {};
  const tracingConfig = runtimeConfig.tracing || {};
  const metricsConfig = runtimeConfig.metrics || {};
  const logsConfig = runtimeConfig.logs || {};
  const clientConfig = runtimeConfig.client || {};
  const query = readQueryOverrides();
  const siteBaseUrl =
    resolveBaseUrl(runtimeConfig.siteBaseUrl, globalThis.location?.href)
    || normalizeUrl(globalThis.location?.origin);
  const tracingConfiguredEnabled = parseBoolean(tracingConfig.enabled, false);
  const metricsConfiguredEnabled = parseBoolean(metricsConfig.enabled, tracingConfiguredEnabled);
  const logsConfiguredEnabled = parseBoolean(logsConfig.enabled, tracingConfiguredEnabled);
  const telemetryDefaultMode = normalizeTelemetryDefaultMode(
    clientConfig?.telemetry?.defaultMode,
    tracingConfiguredEnabled ? "enabled" : "opt-in",
  );
  const sessionTelemetryState = readClientSessionTelemetryState(telemetryDefaultMode);
  const enabledOverride = parseOptionalBoolean(query.enabledOverride);
  let telemetryEffectiveEnabled = parseBoolean(
    sessionTelemetryState.effectiveEnabled,
    false,
  );
  let telemetryReason =
    normalizeString(sessionTelemetryState.reason)
    || (telemetryEffectiveEnabled ? "enabled" : "disabled");
  if (enabledOverride === false && telemetryEffectiveEnabled) {
    telemetryEffectiveEnabled = false;
    telemetryReason = "disabled-by-query";
  }
  const enabled = telemetryEffectiveEnabled && tracingConfiguredEnabled;
  const metricsEnabled = telemetryEffectiveEnabled && metricsConfiguredEnabled;
  const logsEnabled = telemetryEffectiveEnabled && logsConfiguredEnabled;
  const sampleRatio = clampSampleRatio(
    query.sampleOverride,
    clampSampleRatio(tracingConfig.sampleRatio, 0.25),
  );
  return {
    telemetryDefaultMode,
    telemetryPreference: normalizeString(sessionTelemetryState.choice) || "unset",
    telemetryEffectiveEnabled,
    telemetryReason,
    telemetrySource: normalizeString(sessionTelemetryState.source) || "runtime-default",
    tracingConfiguredEnabled,
    metricsConfiguredEnabled,
    logsConfiguredEnabled,
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
    metricsEnabled,
    metricsExporterEndpoint: resolveAbsoluteUrl(metricsConfig.exporterEndpoint, siteBaseUrl),
    metricsExportIntervalMs: parsePositiveInteger(metricsConfig.exportIntervalMs, 5000),
    logsEnabled,
    logsExporterEndpoint: resolveAbsoluteUrl(logsConfig.exporterEndpoint, siteBaseUrl),
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
  if (config.logsExporterEndpoint) {
    patterns.push(new RegExp(`^${escapeRegExp(config.logsExporterEndpoint)}(?:$|[?#])`));
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

function truncateString(value, maxLength = 2048) {
  const normalized = normalizeString(value);
  if (!normalized || normalized.length <= maxLength) {
    return normalized;
  }
  if (maxLength <= 3) {
    return normalized.slice(0, maxLength);
  }
  return `${normalized.slice(0, maxLength - 3)}...`;
}

function normalizeLogBody(value, fallback = "") {
  if (value instanceof Error) {
    return truncateString(`${errorName(value)}: ${errorMessage(value)}`, 1024);
  }
  if (typeof value === "string") {
    return truncateString(value, 1024);
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (value == null) {
    return normalizeString(fallback);
  }
  try {
    return truncateString(JSON.stringify(value), 1024);
  } catch {
    return truncateString(String(value), 1024);
  }
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

function browserLocationAttributes() {
  return normalizeAttributes({
    "url.full": normalizeString(globalThis.location?.href),
    "url.path": normalizeString(globalThis.location?.pathname),
    "url.query": normalizeString(globalThis.location?.search),
  });
}

function currentPagePath(globalRef = globalThis) {
  return normalizeString(globalRef?.location?.pathname) || "/";
}

function safePerformanceNow(globalRef = globalThis) {
  const performanceRef = globalRef?.performance;
  if (!performanceRef || typeof performanceRef.now !== "function") {
    return null;
  }
  const value = performanceRef.now();
  return Number.isFinite(value) ? value : null;
}

function readNavigationTiming(globalRef = globalThis) {
  const performanceRef = globalRef?.performance;
  if (!performanceRef || typeof performanceRef.getEntriesByType !== "function") {
    return null;
  }
  const [entry] = performanceRef.getEntriesByType("navigation");
  return entry && typeof entry === "object" ? entry : null;
}

function currentPageReadyDurationMs(globalRef = globalThis) {
  const navigationTiming = readNavigationTiming(globalRef);
  const navigationCandidates = [
    navigationTiming?.loadEventEnd,
    navigationTiming?.domComplete,
    navigationTiming?.loadEventStart,
    navigationTiming?.domInteractive,
    navigationTiming?.duration,
  ];
  for (const candidate of navigationCandidates) {
    if (Number.isFinite(candidate) && candidate > 0) {
      return candidate;
    }
  }
  return safePerformanceNow(globalRef) ?? 0;
}

function createBrowserOperatorMetrics({ meter, globalRef = globalThis } = {}) {
  if (!meter) {
    return Object.freeze({
      enabled: false,
      recordFrontendError() {
        return false;
      },
      recordPageReady() {
        return false;
      },
      recordSessionStarted() {
        return false;
      },
    });
  }

  const sessionStarted = meter.createCounter("fishystuff.site.session_started", {
    description: "Browser sessions observed by the current site runtime.",
  });
  const pageReady = meter.createHistogram("fishystuff.site.page_ready", {
    description: "Navigation-to-page-ready duration observed in the browser runtime.",
    unit: "ms",
  });
  const frontendError = meter.createCounter("fishystuff.site.frontend_error", {
    description: "Frontend error signals observed by the browser telemetry hooks.",
  });
  let sessionStartedRecorded = false;
  let pageReadyRecorded = false;

  function defaultAttributes() {
    return normalizeAttributes({
      page_path: currentPagePath(globalRef),
    });
  }

  return Object.freeze({
    enabled: true,
    recordSessionStarted(attributes = {}) {
      if (sessionStartedRecorded) {
        return false;
      }
      sessionStarted.add(1, {
        ...defaultAttributes(),
        ...normalizeAttributes(attributes),
      });
      sessionStartedRecorded = true;
      return true;
    },
    recordPageReady(durationMs = currentPageReadyDurationMs(globalRef), attributes = {}) {
      if (pageReadyRecorded) {
        return false;
      }
      const numericDurationMs = Number(durationMs);
      if (!Number.isFinite(numericDurationMs) || numericDurationMs < 0) {
        return false;
      }
      pageReady.record(numericDurationMs, {
        ...defaultAttributes(),
        ...normalizeAttributes(attributes),
      });
      pageReadyRecorded = true;
      return true;
    },
    recordFrontendError(attributes = {}) {
      frontendError.add(1, {
        ...defaultAttributes(),
        ...normalizeAttributes(attributes),
      });
      return true;
    },
  });
}

function installPageReadyMetric(browserMetrics, globalRef = globalThis) {
  if (!browserMetrics?.enabled) {
    return;
  }

  const documentRef = globalRef?.document;
  const record = () => {
    browserMetrics.recordPageReady(currentPageReadyDurationMs(globalRef));
  };
  const scheduleRecord = () => {
    const setTimeoutRef = globalRef?.setTimeout;
    if (typeof setTimeoutRef === "function") {
      setTimeoutRef(record, 0);
      return;
    }
    record();
  };

  if (documentRef?.readyState === "complete") {
    scheduleRecord();
    return;
  }

  globalRef?.addEventListener?.("load", scheduleRecord, { once: true });
}

function extractErrorLogAttributes(error) {
  const attributes = {
    "error.type": errorName(error),
    "error.message": errorMessage(error),
    "error.stack": truncateString(error?.stack, 4096),
    "request.id": normalizeString(error?.requestId),
    "trace.id": normalizeString(error?.traceId),
    "span.id": normalizeString(error?.spanId),
  };
  const numericStatusCode = Number.parseInt(String(error?.statusCode ?? ""), 10);
  if (Number.isFinite(numericStatusCode)) {
    attributes["http.response.status_code"] = numericStatusCode;
  }
  return normalizeAttributes(attributes);
}

function firstErrorArgument(values) {
  if (!Array.isArray(values)) {
    return null;
  }
  for (const value of values) {
    if (value instanceof Error) {
      return value;
    }
  }
  return null;
}

function consoleArgsMessage(values, fallback) {
  const parts = (Array.isArray(values) ? values : [])
    .map((value) => normalizeLogBody(value))
    .filter(Boolean);
  return parts.join(" ").trim() || normalizeString(fallback) || "console message";
}

function emitLogRecord(logger, logRecord) {
  if (!logger || typeof logger.emit !== "function") {
    return false;
  }
  try {
    logger.emit(logRecord);
    return true;
  } catch {
    return false;
  }
}

function emitBrowserLog(loggerProvider, config, options = {}) {
  if (!loggerProvider || typeof loggerProvider.getLogger !== "function") {
    return false;
  }
  const loggerName =
    normalizeString(options.loggerName) || `${config.serviceName}.browser`;
  const logger = loggerProvider.getLogger(loggerName);
  const error = options.error instanceof Error ? options.error : null;
  const attributes = {
    "fishystuff.log.kind": "browser",
    "fishystuff.log.source": normalizeString(options.source) || "browser",
    ...browserLocationAttributes(),
    ...normalizeAttributes(options.attributes),
    ...(error ? extractErrorLogAttributes(error) : {}),
  };
  const body =
    normalizeLogBody(
      options.body,
      error ? errorMessage(error) : "browser log",
    ) || "browser log";
  return emitLogRecord(logger, {
    eventName: normalizeString(options.eventName) || undefined,
    severityNumber: options.severityNumber ?? SeverityNumber.INFO,
    severityText: normalizeString(options.severityText) || undefined,
    body,
    attributes,
    context: options.context || otelContext.active(),
  });
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

function dispatchTelemetryStatusChange(bridge, reason) {
  if (typeof globalThis.dispatchEvent !== "function") {
    return;
  }
  try {
    globalThis.dispatchEvent(new CustomEvent(TELEMETRY_STATUS_EVENT, {
      detail: {
        reason: normalizeString(reason) || "update",
        status: typeof bridge?.status === "function" ? bridge.status() : null,
      },
    }));
  } catch {
  }
}

function createOtlpHttpExporter({
  url,
  serializer,
  unavailableMessage,
  busyMessage,
  failureLabel,
  concurrencyLimit = 1,
  timeoutMillis = DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
} = {}) {
  let shutdown = false;
  const inFlight = new Set();
  let lastExportResult = null;

  function finish(resultCallback, result) {
    lastExportResult = {
      ok: result?.code === 0,
      code: result?.code,
      error: result?.error instanceof Error ? result.error.message : normalizeString(result?.error),
      at: new Date().toISOString(),
    };
    queueMicrotask(() => resultCallback(result));
  }

  return {
    export(items, resultCallback) {
      if (shutdown || !url || typeof globalThis.fetch !== "function") {
        finish(resultCallback, {
          code: 1,
          error: new Error(unavailableMessage || "OTLP exporter is unavailable"),
        });
        return;
      }
      if (inFlight.size >= Math.max(1, concurrencyLimit)) {
        finish(resultCallback, {
          code: 1,
          error: new Error(busyMessage || "OTLP exporter is busy"),
        });
        return;
      }

      const controller =
        typeof AbortController === "function" ? new AbortController() : null;
      const timeoutId =
        controller && timeoutMillis > 0
          ? globalThis.setTimeout(() => controller.abort(), timeoutMillis)
          : 0;
      const body = serializer.serializeRequest(items);
      const request = Promise.resolve(
        globalThis.fetch(url, {
          method: "POST",
          headers: {
            "content-type": OTLP_PROTOBUF_CONTENT_TYPE,
          },
          body,
          keepalive: true,
          signal: controller?.signal,
        }),
      )
        .then((response) => {
          if (!response?.ok) {
            throw new Error(
              `${failureLabel || "OTLP export"} failed with HTTP ${response?.status ?? "unknown"}`,
            );
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
      if (!inFlight.size) {
        return Promise.resolve(lastExportResult?.ok !== false);
      }
      return Promise.allSettled(Array.from(inFlight)).then(
        () => lastExportResult?.ok !== false,
        () => false,
      );
    },
    lastExportResult() {
      return lastExportResult ? { ...lastExportResult } : null;
    },
    shutdown() {
      shutdown = true;
      return this.forceFlush();
    },
  };
}

function createOtlpHttpMetricExporter({
  url,
  concurrencyLimit = 1,
  timeoutMillis = DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
} = {}) {
  const exporter = createOtlpHttpExporter({
    url,
    serializer: ProtobufMetricsSerializer,
    unavailableMessage: "OTLP metric exporter is unavailable",
    busyMessage: "OTLP metric exporter is busy",
    failureLabel: "OTLP metrics export",
    concurrencyLimit,
    timeoutMillis,
  });

  return {
    export: exporter.export,
    forceFlush() {
      return exporter.forceFlush();
    },
    lastExportResult() {
      return exporter.lastExportResult();
    },
    selectAggregationTemporality() {
      return AggregationTemporality.CUMULATIVE;
    },
    shutdown() {
      return exporter.shutdown();
    },
  };
}

function createOtlpHttpTraceExporter({
  url,
  concurrencyLimit = 1,
  timeoutMillis = DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
} = {}) {
  return createOtlpHttpExporter({
    url,
    serializer: ProtobufTraceSerializer,
    unavailableMessage: "OTLP trace exporter is unavailable",
    busyMessage: "OTLP trace exporter is busy",
    failureLabel: "OTLP traces export",
    concurrencyLimit,
    timeoutMillis,
  });
}

function createOtlpHttpLogExporter({
  url,
  concurrencyLimit = 1,
  timeoutMillis = DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
} = {}) {
  return createOtlpHttpExporter({
    url,
    serializer: ProtobufLogsSerializer,
    unavailableMessage: "OTLP log exporter is unavailable",
    busyMessage: "OTLP log exporter is busy",
    failureLabel: "OTLP logs export",
    concurrencyLimit,
    timeoutMillis,
  });
}

function installBrowserLogHooks(loggerProvider, config, browserMetrics) {
  if (!loggerProvider || globalThis[OTEL_LOG_HOOK_KEY]) {
    return;
  }

  const runtimeLoggerName = `${config.serviceName}.browser.runtime`;
  const consoleLoggerName = `${config.serviceName}.browser.console`;
  const consoleRef = globalThis.console;
  const originalWarn =
    consoleRef && typeof consoleRef.warn === "function"
      ? consoleRef.warn.bind(consoleRef)
      : null;
  const originalError =
    consoleRef && typeof consoleRef.error === "function"
      ? consoleRef.error.bind(consoleRef)
      : null;

  globalThis.addEventListener?.("error", (event) => {
    const body = normalizeString(event?.message) || "Unhandled browser error";
    const error =
      event?.error instanceof Error ? event.error : new Error(body);
    browserMetrics?.recordFrontendError({
      source: "window.error",
    });
    emitBrowserLog(loggerProvider, config, {
      loggerName: runtimeLoggerName,
      source: "window.error",
      eventName: "window.error",
      severityNumber: SeverityNumber.ERROR,
      severityText: "ERROR",
      body,
      error,
      attributes: {
        "code.filepath": normalizeString(event?.filename),
        "code.lineno": Number.isFinite(event?.lineno) ? event.lineno : undefined,
        "code.colno": Number.isFinite(event?.colno) ? event.colno : undefined,
      },
    });
  });

  globalThis.addEventListener?.("unhandledrejection", (event) => {
    const reason = event?.reason;
    browserMetrics?.recordFrontendError({
      source: "window.unhandledrejection",
    });
    emitBrowserLog(loggerProvider, config, {
      loggerName: runtimeLoggerName,
      source: "window.unhandledrejection",
      eventName: "window.unhandledrejection",
      severityNumber: SeverityNumber.ERROR,
      severityText: "ERROR",
      body: normalizeLogBody(reason, "Unhandled promise rejection"),
      error: reason instanceof Error ? reason : null,
      attributes: {
        "fishystuff.rejection.kind":
          reason instanceof Error ? "error" : typeof reason,
      },
    });
  });

  if (consoleRef) {
    if (originalWarn) {
      consoleRef.warn = (...args) => {
        emitBrowserLog(loggerProvider, config, {
          loggerName: consoleLoggerName,
          source: "console.warn",
          eventName: "console.warn",
          severityNumber: SeverityNumber.WARN,
          severityText: "WARN",
          body: consoleArgsMessage(args, "console warn"),
          error: firstErrorArgument(args),
          attributes: {
            "fishystuff.console.arg_count": args.length,
          },
        });
        return originalWarn(...args);
      };
    }
    if (originalError) {
      consoleRef.error = (...args) => {
        browserMetrics?.recordFrontendError({
          source: "console.error",
        });
        emitBrowserLog(loggerProvider, config, {
          loggerName: consoleLoggerName,
          source: "console.error",
          eventName: "console.error",
          severityNumber: SeverityNumber.ERROR,
          severityText: "ERROR",
          body: consoleArgsMessage(args, "console error"),
          error: firstErrorArgument(args),
          attributes: {
            "fishystuff.console.arg_count": args.length,
          },
        });
        return originalError(...args);
      };
    }
  }

  globalThis[OTEL_LOG_HOOK_KEY] = true;
}

function createTelemetryBridge(
  config,
  tracerProvider,
  meterProvider,
  loggerProvider,
  browserMetrics = null,
  logExporter = null,
) {
  const tracer = tracerProvider ? tracerProvider.getTracer(config.serviceName) : null;
  const diagnosticReportsEnabled = Boolean(
    loggerProvider
    && config.telemetryEffectiveEnabled === true
    && config.logsEnabled === true,
  );

  function telemetryStatus() {
    let diagnosticReason = "ready";
    if (config.telemetryDefaultMode === "disabled") {
      diagnosticReason = "disabled-by-runtime-policy";
    } else if (config.telemetryEffectiveEnabled !== true) {
      diagnosticReason = config.telemetryReason || "automatic-telemetry-disabled";
    } else if (config.logsConfiguredEnabled !== true) {
      diagnosticReason = "logs-disabled";
    } else if (!config.logsExporterEndpoint) {
      diagnosticReason = "missing-logs-exporter";
    } else if (!loggerProvider) {
      diagnosticReason = "logs-unavailable";
    }
    const lastLogExport = logExporter?.lastExportResult?.() || null;
    return {
      initialized: Boolean(tracerProvider || meterProvider || loggerProvider),
      automaticTelemetryActive: config.telemetryEffectiveEnabled === true,
      tracingEnabled: Boolean(tracerProvider),
      metricsEnabled: Boolean(meterProvider),
      loggingEnabled: Boolean(loggerProvider),
      diagnosticReportsEnabled,
      reason: diagnosticReportsEnabled ? "ready" : diagnosticReason,
      telemetryDefaultMode: config.telemetryDefaultMode,
      telemetryPreference: config.telemetryPreference,
      telemetryReason: config.telemetryReason,
      telemetrySource: config.telemetrySource,
      logsExporterEndpoint: config.logsExporterEndpoint,
      lastLogExport,
    };
  }

  function diagnosticReportAttributes(report, extraAttributes = {}) {
    const issues = Array.isArray(report?.health?.issues) ? report.health.issues : [];
    const mode = normalizeString(report?.report?.mode) || "manual";
    return {
      "fishystuff.health.report_id": normalizeString(report?.report?.id),
      "fishystuff.health.issue_count": issues.length,
      "fishystuff.health.issue_ids": issues.map((issue) => normalizeString(issue?.id)).filter(Boolean).join(","),
      "fishystuff.health.issue_sources": Array.from(
        new Set(issues.map((issue) => normalizeString(issue?.source)).filter(Boolean)),
      ).join(","),
      "fishystuff.health.report_mode": mode,
      "fishystuff.health.manual_report": mode === "manual",
      "fishystuff.health.automatic_report": mode === "automatic",
      ...normalizeAttributes(extraAttributes),
    };
  }

  function diagnosticReportSeverity(report) {
    const issues = Array.isArray(report?.health?.issues) ? report.health.issues : [];
    return issues.some((issue) => normalizeString(issue?.severity).toLowerCase() === "error")
      ? { severityNumber: SeverityNumber.ERROR, severityText: "ERROR" }
      : { severityNumber: SeverityNumber.WARN, severityText: "WARN" };
  }

  return Object.freeze({
    initialized: Boolean(tracerProvider || meterProvider || loggerProvider),
    enabled: Boolean(tracerProvider || meterProvider || loggerProvider),
    tracingEnabled: Boolean(tracerProvider),
    metricsEnabled: Boolean(meterProvider),
    loggingEnabled: Boolean(loggerProvider),
    automaticLoggingEnabled: config.logsEnabled === true,
    diagnosticReportsEnabled,
    manualDiagnosticReportsEnabled: diagnosticReportsEnabled,
    telemetryDefaultMode: config.telemetryDefaultMode,
    telemetryPreference: config.telemetryPreference,
    telemetryEffectiveEnabled: config.telemetryEffectiveEnabled,
    telemetryReason: config.telemetryReason,
    telemetrySource: config.telemetrySource,
    serviceName: config.serviceName,
    deploymentEnvironment: config.deploymentEnvironment,
    serviceVersion: config.serviceVersion,
    sampleRatio: config.sampleRatio,
    exporterEndpoint: config.exporterEndpoint,
    metricsExporterEndpoint: config.metricsExporterEndpoint,
    logsExporterEndpoint: config.logsExporterEndpoint,
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
    getLogger(name, version, options) {
      if (!loggerProvider) {
        return null;
      }
      return loggerProvider.getLogger(
        normalizeString(name) || `${config.serviceName}.browser`,
        normalizeString(version) || undefined,
        options,
      );
    },
    emitLog(options) {
      return emitBrowserLog(loggerProvider, config, {
        loggerName: `${config.serviceName}.browser.app`,
        source: "bridge.emitLog",
        ...(options && typeof options === "object" ? options : {}),
      });
    },
    emitError(error, attributes = {}, options = {}) {
      browserMetrics?.recordFrontendError({
        source: "bridge.emitError",
      });
      return emitBrowserLog(loggerProvider, config, {
        loggerName: `${config.serviceName}.browser.app`,
        source: "bridge.emitError",
        eventName: "bridge.emitError",
        severityNumber: SeverityNumber.ERROR,
        severityText: "ERROR",
        body: errorMessage(error),
        error: error instanceof Error ? error : new Error(errorMessage(error)),
        attributes,
        ...(options && typeof options === "object" ? options : {}),
      });
    },
    async emitDiagnosticReport(report, options = {}) {
      const status = telemetryStatus();
      if (!status.diagnosticReportsEnabled) {
        return { sent: false, reason: status.reason || "telemetry-unavailable", status };
      }
      const mode = normalizeString(options.mode || report?.report?.mode) || "manual";
      const emitted = emitBrowserLog(loggerProvider, config, {
        loggerName: `${config.serviceName}.browser.app`,
        source: mode === "automatic"
          ? "page-health.automatic-report"
          : "page-health.manual-report",
        eventName: mode === "automatic"
          ? "fishystuff.page_health.automatic_report"
          : "fishystuff.page_health.manual_report",
        body: JSON.stringify(report || {}),
        attributes: diagnosticReportAttributes(
          {
            ...(report || {}),
            report: {
              ...(report?.report || {}),
              mode,
            },
          },
          options.attributes,
        ),
        ...diagnosticReportSeverity(report),
      });
      if (!emitted) {
        return { sent: false, reason: "telemetry-unavailable", status: telemetryStatus() };
      }
      const flushed = await this.forceFlushLogs();
      const nextStatus = telemetryStatus();
      if (!flushed) {
        dispatchTelemetryStatusChange(this, "diagnostic-report-failed");
        return { sent: false, reason: "telemetry-export-failed", status: nextStatus };
      }
      dispatchTelemetryStatusChange(this, "diagnostic-report-sent");
      return { sent: true, reason: "sent", status: nextStatus };
    },
    forceFlush() {
      const flushables = [tracerProvider, meterProvider, loggerProvider].filter(Boolean);
      if (!flushables.length) {
        return Promise.resolve(false);
      }
      return Promise.allSettled(
        flushables.map((provider) => Promise.resolve(provider.forceFlush?.())),
      ).then((results) => {
        const providersFlushed = results.every((result) => result.status === "fulfilled");
        const logResult = logExporter?.lastExportResult?.();
        return providersFlushed && logResult?.ok !== false;
      });
    },
    forceFlushLogs() {
      if (!loggerProvider || typeof loggerProvider.forceFlush !== "function") {
        return Promise.resolve(false);
      }
      return Promise.resolve(loggerProvider.forceFlush())
        .then(() => {
          const logResult = logExporter?.lastExportResult?.();
          return logResult?.ok !== false;
        })
        .catch(() => false);
    },
    status: telemetryStatus,
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
  const traceRequested = config.telemetryEffectiveEnabled && config.tracingConfiguredEnabled;
  const metricsRequested =
    config.telemetryEffectiveEnabled && config.metricsConfiguredEnabled;
  const logsRequested = config.telemetryEffectiveEnabled && config.logsConfiguredEnabled;
  const traceEnabled = traceRequested && Boolean(config.exporterEndpoint);
  const metricsEnabled =
    metricsRequested && Boolean(config.metricsExporterEndpoint);
  const logsEnabled =
    logsRequested && Boolean(config.logsExporterEndpoint);
  const disabledBridge = createTelemetryBridge(config, null, null, null);
  if (!traceEnabled && !metricsEnabled && !logsEnabled) {
    const missingExporterEndpoint =
      (traceRequested && !config.exporterEndpoint)
      || (metricsRequested && !config.metricsExporterEndpoint)
      || (logsRequested && !config.logsExporterEndpoint);
    const bridge = Object.freeze({
      ...disabledBridge,
      reason: missingExporterEndpoint
        ? "missing-exporter-endpoint"
        : config.telemetryReason || "disabled",
    });
    globalThis[OTEL_GLOBAL_KEY] = bridge;
    dispatchTelemetryStatusChange(bridge, "initialized-disabled");
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
  let loggerProvider = null;
  let logExporter = null;
  if (traceEnabled) {
    const traceExporter = createOtlpHttpTraceExporter({
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
  }

  if (logsEnabled) {
    logExporter = createOtlpHttpLogExporter({
      url: config.logsExporterEndpoint,
      timeoutMillis: DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
    });
    loggerProvider = new LoggerProvider({
      resource,
      processors: [
        new BatchLogRecordProcessor(
          logExporter,
          {
            maxQueueSize: DEFAULT_LOG_EXPORT_QUEUE_SIZE,
            maxExportBatchSize: DEFAULT_LOG_EXPORT_BATCH_SIZE,
            scheduledDelayMillis: DEFAULT_LOG_EXPORT_DELAY_MS,
            exportTimeoutMillis: DEFAULT_METRIC_EXPORT_TIMEOUT_MS,
          },
        ),
      ],
    });
    otelLogs.setGlobalLoggerProvider(loggerProvider);
  }

  let meterProvider = null;
  let browserMetrics = null;
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
    browserMetrics = createBrowserOperatorMetrics({
      meter: meterProvider.getMeter(`${config.serviceName}.browser`),
    });
    browserMetrics.recordSessionStarted();
    installPageReadyMetric(browserMetrics);
  }

  if (logsEnabled) {
    installBrowserLogHooks(loggerProvider, config, browserMetrics);
  }

  if (traceEnabled) {
    registerInstrumentations({
      loggerProvider,
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

  installFlushHooks([tracerProvider, meterProvider, loggerProvider]);

  globalThis[OTEL_GLOBAL_KEY] = createTelemetryBridge(
    config,
    tracerProvider,
    meterProvider,
    loggerProvider,
    browserMetrics,
    logExporter,
  );
  dispatchTelemetryStatusChange(globalThis[OTEL_GLOBAL_KEY], "initialized");
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
  createBrowserOperatorMetrics,
  createHttpError,
  createOtlpHttpLogExporter,
  createTelemetryBridge,
  currentPageReadyDurationMs,
  extractFishystuffResponseContext,
  resolveAbsoluteUrl,
  resolveBaseUrl,
  resolveRuntimeConfig,
};
