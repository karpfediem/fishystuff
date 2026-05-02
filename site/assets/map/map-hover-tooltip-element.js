import {
  buildLayerHoverFactRows,
  patchTouchesLayerHoverFactSignals,
} from "./map-hover-facts.js";
import {
  buildLandmarkHoverRows,
  patchTouchesLandmarkHoverSignals,
} from "./map-hover-landmarks.js";
import { readMapShellSignals } from "./map-shell-signals.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const HOVER_TOOLTIP_TAG_NAME = "fishymap-hover-tooltip";
const HOVER_REST_SAMPLE_LIMIT = 3;
const HOVER_SAMPLE_ZONE_LIMIT = 3;
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
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-${escapeHtml(name)}"></use></svg>`;
}

function pointSampleZoneIndicatorMarkup(zone) {
  if (zone?.zoneKind === "partial") {
    const style = trimString(zone?.swatchRgb)
      ? ` style="--fishymap-layer-fact-rgb:${escapeHtml(zone.swatchRgb)};"`
      : "";
    return `<svg class="fishy-icon size-4 fishymap-point-sample-zone-icon" viewBox="0 0 24 24" aria-hidden="true"${style}><use width="100%" height="100%" href="#fishy-ring-partial"></use></svg>`;
  }
  return trimString(zone?.swatchRgb)
    ? `<span class="fishymap-layer-fact-swatch" style="--fishymap-layer-fact-rgb:${escapeHtml(zone.swatchRgb)};"></span>`
    : "";
}

function overviewRowMarkup(row) {
  const baseIcon = trimString(row?.icon || "information-circle");
  const swatchRgb = trimString(row?.swatchRgb);
  const statusIcon = trimString(row?.statusIcon);
  const statusIconTone = trimString(row?.statusIconTone);
  const rowKind = trimString(row?.kind);
  const kindClass = rowKind === "landmark-hover" ? " fishymap-landmark-hover-row" : "";
  const dataKind = rowKind ? ` data-hover-row-kind="${escapeHtml(rowKind)}"` : "";
  return `
    <div class="fishymap-overview-row${kindClass}"${dataKind}>
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

function itemGradeTone(grade, isPrize) {
  const resolver = globalThis.window?.__fishystuffItemPresentation?.resolveGradeTone;
  if (typeof resolver === "function") {
    return resolver(grade, isPrize);
  }
  const normalized = trimString(grade).toLowerCase();
  if (isPrize === true || normalized === "prize" || normalized === "red") {
    return "red";
  }
  switch (normalized) {
    case "rare":
    case "yellow":
      return "yellow";
    case "highquality":
    case "high_quality":
    case "high-quality":
    case "blue":
      return "blue";
    case "general":
    case "green":
      return "green";
    case "trash":
    case "white":
      return "white";
    default:
      return "unknown";
  }
}

function itemIconMarkup(row, size = "is-xs") {
  const name = trimString(row?.fishName) || "Unknown fish";
  const gradeTone = itemGradeTone(row?.grade, row?.isPrize === true);
  const toneClass = `fishy-item-grade-${escapeHtml(gradeTone)}`;
  const iconUrl = trimString(row?.iconUrl);
  return iconUrl
    ? `<span class="fishy-item-icon-frame ${escapeHtml(size)} ${toneClass}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishy-item-icon-frame ${escapeHtml(size)} ${toneClass}"><span class="fishy-item-icon-fallback ${toneClass}">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
}

function pointSampleZoneMarkup(row) {
  const zones = Array.isArray(row?.zones) ? row.zones : [];
  if (!zones.length) {
    return "";
  }
  const visibleZones = zones.slice(0, HOVER_SAMPLE_ZONE_LIMIT);
  const hiddenZoneCount = Math.max(0, zones.length - visibleZones.length);
  return `
    <div class="fishymap-point-sample-zones">
      <span class="fishymap-point-sample-zone-list">
        ${visibleZones
          .map((zone) => `
            <span class="fishymap-point-sample-zone">
              ${pointSampleZoneIndicatorMarkup(zone)}
              <span class="truncate">${escapeHtml(zone?.name || "")}</span>
            </span>
          `)
          .join("")}
        ${
          hiddenZoneCount
            ? `<span class="fishymap-point-sample-zone fishymap-point-sample-zone-more">+${hiddenZoneCount} zones</span>`
            : ""
        }
      </span>
    </div>
  `;
}

