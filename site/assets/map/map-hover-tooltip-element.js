import {
  buildHoverTooltipRows,
  patchTouchesHoverTooltipSignals,
} from "./map-hover-facts.js";
import { readMapShellSignals } from "./map-shell-signals.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const ICON_SPRITE_URL = "/img/icons.svg?v=20260419-2";
const HOVER_TOOLTIP_TAG_NAME = "fishymap-hover-tooltip";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

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

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  element[propertyName] = Boolean(value);
}

function setMarkup(element, renderKey, markup) {
  if (!element) {
    return;
  }
  const normalizedKey = String(renderKey ?? "");
  if (element.dataset.renderKey === normalizedKey) {
    return;
  }
  element.dataset.renderKey = normalizedKey;
  element.innerHTML = String(markup ?? "");
}

function spriteIcon(name, sizeClass = "size-4") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${escapeHtml(name)}"></use></svg>`;
}

function overviewRowMarkup(row) {
  const baseIcon = trimString(row?.icon || "information-circle");
  const swatchRgb = trimString(row?.swatchRgb);
  const statusIcon = trimString(row?.statusIcon);
  const statusIconTone = trimString(row?.statusIconTone);
  return `
    <div class="fishymap-overview-row">
      <span class="fishymap-overview-row-icon" aria-hidden="true">
        ${
          swatchRgb
            ? `<span class="fishymap-layer-fact-swatch" style="--fishymap-layer-fact-rgb:${escapeHtml(swatchRgb)};"></span>`
            : spriteIcon(baseIcon, "size-4")
        }
      </span>
      <span class="fishymap-overview-row-label">${escapeHtml(row?.label || "")}</span>
      <span class="fishymap-overview-row-value">
        ${escapeHtml(row?.value || "")}
        ${
          statusIcon
            ? `<span class="fishymap-overview-status ${
                statusIconTone === "subtle" ? "fishymap-overview-status--subtle" : ""
              }" aria-hidden="true">${spriteIcon(statusIcon, "size-4")}</span>`
            : ""
        }
      </span>
    </div>
  `;
}

function buildStateBundle(signals) {
  return {
    state: {
      catalog: {
        layers: Array.isArray(signals?._map_runtime?.catalog?.layers)
          ? cloneJson(signals._map_runtime.catalog.layers)
          : [],
      },
    },
    inputState: {
      filters: isPlainObject(signals?._map_bridged?.filters)
        ? cloneJson(signals._map_bridged.filters)
        : {},
    },
  };
}

function normalizeHoverEventDetail(detail) {
  if (isPlainObject(detail?.hover)) {
    return cloneJson(detail.hover);
  }
  return isPlainObject(detail) ? cloneJson(detail) : {};
}

export function readMapHoverTooltipShellSignals(shell) {
  return readMapShellSignals(shell);
}

function ensureHoverTooltipMarkup(host, documentRef = globalThis.document) {
  const existing = host.querySelector?.("#fishymap-hover-layers");
  if (existing) {
    return existing;
  }
  if (documentRef && typeof documentRef.createElement === "function") {
    const layers = documentRef.createElement("div");
    layers.id = "fishymap-hover-layers";
    layers.hidden = true;
    host.replaceChildren?.(layers);
    return layers;
  }
  host.innerHTML = '<div id="fishymap-hover-layers" hidden></div>';
  return host.querySelector?.("#fishymap-hover-layers") || null;
}

export class FishyMapHoverTooltipElement extends HTMLElementBase {
  constructor() {
    super();
    this._shell = null;
    this._canvas = null;
    this._rafId = 0;
    this._zoneCatalog = [];
    this._elements = null;
    this._state = {
      pointerActive: false,
      hover: null,
    };
    this._handleCanvasPointerMove = (event) => {
      this._state.pointerActive = true;
      this.writePointerPosition(event?.clientX ?? 0, event?.clientY ?? 0);
      if (this.hidden) {
        this.scheduleRender();
      }
    };
    this._handleCanvasPointerLeave = () => {
      this._state.pointerActive = false;
      setBooleanProperty(this._elements?.hoverLayers, "hidden", true);
      setBooleanProperty(this, "hidden", true);
    };
    this._handleHoverChanged = (event) => {
      this._state.hover = normalizeHoverEventDetail(event?.detail);
      this.scheduleRender();
    };
    this._handleSignalPatched = (event) => {
      if (patchTouchesHoverTooltipSignals(event?.detail)) {
        this.scheduleRender();
      }
    };
    this._handleZoneCatalogReady = (event) => {
      this._zoneCatalog = Array.isArray(event?.detail?.zoneCatalog)
        ? cloneJson(event.detail.zoneCatalog)
        : [];
      this.scheduleRender();
    };
  }

  connectedCallback() {
    this._shell = this.closest?.("#map-page-shell") || null;
    this._canvas = this._shell?.querySelector?.("#bevy") || globalThis.document?.getElementById?.("bevy") || null;
    const hoverLayers =
      ensureHoverTooltipMarkup(this, globalThis.document) || this.querySelector?.("#fishymap-hover-layers");
    this._elements = {
      hoverLayers,
    };
    this._canvas?.addEventListener?.("pointermove", this._handleCanvasPointerMove);
    this._canvas?.addEventListener?.("pointerleave", this._handleCanvasPointerLeave);
    this._shell?.addEventListener?.("fishymap:hover-changed", this._handleHoverChanged);
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this.render();
  }

  disconnectedCallback() {
    this._canvas?.removeEventListener?.("pointermove", this._handleCanvasPointerMove);
    this._canvas?.removeEventListener?.("pointerleave", this._handleCanvasPointerLeave);
    this._shell?.removeEventListener?.("fishymap:hover-changed", this._handleHoverChanged);
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    this._rafId = 0;
    this._shell = null;
    this._canvas = null;
    this._elements = null;
  }

  signals() {
    return readMapShellSignals(this._shell);
  }

  writePointerPosition(clientX, clientY) {
    this.style.setProperty("--fishymap-hover-x", `${Math.round(clientX)}px`);
    this.style.setProperty("--fishymap-hover-y", `${Math.round(clientY)}px`);
  }

  render() {
    this._rafId = 0;
    const rows = buildHoverTooltipRows({
      hover: this._state.hover,
      stateBundle: buildStateBundle(this.signals()),
      visibilityByLayer: this.signals()?._map_ui?.layers?.hoverFactsVisibleByLayer || {},
      zoneCatalog: this._zoneCatalog,
    });
    if (!this._state.pointerActive || rows.length === 0) {
      setMarkup(this._elements?.hoverLayers, "[]", "");
      setBooleanProperty(this._elements?.hoverLayers, "hidden", true);
      setBooleanProperty(this, "hidden", true);
      return;
    }
    setMarkup(
      this._elements?.hoverLayers,
      JSON.stringify(rows.map((row) => [row.layerId, row.key, row.value])),
      rows.map((row) => overviewRowMarkup(row)).join(""),
    );
    setBooleanProperty(this._elements?.hoverLayers, "hidden", false);
    setBooleanProperty(this, "hidden", false);
  }

  scheduleRender() {
    if (this._rafId) {
      return;
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

export function registerFishyMapHoverTooltipElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (registry.get(HOVER_TOOLTIP_TAG_NAME)) {
    return true;
  }
  registry.define(HOVER_TOOLTIP_TAG_NAME, FishyMapHoverTooltipElement);
  return true;
}

registerFishyMapHoverTooltipElement();
