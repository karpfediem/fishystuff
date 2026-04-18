import { expect, test } from "bun:test";

import {
  applyFishystuffResponseAttributes,
  buildBaseUrlPrefixPattern,
  buildIgnorePatterns,
  buildPropagationTargets,
  classifyFetchTarget,
  createHttpError,
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
    exporterEndpoint: "http://localhost:1990/otel/v1/traces",
    metricsExporterEndpoint: "http://localhost:1990/otel/v1/metrics",
    cdnBaseUrl: "http://localhost:4040",
  });

  expect(patterns).toHaveLength(3);
  expect(patterns[0].test("http://localhost:1990/otel/v1/traces")).toBe(true);
  expect(patterns[0].test("http://localhost:1990/otel/v1/traces?x=1")).toBe(true);
  expect(patterns[1].test("http://localhost:1990/otel/v1/metrics")).toBe(true);
  expect(patterns[2].test("http://localhost:4040/map/runtime-manifest.json")).toBe(true);
  expect(patterns[2].test("http://localhost:8080/api/v1/meta")).toBe(false);
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

test("resolveRuntimeConfig keeps browser metrics separate from trace export config", () => {
  globalThis.location = new URL("http://127.0.0.1:1990/map/");
  globalThis.__fishystuffRuntimeConfig = {
    siteBaseUrl: "http://127.0.0.1:1990",
    tracing: {
      enabled: true,
      exporterEndpoint: "http://127.0.0.1:4818/v1/traces",
      serviceName: "fishystuff-site-local",
      deploymentEnvironment: "local",
      serviceVersion: "dev",
      sampleRatio: 0.25,
    },
    metrics: {
      enabled: true,
      exporterEndpoint: "http://127.0.0.1:4818/v1/metrics",
      exportIntervalMs: 3000,
    },
  };

  const config = resolveRuntimeConfig();

  expect(config.exporterEndpoint).toBe("http://127.0.0.1:4818/v1/traces");
  expect(config.metricsExporterEndpoint).toBe("http://127.0.0.1:4818/v1/metrics");
  expect(config.metricsExportIntervalMs).toBe(3000);

  delete globalThis.__fishystuffRuntimeConfig;
  delete globalThis.location;
});
