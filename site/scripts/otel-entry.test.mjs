import { expect, test } from "bun:test";

import {
  buildBaseUrlPrefixPattern,
  buildIgnorePatterns,
  buildPropagationTargets,
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
    cdnBaseUrl: "http://localhost:4040",
  });

  expect(patterns).toHaveLength(2);
  expect(patterns[0].test("http://localhost:1990/otel/v1/traces")).toBe(true);
  expect(patterns[0].test("http://localhost:1990/otel/v1/traces?x=1")).toBe(true);
  expect(patterns[1].test("http://localhost:4040/map/runtime-manifest.json")).toBe(true);
  expect(patterns[1].test("http://localhost:8080/api/v1/meta")).toBe(false);
});