function pointSampleMarkup(row) {
  const name = trimString(row?.fishName) || "Unknown fish";
  const count = Math.max(1, Number.parseInt(row?.sampleCount, 10) || 1);
  const sampleBadge = count > 1 ? `x${count}` : "";
  return `
    <div class="fishymap-point-sample-card" data-zone-kind="${escapeHtml(row?.zoneKind || "")}">
      <div class="fishymap-point-sample-main">
        <span class="fishy-item-row min-w-0">
          ${itemIconMarkup(row, "is-native")}
          <span class="fishymap-point-sample-fish min-w-0">
            <span class="fishymap-point-sample-name truncate">${escapeHtml(name)}</span>
          </span>
        </span>
        ${sampleBadge ? `<span class="badge badge-soft badge-sm">${escapeHtml(sampleBadge)}</span>` : ""}
      </div>
      ${
        trimString(row?.dateText)
          ? `<div class="fishymap-point-sample-date">${spriteIcon("date-confirmed", "size-4")}<span>${escapeHtml(row.dateText)}</span></div>`
          : ""
      }
      ${pointSampleZoneMarkup(row)}
    </div>
  `;
}

function pointSampleBadgeText(row) {
  const count = Math.max(1, Number.parseInt(row?.sampleCount, 10) || 1);
  return `x${count}`;
}

function remainingPointRows(rows) {
  const byFish = new Map();
  for (const row of rows) {
    const fishId = Number.parseInt(row?.fishId, 10);
    const itemId = Number.parseInt(row?.itemId, 10);
    const key = [
      Number.isInteger(fishId) ? fishId : "",
      Number.isInteger(itemId) ? itemId : "",
      trimString(row?.fishName),
      trimString(row?.iconUrl),
    ].join(":");
    const current = byFish.get(key);
    const sampleCount = Math.max(1, Number.parseInt(row?.sampleCount, 10) || 1);
    if (current) {
      current.sampleCount += sampleCount;
    } else {
      byFish.set(key, { ...row, sampleCount });
    }
  }
  const sortedRows = [...byFish.values()].sort(
    (left, right) =>
      Math.max(1, Number.parseInt(right?.sampleCount, 10) || 1) -
        Math.max(1, Number.parseInt(left?.sampleCount, 10) || 1) ||
      trimString(left?.fishName).localeCompare(trimString(right?.fishName)),
  );
  const visibleRows = sortedRows.slice(0, HOVER_REST_SAMPLE_LIMIT);
  const hiddenSampleCount = sortedRows
    .slice(HOVER_REST_SAMPLE_LIMIT)
    .reduce((total, row) => total + Math.max(1, Number.parseInt(row?.sampleCount, 10) || 1), 0);
  return { hiddenSampleCount, visibleRows };
}

function remainingPointSampleMarkup(row) {
  const name = trimString(row?.fishName) || "Unknown fish";
  const badge = pointSampleBadgeText(row);
  const title = badge ? `${name} ${badge}` : name;
  return `
    <span class="fishymap-point-sample-rest-item" title="${escapeHtml(title)}">
      ${itemIconMarkup(row, "is-hover-rest")}
      ${badge ? `<span class="fishymap-point-sample-rest-count">${escapeHtml(badge)}</span>` : ""}
    </span>
  `;
}

