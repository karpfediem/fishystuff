import FishyMapBridge, {
  FISHYMAP_EVENTS,
  FISHYMAP_POINT_ICON_SCALE_MAX,
  FISHYMAP_POINT_ICON_SCALE_MIN,
  resolveApiBaseUrl,
} from "./map-host.js";

const FIXED_GROUND_LAYER_IDS = new Set(["minimap"]);

function dispatchMapEvent(target, type, detail) {
  target.dispatchEvent(new CustomEvent(type, { detail }));
}

function dispatchMapState(target, patch) {
  dispatchMapEvent(target, FISHYMAP_EVENTS.setState, patch);
}

function dispatchMapCommand(target, command) {
  dispatchMapEvent(target, FISHYMAP_EVENTS.command, command);
}

function supportsWebgl2(doc = document) {
  const probe = doc?.createElement?.("canvas");
  if (!probe?.getContext) {
    return false;
  }
  try {
    return !!probe.getContext("webgl2");
  } catch (_) {
    return false;
  }
}

function formatLoaderError(error) {
  if (!error) {
    return "Unknown renderer error.";
  }
  if (typeof error === "string") {
    return error;
  }
  if (typeof error === "object") {
    if (typeof error.stack === "string" && error.stack.trim()) {
      return error.stack;
    }
    if (typeof error.message === "string" && error.message.trim()) {
      return error.message;
    }
    if (typeof error.reason === "object" || typeof error.reason === "string") {
      return formatLoaderError(error.reason);
    }
  }
  return String(error);
}

function shouldHandleRendererError(error, fallbackMessage = "") {
  const text = `${formatLoaderError(error)} ${fallbackMessage}`.toLowerCase();
  return (
    text.includes("fishystuff_ui_bevy") ||
    text.includes("wgpu surface") ||
    text.includes("webgl2") ||
    text.includes("renderer/mod.rs") ||
    text.includes("canvas.getcontext")
  );
}

function setMapError(elements, error) {
  const message = formatLoaderError(error);
  elements.readyPill.textContent = "Error";
  elements.readyPill.className = "badge badge-error badge-sm";
  elements.statusLines.innerHTML = "";
  const status = document.createElement("p");
  status.textContent = "The map renderer failed to start.";
  elements.statusLines.appendChild(status);
  elements.diagnosticJson.textContent = message;
  if (elements.errorMessage) {
    elements.errorMessage.textContent = message;
  }
  if (elements.errorOverlay) {
    elements.errorOverlay.hidden = false;
  }
  if (elements.canvas) {
    elements.canvas.hidden = true;
  }
}

function installRendererErrorHandlers(elements) {
  const onError = (event) => {
    if (!shouldHandleRendererError(event?.error, event?.message || event?.filename || "")) {
      return;
    }
    FishyMapBridge.destroy?.();
    setMapError(elements, event?.error || event?.message || event);
  };
  const onRejection = (event) => {
    if (!shouldHandleRendererError(event?.reason)) {
      return;
    }
    FishyMapBridge.destroy?.();
    setMapError(elements, event?.reason || event);
  };
  window.addEventListener("error", onError);
  window.addEventListener("unhandledrejection", onRejection);
}

function requestBridgeState(target) {
  const detail = {};
  dispatchMapEvent(target, FISHYMAP_EVENTS.requestState, detail);
  return {
    state: detail.state || FishyMapBridge.getCurrentState(),
    inputState:
      detail.inputState ||
      (typeof FishyMapBridge.getCurrentInputState === "function"
        ? FishyMapBridge.getCurrentInputState()
        : {}),
  };
}

function applyThemeToShell(shell) {
  if (!shell) {
    return;
  }
  const background =
    window.__fishystuffTheme?.colors?.base100 ||
    window.getComputedStyle(document.documentElement).getPropertyValue("--color-base-100");
  if (background) {
    shell.style.backgroundColor = background.trim();
  }
}

function setTextContent(element, text) {
  if (!element) {
    return;
  }
  const nextText = String(text ?? "");
  if (element.textContent !== nextText) {
    element.textContent = nextText;
  }
}

function setClassName(element, className) {
  if (!element) {
    return;
  }
  if (element.className !== className) {
    element.className = className;
  }
}

function setAttributeValue(element, name, value) {
  if (!element) {
    return;
  }
  const nextValue = String(value ?? "");
  if (element.getAttribute(name) !== nextValue) {
    element.setAttribute(name, nextValue);
  }
}

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  const nextValue = Boolean(value);
  if (element[propertyName] !== nextValue) {
    element[propertyName] = nextValue;
  }
}

function setMarkup(element, renderKey, html) {
  if (!element) {
    return false;
  }
  const nextKey = String(renderKey ?? "");
  if (element.dataset.markupKey === nextKey) {
    return false;
  }
  element.dataset.markupKey = nextKey;
  element.innerHTML = html;
  return true;
}

function buildFishLookup(catalogFish) {
  const map = new Map();
  for (const fish of catalogFish || []) {
    map.set(fish.fishId, fish);
    if (Number.isFinite(fish.itemId)) {
      map.set(fish.itemId, fish);
    }
  }
  return map;
}

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function hasOwnKey(object, key) {
  return !!object && Object.prototype.hasOwnProperty.call(object, key);
}

function mergeZoneEvidenceIntoFishLookup(fishLookup, zoneStats) {
  const distribution = Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : [];
  for (const entry of distribution) {
    const fishId = Number.parseInt(entry?.fishId, 10);
    if (!Number.isFinite(fishId)) {
      continue;
    }
    const existing = fishLookup.get(fishId) || {};
    fishLookup.set(fishId, {
      fishId,
      itemId: existing.itemId,
      name: entry.fishName || existing.name || `Fish ${fishId}`,
      iconUrl: entry.iconUrl || existing.iconUrl || "",
      isPrize: existing.isPrize || false,
    });
  }
  return fishLookup;
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

function fishIconUrl(fish) {
  const value = fish?.iconUrl;
  if (typeof value !== "string" || !value.trim()) {
    return "";
  }
  const raw = value.trim();
  if (raw.startsWith("http://") || raw.startsWith("https://") || raw.startsWith("data:")) {
    return raw;
  }
  if (raw.startsWith("/")) {
    return `${resolveApiBaseUrl(window.location)}${raw}`;
  }
  return raw;
}

function clampPointIconScale(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return FISHYMAP_POINT_ICON_SCALE_MIN;
  }
  return Math.min(FISHYMAP_POINT_ICON_SCALE_MAX, Math.max(FISHYMAP_POINT_ICON_SCALE_MIN, number));
}

function pointIconScaleValue(scale) {
  return String(Math.round(clampPointIconScale(scale) * 100) / 100);
}

function pointIconScaleLabel(scale) {
  return `${Math.round(clampPointIconScale(scale) * 100)}%`;
}

function clampLayerOpacity(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return 1;
  }
  return Math.min(1, Math.max(0, number));
}

function layerOpacityValue(opacity) {
  return String(Math.round(clampLayerOpacity(opacity) * 100) / 100);
}

function layerOpacityLabel(opacity) {
  return `${Math.round(clampLayerOpacity(opacity) * 100)}%`;
}

function isFixedGroundLayer(layerId) {
  return FIXED_GROUND_LAYER_IDS.has(String(layerId || "").trim());
}

