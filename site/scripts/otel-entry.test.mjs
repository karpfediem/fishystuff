import { expect, test } from "bun:test";

import {
  applyFishystuffResponseAttributes,
  buildBaseUrlPrefixPattern,
  buildIgnorePatterns,
  buildPropagationTargets,
  classifyFetchTarget,
  createBrowserOperatorMetrics,
  createHttpError,
  createOtlpHttpLogExporter,
  currentPageReadyDurationMs,
  extractFishystuffResponseContext,
  resolveRuntimeConfig,
} from "./otel-entry.mjs";

test("buildBaseUrlPrefixPattern matches URLs rooted under the configured base", () => {
  const pattern = buildBaseUrlPrefixPattern("http://localhost:8080");

  expect(pattern).toBeInstanceOf(RegExp);
  expect(pattern.test("http://localhost:8080")).toBe(true);
  expect(pattern.test("http://localhost:8080/api/v1/meta")).toBe(true);
  expect(pattern.test("http://localhost:8080?foo=bar")).toBe(true);
  expect(pattern.test("http://localhost:1990/api/v1/meta")).toBe(false);
  expect(pattern.test("http://localhost:80801/api/v1/meta")).toBe(false);
});

test("buildPropagationTargets uses prefix patterns so cross-origin API paths still propagate", () => {
  const targets = buildPropagationTargets({
    apiBaseUrl: "http://localhost:8080",
  });

  expect(targets).toHaveLength(1);
  expect(targets[0]).toBeInstanceOf(RegExp);
  expect(targets[0].test("http://localhost:8080/api/v1/zone_stats")).toBe(true);
  expect(targets[0].test("http://localhost:8080/api/v1/events_snapshot_meta")).toBe(true);
  expect(targets[0].test("http://127.0.0.1:8080/api/v1/zone_stats")).toBe(false);
});

test("buildIgnorePatterns keeps ignoring the exporter endpoint and CDN prefix", () => {
  const patterns = buildIgnorePatterns({
    exporterEndpoint: "https://telemetry.beta.fishystuff.fish/v1/traces",
    metricsExporterEndpoint: "https://telemetry.beta.fishystuff.fish/v1/metrics",
    logsExporterEndpoint: "https://telemetry.beta.fishystuff.fish/v1/logs",
    cdnBaseUrl: "http://localhost:4040",
  });

  expect(patterns).toHaveLength(4);
  expect(patterns[0].test("https://telemetry.beta.fishystuff.fish/v1/traces")).toBe(true);
  expect(patterns[0].test("https://telemetry.beta.fishystuff.fish/v1/traces?x=1")).toBe(true);
  expect(patterns[1].test("https://telemetry.beta.fishystuff.fish/v1/metrics")).toBe(true);
  expect(patterns[2].test("https://telemetry.beta.fishystuff.fish/v1/logs")).toBe(true);
  expect(patterns[3].test("http://localhost:4040/map/runtime-manifest.json")).toBe(true);
  expect(patterns[3].test("http://localhost:8080/api/v1/meta")).toBe(false);
});

test("classifyFetchTarget consistently labels API, site, and CDN requests", () => {
  const config = {
    apiBaseUrl: "https://api.beta.fishystuff.fish",
    cdnBaseUrl: "https://cdn.beta.fishystuff.fish",
    siteBaseUrl: "https://beta.fishystuff.fish",
  };

  expect(classifyFetchTarget("https://api.beta.fishystuff.fish/api/v1/meta", config)).toBe("api");
  expect(classifyFetchTarget("https://cdn.beta.fishystuff.fish/map/runtime-manifest.json", config)).toBe("cdn");
  expect(classifyFetchTarget("https://beta.fishystuff.fish/fishydex/", config)).toBe("site");
  expect(classifyFetchTarget("https://example.com/elsewhere", config)).toBe("other");
});

test("extractFishystuffResponseContext reads request and trace identifiers from headers", () => {
  const context = extractFishystuffResponseContext({
    status: 503,
    headers: new Headers({
      "x-request-id": "req-123",
      "x-trace-id": "trace-abc",
      "x-span-id": "span-def",
    }),
  });

  expect(context).toEqual({
    statusCode: 503,
    requestId: "req-123",
    traceId: "trace-abc",
    spanId: "span-def",
  });
});

test("applyFishystuffResponseAttributes adds correlation identifiers to spans", () => {
  const attributes = {};
  const span = {
    setAttribute(key, value) {
      attributes[key] = value;
    },
  };

  applyFishystuffResponseAttributes(span, {
    headers: new Headers({
      "x-request-id": "req-123",
      "x-trace-id": "trace-abc",
      "x-span-id": "span-def",
    }),
  });

  expect(attributes).toEqual({
    "fishystuff.response.request_id": "req-123",
    "fishystuff.response.trace_id": "trace-abc",
    "fishystuff.response.span_id": "span-def",
  });
});

