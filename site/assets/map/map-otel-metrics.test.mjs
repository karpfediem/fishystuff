import test from "node:test";
import assert from "node:assert/strict";

import {
  collectMapTelemetrySample,
  createMapLifecycleMetrics,
  createMapOtelMetricsReporter,
} from "./map-otel-metrics.js";

test("collectMapTelemetrySample prefers the lightweight bridge telemetry sample", () => {
  const sample = collectMapTelemetrySample({
    getTelemetrySample() {
      return {
        ready: true,
        bevyFps: 59.8,
        bevyFrameTimeMs: 16.7,
        terrainReady: 1,
        terrainChunksRequested: 8,
        terrainChunksReady: 6,
        terrainCacheHits: 42,
        terrainCacheMisses: 3,
        terrainAvgBuildMs: 7.1,
        layers: [
          {
            layerId: "zone_mask",
            kind: "tiled-raster",
            visible: true,
            visibleTileCount: 12,
            residentTileCount: 18,
            pendingCount: 2,
            inflightCount: 1,
            vectorFeatureCount: 0,
            vectorBuildMs: 0,
          },
        ],
      };
    },
  });

  assert.equal(sample.ready, 1);
  assert.equal(sample.visibleLayers, 1);
  assert.equal(sample.bevyFps, 59.8);
  assert.equal(sample.profileFrameTimeP95Ms, 0);
  assert.equal(sample.terrainChunksReady, 6);
  assert.deepEqual(sample.layers[0], {
    attributes: {
      layer_id: "zone_mask",
      layer_kind: "tiled-raster",
    },
    visibleTiles: 12,
    residentTiles: 18,
    pendingTiles: 2,
    inflightTiles: 1,
    vectorFeatureCount: 0,
    vectorBuildMs: 0,
  });
});

test("createMapOtelMetricsReporter registers a batch callback when OTEL metrics are enabled", () => {
  const registered = [];
  const meter = {
    createObservableGauge(name) {
      return { name };
    },
    addBatchObservableCallback(callback, observables) {
      registered.push({ callback, observables });
    },
    removeBatchObservableCallback(callback, observables) {
      registered.push({ removed: true, callback, observables });
    },
  };

  const reporter = createMapOtelMetricsReporter({
    bridge: {
      getTelemetrySample() {
        return {
          ready: false,
          layers: [],
          bevyFps: 0,
          bevyFrameTimeMs: 0,
          terrainReady: 0,
          terrainChunksRequested: 0,
          terrainChunksReady: 0,
          terrainCacheHits: 0,
          terrainCacheMisses: 0,
          terrainAvgBuildMs: 0,
        };
      },
    },
    globalRef: {
      __fishystuffOtel: {
        getMeter() {
          return meter;
        },
      },
    },
  });

  assert.equal(reporter.enabled, true);
  assert.equal(registered.length, 1);
  assert.ok(Array.isArray(registered[0].observables));

  reporter.shutdown();
  assert.equal(registered.length, 2);
  assert.equal(registered[1].removed, true);
});

test("createMapLifecycleMetrics records startup duration once", () => {
  const histograms = [];
  const lifecycleMetrics = createMapLifecycleMetrics({
    globalRef: {
      location: {
        pathname: "/map/",
      },
      __fishystuffOtel: {
        getMeter() {
          return {
            createHistogram(name) {
              return {
                record(value, attributes) {
                  histograms.push({ name, value, attributes });
                },
              };
            },
          };
        },
      },
    },
  });

  assert.equal(lifecycleMetrics.enabled, true);
  assert.equal(lifecycleMetrics.recordRuntimeReady(215.6), true);
  assert.equal(lifecycleMetrics.recordRuntimeReady(300), false);
  assert.deepEqual(histograms, [
    {
      name: "fishystuff.map.runtime.ready_duration",
      value: 215.6,
      attributes: {
        page_path: "/map/",
      },
    },
  ]);
});
