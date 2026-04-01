import { FISHYMAP_POINT_ICON_SCALE_MAX, FISHYMAP_POINT_ICON_SCALE_MIN } from "./map-host.js";
import { nextHoverFactVisibilityByLayer } from "./map-hover-facts.js";
import { renderLayerStack } from "./map-layer-panel.js";
import {
  dispatchShellSignalPatch,
  FISHYMAP_SIGNAL_PATCHED_EVENT,
} from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";
import {
  buildLayerClipMaskPatch,
  buildLayerOpacityPatch,
  buildLayerPointIconsPatch,
  buildLayerPointIconScalePatch,
  buildLayerWaypointConnectionsPatch,
  buildLayerWaypointLabelsPatch,
  clampLayerOpacity,
  clampPointIconScale,
  layerOpacityLabel,
  layerOpacityValue,
  moveLayerIdBefore,
  pointIconScaleLabel,
  pointIconScaleValue,
  resolveLayerEntries,
  resolveVisibleLayerIds,
} from "./map-layer-state.js";

const ICON_SPRITE_URL = "/img/icons.svg";
const DRAG_ATTACH_EDGE_PX_MIN = 14;
const DRAG_ATTACH_EDGE_PX_MAX = 22;

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (char) =>
      (
        {
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&#39;",
        }[char] || char
      ),
  );
}

function spriteIcon(name, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${name}"></use></svg>`;
}

function dragHandleIcon() {
  return spriteIcon("drag-handle");
}

function layerSettingsIcon() {
  return spriteIcon("settings-1");
}

function eyeIcon(visible) {
  return visible ? spriteIcon("eye") : spriteIcon("eye-slash");
}

function renderLoadingInlineMarkup(label, { size = "xs", toneClass = "text-base-content/70" } = {}) {
  return `<span class="inline-flex items-center gap-2 ${toneClass}"><span class="loading loading-spinner loading-${escapeHtml(size)}" aria-hidden="true"></span><span>${escapeHtml(label)}</span></span>`;
}

function renderLoadingPanelMarkup(label) {
  return `<div class="rounded-box border border-base-300/70 bg-base-200 px-3 py-3"><div class="flex items-center gap-2 text-xs text-base-content/60">${renderLoadingInlineMarkup(label)}</div></div>`;
}

function normalizeExpandedLayerIds(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = String(value ?? "").trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

export function toggleExpandedLayerIds(values, layerId) {
  const normalizedLayerId = String(layerId ?? "").trim();
  if (!normalizedLayerId) {
    return normalizeExpandedLayerIds(values);
  }
  const current = normalizeExpandedLayerIds(values);
  if (current.includes(normalizedLayerId)) {
    return current.filter((candidate) => candidate !== normalizedLayerId);
  }
  return current.concat(normalizedLayerId);
}

export function patchTouchesLayerPanelSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  if (patch._map_runtime?.ready != null) {
    return true;
  }
  if (patch._map_runtime?.catalog?.layers != null) {
    return true;
  }
  if (patch._map_runtime?.selection != null) {
    return true;
  }
  if (patch._map_bridged?.filters != null) {
    return true;
  }
  if (patch._map_ui?.layers != null) {
    return true;
  }
  return false;
}

export function buildLayerPanelStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const bridged = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  return {
    state: {
      ready: runtime.ready === true,
      catalog: {
        layers: Array.isArray(runtime.catalog?.layers) ? cloneJson(runtime.catalog.layers) : [],
      },
    },
    inputState: {
      filters: cloneJson(bridged),
    },
  };
}

function syncLayerOpacityControl(container, layerId, opacity) {
  if (!container || !layerId) {
    return false;
  }
  const slider = Array.from(container.querySelectorAll("input[data-layer-opacity]")).find(
    (candidate) => candidate.getAttribute("data-layer-opacity") === layerId,
  );
  if (!slider) {
    return false;
  }
  const normalized = clampLayerOpacity(opacity);
  const value = layerOpacityValue(normalized);
  if (slider.value !== value) {
    slider.value = value;
  }
  const label = slider
    .closest(".fishymap-layer-opacity-control")
    ?.querySelector?.("[data-layer-opacity-value]");
  if (label) {
    label.textContent = layerOpacityLabel(normalized);
  }
  return true;
}