test("createHttpError carries status and trace context into the thrown message", () => {
  const error = createHttpError(
    {
      status: 500,
      headers: new Headers({
        "x-request-id": "req-123",
        "x-trace-id": "trace-abc",
      }),
    },
    "best spots request failed",
  );

  expect(error.message).toBe(
    "best spots request failed (HTTP 500 request_id=req-123 trace_id=trace-abc)",
  );
  expect(error.statusCode).toBe(500);
  expect(error.requestId).toBe("req-123");
  expect(error.traceId).toBe("trace-abc");
  expect(error.spanId).toBe("");
});

test("resolveRuntimeConfig keeps browser metrics and logs separate from trace export config", () => {
  globalThis.location = new URL("http://127.0.0.1:1990/map/");
  globalThis.__fishystuffRuntimeConfig = {
    client: {
      telemetry: {
        defaultMode: "enabled",
      },
    },
    siteBaseUrl: "http://127.0.0.1:1990",
    tracing: {
      enabled: true,
      exporterEndpoint: "http://telemetry.localhost:1990/v1/traces",
      serviceName: "fishystuff-site-local",
      deploymentEnvironment: "local",
      serviceVersion: "dev",
      sampleRatio: 0.25,
    },
    metrics: {
      enabled: true,
      exporterEndpoint: "http://telemetry.localhost:1990/v1/metrics",
      exportIntervalMs: 3000,
    },
    logs: {
      enabled: true,
      exporterEndpoint: "http://telemetry.localhost:1990/v1/logs",
    },
  };

  const config = resolveRuntimeConfig();

  expect(config.exporterEndpoint).toBe("http://telemetry.localhost:1990/v1/traces");
  expect(config.metricsExporterEndpoint).toBe("http://telemetry.localhost:1990/v1/metrics");
  expect(config.metricsExportIntervalMs).toBe(3000);
  expect(config.logsExporterEndpoint).toBe("http://telemetry.localhost:1990/v1/logs");
  expect(config.logsEnabled).toBe(true);
  expect(config.telemetryDefaultMode).toBe("enabled");
  expect(config.telemetryEffectiveEnabled).toBe(true);

  delete globalThis.__fishystuffRuntimeConfig;
  delete globalThis.location;
});

test("resolveRuntimeConfig requires explicit opt-in when the runtime defaults to opt-in", () => {
  globalThis.location = new URL("https://fishystuff.fish/map/");
  globalThis.__fishystuffRuntimeConfig = {
    client: {
      telemetry: {
        defaultMode: "opt-in",
      },
    },
    tracing: {
      enabled: true,
      exporterEndpoint: "https://telemetry.fishystuff.fish/v1/traces",
    },
    metrics: {
      enabled: true,
      exporterEndpoint: "https://telemetry.fishystuff.fish/v1/metrics",
    },
    logs: {
      enabled: true,
      exporterEndpoint: "https://telemetry.fishystuff.fish/v1/logs",
    },
  };

  const config = resolveRuntimeConfig();

  expect(config.telemetryEffectiveEnabled).toBe(false);
  expect(config.telemetryReason).toBe("opt-in-required");
  expect(config.enabled).toBe(false);
  expect(config.metricsEnabled).toBe(false);
  expect(config.logsEnabled).toBe(false);

  delete globalThis.__fishystuffRuntimeConfig;
  delete globalThis.location;
});

test("resolveRuntimeConfig honors client-session consent and ignores opt-in bypass query attempts", () => {
  globalThis.location = new URL("https://fishystuff.fish/map/?trace=true");
  globalThis.__fishystuffRuntimeConfig = {
    client: {
      telemetry: {
        defaultMode: "opt-in",
      },
    },
    tracing: {
      enabled: true,
      exporterEndpoint: "https://telemetry.fishystuff.fish/v1/traces",
    },
  };
  globalThis.__fishystuffClientSession = {
    telemetryState() {
      return {
        continuous: {
          defaultMode: "opt-in",
          choice: "disabled",
          effectiveEnabled: false,
          source: "user",
          reason: "disabled-by-user",
        },
      };
    },
  };

  const config = resolveRuntimeConfig();

  expect(config.telemetryEffectiveEnabled).toBe(false);
  expect(config.telemetryReason).toBe("disabled-by-user");
  expect(config.enabled).toBe(false);

  delete globalThis.__fishystuffClientSession;
  delete globalThis.__fishystuffRuntimeConfig;
  delete globalThis.location;
});