function pointSampleGroupMarkup(pointRows) {
  if (!pointRows.length) {
    return "";
  }
  const [topRow, ...remainingRows] = pointRows;
  const { hiddenSampleCount, visibleRows: restRows } = remainingPointRows(remainingRows);
  return `
    <div class="fishymap-point-sample-group">
      ${pointSampleMarkup(topRow)}
      ${
        restRows.length
          ? `<div class="fishymap-point-sample-rest">${restRows.map((row) => remainingPointSampleMarkup(row)).join("")}${
              hiddenSampleCount > 0
                ? `<span class="fishymap-point-sample-rest-more" title="${escapeHtml(`${hiddenSampleCount} more samples`)}">+${escapeHtml(hiddenSampleCount)}</span>`
                : ""
            }</div>`
          : ""
      }
    </div>
  `;
}

function landmarkHoverInfoMarkup(landmarkRows) {
  const pointRows = landmarkRows.filter((row) => row?.kind === "point-sample");
  const targetRows = landmarkRows.filter((row) => row?.kind !== "point-sample");
  return [
    pointSampleGroupMarkup(pointRows),
    targetRows.length
      ? `<div class="fishymap-landmark-hover-list">${targetRows.map((row) => overviewRowMarkup(row)).join("")}</div>`
      : "",
  ].join("");
}