function layerKindLabel(kind) {
  if (kind === "vector-geojson") {
    return "Vector";
  }
  if (kind === "tiled-raster") {
    return "Raster";
  }
  return "Layer";
}

function dragHandleIcon() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M9 5.25a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM9 12a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM9 18.75a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM18 5.25a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM18 12a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM18 18.75a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Z" />
    </svg>
  `;
}

function eyeIcon(visible) {
  if (visible) {
    return `
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true">
        <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" />
        <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
      </svg>
    `;
  }
  return `
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M3.53 2.47a.75.75 0 0 0-1.06 1.06l18 18a.75.75 0 1 0 1.06-1.06l-18-18ZM22.676 12.553a11.249 11.249 0 0 1-2.631 4.31l-3.099-3.099a5.25 5.25 0 0 0-6.71-6.71L7.759 4.577a11.217 11.217 0 0 1 4.242-.827c4.97 0 9.185 3.223 10.675 7.69.12.362.12.752 0 1.113Z" />
      <path d="M15.75 12c0 .18-.013.357-.037.53l-4.244-4.243A3.75 3.75 0 0 1 15.75 12ZM12.53 15.713l-4.243-4.244a3.75 3.75 0 0 0 4.244 4.243Z" />
      <path d="M6.75 12c0-.619.107-1.213.304-1.764l-3.1-3.1a11.25 11.25 0 0 0-2.63 4.31c-.12.362-.12.752 0 1.114 1.489 4.467 5.704 7.69 10.675 7.69 1.5 0 2.933-.294 4.242-.827l-2.477-2.477A5.25 5.25 0 0 1 6.75 12Z" />
    </svg>
  `;
}

function mapViewIcon() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true">
      <path stroke-linecap="round" stroke-linejoin="round" d="M9 6.75V15m6-6v8.25m.503 3.498 4.875-2.437c.381-.19.622-.58.622-1.006V4.82c0-.836-.88-1.38-1.628-1.006l-3.869 1.934c-.317.159-.69.159-1.006 0L9.503 3.252a1.125 1.125 0 0 0-1.006 0L3.622 5.689C3.24 5.88 3 6.27 3 6.695V19.18c0 .836.88 1.38 1.628 1.006l3.869-1.934c.317-.159.69-.159 1.006 0l4.994 2.497c.317.158.69.158 1.006 0Z" />
    </svg>
  `;
}

function cubeViewIcon() {
  return `
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true">
      <path stroke-linecap="round" stroke-linejoin="round" d="m21 7.5-9-5.25L3 7.5m18 0-9 5.25m9-5.25v9l-9 5.25M3 7.5l9 5.25M3 7.5v9l9 5.25m0-9v9" />
    </svg>
  `;
}

function resolveLayerEntries(stateBundle) {
  const layers = Array.isArray(stateBundle.state?.catalog?.layers)
    ? stateBundle.state.catalog.layers.slice()
    : [];
  const orderedIds = Array.isArray(stateBundle.inputState?.filters?.layerIdsOrdered)
    ? stateBundle.inputState.filters.layerIdsOrdered
    : Array.isArray(stateBundle.state?.filters?.layerIdsOrdered)
      ? stateBundle.state.filters.layerIdsOrdered
      : [];
  const visibleOverride = Array.isArray(stateBundle.inputState?.filters?.layerIdsVisible)
    ? new Set(stateBundle.inputState.filters.layerIdsVisible)
    : null;
  const inputOpacityOverride = isPlainObject(stateBundle.inputState?.filters?.layerOpacities)
    ? stateBundle.inputState.filters.layerOpacities
    : null;
  const stateOpacityOverride = isPlainObject(stateBundle.state?.filters?.layerOpacities)
    ? stateBundle.state.filters.layerOpacities
    : null;
  const byId = new Map(layers.map((layer) => [layer.layerId, layer]));
  const seen = new Set();
  const movable = [];
  const pinned = [];

  const pushLayer = (layer) => {
    if (!layer || seen.has(layer.layerId)) {
      return;
    }
    seen.add(layer.layerId);
    const visible = visibleOverride ? visibleOverride.has(layer.layerId) : Boolean(layer.visible);
    const opacityDefault = clampLayerOpacity(layer.opacityDefault ?? 1);
    let opacity = clampLayerOpacity(layer.opacity);
    if (inputOpacityOverride) {
      opacity = hasOwnKey(inputOpacityOverride, layer.layerId)
        ? clampLayerOpacity(inputOpacityOverride[layer.layerId])
        : opacityDefault;
    } else if (stateOpacityOverride && hasOwnKey(stateOpacityOverride, layer.layerId)) {
      opacity = clampLayerOpacity(stateOpacityOverride[layer.layerId]);
    }
    const entry = {
      ...layer,
      visible,
      opacity,
      opacityDefault,
      locked: isFixedGroundLayer(layer.layerId),
    };
    if (entry.locked) {
      pinned.push(entry);
    } else {
      movable.push(entry);
    }
  };

  for (const layerId of orderedIds) {
    pushLayer(byId.get(layerId));
  }

  const fallback = layers.slice().sort((left, right) => {
    const leftOrder = Number.isFinite(left?.displayOrder) ? left.displayOrder : 0;
    const rightOrder = Number.isFinite(right?.displayOrder) ? right.displayOrder : 0;
    return rightOrder - leftOrder || String(left?.layerId || "").localeCompare(String(right?.layerId || ""));
  });
  for (const layer of fallback) {
    pushLayer(layer);
  }

  return movable.concat(pinned);
}

function resolveVisibleLayerIds(stateBundle) {
  return resolveLayerEntries(stateBundle)
    .filter((layer) => layer.visible)
    .map((layer) => layer.layerId);
}

function moveLayerIdBefore(entries, draggedLayerId, targetLayerId, position) {
  const movableIds = entries.filter((layer) => !layer.locked).map((layer) => layer.layerId);
  const fromIndex = movableIds.indexOf(draggedLayerId);
  const targetIndex = movableIds.indexOf(targetLayerId);
  if (fromIndex < 0 || targetIndex < 0) {
    return movableIds;
  }
  const [dragged] = movableIds.splice(fromIndex, 1);
  const insertIndex = position === "after" ? targetIndex + (fromIndex < targetIndex ? 0 : 1) : targetIndex + (fromIndex < targetIndex ? -1 : 0);
  movableIds.splice(Math.max(0, insertIndex), 0, dragged);
  const pinnedIds = entries.filter((layer) => layer.locked).map((layer) => layer.layerId);
  return movableIds.concat(pinnedIds);
}

function buildLayerOpacityPatch(stateBundle, targetLayerId, opacity) {
  const nextOpacities = {};
  for (const layer of resolveLayerEntries(stateBundle)) {
    if (layer.locked) {
      continue;
    }
    const effectiveOpacity =
      layer.layerId === targetLayerId ? clampLayerOpacity(opacity) : clampLayerOpacity(layer.opacity);
    if (Math.abs(effectiveOpacity - clampLayerOpacity(layer.opacityDefault)) <= 0.0001) {
      continue;
    }
    nextOpacities[layer.layerId] = effectiveOpacity;
  }
  return nextOpacities;
}

