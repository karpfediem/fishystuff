function normalizeString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function defaultPagePath(globalRef) {
  return normalizeString(globalRef?.location?.pathname) || "/map/";
}

function numericOrZero(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
}

function safeCurrentState(bridge) {
  try {
    return bridge?.getCurrentState?.() || null;
  } catch {
    return null;
  }
}

function safePerformanceSnapshot(bridge) {
  try {
    return bridge?.getPerformanceSnapshot?.() || null;
  } catch {
    return null;
  }
}

function safeTelemetrySample(bridge) {
  try {
    return bridge?.getTelemetrySample?.() || null;
  } catch {
    return null;
  }
}

export function collectMapTelemetrySample(bridge) {
  const telemetry = safeTelemetrySample(bridge);
  if (telemetry && typeof telemetry === "object") {
    const layers = Array.isArray(telemetry.layers) ? telemetry.layers : [];
    return {
      ready: telemetry.ready === true ? 1 : numericOrZero(telemetry.ready),
      visibleLayers: layers.filter((layer) => layer?.visible === true).length,
      bevyFps: numericOrZero(telemetry.bevyFps),
      bevyFrameTimeMs: numericOrZero(telemetry.bevyFrameTimeMs),
      profileFrameTimeAvgMs: 0,
      profileFrameTimeP95Ms: 0,
      terrainReady: numericOrZero(telemetry.terrainReady),
      terrainChunksRequested: numericOrZero(telemetry.terrainChunksRequested),
      terrainChunksReady: numericOrZero(telemetry.terrainChunksReady),
      terrainCacheHits: numericOrZero(telemetry.terrainCacheHits),
      terrainCacheMisses: numericOrZero(telemetry.terrainCacheMisses),
      terrainAvgBuildMs: numericOrZero(telemetry.terrainAvgBuildMs),
      layers: layers.map((layer) => ({
        attributes: {
          layer_id: normalizeString(layer?.layerId),
          layer_kind: normalizeString(layer?.kind) || "unknown",
        },
        visibleTiles: numericOrZero(layer?.visibleTileCount),
        residentTiles: numericOrZero(layer?.residentTileCount),
        pendingTiles: numericOrZero(layer?.pendingCount),
        inflightTiles: numericOrZero(layer?.inflightCount),
        vectorFeatureCount: numericOrZero(layer?.vectorFeatureCount),
        vectorBuildMs: numericOrZero(layer?.vectorBuildMs),
      })),
    };
  }

  const state = safeCurrentState(bridge);
  const performance = safePerformanceSnapshot(bridge);
  const layers = Array.isArray(state?.catalog?.layers) ? state.catalog.layers : [];
  const wasmCounters =
    performance && typeof performance === "object" ? performance.wasm?.counters || {} : {};

  return {
    ready: state?.ready === true ? 1 : 0,
    visibleLayers: layers.filter((layer) => layer?.visible === true).length,
    bevyFps: numericOrZero(wasmCounters["bevy.diagnostics.fps"]),
    bevyFrameTimeMs: numericOrZero(wasmCounters["bevy.diagnostics.frame_time_ms"]),
    profileFrameTimeAvgMs: numericOrZero(performance?.frame_time_ms?.avg),
    profileFrameTimeP95Ms: numericOrZero(performance?.frame_time_ms?.p95),
    terrainReady: numericOrZero(wasmCounters["terrain.runtime.ready"]),
    terrainChunksRequested: numericOrZero(wasmCounters["terrain.runtime.chunks_requested"]),
    terrainChunksReady: numericOrZero(wasmCounters["terrain.runtime.chunks_ready"]),
    terrainCacheHits: numericOrZero(wasmCounters["terrain.runtime.cache_hits"]),
    terrainCacheMisses: numericOrZero(wasmCounters["terrain.runtime.cache_misses"]),
    terrainAvgBuildMs: numericOrZero(wasmCounters["terrain.runtime.avg_build_ms"]),
    layers: layers.map((layer) => ({
      attributes: {
        layer_id: normalizeString(layer?.layerId),
        layer_kind: normalizeString(layer?.kind) || "unknown",
      },
      visibleTiles: numericOrZero(layer?.visibleTileCount),
      residentTiles: numericOrZero(layer?.residentTileCount),
      pendingTiles: numericOrZero(layer?.pendingCount),
      inflightTiles: numericOrZero(layer?.inflightCount),
      vectorFeatureCount: numericOrZero(layer?.vectorFeatureCount),
      vectorBuildMs: numericOrZero(layer?.vectorBuildMs),
    })),
  };
}