function buildStateBundle(signals) {
  return {
    state: {
      catalog: {
        layers: Array.isArray(signals?._map_runtime?.catalog?.layers)
          ? cloneJson(signals._map_runtime.catalog.layers)
          : [],
        fish: Array.isArray(signals?._map_runtime?.catalog?.fish)
          ? cloneJson(signals._map_runtime.catalog.fish)
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
  const existingLayers = host.querySelector?.("#fishymap-hover-layers");
  const existingInfo = host.querySelector?.("#fishymap-hover-info");
  if (existingLayers && existingInfo) {
    return {
      hoverLayers: existingLayers,
      hoverInfo: existingInfo,
    };
  }
  if (documentRef && typeof documentRef.createElement === "function") {
    const layers = documentRef.createElement("div");
    layers.id = "fishymap-hover-layers";
    layers.hidden = true;
    const info = documentRef.createElement("div");
    info.id = "fishymap-hover-info";
    info.hidden = true;
    host.replaceChildren?.(layers, info);
    return {
      hoverLayers: layers,
      hoverInfo: info,
    };
  }
  host.innerHTML = '<div id="fishymap-hover-layers" hidden></div><div id="fishymap-hover-info" hidden></div>';
  return {
    hoverLayers: host.querySelector?.("#fishymap-hover-layers") || null,
    hoverInfo: host.querySelector?.("#fishymap-hover-info") || null,
  };
}

export class FishyMapHoverTooltipElement extends HTMLElementBase {
  constructor() {
    super();
    this._shell = null;
    this._canvas = null;
    this._rafId = 0;
    this._pointerRafId = 0;
    this._zoneCatalog = [];
    this._elements = null;
    this._state = {
      pointerActive: false,
      pointerX: 0,
      pointerY: 0,
      pointerDirty: false,
      hover: null,
    };
    this._handleCanvasPointerMove = (event) => {
      this._state.pointerActive = true;
      this.setPointerPosition(event?.clientX ?? 0, event?.clientY ?? 0);
      if (this.hidden) {
        this.scheduleRender();
      } else {
        this.schedulePointerPositionWrite();
      }
    };
    this._handleCanvasPointerLeave = () => {
      this._state.pointerActive = false;
      this._state.pointerDirty = false;
      if (this._pointerRafId && typeof globalThis.cancelAnimationFrame === "function") {
        globalThis.cancelAnimationFrame(this._pointerRafId);
      }
      this._pointerRafId = 0;
      setBooleanProperty(this._elements?.hoverLayers, "hidden", true);
      setBooleanProperty(this._elements?.hoverInfo, "hidden", true);
      setBooleanProperty(this, "hidden", true);
    };
    this._handleHoverChanged = (event) => {
      this._state.hover = normalizeHoverEventDetail(event?.detail);
      this.scheduleRender();
    };
    this._handleSignalPatched = (event) => {
      if (
        patchTouchesLayerHoverFactSignals(event?.detail) ||
        patchTouchesLandmarkHoverSignals(event?.detail)
      ) {
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
    this._elements = ensureHoverTooltipMarkup(this, globalThis.document);
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
    if (this._pointerRafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._pointerRafId);
    }
    this._rafId = 0;
    this._pointerRafId = 0;
    this._shell = null;
    this._canvas = null;
    this._elements = null;
  }

  signals() {
    return readMapShellSignals(this._shell);
  }

  setPointerPosition(clientX, clientY) {
    const pointerX = Math.round(clientX);
    const pointerY = Math.round(clientY);
    if (this._state.pointerX === pointerX && this._state.pointerY === pointerY) {
      return;
    }
    this._state.pointerX = pointerX;
    this._state.pointerY = pointerY;
    this._state.pointerDirty = true;
  }

  writePointerPosition() {
    if (!this._state.pointerDirty) {
      return;
    }
    this._state.pointerDirty = false;
    const x = this._state.pointerX;
    const y = this._state.pointerY;
    if (this._elements?.hoverLayers?.style) {
      this._elements.hoverLayers.style.transform = `translate3d(${x + 18}px, ${y + 22}px, 0)`;
    }
    if (this._elements?.hoverInfo?.style) {
      this._elements.hoverInfo.style.transform = `translate3d(calc(${x}px - 50%), calc(${y}px - 100% - 18px), 0)`;
    }
  }

  schedulePointerPositionWrite() {
    if (this._pointerRafId) {
      return;
    }
    if (typeof globalThis.requestAnimationFrame === "function") {
      this._pointerRafId = globalThis.requestAnimationFrame(() => {
        this._pointerRafId = 0;
        if (this._state.pointerActive && !this.hidden) {
          this.writePointerPosition();
        }
      }) || 0;
      if (this._pointerRafId) {
        return;
      }
    }
    if (this._state.pointerActive && !this.hidden) {
      this.writePointerPosition();
    }
  }

  render() {
    this._rafId = 0;
    const signals = this.signals();
    const stateBundle = buildStateBundle(signals);
    const landmarkRows = buildLandmarkHoverRows({
      hover: this._state.hover,
      stateBundle,
      pointSamplesEnabled:
        signals?._map_ui?.layers?.sampleHoverVisibleByLayer?.fish_evidence !== false,
      zoneCatalog: this._zoneCatalog,
    });
    const factRows = buildLayerHoverFactRows({
      hover: this._state.hover,
      stateBundle,
      visibilityByLayer: signals?._map_ui?.layers?.hoverFactsVisibleByLayer || {},
      zoneCatalog: this._zoneCatalog,
    });
    if (!this._state.pointerActive || (landmarkRows.length === 0 && factRows.length === 0)) {
      setMarkup(this._elements?.hoverLayers, "[]", "");
      setMarkup(this._elements?.hoverInfo, "[]", "");
      setBooleanProperty(this._elements?.hoverLayers, "hidden", true);
      setBooleanProperty(this._elements?.hoverInfo, "hidden", true);
      setBooleanProperty(this, "hidden", true);
      delete this.dataset.landmarkHover;
      delete this.dataset.pointSamples;
      return;
    }
    const pointRows = landmarkRows.filter((row) => row?.kind === "point-sample");
    if (landmarkRows.length) {
      this.dataset.landmarkHover = "true";
    } else {
      delete this.dataset.landmarkHover;
    }
    if (pointRows.length) {
      this.dataset.pointSamples = "true";
    } else {
      delete this.dataset.pointSamples;
    }
    this.writePointerPosition();
    setMarkup(
      this._elements?.hoverLayers,
      JSON.stringify(factRows.map((row) => [row.layerId, row.key, row.value])),
      factRows.map((row) => overviewRowMarkup(row)).join(""),
    );
    setMarkup(
      this._elements?.hoverInfo,
      JSON.stringify(landmarkRows.map((row) => [row.kind, row.key, row.value || row.fishName || ""])),
      landmarkHoverInfoMarkup(landmarkRows),
    );
    setBooleanProperty(this._elements?.hoverLayers, "hidden", factRows.length === 0);
    setBooleanProperty(this._elements?.hoverInfo, "hidden", landmarkRows.length === 0);
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