function renderViewState(elements, state) {
  const viewMode = state?.view?.viewMode === "3d" ? "3d" : "2d";
  if (elements.viewReadout) {
    setTextContent(elements.viewReadout, viewMode === "3d" ? "3D" : "2D");
  }
  if (elements.viewToggle) {
    const nextMode = viewMode === "3d" ? "2D" : "3D";
    const currentMode = viewMode === "3d" ? "3D" : "2D";
    const label = `View mode: ${currentMode}. Click to switch to ${nextMode}.`;
    setAttributeValue(elements.viewToggle, "aria-label", label);
    setAttributeValue(elements.viewToggle, "title", label);
  }
  if (elements.viewToggleIcon) {
    setMarkup(
      elements.viewToggleIcon,
      viewMode,
      viewMode === "3d" ? cubeViewIcon() : mapViewIcon(),
    );
  }
}

function setHoverTooltipPosition(elements, clientX, clientY) {
  if (!elements.hoverTooltip) {
    return;
  }
  elements.hoverTooltip.style.setProperty("--fishymap-hover-x", `${clientX}px`);
  elements.hoverTooltip.style.setProperty("--fishymap-hover-y", `${clientY}px`);
}

function renderHoverTooltip(elements, hover) {
  if (!elements.hoverTooltip || !elements.hoverSummary) {
    return;
  }
  const label = hover?.zoneName || (hover?.zoneRgb != null ? formatZone(hover.zoneRgb) : null);
  if (!label || !elements.hoverPointerActive) {
    setBooleanProperty(elements.hoverTooltip, "hidden", true);
    return;
  }
  setTextContent(elements.hoverSummary, label);
  setBooleanProperty(elements.hoverTooltip, "hidden", false);
}

function hoverFromEventDetail(detail) {
  if (detail?.hover && typeof detail.hover === "object") {
    return detail.hover;
  }
  return {
    worldX: detail?.worldX ?? null,
    worldZ: detail?.worldZ ?? null,
    zoneRgb: detail?.zoneRgb ?? null,
    zoneName: detail?.zoneName ?? null,
  };
}

function renderFishAvatar(fish, sizeClass = "size-6") {
  const name = fish?.name || `Fish ${fish?.fishId ?? "?"}`;
  const iconUrl = fishIconUrl(fish);
  if (iconUrl) {
    return `
      <span class="${sizeClass} shrink-0 overflow-hidden rounded-full bg-base-200 ring-1 ring-base-300/80">
        <img
          class="h-full w-full object-cover"
          src="${escapeHtml(iconUrl)}"
          alt="${escapeHtml(name)}"
          loading="lazy"
          decoding="async"
        />
      </span>
    `;
  }
  const fallback = escapeHtml(String(name).trim().charAt(0).toUpperCase() || "?");
  return `
    <span class="${sizeClass} inline-flex shrink-0 items-center justify-center rounded-full bg-base-300 text-[11px] font-semibold text-base-content/70">
      ${fallback}
    </span>
  `;
}

function resolveSelectedFishIds(stateBundle) {
  const inputFishIds = stateBundle.inputState?.filters?.fishIds;
  if (Array.isArray(inputFishIds)) {
    return inputFishIds;
  }
  const stateFishIds = stateBundle.state?.filters?.fishIds;
  if (Array.isArray(stateFishIds)) {
    return stateFishIds;
  }
  return [];
}

function scoreFishMatch(fish, queryTerms) {
  if (!queryTerms.length) {
    return 0;
  }
  const name = String(fish.name || "").toLowerCase();
  const id = String(fish.fishId || "");
  let score = 0;
  for (const term of queryTerms) {
    if (id === term) {
      score += 200;
      continue;
    }
    const idIndex = id.indexOf(term);
    if (idIndex >= 0) {
      score += 120 - idIndex;
      continue;
    }
    const nameIndex = name.indexOf(term);
    if (nameIndex >= 0) {
      score += 90 - Math.min(nameIndex, 60);
      continue;
    }
    return Number.NEGATIVE_INFINITY;
  }
  return score;
}

function findFishMatches(catalogFish, searchText) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!terms.length) {
    return [];
  }
  const filtered = [];
  for (const fish of catalogFish || []) {
    const score = scoreFishMatch(fish, terms);
    if (!terms.length || Number.isFinite(score)) {
      filtered.push({
        ...fish,
        _score: Number.isFinite(score) ? score : 0,
      });
    }
  }
  filtered.sort((left, right) => {
    if (terms.length && right._score !== left._score) {
      return right._score - left._score;
    }
    return String(left.name || "").localeCompare(String(right.name || ""));
  });
  return filtered;
}

function formatZone(zoneRgb) {
  if (zoneRgb == null) {
    return "none";
  }
  return `0x${Number(zoneRgb).toString(16).padStart(6, "0")}`;
}

function formatPatchDate(startTsUtc) {
  const tsMs = Number(startTsUtc) * 1000;
  if (!Number.isFinite(tsMs)) {
    return "";
  }
  const date = new Date(tsMs);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  return `${year}/${month}/${day}`;
}

