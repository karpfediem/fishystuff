import test from "node:test";
import assert from "node:assert/strict";

import {
  buildRuntimeConfig,
  deriveSiblingBaseUrl,
  joinUrl,
  resolvePublicBaseUrls,
  siblingEndpointUrl,
} from "./write-runtime-config.mjs";

test("runtime config defaults to the production sibling-host layout", () => {
  const runtimeConfig = buildRuntimeConfig({});

  assert.equal(runtimeConfig.siteBaseUrl, "https://fishystuff.fish");
  assert.equal(runtimeConfig.apiBaseUrl, "https://api.fishystuff.fish");
  assert.equal(runtimeConfig.cdnBaseUrl, "https://cdn.fishystuff.fish");
  assert.equal(runtimeConfig.tracing.exporterEndpoint, "https://otel.fishystuff.fish/v1/traces");
  assert.equal(runtimeConfig.metrics.exporterEndpoint, "https://otel.fishystuff.fish/v1/metrics");
  assert.equal(runtimeConfig.metrics.exportIntervalMs, 5000);
});

test("runtime config derives beta sibling hosts from the public site base", () => {
  const runtimeConfig = buildRuntimeConfig({
    FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
  });

  assert.equal(runtimeConfig.siteBaseUrl, "https://beta.fishystuff.fish");
  assert.equal(runtimeConfig.apiBaseUrl, "https://api.beta.fishystuff.fish");
  assert.equal(runtimeConfig.cdnBaseUrl, "https://cdn.beta.fishystuff.fish");
  assert.equal(
    runtimeConfig.tracing.exporterEndpoint,
    "https://otel.beta.fishystuff.fish/v1/traces",
  );
  assert.equal(
    runtimeConfig.metrics.exporterEndpoint,
    "https://otel.beta.fishystuff.fish/v1/metrics",
  );
});

test("public base URL resolution is reusable across site build helpers", () => {
  const publicBaseUrls = resolvePublicBaseUrls({
    FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
  });

  assert.deepEqual(publicBaseUrls, {
    publicSiteBaseUrl: "https://beta.fishystuff.fish",
    publicApiBaseUrl: "https://api.beta.fishystuff.fish",
    publicCdnBaseUrl: "https://cdn.beta.fishystuff.fish",
    publicOtelBaseUrl: "https://otel.beta.fishystuff.fish",
    publicOtelTracesEndpoint: "https://otel.beta.fishystuff.fish/v1/traces",
  });
});

test("runtime config prefers explicit public overrides over derived sibling hosts", () => {
  const runtimeConfig = buildRuntimeConfig({
    FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
    FISHYSTUFF_PUBLIC_API_BASE_URL: "https://api-preview.fishystuff.fish",
    FISHYSTUFF_PUBLIC_CDN_BASE_URL: "https://cdn-preview.fishystuff.fish",
    FISHYSTUFF_PUBLIC_OTEL_TRACES_ENDPOINT: "https://otel-preview.fishystuff.fish/custom/traces",
  });

  assert.equal(runtimeConfig.apiBaseUrl, "https://api-preview.fishystuff.fish");
  assert.equal(runtimeConfig.cdnBaseUrl, "https://cdn-preview.fishystuff.fish");
  assert.equal(
    runtimeConfig.tracing.exporterEndpoint,
    "https://otel-preview.fishystuff.fish/custom/traces",
  );
});

test("runtime config allows explicit local browser metrics overrides", () => {
  const runtimeConfig = buildRuntimeConfig({
    FISHYSTUFF_RUNTIME_OTEL_ENABLED: "true",
    FISHYSTUFF_RUNTIME_OTEL_METRICS_ENDPOINT: "http://127.0.0.1:4818/v1/metrics",
    FISHYSTUFF_RUNTIME_OTEL_METRIC_EXPORT_INTERVAL_MS: "3000",
  });

  assert.equal(runtimeConfig.metrics.enabled, true);
  assert.equal(runtimeConfig.metrics.exporterEndpoint, "http://127.0.0.1:4818/v1/metrics");
  assert.equal(runtimeConfig.metrics.exportIntervalMs, 3000);
});

test("runtime config derives the local metrics endpoint from the resolved trace endpoint", () => {
  const runtimeConfig = buildRuntimeConfig({
    FISHYSTUFF_RUNTIME_OTEL_ENABLED: "true",
    FISHYSTUFF_RUNTIME_OTEL_EXPORTER_ENDPOINT: "http://127.0.0.1:4818/v1/traces",
  });

  assert.equal(runtimeConfig.tracing.exporterEndpoint, "http://127.0.0.1:4818/v1/traces");
  assert.equal(runtimeConfig.metrics.exporterEndpoint, "http://127.0.0.1:4818/v1/metrics");
});

test("sibling-host derivation skips loopback and preserves explicit paths when joined", () => {
  assert.equal(
    deriveSiblingBaseUrl("https://beta.fishystuff.fish", "api"),
    "https://api.beta.fishystuff.fish",
  );
  assert.equal(deriveSiblingBaseUrl("http://localhost:1990", "api"), "");
  assert.equal(
    joinUrl("https://otel.beta.fishystuff.fish", "/v1/traces"),
    "https://otel.beta.fishystuff.fish/v1/traces",
  );
  assert.equal(
    siblingEndpointUrl("http://127.0.0.1:4818/v1/traces", "/v1/metrics"),
    "http://127.0.0.1:4818/v1/metrics",
  );
});