function syncLayerPointIconScaleControl(container, layerId, scale) {
  if (!container || !layerId) {
    return false;
  }
  const slider = Array.from(container.querySelectorAll("input[data-layer-point-icon-scale]")).find(
    (candidate) => candidate.getAttribute("data-layer-point-icon-scale") === layerId,
  );
  if (!slider) {
    return false;
  }
  const normalized = clampPointIconScale(scale);
  const value = pointIconScaleValue(normalized);
  if (slider.value !== value) {
    slider.value = value;
  }
  const label = slider
    .closest(".fieldset")
    ?.querySelector?.("[data-layer-point-icon-scale-value]");
  if (label) {
    label.textContent = pointIconScaleLabel(normalized);
  }
  return true;
}

export function createMapLayerPanelController({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  listenToSignalPatches = true,
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapLayerPanelController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapLayerPanelController requires getSignals()");
  }

  const container = shell.querySelector("#fishymap-layers");
  const count = shell.querySelector("#fishymap-layers-count");
  if (!(container instanceof HTMLElement)) {
    throw new Error("createMapLayerPanelController requires #fishymap-layers");
  }

  const state = {
    frameId: 0,
    activeOpacityLayerId: "",
    activeOpacityValue: null,
    activePointIconScaleLayerId: "",
    activePointIconScaleValue: null,
    draggingLayerId: "",
    overLayerId: "",
    dropMode: "",
    hover: null,
  };
  let currentZoneCatalog = [];
  const canvas = shell.querySelector("#bevy");

  function signals() {
    return getSignals() || null;
  }

  function stateBundle() {
    return buildLayerPanelStateBundle(signals());
  }

  function clearDropState() {
    state.overLayerId = "";
    state.dropMode = "";
    container
      .querySelectorAll(".fishymap-layer-card[data-drop-position]")
      .forEach((card) => {
        delete card.dataset.dropPosition;
      });
  }

  function applyDropState(layerId, mode) {
    state.overLayerId = String(layerId || "");
    state.dropMode = String(mode || "");
    container
      .querySelectorAll(".fishymap-layer-card")
      .forEach((card) => {
        if (card.getAttribute("data-layer-id") === state.overLayerId) {
          card.dataset.dropPosition = state.dropMode;
          return;
        }
        delete card.dataset.dropPosition;
      });
  }

  function writeExpandedLayerIds(nextExpandedLayerIds) {
    dispatchPatch(shell, {
      _map_ui: {
        layers: {
          expandedLayerIds: normalizeExpandedLayerIds(nextExpandedLayerIds),
        },
      },
    });
    scheduleRender();
  }

  function writeBridgedFilters(mutator) {
    const nextFilters = cloneJson(signals()?._map_bridged?.filters || {});
    mutator(nextFilters, signals());
    dispatchPatch(shell, {
      _map_bridged: {
        filters: nextFilters,
      },
    });
    scheduleRender();
  }

  function render() {
    state.frameId = 0;
    const liveSignals = signals();
    const bundle = buildLayerPanelStateBundle(liveSignals);
    const runtimeLayers = Array.isArray(liveSignals?._map_runtime?.catalog?.layers)
      ? liveSignals._map_runtime.catalog.layers
      : [];
    const expandedLayerIds = new Set(
      normalizeExpandedLayerIds(liveSignals?._map_ui?.layers?.expandedLayerIds),
    );

    const activeOpacityInteraction =
      state.activeOpacityLayerId &&
      Number.isFinite(state.activeOpacityValue) &&
      syncLayerOpacityControl(container, state.activeOpacityLayerId, state.activeOpacityValue);
    const activePointIconScaleInteraction =
      !activeOpacityInteraction &&
      state.activePointIconScaleLayerId &&
      Number.isFinite(state.activePointIconScaleValue) &&
      syncLayerPointIconScaleControl(
        container,
        state.activePointIconScaleLayerId,
        state.activePointIconScaleValue,
      );

    if (!activeOpacityInteraction && !activePointIconScaleInteraction) {
      renderLayerStack(
        container,
        bundle.state.ready === true ? bundle : { state: { catalog: { layers: [] } }, inputState: {} },
        {
          expandedLayerIds,
          hover: state.hover,
          selection: liveSignals?._map_runtime?.selection || {},
          zoneCatalog: currentZoneCatalog,
          hoverFactVisibilityByLayer:
            liveSignals?._map_ui?.layers?.hoverFactsVisibleByLayer || {},
          renderLoadingPanelMarkup,
          escapeHtml,
          dragHandleIcon,
          layerSettingsIcon,
          eyeIcon,
        },
      );
    }

    if (count) {
      count.textContent = String(bundle.state.ready === true ? runtimeLayers.length : 0);
    }

    if (state.draggingLayerId) {
      const draggingCard = container.querySelector(
        `.fishymap-layer-card[data-layer-id="${CSS.escape(state.draggingLayerId)}"]`,
      );
      if (draggingCard) {
        draggingCard.dataset.dragging = "true";
      }
    }
    if (state.overLayerId && state.dropMode) {
      applyDropState(state.overLayerId, state.dropMode);
    }
  }

  function scheduleRender() {
    if (state.frameId) {
      return;
    }
    if (typeof requestAnimationFrameImpl === "function") {
      state.frameId = requestAnimationFrameImpl(() => {
        render();
      });
      return;
    }
    render();
  }

  function hasExpandedLayerSettings() {
    return normalizeExpandedLayerIds(signals()?._map_ui?.layers?.expandedLayerIds).length > 0;
  }

  shell.addEventListener("fishymap:hover-changed", (event) => {
    state.hover =
      event?.detail?.hover && typeof event.detail.hover === "object" ? cloneJson(event.detail.hover) : null;
    if (hasExpandedLayerSettings()) {
      scheduleRender();
    }
  });

  canvas?.addEventListener?.("pointerleave", () => {
    if (!state.hover) {
      return;
    }
    state.hover = null;
    if (hasExpandedLayerSettings()) {
      scheduleRender();
    }
  });

  if (listenToSignalPatches) {
    shell.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, (event) => {
      if (patchTouchesLayerPanelSignals(event?.detail || null)) {
        scheduleRender();
      }
    });
    shell.addEventListener(FISHYMAP_ZONE_CATALOG_READY_EVENT, (event) => {
      currentZoneCatalog = Array.isArray(event?.detail?.zoneCatalog)
        ? cloneJson(event.detail.zoneCatalog)
        : [];
      scheduleRender();
    });
  }

  container.addEventListener("click", (event) => {
    const settingsButton = event.target.closest("button[data-layer-settings-toggle]");
    if (settingsButton) {
      writeExpandedLayerIds(
        toggleExpandedLayerIds(
          signals()?._map_ui?.layers?.expandedLayerIds,
          settingsButton.getAttribute("data-layer-settings-toggle"),
        ),
      );
      return;
    }

    const visibilityButton = event.target.closest("button[data-layer-visibility]");
    if (!visibilityButton) {
      return;
    }
    const layerId = String(visibilityButton.getAttribute("data-layer-visibility") || "").trim();
    if (!layerId) {
      return;
    }
    const bundle = stateBundle();
    const visibleIds = new Set(resolveVisibleLayerIds(bundle));
    if (visibleIds.has(layerId)) {
      visibleIds.delete(layerId);
    } else {
      visibleIds.add(layerId);
    }
    const orderedVisibleIds = resolveLayerEntries(bundle)
      .map((layer) => layer.layerId)
      .filter((candidateId) => visibleIds.has(candidateId));
    writeBridgedFilters((filters) => {
      filters.layerIdsVisible = orderedVisibleIds;
    });
  });

  container.addEventListener("change", (event) => {
    const connectionToggle = event.target.closest("input[data-layer-waypoint-connections]");
    if (connectionToggle) {
      const layerId = String(connectionToggle.getAttribute("data-layer-waypoint-connections") || "").trim();
      if (!layerId) {
        return;
      }
      const next = buildLayerWaypointConnectionsPatch(stateBundle(), layerId, connectionToggle.checked);
      writeBridgedFilters((filters) => {
        filters.layerWaypointConnectionsVisible = next;
      });
      return;
    }

    const labelToggle = event.target.closest("input[data-layer-waypoint-labels]");
    if (labelToggle) {
      const layerId = String(labelToggle.getAttribute("data-layer-waypoint-labels") || "").trim();
      if (!layerId) {
        return;
      }
      const next = buildLayerWaypointLabelsPatch(stateBundle(), layerId, labelToggle.checked);
      writeBridgedFilters((filters) => {
        filters.layerWaypointLabelsVisible = next;
      });
      return;
    }

    const pointIconsToggle = event.target.closest("input[data-layer-point-icons]");
    if (pointIconsToggle) {
      const layerId = String(pointIconsToggle.getAttribute("data-layer-point-icons") || "").trim();
      if (!layerId) {
        return;
      }
      const next = buildLayerPointIconsPatch(stateBundle(), layerId, pointIconsToggle.checked);
      writeBridgedFilters((filters) => {
        filters.layerPointIconsVisible = next;
      });
      return;
    }

    const hoverFactToggle = event.target.closest("input[data-layer-hover-fact-key]");
    if (hoverFactToggle) {
      const layerId = String(
        hoverFactToggle.getAttribute("data-layer-hover-fact-layer-id") || "",
      ).trim();
      const factKey = String(hoverFactToggle.getAttribute("data-layer-hover-fact-key") || "").trim();
      if (!layerId || !factKey) {
        return;
      }
      dispatchPatch(shell, {
        _map_ui: {
          layers: {
            hoverFactsVisibleByLayer: nextHoverFactVisibilityByLayer(
              signals()?._map_ui?.layers?.hoverFactsVisibleByLayer,
              layerId,
              factKey,
              hoverFactToggle.checked,
            ),
          },
        },
      });
      scheduleRender();
      return;
    }

    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      state.activePointIconScaleLayerId = "";
      state.activePointIconScaleValue = null;
      scheduleRender();
      return;
    }

    const opacitySlider = event.target.closest("input[data-layer-opacity]");
    if (opacitySlider) {
      state.activeOpacityLayerId = "";
      state.activeOpacityValue = null;
      scheduleRender();
    }
  });

  container.addEventListener("input", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      const layerId = String(pointIconScaleSlider.getAttribute("data-layer-point-icon-scale") || "").trim();
      if (!layerId) {
        return;
      }
      state.activePointIconScaleLayerId = layerId;
      state.activePointIconScaleValue = clampPointIconScale(pointIconScaleSlider.value);
      syncLayerPointIconScaleControl(container, layerId, state.activePointIconScaleValue);
      const next = buildLayerPointIconScalePatch(stateBundle(), layerId, pointIconScaleSlider.value);
      writeBridgedFilters((filters) => {
        filters.layerPointIconScales = next;
      });
      return;
    }

    const opacitySlider = event.target.closest("input[data-layer-opacity]");
    if (!opacitySlider) {
      return;
    }
    const layerId = String(opacitySlider.getAttribute("data-layer-opacity") || "").trim();
    if (!layerId) {
      return;
    }
    state.activeOpacityLayerId = layerId;
    state.activeOpacityValue = clampLayerOpacity(opacitySlider.value);
    syncLayerOpacityControl(container, layerId, state.activeOpacityValue);
    const next = buildLayerOpacityPatch(stateBundle(), layerId, opacitySlider.value);
    writeBridgedFilters((filters) => {
      filters.layerOpacities = next;
    });
  });

  container.addEventListener("focusout", (event) => {
    const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
    if (pointIconScaleSlider) {
      queueMicrotask(() => {
        state.activePointIconScaleLayerId = "";
        state.activePointIconScaleValue = null;
        scheduleRender();
      });
      return;
    }
    const opacitySlider = event.target.closest("input[data-layer-opacity]");
    if (!opacitySlider) {
      return;
    }
    queueMicrotask(() => {
      state.activeOpacityLayerId = "";
      state.activeOpacityValue = null;
      scheduleRender();
    });
  });

  container.addEventListener("dragstart", (event) => {
    const handle = event.target.closest("button[data-layer-drag][draggable='true']");
    const card = handle?.closest(".fishymap-layer-card");
    if (!handle || !card) {
      return;
    }
    const layerId = String(card.getAttribute("data-layer-id") || "").trim();
    if (!layerId) {
      return;
    }
    state.draggingLayerId = layerId;
    card.dataset.dragging = "true";
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", layerId);
    }
  });

  container.addEventListener("dragover", (event) => {
    if (!state.draggingLayerId) {
      return;
    }
    event.preventDefault();
    const card = event.target.closest(".fishymap-layer-card");
    if (!card) {
      clearDropState();
      return;
    }
    const targetLayerId = String(card.getAttribute("data-layer-id") || "").trim();
    if (!targetLayerId || targetLayerId === state.draggingLayerId) {
      clearDropState();
      return;
    }
    const rect = card.getBoundingClientRect();
    const offsetY = event.clientY - rect.top;
    const locked = card.getAttribute("data-locked") === "true";
    const canAttach = card.getAttribute("data-clip-mask-source") === "true";
    let mode = "before";
    if (locked) {
      mode = "before";
    } else if (!canAttach) {
      mode = offsetY >= rect.height / 2 ? "after" : "before";
    } else {
      const edgeThreshold = Math.max(
        DRAG_ATTACH_EDGE_PX_MIN,
        Math.min(rect.height * 0.24, DRAG_ATTACH_EDGE_PX_MAX),
      );
      if (offsetY <= edgeThreshold) {
        mode = "before";
      } else if (offsetY >= rect.height - edgeThreshold) {
        mode = "after";
      } else {
        mode = "attach";
      }
    }
    applyDropState(targetLayerId, mode);
  });

  container.addEventListener("drop", (event) => {
    if (!state.draggingLayerId || !state.overLayerId || !state.dropMode) {
      clearDropState();
      return;
    }
    event.preventDefault();
    const bundle = stateBundle();
    const nextOrder = moveLayerIdBefore(
      resolveLayerEntries(bundle),
      state.draggingLayerId,
      state.overLayerId,
      state.dropMode === "after" ? "after" : "before",
    );
    const nextClipMasks = buildLayerClipMaskPatch(
      bundle,
      state.draggingLayerId,
      state.dropMode === "attach" ? state.overLayerId : "",
    );
    writeBridgedFilters((filters) => {
      filters.layerIdsOrdered = nextOrder;
      filters.layerClipMasks = nextClipMasks;
    });
    state.draggingLayerId = "";
    clearDropState();
  });

  container.addEventListener("dragend", () => {
    state.draggingLayerId = "";
    container
      .querySelectorAll(".fishymap-layer-card[data-dragging]")
      .forEach((card) => {
        delete card.dataset.dragging;
      });
    clearDropState();
  });

  container.addEventListener("dragleave", (event) => {
    const related = event.relatedTarget;
    if (related instanceof Node && container.contains(related)) {
      return;
    }
    clearDropState();
  });

  return Object.freeze({
    render,
    scheduleRender,
  });
}