function formatTimestampUtc(tsUtc) {
  const tsMs = Number(tsUtc) * 1000;
  if (!Number.isFinite(tsMs) || tsMs <= 0) {
    return "";
  }
  const date = new Date(tsMs);
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function formatDecimal(value, digits = 3) {
  const number = Number(value);
  return Number.isFinite(number) ? number.toFixed(digits) : "n/a";
}

function formatPercent(value, digits = 1) {
  const number = Number(value);
  return Number.isFinite(number) ? `${(number * 100).toFixed(digits)}%` : "n/a";
}

function formatZoneStatus(status) {
  const raw = String(status || "").trim();
  if (!raw) {
    return "Unknown";
  }
  return raw
    .toLowerCase()
    .split(/[_\s-]+/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function buildZoneEvidenceSummary(zoneStats) {
  if (!zoneStats) {
    return "Click a zone on the map to load evidence.";
  }
  const parts = [];
  const confidence = zoneStats.confidence || {};
  const status = formatZoneStatus(confidence.status);
  if (status) {
    parts.push(status);
  }
  if (Number.isFinite(confidence.ess)) {
    parts.push(`ESS ${formatDecimal(confidence.ess, 1)}`);
  }
  if (Number.isFinite(confidence.totalWeight)) {
    parts.push(`weight ${formatDecimal(confidence.totalWeight, 2)}`);
  }
  const lastSeen = formatTimestampUtc(confidence.lastSeenTsUtc);
  if (lastSeen) {
    parts.push(`last seen ${lastSeen}`);
  } else if (Number.isFinite(confidence.ageDaysLast)) {
    parts.push(`last seen ${formatDecimal(confidence.ageDaysLast, 1)}d ago`);
  }
  const drift = confidence.drift;
  if (drift && Number.isFinite(drift.pDrift)) {
    parts.push(`drift ${formatPercent(drift.pDrift, 1)}`);
  }
  if (Array.isArray(confidence.notes) && confidence.notes.length) {
    parts.push(confidence.notes.join(" · "));
  }
  return parts.join(" · ") || "No confidence data.";
}

function ensureZoneEvidenceElements(elements) {
  if (elements.zoneEvidenceStatus && elements.zoneEvidenceSummary && elements.zoneEvidenceList) {
    return elements;
  }
  if (!elements.panelBody) {
    return elements;
  }

  const section = document.createElement("div");
  section.className = "space-y-2";
  section.innerHTML = `
    <div class="flex items-center justify-between gap-3">
      <span class="text-sm font-semibold">Zone evidence</span>
      <span id="fishymap-zone-evidence-status" class="text-xs text-base-content/60">zone stats: idle</span>
    </div>
    <p id="fishymap-zone-evidence-summary" class="text-xs text-base-content/70">Click a zone on the map to load evidence.</p>
    <div id="fishymap-zone-evidence-list" class="max-h-72 overflow-y-auto rounded-box border border-base-300/70 bg-base-200/60 p-2"></div>
  `;

  if (elements.legend?.parentNode === elements.panelBody) {
    elements.panelBody.insertBefore(section, elements.legend);
  } else {
    elements.panelBody.appendChild(section);
  }

  elements.zoneEvidenceStatus = section.querySelector("#fishymap-zone-evidence-status");
  elements.zoneEvidenceSummary = section.querySelector("#fishymap-zone-evidence-summary");
  elements.zoneEvidenceList = section.querySelector("#fishymap-zone-evidence-list");
  return elements;
}

function orderPatchesByStart(patches) {
  return [...(patches || [])].sort(
    (left, right) => Number(left?.startTsUtc || 0) - Number(right?.startTsUtc || 0),
  );
}

function normalizePatchRangeSelection(patches, fromPatchId, toPatchId) {
  const ordered = orderPatchesByStart(patches);
  if (!ordered.length) {
    return {
      ordered,
      fromPatchId: "",
      toPatchId: "",
    };
  }

  const indexById = new Map(ordered.map((patch, index) => [patch.patchId, index]));
  let fromIndex = indexById.get(String(fromPatchId || ""));
  let toIndex = indexById.get(String(toPatchId || ""));

  if (!Number.isInteger(fromIndex)) {
    fromIndex = 0;
  }
  if (!Number.isInteger(toIndex)) {
    toIndex = ordered.length - 1;
  }
  if (toIndex < fromIndex) {
    [fromIndex, toIndex] = [toIndex, fromIndex];
  }

  return {
    ordered,
    fromPatchId: ordered[fromIndex]?.patchId || "",
    toPatchId: ordered[toIndex]?.patchId || "",
  };
}

function renderPatchOptions(select, orderedPatches, selectedPatchId, emptyLabel) {
  if (!select) {
    return;
  }
  if (!orderedPatches.length) {
    setMarkup(select, `empty:${emptyLabel}`, `<option value="">${emptyLabel}</option>`);
    select.value = "";
    return;
  }

  const options = orderedPatches.map((patch) => {
    const name = patch.patchName || patch.patchId;
    const date = formatPatchDate(patch.startTsUtc);
    const label = date ? `${name} (${date})` : name;
    return {
      patchId: patch.patchId,
      label,
      html: `<option value="${patch.patchId.replace(/"/g, "&quot;")}">${label}</option>`,
    };
  });

  setMarkup(
    select,
    JSON.stringify(options.map((option) => [option.patchId, option.label])),
    options.map((option) => option.html).join(""),
  );
  const nextValue = selectedPatchId || orderedPatches[0].patchId;
  if (select.value !== nextValue) {
    select.value = nextValue;
  }
}

function renderLayerStack(container, stateBundle) {
  const layers = resolveLayerEntries(stateBundle);
  if (!layers.length) {
    container.innerHTML =
      '<p class="rounded-box border border-base-300/70 bg-base-200/60 px-3 py-3 text-xs text-base-content/60">Layer registry is loading…</p>';
    delete container.dataset.renderKey;
    return;
  }
  const renderKey = JSON.stringify(
    layers.map((layer) => [
      layer.layerId,
      layer.name,
      layer.kind,
      Boolean(layer.visible),
      Math.round(clampLayerOpacity(layer.opacity) * 1000),
      Math.round(clampLayerOpacity(layer.opacityDefault) * 1000),
      Number.isFinite(layer.displayOrder) ? layer.displayOrder : 0,
      layer.locked ? 1 : 0,
    ]),
  );
  if (container.dataset.renderKey === renderKey) {
    return;
  }
  container.dataset.renderKey = renderKey;
  container.innerHTML = layers
    .map((layer) => {
      const visible = Boolean(layer.visible);
      const locked = Boolean(layer.locked);
      const kind = layerKindLabel(layer.kind);
      const visibilityLabel = visible ? "Hide" : "Show";
      return `
        <article
          class="fishymap-layer-card"
          data-layer-id="${layer.layerId.replace(/"/g, "&quot;")}"
          data-locked="${locked ? "true" : "false"}"
        >
          <button
            class="fishymap-layer-drag"
            data-layer-drag="${layer.layerId.replace(/"/g, "&quot;")}"
            type="button"
            aria-label="${locked ? `${layer.name} is pinned to the ground layer` : `Drag ${layer.name}`}"
            draggable="${locked ? "false" : "true"}"
            ${locked ? "disabled" : ""}
            tabindex="-1"
          >
            ${dragHandleIcon()}
          </button>
          <div class="fishymap-layer-body min-w-0">
            <div class="flex items-center gap-2">
              <span class="truncate text-sm font-semibold">${escapeHtml(layer.name)}</span>
              <span class="badge badge-ghost badge-xs">${kind}</span>
              ${locked ? '<span class="badge badge-outline badge-xs">Ground</span>' : ""}
            </div>
            <p class="truncate text-[11px] text-base-content/55">${
              locked ? "Pinned as the base layer." : "Drag to reorder the stack."
            }</p>
            ${
              locked
                ? ""
                : `
                  <label class="fishymap-layer-opacity-control">
                    <div class="flex items-center justify-between gap-3">
                      <span class="text-[11px] uppercase tracking-[0.18em] text-base-content/45">Opacity</span>
                      <span class="text-xs font-semibold text-base-content/60" data-layer-opacity-value>${layerOpacityLabel(layer.opacity)}</span>
                    </div>
                    <input
                      class="fishymap-layer-opacity-range range range-primary range-xs"
                      data-layer-opacity="${layer.layerId.replace(/"/g, "&quot;")}"
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      value="${layerOpacityValue(layer.opacity)}"
                      aria-label="Opacity for ${escapeHtml(layer.name)}"
                    >
                  </label>
                `
            }
          </div>
          <button
            class="fishymap-layer-visibility"
            data-layer-visibility="${layer.layerId.replace(/"/g, "&quot;")}"
            data-layer-visible="${visible ? "true" : "false"}"
            type="button"
            aria-label="${visibilityLabel} ${escapeHtml(layer.name)}"
            title="${visibilityLabel} ${escapeHtml(layer.name)}"
          >
            ${eyeIcon(visible)}
          </button>
        </article>
      `;
    })
    .join("");
}

function resolveCurrentFishId(stateBundle) {
  const stateSelectionFishId = stateBundle.state?.selection?.fishId;
  if (Number.isFinite(stateSelectionFishId)) {
    return stateSelectionFishId;
  }

  const selectedFishIds = resolveSelectedFishIds(stateBundle);
  if (selectedFishIds.length) {
    return selectedFishIds[selectedFishIds.length - 1];
  }

  return null;
}

function moveFishIdToCurrent(selectedFishIds, fishId) {
  return selectedFishIds.filter((id) => id !== fishId).concat(fishId);
}

function removeSelectedFishId(selectedFishIds, fishId) {
  return selectedFishIds.filter((id) => id !== fishId);
}

function buildSearchMatches(stateBundle, searchText) {
  const catalogFish = stateBundle.state?.catalog?.fish || [];
  const matches = findFishMatches(catalogFish, searchText);
  const selectedFishIds = new Set(resolveSelectedFishIds(stateBundle));
  return matches.filter((fish) => !selectedFishIds.has(fish.fishId));
}

function renderSearchSelection(elements, stateBundle, fishLookup) {
  const selectedFishIds = resolveSelectedFishIds(stateBundle);
  const currentFishId = resolveCurrentFishId(stateBundle);
  const renderKey = JSON.stringify({
    selectedFishIds,
    currentFishId,
    selectedFish: selectedFishIds.map((fishId) => {
      const fish = fishLookup.get(fishId);
      return [fishId, fish?.name || "", fish?.iconUrl || ""];
    }),
  });
  if (elements.searchSelection.dataset.renderKey === renderKey) {
    return;
  }
  elements.searchSelection.dataset.renderKey = renderKey;

  if (!selectedFishIds.length) {
    elements.searchSelection.innerHTML = "";
    elements.searchSelection.hidden = true;
    if (elements.searchSelectionShell) {
      elements.searchSelectionShell.hidden = true;
    }
    if (elements.searchDock) {
      elements.searchDock.dataset.hasSelection = "false";
    }
    return;
  }

  elements.searchSelection.hidden = false;
  if (elements.searchSelectionShell) {
    elements.searchSelectionShell.hidden = false;
  }
  if (elements.searchDock) {
    elements.searchDock.dataset.hasSelection = "true";
  }

  elements.searchSelection.innerHTML = selectedFishIds
    .map((fishId) => {
      const fish = fishLookup.get(fishId);
      const active = fishId === currentFishId;
      const name = fish?.name || `Fish ${fishId}`;
      return `
        <div class="inline-flex items-center gap-1 rounded-full border px-2 py-1 ${
          active
            ? "border-primary bg-primary text-primary-content"
            : "border-base-300 bg-base-100 text-base-content"
        }">
          <button
            class="fishymap-selection-focus btn btn-ghost btn-xs h-auto min-h-0 gap-2 rounded-full border-0 px-2 ${
              active ? "text-primary-content hover:bg-primary-content/10" : "text-inherit"
            }"
            data-fish-id="${fishId}"
            type="button"
          >
            ${renderFishAvatar(fish, "size-5")}
            <span class="truncate max-w-36">${escapeHtml(name)}</span>
          </button>
          <button
            class="fishymap-selection-remove btn btn-ghost btn-xs h-auto min-h-0 rounded-full border-0 px-2 ${
              active ? "text-primary-content hover:bg-primary-content/10" : "text-inherit"
            }"
            data-fish-id="${fishId}"
            type="button"
            aria-label="Remove ${escapeHtml(name)}"
          >
            ×
          </button>
        </div>
      `;
    })
    .join("");
}

function renderSearchResults(elements, matches, stateBundle) {
  const query = String(stateBundle.inputState?.filters?.searchText || "").trim();
  const showResults = Boolean(query);
  const activeMatches = matches.slice(0, 12);
  const currentFishId = resolveCurrentFishId(stateBundle);
  const renderKey = JSON.stringify({
    query,
    currentFishId,
    resultIds: activeMatches.map((fish) => fish.fishId),
    total: matches.length,
  });
  if (elements.searchResultsShell) {
    elements.searchResultsShell.hidden = !showResults;
  }
  if (elements.searchCount) {
    setTextContent(elements.searchCount, `${matches.length} fish`);
  }
  if (elements.searchResults.dataset.renderKey === renderKey) {
    return;
  }
  elements.searchResults.dataset.renderKey = renderKey;
  if (!matches.length) {
    elements.searchResults.innerHTML = `<div class="px-2 py-3 text-xs text-base-content/60">${
      query ? "No fish match the current filter." : "Start typing to filter fish."
    }</div>`;
    return;
  }
  elements.searchResults.innerHTML = activeMatches
    .map(
      (fish) => {
        return `
        <button
          class="btn btn-sm w-full justify-start rounded-xl px-3 btn-ghost"
          data-fish-id="${fish.fishId}"
          type="button"
        >
          ${renderFishAvatar(fish)}
          <span class="truncate">${escapeHtml(fish.name)}</span>
        </button>
      `;
      },
    )
    .join("");
}

function renderZoneEvidence(elements, stateBundle, fishLookup) {
  if (!elements.zoneEvidenceStatus || !elements.zoneEvidenceSummary || !elements.zoneEvidenceList) {
    return;
  }
  const zoneStats = stateBundle.state?.selection?.zoneStats || null;
  const currentFishId = resolveCurrentFishId(stateBundle);
  const zoneStatsStatus = stateBundle.state?.statuses?.zoneStatsStatus || "zone stats: idle";
  const summary = buildZoneEvidenceSummary(zoneStats);

  setTextContent(elements.zoneEvidenceStatus, zoneStatsStatus);
  setTextContent(elements.zoneEvidenceSummary, summary);

  const distribution = Array.isArray(zoneStats?.distribution) ? zoneStats.distribution : [];
  const renderKey = JSON.stringify({
    zoneRgb: zoneStats?.zoneRgb ?? null,
    status: zoneStatsStatus,
    summary,
    currentFishId,
    distribution: distribution.map((entry) => {
      const fish = fishLookup.get(entry.fishId);
      return [
        entry.fishId,
        entry.fishName || "",
        fishIconUrl({ iconUrl: entry.iconUrl || fish?.iconUrl || "" }),
        entry.evidenceWeight,
        entry.pMean,
        entry.ciLow,
        entry.ciHigh,
      ];
    }),
  });
  if (elements.zoneEvidenceList.dataset.renderKey === renderKey) {
    return;
  }
  elements.zoneEvidenceList.dataset.renderKey = renderKey;

  if (!zoneStats) {
    elements.zoneEvidenceList.innerHTML =
      '<div class="px-2 py-3 text-xs text-base-content/60">Click a zone on the map to load evidence.</div>';
    return;
  }
  if (!distribution.length) {
    elements.zoneEvidenceList.innerHTML =
      '<div class="px-2 py-3 text-xs text-base-content/60">No fish evidence in this window.</div>';
    return;
  }

  elements.zoneEvidenceList.innerHTML = distribution
    .map((entry) => {
      const fish = fishLookup.get(entry.fishId);
      const evidenceFish = {
        fishId: entry.fishId,
        name: fish?.name || entry.fishName || `Fish ${entry.fishId}`,
        iconUrl: entry.iconUrl || fish?.iconUrl || "",
      };
      const active = evidenceFish.fishId === currentFishId;
      const ci =
        Number.isFinite(entry.ciLow) && Number.isFinite(entry.ciHigh)
          ? `${formatDecimal(entry.ciLow)}-${formatDecimal(entry.ciHigh)}`
          : "n/a";
      return `
        <button
          class="btn btn-sm h-auto min-h-0 w-full justify-start rounded-xl px-3 py-2 ${
            active ? "btn-primary text-primary-content" : "btn-ghost"
          }"
          data-zone-evidence-fish-id="${evidenceFish.fishId}"
          type="button"
        >
          ${renderFishAvatar(evidenceFish)}
          <span class="min-w-0 flex-1 text-left">
            <span class="block truncate">${escapeHtml(evidenceFish.name)}</span>
            <span class="block text-[11px] ${
              active ? "text-primary-content/80" : "text-base-content/55"
            }">
              p ${formatDecimal(entry.pMean)} · weight ${formatDecimal(entry.evidenceWeight)} · ci ${ci}
            </span>
          </span>
          <span class="text-[11px] ${
            active ? "text-primary-content/80" : "text-base-content/45"
          }">${formatPercent(entry.pMean)}</span>
        </button>
      `;
    })
    .join("");
}

function renderStatusLines(container, statuses) {
  const lines = [
    statuses?.metaStatus,
    statuses?.layersStatus,
    statuses?.zonesStatus,
    statuses?.pointsStatus,
    statuses?.fishStatus,
    statuses?.zoneStatsStatus,
  ].filter(Boolean);
  setMarkup(
    container,
    JSON.stringify(lines),
    lines.map((line) => `<p>${line}</p>`).join(""),
  );
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
    setTextContent(label, layerOpacityLabel(normalized));
  }
  return true;
}