function observeScalar(result, instrument, value) {
  if (!instrument) {
    return;
  }
  result.observe(instrument, numericOrZero(value));
}

function observeLayerGauge(result, instrument, layers, selector) {
  if (!instrument) {
    return;
  }
  for (const layer of Array.isArray(layers) ? layers : []) {
    if (!layer?.attributes?.layer_id) {
      continue;
    }
    result.observe(instrument, numericOrZero(selector(layer)), layer.attributes);
  }
}

export function createMapOtelMetricsReporter({
  bridge,
  globalRef = globalThis,
  instrumentationName = "fishystuff.map",
} = {}) {
  const meter = globalRef?.__fishystuffOtel?.getMeter?.(instrumentationName);
  if (!bridge || !meter) {
    return Object.freeze({
      enabled: false,
      shutdown() {},
    });
  }

  const runtimeReady = meter.createObservableGauge("fishystuff.map.runtime.ready", {
    description: "Whether the map bridge reports ready.",
  });
  const visibleLayers = meter.createObservableGauge("fishystuff.map.runtime.visible_layers", {
    description: "Visible map layers in the current bridged runtime state.",
  });
  const bevyFps = meter.createObservableGauge("fishystuff.map.bevy.fps", {
    description: "Short-window Bevy FPS from the live browser runtime.",
  });
  const bevyFrameTimeMs = meter.createObservableGauge("fishystuff.map.bevy.frame_time", {
    description: "Short-window Bevy frame time from the live browser runtime.",
    unit: "ms",
  });
  const terrainReady = meter.createObservableGauge("fishystuff.map.terrain.ready", {
    description: "Whether terrain runtime assets are ready.",
  });
  const terrainChunksRequested = meter.createObservableGauge(
    "fishystuff.map.terrain.chunks_requested",
    {
      description: "Terrain chunks requested by the Bevy runtime.",
    },
  );
  const terrainChunksReady = meter.createObservableGauge(
    "fishystuff.map.terrain.chunks_ready",
    {
      description: "Terrain chunks currently ready in the Bevy runtime.",
    },
  );
  const terrainCacheHits = meter.createObservableGauge("fishystuff.map.terrain.cache_hits", {
    description: "Terrain cache hits accumulated by the current Bevy runtime.",
  });
  const terrainCacheMisses = meter.createObservableGauge(
    "fishystuff.map.terrain.cache_misses",
    {
      description: "Terrain cache misses accumulated by the current Bevy runtime.",
    },
  );
  const terrainAvgBuildMs = meter.createObservableGauge(
    "fishystuff.map.terrain.avg_build_time",
    {
      description: "Average terrain chunk build time observed by the current Bevy runtime.",
      unit: "ms",
    },
  );
  const layerVisibleTiles = meter.createObservableGauge(
    "fishystuff.map.layer.visible_tiles",
    {
      description: "Visible tile count by map layer.",
    },
  );
  const layerResidentTiles = meter.createObservableGauge(
    "fishystuff.map.layer.resident_tiles",
    {
      description: "Resident tile count by map layer.",
    },
  );
  const layerPendingTiles = meter.createObservableGauge(
    "fishystuff.map.layer.pending_tiles",
    {
      description: "Pending tile count by map layer.",
    },
  );
  const layerInflightTiles = meter.createObservableGauge(
    "fishystuff.map.layer.inflight_tiles",
    {
      description: "Inflight tile count by map layer.",
    },
  );
  const layerVectorFeatureCount = meter.createObservableGauge(
    "fishystuff.map.layer.vector_feature_count",
    {
      description: "Vector feature count by map layer.",
    },
  );
  const layerVectorBuildMs = meter.createObservableGauge(
    "fishystuff.map.layer.vector_build_time",
    {
      description: "Vector build time by map layer.",
      unit: "ms",
    },
  );

  const observables = [
    runtimeReady,
    visibleLayers,
    bevyFps,
    bevyFrameTimeMs,
    terrainReady,
    terrainChunksRequested,
    terrainChunksReady,
    terrainCacheHits,
    terrainCacheMisses,
    terrainAvgBuildMs,
    layerVisibleTiles,
    layerResidentTiles,
    layerPendingTiles,
    layerInflightTiles,
    layerVectorFeatureCount,
    layerVectorBuildMs,
  ];

  const callback = (result) => {
    const sample = collectMapTelemetrySample(bridge);
    observeScalar(result, runtimeReady, sample.ready);
    observeScalar(result, visibleLayers, sample.visibleLayers);
    observeScalar(result, bevyFps, sample.bevyFps);
    observeScalar(result, bevyFrameTimeMs, sample.bevyFrameTimeMs);
    observeScalar(result, terrainReady, sample.terrainReady);
    observeScalar(result, terrainChunksRequested, sample.terrainChunksRequested);
    observeScalar(result, terrainChunksReady, sample.terrainChunksReady);
    observeScalar(result, terrainCacheHits, sample.terrainCacheHits);
    observeScalar(result, terrainCacheMisses, sample.terrainCacheMisses);
    observeScalar(result, terrainAvgBuildMs, sample.terrainAvgBuildMs);
    observeLayerGauge(result, layerVisibleTiles, sample.layers, (layer) => layer.visibleTiles);
    observeLayerGauge(result, layerResidentTiles, sample.layers, (layer) => layer.residentTiles);
    observeLayerGauge(result, layerPendingTiles, sample.layers, (layer) => layer.pendingTiles);
    observeLayerGauge(result, layerInflightTiles, sample.layers, (layer) => layer.inflightTiles);
    observeLayerGauge(
      result,
      layerVectorFeatureCount,
      sample.layers,
      (layer) => layer.vectorFeatureCount,
    );
    observeLayerGauge(result, layerVectorBuildMs, sample.layers, (layer) => layer.vectorBuildMs);
  };

  meter.addBatchObservableCallback(callback, observables);

  return Object.freeze({
    enabled: true,
    shutdown() {
      meter.removeBatchObservableCallback?.(callback, observables);
    },
  });
}

