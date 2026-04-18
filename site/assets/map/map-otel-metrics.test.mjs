import test from "node:test";
import assert from "node:assert/strict";

import {
  collectMapTelemetrySample,
  createMapLifecycleMetrics,
  createMapOtelMetricsReporter,
} from "./map-otel-metrics.js";

test("collectMapTelemetrySample extracts live Bevy and layer metrics from the bridge", () => {
  const sample = collectMapTelemetrySample({
    getCurrentState() {
      return {
        ready: true,
        catalog: {
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
        },
      };
    },
    getPerformanceSnapshot() {
      return {
        frame_time_ms: {
          avg: 16.7,
          p95: 24.2,
        },
        wasm: {
          counters: {
            "bevy.diagnostics.fps": 59.8,
            "bevy.diagnostics.frame_time_ms": 16.7,
            "terrain.runtime.ready": 1,
            "terrain.runtime.chunks_requested": 8,
            "terrain.runtime.chunks_ready": 6,
            "terrain.runtime.cache_hits": 42,
            "terrain.runtime.cache_misses": 3,
            "terrain.runtime.avg_build_ms": 7.1,
          },
        },
      };
    },
  });

  assert.equal(sample.ready, 1);
  assert.equal(sample.visibleLayers, 1);
  assert.equal(sample.bevyFps, 59.8);
  assert.equal(sample.profileFrameTimeP95Ms, 24.2);
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
      getCurrentState() {
        return {
          ready: false,
          catalog: { layers: [] },
        };
      },
      getPerformanceSnapshot() {
        return { wasm: { counters: {} }, frame_time_ms: {} };
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