function renderPanel(elements, stateBundle) {
  const state = stateBundle.state || {};
  const inputState = stateBundle.inputState || {};
  const catalogFish = state.catalog?.fish || [];
  const currentFishId = resolveCurrentFishId(stateBundle);
  const patchRange = normalizePatchRangeSelection(
    state.catalog?.patches || [],
    inputState.filters?.fromPatchId ??
      state.filters?.fromPatchId ??
      inputState.filters?.patchId ??
      state.filters?.patchId ??
      null,
    inputState.filters?.toPatchId ??
      state.filters?.toPatchId ??
      inputState.filters?.patchId ??
      state.filters?.patchId ??
      null,
  );
  const searchText = inputState.filters?.searchText || "";
  const showPoints = (inputState.ui?.showPoints ?? state.ui?.showPoints) !== false;
  const showPointIcons =
    (inputState.ui?.showPointIcons ?? state.ui?.showPointIcons) !== false;
  const pointIconScale = clampPointIconScale(
    inputState.ui?.pointIconScale ?? state.ui?.pointIconScale ?? FISHYMAP_POINT_ICON_SCALE_MIN,
  );
  const fishLookup = mergeZoneEvidenceIntoFishLookup(
    buildFishLookup(catalogFish),
    state.selection?.zoneStats || null,
  );

  applyThemeToShell(elements.shell);

  setTextContent(elements.readyPill, state.ready ? "Ready" : "Loading");
  setClassName(
    elements.readyPill,
    `badge badge-sm ${state.ready ? "badge-success" : "badge-outline"}`,
  );
  renderViewState(elements, state);
  if (elements.pointIconScale) {
    const sliderValue = pointIconScaleValue(pointIconScale);
    if (elements.pointIconScale.value !== sliderValue) {
      elements.pointIconScale.value = sliderValue;
    }
    setBooleanProperty(elements.pointIconScale, "disabled", !showPoints || !showPointIcons);
  }
  if (elements.pointIconScaleValue) {
    setTextContent(elements.pointIconScaleValue, pointIconScaleLabel(pointIconScale));
  }
  if (elements.showPoints) {
    setBooleanProperty(elements.showPoints, "checked", showPoints);
  }
  if (elements.showPointIcons) {
    setBooleanProperty(elements.showPointIcons, "checked", showPointIcons);
  }

  if (elements.search.value !== searchText) {
    elements.search.value = searchText;
  }

  renderPatchOptions(
    elements.patchFrom,
    patchRange.ordered,
    patchRange.fromPatchId,
    "Loading patches…",
  );
  renderPatchOptions(
    elements.patchTo,
    patchRange.ordered,
    patchRange.toPatchId,
    "Loading patches…",
  );
  if (
    elements.layerOpacityInteraction?.activeLayerId &&
    Number.isFinite(elements.layerOpacityInteraction?.activeValue) &&
    syncLayerOpacityControl(
      elements.layers,
      elements.layerOpacityInteraction.activeLayerId,
      elements.layerOpacityInteraction.activeValue,
    )
  ) {
    // Keep the active slider mounted while the user is dragging it.
  } else {
    renderLayerStack(elements.layers, stateBundle);
  }
  if (elements.layersCount) {
    setTextContent(elements.layersCount, String((state.catalog?.layers || []).length));
  }

  const matches = buildSearchMatches(stateBundle, searchText);
  renderSearchSelection(elements, stateBundle, fishLookup);
  renderSearchResults(elements, matches, stateBundle);
  renderZoneEvidence(elements, stateBundle, fishLookup);

  if (elements.legend) {
    setBooleanProperty(elements.legend, "open", Boolean(inputState.ui?.legendOpen));
  }
  if (elements.diagnostics) {
    setBooleanProperty(elements.diagnostics, "open", Boolean(inputState.ui?.diagnosticsOpen));
  }

  const panelOpen = inputState.ui?.leftPanelOpen !== false;
  setBooleanProperty(elements.panel, "hidden", !panelOpen);
  setBooleanProperty(elements.panelOpen, "hidden", panelOpen);

  const zoneName =
    state.selection?.zoneName ||
    (state.selection?.zoneRgb != null ? `Zone ${formatZone(state.selection.zoneRgb)}` : null);
  if (elements.selectionSummary) {
    if (zoneName) {
      setTextContent(elements.selectionSummary, zoneName);
    } else {
      setTextContent(elements.selectionSummary, "No zone selected.");
    }
  }

  renderHoverTooltip(elements, state.hover || null);

  renderStatusLines(elements.statusLines, state.statuses || {});
  setTextContent(
    elements.diagnosticJson,
    JSON.stringify(state.lastDiagnostic || state.statuses || {}, null, 2),
  );
}