test("resolveRuntimeConfig allows query-based telemetry suppression without bypassing consent", () => {
  globalThis.location = new URL("http://127.0.0.1:1990/map/?trace=0");
  globalThis.__fishystuffRuntimeConfig = {
    client: {
      telemetry: {
        defaultMode: "enabled",
      },
    },
    tracing: {
      enabled: true,
      exporterEndpoint: "http://127.0.0.1:4821/v1/traces",
    },
    metrics: {
      enabled: true,
      exporterEndpoint: "http://127.0.0.1:4821/v1/metrics",
    },
    logs: {
      enabled: true,
      exporterEndpoint: "http://127.0.0.1:4820/v1/logs",
    },
  };

  const config = resolveRuntimeConfig();

  expect(config.telemetryEffectiveEnabled).toBe(false);
  expect(config.telemetryReason).toBe("disabled-by-query");
  expect(config.enabled).toBe(false);
  expect(config.metricsEnabled).toBe(false);
  expect(config.logsEnabled).toBe(false);

  delete globalThis.__fishystuffRuntimeConfig;
  delete globalThis.location;
});

test("currentPageReadyDurationMs prefers completed navigation timing data", () => {
  globalThis.performance = {
    getEntriesByType(type) {
      if (type !== "navigation") {
        return [];
      }
      return [
        {
          loadEventEnd: 912.4,
          domComplete: 801.2,
        },
      ];
    },
    now() {
      return 999.9;
    },
  };

  expect(currentPageReadyDurationMs()).toBe(912.4);

  delete globalThis.performance;
});

test("currentPageReadyDurationMs skips zero-valued navigation timing fields", () => {
  globalThis.performance = {
    getEntriesByType(type) {
      if (type !== "navigation") {
        return [];
      }
      return [
        {
          loadEventEnd: 0,
          domComplete: 0,
          loadEventStart: 0,
          domInteractive: 412.8,
          duration: 500.1,
        },
      ];
    },
    now() {
      return 999.9;
    },
  };

  expect(currentPageReadyDurationMs()).toBe(412.8);

  delete globalThis.performance;
});

test("createBrowserOperatorMetrics records session, readiness, and frontend error counters once", () => {
  const counters = [];
  const histograms = [];
  const meter = {
    createCounter(name) {
      return {
        add(value, attributes) {
          counters.push({ name, value, attributes });
        },
      };
    },
    createHistogram(name) {
      return {
        record(value, attributes) {
          histograms.push({ name, value, attributes });
        },
      };
    },
  };

  const metrics = createBrowserOperatorMetrics({
    meter,
    globalRef: {
      location: {
        pathname: "/map/",
      },
    },
  });

  expect(metrics.enabled).toBe(true);
  expect(metrics.recordSessionStarted()).toBe(true);
  expect(metrics.recordSessionStarted()).toBe(false);
  expect(metrics.recordPageReady(432.1)).toBe(true);
  expect(metrics.recordPageReady(123.4)).toBe(false);
  expect(metrics.recordFrontendError({ source: "window.error" })).toBe(true);

  expect(counters).toEqual([
    {
      name: "fishystuff.site.session_started",
      value: 1,
      attributes: {
        page_path: "/map/",
      },
    },
    {
      name: "fishystuff.site.frontend_error",
      value: 1,
      attributes: {
        page_path: "/map/",
        source: "window.error",
      },
    },
  ]);
  expect(histograms).toEqual([
    {
      name: "fishystuff.site.page_ready",
      value: 432.1,
      attributes: {
        page_path: "/map/",
      },
    },
  ]);
});

test("createOtlpHttpLogExporter sends standard OTLP protobuf requests", async () => {
  const originalFetch = globalThis.fetch;
  const calls = [];
  globalThis.fetch = async (url, init) => {
    calls.push({ url, init });
    return new Response("", { status: 200 });
  };

  try {
    const exporter = createOtlpHttpLogExporter({
      url: "http://telemetry.localhost:1990/v1/logs",
    });

    await new Promise((resolve, reject) => {
      exporter.export([], (result) => {
        if (result.code === 0) {
          resolve();
          return;
        }
        reject(result.error || new Error("log export failed"));
      });
    });

    expect(calls).toHaveLength(1);
    expect(calls[0].url).toBe("http://telemetry.localhost:1990/v1/logs");
    expect(calls[0].init.mode).toBeUndefined();
    expect(calls[0].init.headers["content-type"]).toBe("application/x-protobuf");
    expect(calls[0].init.body).toBeInstanceOf(Uint8Array);
  } finally {
    globalThis.fetch = originalFetch;
  }
});
