import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";
import {
  buildLayerPanelStateBundle,
  patchTouchesLayerPanelSignals,
  toggleExpandedLayerIds,
} from "./map-layer-panel-state.js";
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

const LAYER_PANEL_TAG_NAME = "fishymap-layer-panel";
const FISHYMAP_LIVE_INIT_EVENT = "fishymap-live-init";
const FISHYMAP_POINT_ICON_SCALE_MAX = 3;
const FISHYMAP_POINT_ICON_SCALE_MIN = 0.25;
const ICON_SPRITE_URL = "/img/icons.svg";
const DRAG_ATTACH_EDGE_PX_MIN = 14;
const DRAG_ATTACH_EDGE_PX_MAX = 22;
const HTMLElementBase = globalThis.HTMLElement ?? class {};

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

function readMapShellSignals(shell) {
  if (!shell || typeof shell !== "object") {
    return null;
  }
  const liveSignals = shell.__fishymapLiveSignals;
  if (liveSignals && typeof liveSignals === "object") {
    return liveSignals;
  }
  const initialSignals = shell.__fishymapInitialSignals;
  return initialSignals && typeof initialSignals === "object" ? initialSignals : null;
}

export function readMapLayerPanelShellSignals(shell) {
  return readMapShellSignals(shell);
}

function setTextContent(element, text) {
  if (!element) {
    return;
  }
  element.textContent = String(text ?? "");
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

export class FishyMapLayerPanelElement extends HTMLElementBase {
  constructor() {
    super();
    this._shell = null;
    this._signalPatchTarget = null;
    this._rafId = 0;
    this._zoneCatalog = [];
    this._state = {
      activeOpacityLayerId: "",
      activeOpacityValue: null,
      activePointIconScaleLayerId: "",
      activePointIconScaleValue: null,
      draggingLayerId: "",
      overLayerId: "",
      dropMode: "",
      hover: null,
    };
    this._count = null;
    this._canvas = null;
    this._handleSignalPatched = (event) => {
      if (patchTouchesLayerPanelSignals(event?.detail || null)) {
        this.scheduleRender();
      }
    };
    this._handleZoneCatalogReady = (event) => {
      this._zoneCatalog = Array.isArray(event?.detail?.zoneCatalog) ? cloneJson(event.detail.zoneCatalog) : [];
      this.scheduleRender();
    };
    this._handleLiveInit = () => {
      this.scheduleRender();
    };
    this._handleHoverChanged = (event) => {
      this._state.hover =
        event?.detail?.hover && typeof event.detail.hover === "object"
          ? cloneJson(event.detail.hover)
          : null;
      if (this.hasExpandedLayerSettings()) {
        this.scheduleRender();
      }
    };
    this._handleCanvasPointerLeave = () => {
      if (!this._state.hover) {
        return;
      }
      this._state.hover = null;
      if (this.hasExpandedLayerSettings()) {
        this.scheduleRender();
      }
    };
    this._handleClick = (event) => {
      const settingsButton = event.target.closest("button[data-layer-settings-toggle]");
      if (settingsButton) {
        this.writeExpandedLayerIds(
          toggleExpandedLayerIds(
            this.signals()?._map_ui?.layers?.expandedLayerIds,
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
      const bundle = this.bundle();
      const visibleIds = new Set(resolveVisibleLayerIds(bundle));
      if (visibleIds.has(layerId)) {
        visibleIds.delete(layerId);
      } else {
        visibleIds.add(layerId);
      }
      const orderedVisibleIds = resolveLayerEntries(bundle)
        .map((layer) => layer.layerId)
        .filter((candidateId) => visibleIds.has(candidateId));
      this.writeBridgedFilters((filters) => {
        filters.layerIdsVisible = orderedVisibleIds;
      });
    };
    this._handleChange = (event) => {
      const connectionToggle = event.target.closest("input[data-layer-waypoint-connections]");
      if (connectionToggle) {
        const layerId = String(connectionToggle.getAttribute("data-layer-waypoint-connections") || "").trim();
        if (!layerId) {
          return;
        }
        const next = buildLayerWaypointConnectionsPatch(this.bundle(), layerId, connectionToggle.checked);
        this.writeBridgedFilters((filters) => {
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
        const next = buildLayerWaypointLabelsPatch(this.bundle(), layerId, labelToggle.checked);
        this.writeBridgedFilters((filters) => {
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
        const next = buildLayerPointIconsPatch(this.bundle(), layerId, pointIconsToggle.checked);
        this.writeBridgedFilters((filters) => {
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
        this.dispatchPatch({
          _map_ui: {
            layers: {
              hoverFactsVisibleByLayer: nextHoverFactVisibilityByLayer(
                this.signals()?._map_ui?.layers?.hoverFactsVisibleByLayer,
                layerId,
                factKey,
                hoverFactToggle.checked,
              ),
            },
          },
        });
        this.scheduleRender();
        return;
      }

      const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
      if (pointIconScaleSlider) {
        this._state.activePointIconScaleLayerId = "";
        this._state.activePointIconScaleValue = null;
        this.scheduleRender();
        return;
      }

      const opacitySlider = event.target.closest("input[data-layer-opacity]");
      if (opacitySlider) {
        this._state.activeOpacityLayerId = "";
        this._state.activeOpacityValue = null;
        this.scheduleRender();
      }
    };
    this._handleInput = (event) => {
      const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
      if (pointIconScaleSlider) {
        const layerId = String(pointIconScaleSlider.getAttribute("data-layer-point-icon-scale") || "").trim();
        if (!layerId) {
          return;
        }
        this._state.activePointIconScaleLayerId = layerId;
        this._state.activePointIconScaleValue = clampPointIconScale(pointIconScaleSlider.value);
        syncLayerPointIconScaleControl(this, layerId, this._state.activePointIconScaleValue);
        const next = buildLayerPointIconScalePatch(this.bundle(), layerId, pointIconScaleSlider.value);
        this.writeBridgedFilters((filters) => {
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
      this._state.activeOpacityLayerId = layerId;
      this._state.activeOpacityValue = clampLayerOpacity(opacitySlider.value);
      syncLayerOpacityControl(this, layerId, this._state.activeOpacityValue);
      const next = buildLayerOpacityPatch(this.bundle(), layerId, opacitySlider.value);
      this.writeBridgedFilters((filters) => {
        filters.layerOpacities = next;
      });
    };
    this._handleFocusOut = (event) => {
      const pointIconScaleSlider = event.target.closest("input[data-layer-point-icon-scale]");
      if (pointIconScaleSlider) {
        queueMicrotask(() => {
          this._state.activePointIconScaleLayerId = "";
          this._state.activePointIconScaleValue = null;
          this.scheduleRender();
        });
        return;
      }
      const opacitySlider = event.target.closest("input[data-layer-opacity]");
      if (!opacitySlider) {
        return;
      }
      queueMicrotask(() => {
        this._state.activeOpacityLayerId = "";
        this._state.activeOpacityValue = null;
        this.scheduleRender();
      });
    };
    this._handleDragStart = (event) => {
      const handle = event.target.closest("button[data-layer-drag][draggable='true']");
      const card = handle?.closest(".fishymap-layer-card");
      if (!handle || !card) {
        return;
      }
      const layerId = String(card.getAttribute("data-layer-id") || "").trim();
      if (!layerId) {
        return;
      }
      this._state.draggingLayerId = layerId;
      card.dataset.dragging = "true";
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", layerId);
      }
    };
    this._handleDragOver = (event) => {
      if (!this._state.draggingLayerId) {
        return;
      }
      event.preventDefault();
      const card = event.target.closest(".fishymap-layer-card");
      if (!card) {
        this.clearDropState();
        return;
      }
      const targetLayerId = String(card.getAttribute("data-layer-id") || "").trim();
      if (!targetLayerId || targetLayerId === this._state.draggingLayerId) {
        this.clearDropState();
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
      this.applyDropState(targetLayerId, mode);
    };
    this._handleDrop = (event) => {
      if (!this._state.draggingLayerId || !this._state.overLayerId || !this._state.dropMode) {
        this.clearDropState();
        return;
      }
      event.preventDefault();
      const bundle = this.bundle();
      const nextOrder = moveLayerIdBefore(
        resolveLayerEntries(bundle),
        this._state.draggingLayerId,
        this._state.overLayerId,
        this._state.dropMode === "after" ? "after" : "before",
      );
      const nextClipMasks = buildLayerClipMaskPatch(
        bundle,
        this._state.draggingLayerId,
        this._state.dropMode === "attach" ? this._state.overLayerId : "",
      );
      this.writeBridgedFilters((filters) => {
        filters.layerIdsOrdered = nextOrder;
        filters.layerClipMasks = nextClipMasks;
      });
      this._state.draggingLayerId = "";
      this.clearDropState();
    };
    this._handleDragEnd = () => {
      this._state.draggingLayerId = "";
      this.querySelectorAll(".fishymap-layer-card[data-dragging]").forEach((card) => {
        delete card.dataset.dragging;
      });
      this.clearDropState();
    };
    this._handleDragLeave = (event) => {
      const related = event.relatedTarget;
      if (related instanceof Node && this.contains(related)) {
        return;
      }
      this.clearDropState();
    };
  }

  connectedCallback() {
    this._shell = this.closest?.("#map-page-shell") || null;
    this._count = this._shell?.querySelector?.("#fishymap-layers-count") || null;
    this._canvas = this._shell?.querySelector?.("#bevy") || null;
    this.addEventListener("click", this._handleClick);
    this.addEventListener("change", this._handleChange);
    this.addEventListener("input", this._handleInput);
    this.addEventListener("focusout", this._handleFocusOut);
    this.addEventListener("dragstart", this._handleDragStart);
    this.addEventListener("dragover", this._handleDragOver);
    this.addEventListener("drop", this._handleDrop);
    this.addEventListener("dragend", this._handleDragEnd);
    this.addEventListener("dragleave", this._handleDragLeave);
    this._canvas?.addEventListener?.("pointerleave", this._handleCanvasPointerLeave);
    this._signalPatchTarget =
      globalThis.document && typeof globalThis.document.addEventListener === "function"
        ? globalThis.document
        : this._shell;
    this._signalPatchTarget?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.addEventListener?.("fishymap:hover-changed", this._handleHoverChanged);
    this._shell?.addEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this.scheduleRender();
  }

  disconnectedCallback() {
    this.removeEventListener("click", this._handleClick);
    this.removeEventListener("change", this._handleChange);
    this.removeEventListener("input", this._handleInput);
    this.removeEventListener("focusout", this._handleFocusOut);
    this.removeEventListener("dragstart", this._handleDragStart);
    this.removeEventListener("dragover", this._handleDragOver);
    this.removeEventListener("drop", this._handleDrop);
    this.removeEventListener("dragend", this._handleDragEnd);
    this.removeEventListener("dragleave", this._handleDragLeave);
    this._canvas?.removeEventListener?.("pointerleave", this._handleCanvasPointerLeave);
    this._signalPatchTarget?.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.removeEventListener?.("fishymap:hover-changed", this._handleHoverChanged);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
  }

  signals() {
    return readMapShellSignals(this._shell);
  }

  bundle() {
    return buildLayerPanelStateBundle(this.signals());
  }

  dispatchPatch(patch) {
    dispatchShellSignalPatch(this._shell, patch);
  }

  hasExpandedLayerSettings() {
    return normalizeExpandedLayerIds(this.signals()?._map_ui?.layers?.expandedLayerIds).length > 0;
  }

  writeExpandedLayerIds(nextExpandedLayerIds) {
    this.dispatchPatch({
      _map_ui: {
        layers: {
          expandedLayerIds: normalizeExpandedLayerIds(nextExpandedLayerIds),
        },
      },
    });
    this.scheduleRender();
  }

  writeBridgedFilters(mutator) {
    const nextFilters = cloneJson(this.signals()?._map_bridged?.filters || {});
    mutator(nextFilters, this.signals());
    this.dispatchPatch({
      _map_bridged: {
        filters: nextFilters,
      },
    });
    this.scheduleRender();
  }

  clearDropState() {
    this._state.overLayerId = "";
    this._state.dropMode = "";
    this.querySelectorAll(".fishymap-layer-card[data-drop-position]").forEach((card) => {
      delete card.dataset.dropPosition;
    });
  }

  applyDropState(layerId, mode) {
    this._state.overLayerId = String(layerId || "");
    this._state.dropMode = String(mode || "");
    this.querySelectorAll(".fishymap-layer-card").forEach((card) => {
      if (card.getAttribute("data-layer-id") === this._state.overLayerId) {
        card.dataset.dropPosition = this._state.dropMode;
        return;
      }
      delete card.dataset.dropPosition;
    });
  }

  render() {
    this._rafId = 0;
    const liveSignals = this.signals();
    const bundle = buildLayerPanelStateBundle(liveSignals);
    const runtimeLayers = Array.isArray(liveSignals?._map_runtime?.catalog?.layers)
      ? liveSignals._map_runtime.catalog.layers
      : [];
    const expandedLayerIds = new Set(
      normalizeExpandedLayerIds(liveSignals?._map_ui?.layers?.expandedLayerIds),
    );

    const activeOpacityInteraction =
      this._state.activeOpacityLayerId &&
      Number.isFinite(this._state.activeOpacityValue) &&
      syncLayerOpacityControl(this, this._state.activeOpacityLayerId, this._state.activeOpacityValue);
    const activePointIconScaleInteraction =
      !activeOpacityInteraction &&
      this._state.activePointIconScaleLayerId &&
      Number.isFinite(this._state.activePointIconScaleValue) &&
      syncLayerPointIconScaleControl(
        this,
        this._state.activePointIconScaleLayerId,
        this._state.activePointIconScaleValue,
      );

    if (!activeOpacityInteraction && !activePointIconScaleInteraction) {
      renderLayerStack(
        this,
        bundle.state.ready === true ? bundle : { state: { catalog: { layers: [] } }, inputState: {} },
        {
          expandedLayerIds,
          hover: this._state.hover,
          selection: liveSignals?._map_runtime?.selection || {},
          zoneCatalog: this._zoneCatalog,
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

    setTextContent(this._count, bundle.state.ready === true ? runtimeLayers.length : 0);

    if (this._state.draggingLayerId) {
      const draggingCard = this.querySelector(
        `.fishymap-layer-card[data-layer-id="${CSS.escape(this._state.draggingLayerId)}"]`,
      );
      if (draggingCard) {
        draggingCard.dataset.dragging = "true";
      }
    }
    if (this._state.overLayerId && this._state.dropMode) {
      this.applyDropState(this._state.overLayerId, this._state.dropMode);
    }
  }

  scheduleRender() {
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    if (typeof globalThis.requestAnimationFrame === "function") {
      this._rafId = globalThis.requestAnimationFrame(() => {
        this.render();
      }) || 0;
      if (this._rafId) {
        return;
      }
    }
    this.render();
  }
}

export function registerFishyMapLayerPanelElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (registry.get(LAYER_PANEL_TAG_NAME)) {
    return true;
  }
  registry.define(LAYER_PANEL_TAG_NAME, FishyMapLayerPanelElement);
  return true;
}

registerFishyMapLayerPanelElement();