export function createMapLifecycleMetrics({
  globalRef = globalThis,
  instrumentationName = "fishystuff.map",
} = {}) {
  const meter = globalRef?.__fishystuffOtel?.getMeter?.(instrumentationName);
  if (!meter) {
    return Object.freeze({
      enabled: false,
      recordRuntimeReady() {
        return false;
      },
    });
  }

  const runtimeReadyDuration = meter.createHistogram("fishystuff.map.runtime.ready_duration", {
    description: "Browser-observed startup duration until the map bridge reports ready.",
    unit: "ms",
  });
  let runtimeReadyRecorded = false;

  return Object.freeze({
    enabled: true,
    recordRuntimeReady(durationMs, attributes = {}) {
      if (runtimeReadyRecorded) {
        return false;
      }
      const numericDurationMs = Number(durationMs);
      if (!Number.isFinite(numericDurationMs) || numericDurationMs < 0) {
        return false;
      }
      runtimeReadyDuration.record(numericDurationMs, {
        page_path: defaultPagePath(globalRef),
        ...Object.fromEntries(
          Object.entries(attributes || {}).filter(([, value]) => {
            return (
              (typeof value === "string" && normalizeString(value))
              || (typeof value === "number" && Number.isFinite(value))
              || typeof value === "boolean"
            );
          }),
        ),
      });
      runtimeReadyRecorded = true;
      return true;
    },
  });
}