function bindUi(shell, elements) {
  let isRendering = false;
  let latestStateBundle = requestBridgeState(shell);
  const layerDragState = {
    draggingLayerId: null,
    overLayerId: null,
    position: null,
  };
  const layerOpacityInteraction = {
    activeLayerId: null,
    activeValue: null,
  };
  elements.layerOpacityInteraction = layerOpacityInteraction;

  function stateBundleFromEvent(event) {
    return {
      state: event.detail?.state || FishyMapBridge.getCurrentState(),
      inputState:
        event.detail?.inputState ||
        (typeof FishyMapBridge.getCurrentInputState === "function"
          ? FishyMapBridge.getCurrentInputState()
          : {}),
    };
  }

  function renderCurrentState(stateBundle = requestBridgeState(shell)) {
    latestStateBundle = stateBundle;
    isRendering = true;
    try {
      renderPanel(elements, stateBundle);
    } finally {
      isRendering = false;
    }
  }

  function clearLayerDropState() {
    layerDragState.overLayerId = null;
    layerDragState.position = null;
    elements.layers
      ?.querySelectorAll?.(".fishymap-layer-card[data-drop-position]")
      ?.forEach?.((card) => {
        delete card.dataset.dropPosition;
      });
  }

  function applyLayerDropState(targetLayerId, position) {
    clearLayerDropState();
    layerDragState.overLayerId = targetLayerId;
    layerDragState.position = position;
    const card = Array.from(
      elements.layers?.querySelectorAll?.(".fishymap-layer-card") || [],
    ).find((candidate) => candidate.getAttribute("data-layer-id") === targetLayerId);
    if (card) {
      card.dataset.dropPosition = position;
    }
  }

  function setActiveLayerOpacity(slider) {
    if (!slider) {
      return;
    }
    const layerId = slider.getAttribute("data-layer-opacity");
    if (!layerId) {
      return;
    }
    layerOpacityInteraction.activeLayerId = layerId;
    layerOpacityInteraction.activeValue = clampLayerOpacity(slider.value);
    syncLayerOpacityControl(elements.layers, layerId, layerOpacityInteraction.activeValue);
  }

  function clearActiveLayerOpacity() {
    if (!layerOpacityInteraction.activeLayerId) {
      return;
    }
    latestStateBundle = requestBridgeState(shell);
    layerOpacityInteraction.activeLayerId = null;
    layerOpacityInteraction.activeValue = null;
    renderCurrentState(latestStateBundle);
  }

  elements.canvas.addEventListener("pointermove", (event) => {
    elements.hoverPointerActive = true;
    setHoverTooltipPosition(elements, event.clientX, event.clientY);
  });

  elements.canvas.addEventListener("pointerleave", () => {
    elements.hoverPointerActive = false;
    renderHoverTooltip(elements, null);
  });

  function pushSearchPatch() {
    const searchText = elements.search.value;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  }

  function pushPatchRangePatch() {
    const current = requestBridgeState(shell);
    const patchRange = normalizePatchRangeSelection(
      current.state.catalog?.patches || [],
      elements.patchFrom.value || null,
      elements.patchTo.value || null,
    );
    if (!patchRange.ordered.length) {
      return;
    }

    elements.patchFrom.value = patchRange.fromPatchId;
    elements.patchTo.value = patchRange.toPatchId;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        fromPatchId: patchRange.fromPatchId,
        toPatchId: patchRange.toPatchId,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  }

  elements.search.addEventListener("input", () => {
    if (isRendering) {
      return;
    }
    pushSearchPatch();
  });

  elements.search.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") {
      return;
    }
    const current = requestBridgeState(shell);
    const matches = findFishMatches(
      current.state.catalog?.fish || [],
      elements.search.value,
    );
    const selectedFishIds = new Set(resolveSelectedFishIds(current));
    const top = matches.find((fish) => !selectedFishIds.has(fish.fishId));
    if (!top) {
      return;
    }
    event.preventDefault();
    elements.search.value = "";
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText: "",
        fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), top.fishId),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.searchResults.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-fish-id]");
    if (!button) {
      return;
    }
    const fishId = Number.parseInt(button.getAttribute("data-fish-id"), 10);
    const current = requestBridgeState(shell);
    elements.search.value = "";
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText: "",
        fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), fishId),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.searchSelection.addEventListener("click", (event) => {
    const focusButton = event.target.closest("button.fishymap-selection-focus[data-fish-id]");
    if (focusButton) {
      const fishId = Number.parseInt(focusButton.getAttribute("data-fish-id"), 10);
      const current = requestBridgeState(shell);
      dispatchMapState(shell, {
        version: 1,
        filters: {
          fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), fishId),
        },
      });
      renderCurrentState(requestBridgeState(shell));
      return;
    }

    const removeButton = event.target.closest("button.fishymap-selection-remove[data-fish-id]");
    if (!removeButton) {
      return;
    }
    const fishId = Number.parseInt(removeButton.getAttribute("data-fish-id"), 10);
    const current = requestBridgeState(shell);
    dispatchMapState(shell, {
      version: 1,
      filters: {
        fishIds: removeSelectedFishId(resolveSelectedFishIds(current), fishId),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  if (elements.zoneEvidenceList) {
    elements.zoneEvidenceList.addEventListener("click", (event) => {
      const button = event.target.closest("button[data-zone-evidence-fish-id]");
      if (!button) {
        return;
      }
      const fishId = Number.parseInt(button.getAttribute("data-zone-evidence-fish-id"), 10);
      if (!Number.isFinite(fishId)) {
        return;
      }
      const current = requestBridgeState(shell);
      dispatchMapState(shell, {
        version: 1,
        filters: {
          fishIds: moveFishIdToCurrent(resolveSelectedFishIds(current), fishId),
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  elements.patchFrom.addEventListener("change", () => {
    if (isRendering) {
      return;
    }
    pushPatchRangePatch();
  });

  elements.patchTo.addEventListener("change", () => {
    if (isRendering) {
      return;
    }
    pushPatchRangePatch();
  });

  if (elements.viewToggle) {
    elements.viewToggle.addEventListener("click", () => {
      const current = latestStateBundle?.state || requestBridgeState(shell).state;
      const nextViewMode = current?.view?.viewMode === "3d" ? "2d" : "3d";
      dispatchMapCommand(shell, {
        setViewMode: nextViewMode,
      });
    });
  }

  if (elements.pointIconScale) {
    elements.pointIconScale.addEventListener("input", () => {
      if (isRendering) {
        return;
      }
      const pointIconScale = clampPointIconScale(elements.pointIconScale.value);
      if (elements.pointIconScaleValue) {
        elements.pointIconScaleValue.textContent = pointIconScaleLabel(pointIconScale);
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          pointIconScale,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  if (elements.showPoints) {
    elements.showPoints.addEventListener("change", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          showPoints: elements.showPoints.checked,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  if (elements.showPointIcons) {
    elements.showPointIcons.addEventListener("change", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          showPointIcons: elements.showPointIcons.checked,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  elements.layers.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-layer-visibility]");
    if (isRendering || !button) {
      return;
    }
    const layerId = button.getAttribute("data-layer-visibility");
    if (!layerId) {
      return;
    }
    const current = requestBridgeState(shell);
    const visibleIds = new Set(resolveVisibleLayerIds(current));
    if (visibleIds.has(layerId)) {
      visibleIds.delete(layerId);
    } else {
      visibleIds.add(layerId);
    }
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerIdsVisible: resolveLayerEntries(current)
          .map((layer) => layer.layerId)
          .filter((candidateId) => visibleIds.has(candidateId)),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.layers.addEventListener("input", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (isRendering || !slider) {
      return;
    }
    setActiveLayerOpacity(slider);
    const layerId = slider.getAttribute("data-layer-opacity");
    if (!layerId) {
      return;
    }
    const current = latestStateBundle || requestBridgeState(shell);
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerOpacities: buildLayerOpacityPatch(current, layerId, slider.value),
      },
    });
    latestStateBundle = requestBridgeState(shell);
  });

  elements.layers.addEventListener("pointerdown", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
  });

  elements.layers.addEventListener("focusin", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
  });

  elements.layers.addEventListener("change", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    setActiveLayerOpacity(slider);
    clearActiveLayerOpacity();
  });

  elements.layers.addEventListener("focusout", (event) => {
    const slider = event.target.closest("input[data-layer-opacity]");
    if (!slider) {
      return;
    }
    queueMicrotask(() => {
      clearActiveLayerOpacity();
    });
  });

  elements.layers.addEventListener("dragstart", (event) => {
    const handle = event.target.closest("button[data-layer-drag][draggable='true']");
    const card = handle?.closest(".fishymap-layer-card");
    if (!card || !handle || isRendering) {
      return;
    }
    const layerId = card.getAttribute("data-layer-id");
    if (!layerId) {
      return;
    }
    layerDragState.draggingLayerId = layerId;
    card.dataset.dragging = "true";
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", layerId);
    }
  });

  elements.layers.addEventListener("dragover", (event) => {
    if (!layerDragState.draggingLayerId) {
      return;
    }
    const card = event.target.closest(".fishymap-layer-card");
    if (!card || card.getAttribute("data-locked") === "true") {
      clearLayerDropState();
      return;
    }
    const targetLayerId = card.getAttribute("data-layer-id");
    if (!targetLayerId || targetLayerId === layerDragState.draggingLayerId) {
      clearLayerDropState();
      return;
    }
    event.preventDefault();
    const rect = card.getBoundingClientRect();
    const position = event.clientY >= rect.top + rect.height / 2 ? "after" : "before";
    applyLayerDropState(targetLayerId, position);
  });

  elements.layers.addEventListener("drop", (event) => {
    if (
      !layerDragState.draggingLayerId ||
      !layerDragState.overLayerId ||
      !layerDragState.position
    ) {
      clearLayerDropState();
      return;
    }
    event.preventDefault();
    const current = requestBridgeState(shell);
    const nextOrder = moveLayerIdBefore(
      resolveLayerEntries(current),
      layerDragState.draggingLayerId,
      layerDragState.overLayerId,
      layerDragState.position,
    );
    clearLayerDropState();
    layerDragState.draggingLayerId = null;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerIdsOrdered: nextOrder,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.layers.addEventListener("dragend", () => {
    elements.layers
      ?.querySelectorAll?.(".fishymap-layer-card[data-dragging]")
      ?.forEach?.((card) => {
        delete card.dataset.dragging;
      });
    layerDragState.draggingLayerId = null;
    clearLayerDropState();
  });

  elements.layers.addEventListener("dragleave", (event) => {
    const related = event.relatedTarget;
    if (related && elements.layers.contains(related)) {
      return;
    }
    clearLayerDropState();
  });

  elements.resetView.addEventListener("click", () => {
    dispatchMapCommand(shell, { resetView: true });
  });

  if (elements.legend) {
    elements.legend.addEventListener("toggle", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          legendOpen: elements.legend.open,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  if (elements.diagnostics) {
    elements.diagnostics.addEventListener("toggle", () => {
      if (isRendering) {
        return;
      }
      dispatchMapState(shell, {
        version: 1,
        ui: {
          diagnosticsOpen: elements.diagnostics.open,
        },
      });
      renderCurrentState(requestBridgeState(shell));
    });
  }

  elements.panelClose.addEventListener("click", () => {
    dispatchMapState(shell, {
      version: 1,
      ui: {
        leftPanelOpen: false,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.panelOpen.addEventListener("click", () => {
    dispatchMapState(shell, {
      version: 1,
      ui: {
        leftPanelOpen: true,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  shell.addEventListener(FISHYMAP_EVENTS.ready, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.selectionChanged, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.diagnostic, (event) => {
    renderCurrentState(stateBundleFromEvent(event));
  });

  shell.addEventListener(FISHYMAP_EVENTS.viewChanged, (event) => {
    latestStateBundle = stateBundleFromEvent(event);
    renderViewState(elements, latestStateBundle.state);
  });

  shell.addEventListener(FISHYMAP_EVENTS.hoverChanged, (event) => {
    const hover = hoverFromEventDetail(event.detail || {});
    if (latestStateBundle?.state) {
      latestStateBundle.state = {
        ...latestStateBundle.state,
        hover,
      };
    }
    renderHoverTooltip(elements, hover);
  });

  window.addEventListener("fishystuff:themechange", () => applyThemeToShell(elements.shell));

  renderCurrentState();
}

async function main() {
  const shell = document.getElementById("map-page-shell");
  const canvas = document.getElementById("bevy");
  if (!shell || !canvas) {
    return;
  }

  const elements = {
    shell,
    searchDock: document.getElementById("fishymap-search-dock"),
    panel: document.getElementById("fishymap-panel"),
    panelBody: document.getElementById("fishymap-panel-body"),
    panelOpen: document.getElementById("fishymap-panel-open"),
    panelClose: document.getElementById("fishymap-panel-close"),
    readyPill: document.getElementById("fishymap-ready-pill"),
    search: document.getElementById("fishymap-search"),
    searchSelectionShell: document.getElementById("fishymap-search-selection-shell"),
    searchSelection: document.getElementById("fishymap-search-selection"),
    searchResultsShell: document.getElementById("fishymap-search-results-shell"),
    searchResults: document.getElementById("fishymap-search-results"),
    searchCount: document.getElementById("fishymap-search-count"),
    patchFrom: document.getElementById("fishymap-patch-from"),
    patchTo: document.getElementById("fishymap-patch-to"),
    viewToggle: document.getElementById("fishymap-view-toggle"),
    viewToggleIcon: document.getElementById("fishymap-view-toggle-icon"),
    showPoints: document.getElementById("fishymap-show-points"),
    showPointIcons: document.getElementById("fishymap-show-point-icons"),
    pointIconScale: document.getElementById("fishymap-point-icon-scale"),
    pointIconScaleValue: document.getElementById("fishymap-point-icon-scale-value"),
    layers: document.getElementById("fishymap-layers"),
    layersCount: document.getElementById("fishymap-layers-count"),
    resetView: document.getElementById("fishymap-reset-view"),
    legend: document.getElementById("fishymap-legend"),
    diagnostics: document.getElementById("fishymap-diagnostics"),
    statusLines: document.getElementById("fishymap-status-lines"),
    diagnosticJson: document.getElementById("fishymap-diagnostic-json"),
    selectionSummary: document.getElementById("fishymap-selection-summary"),
    zoneEvidenceStatus: document.getElementById("fishymap-zone-evidence-status"),
    zoneEvidenceSummary: document.getElementById("fishymap-zone-evidence-summary"),
    zoneEvidenceList: document.getElementById("fishymap-zone-evidence-list"),
    hoverTooltip: document.getElementById("fishymap-hover-tooltip"),
    hoverSummary: document.getElementById("fishymap-hover-summary"),
    viewReadout: document.getElementById("fishymap-view-readout"),
    errorOverlay: document.getElementById("fishymap-error-overlay"),
    errorMessage: document.getElementById("fishymap-error-message"),
    canvas,
  };

  ensureZoneEvidenceElements(elements);

  bindUi(shell, elements);
  applyThemeToShell(shell);
  installRendererErrorHandlers(elements);

  if (!supportsWebgl2(document)) {
    setMapError(
      elements,
      "WebGL2 is required to render the map, but this browser/runtime did not provide a WebGL2 context.",
    );
    return;
  }

  try {
    await FishyMapBridge.mount(shell, { canvas });
  } catch (error) {
    console.error("Failed to mount FishyMap bridge", error);
    setMapError(elements, error);
  }
}

main();
