import { FISH_FILTER_TERM_ORDER } from "./map-search-contract.js";
import { renderSearchResults, renderSearchSelection } from "./map-search-panel.js";
import {
  buildDefaultFishFilterMatches,
  buildSearchExpressionDragSignalPatch,
  buildSearchExpressionOperatorSignalPatch,
  buildSearchMatches,
  buildSearchMatchSignalPatch,
  buildSearchPanelStateBundle,
  buildSearchSelectionRemovalSignalPatch,
  buildSemanticTermLookup,
  fishFilterTermMetadata,
  patchTouchesSearchPanelSignals,
  resolveSelectedFishFilterTerms,
  resolveSelectedFishIds,
  resolveSelectedSemanticFieldIdsByLayer,
  resolveSelectedZoneRgbs,
} from "./map-search-state.js";
import { FISHYMAP_LIVE_INIT_EVENT, readMapShellSignals } from "./map-shell-signals.js";
import { dispatchShellSignalPatch, FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const ICON_SPRITE_URL = "/img/icons.svg";
const SEARCH_PANEL_TAG_NAME = "fishymap-search-panel";
const HTMLElementBase = globalThis.HTMLElement ?? class {};
const EXPRESSION_DRAG_PROXY_SCALE = 0.78;
const EXPRESSION_DRAG_PROXY_HOTSPOT = 8;
let expressionDragProxyElement = null;

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (char) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[char] || char,
  );
}

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  element[propertyName] = Boolean(value);
}

function setTextContent(element, text) {
  if (!element) {
    return;
  }
  element.textContent = String(text ?? "");
}

function normalizeExpressionPath(path) {
  const normalized = String(path || "").trim();
  return normalized ? normalized.split(".").filter(Boolean) : [];
}

function expressionPathStartsWith(prefixPath, candidatePath) {
  const prefix = normalizeExpressionPath(prefixPath);
  const candidate = normalizeExpressionPath(candidatePath);
  if (!prefix.length || prefix.length > candidate.length) {
    return false;
  }
  return prefix.every((value, index) => candidate[index] === value);
}

function parentExpressionPath(path) {
  const parts = normalizeExpressionPath(path);
  if (parts.length <= 1) {
    return "root";
  }
  return parts.slice(0, -1).join(".");
}

function removeExpressionDragProxyElement() {
  expressionDragProxyElement?.remove?.();
  expressionDragProxyElement = null;
}

function resolveExpressionDragProxyElement(sourceElement) {
  removeExpressionDragProxyElement();
  if (!sourceElement) {
    return null;
  }
  const clonedElement =
    typeof sourceElement.cloneNode === "function" ? sourceElement.cloneNode(true) : null;
  const parent =
    globalThis.document?.body ||
    globalThis.document?.documentElement ||
    null;
  if (!clonedElement || typeof parent?.appendChild !== "function") {
    return sourceElement;
  }
  if (clonedElement.dataset) {
    delete clonedElement.dataset.dragging;
    delete clonedElement.dataset.expressionDropMode;
  }
  clonedElement.removeAttribute?.("id");
  clonedElement.removeAttribute?.("draggable");
  const rect =
    typeof sourceElement.getBoundingClientRect === "function"
      ? sourceElement.getBoundingClientRect()
      : null;
  const width = Number.isFinite(rect?.width) && rect.width > 0 ? `${rect.width}px` : "";
  const height = Number.isFinite(rect?.height) && rect.height > 0 ? `${rect.height}px` : "";
  const cssText = [
    "position: fixed",
    "left: -10000px",
    "top: -10000px",
    "margin: 0",
    "pointer-events: none",
    `transform: scale(${EXPRESSION_DRAG_PROXY_SCALE})`,
    "transform-origin: top left",
    "z-index: -1",
    width ? `width: ${width}` : "",
    height ? `height: ${height}` : "",
  ]
    .filter(Boolean)
    .join("; ");
  if (typeof clonedElement.setAttribute === "function") {
    clonedElement.setAttribute("style", cssText);
  }
  if (clonedElement.style && typeof clonedElement.style === "object") {
    clonedElement.style.position = "fixed";
    clonedElement.style.left = "-10000px";
    clonedElement.style.top = "-10000px";
    clonedElement.style.margin = "0";
    clonedElement.style.pointerEvents = "none";
    clonedElement.style.transform = `scale(${EXPRESSION_DRAG_PROXY_SCALE})`;
    clonedElement.style.transformOrigin = "top left";
    clonedElement.style.zIndex = "-1";
    if (width) {
      clonedElement.style.width = width;
    }
    if (height) {
      clonedElement.style.height = height;
    }
  }
  parent.appendChild(clonedElement);
  expressionDragProxyElement = clonedElement;
  return expressionDragProxyElement;
}

function spriteIcon(name, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${name}"></use></svg>`;
}

function resolveItemGrade(grade, isPrize) {
  const resolver = globalThis.window?.__fishystuffItemPresentation?.resolveGradeTone;
  if (typeof resolver === "function") {
    return resolver(grade, isPrize);
  }
  const normalized = String(grade ?? "").trim().toLowerCase();
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

function fishFilterTermIconMarkup(term, sizeClass = "size-4") {
  const metadata = fishFilterTermMetadata(term);
  return spriteIcon(
    metadata?.icon || "question-mark",
    `${sizeClass} shrink-0 ${metadata?.iconClass || "text-base-content/60"}`.trim(),
  );
}

function fishIdentityMarkup(fish) {
  const grade = resolveFishGrade(fish);
  const fishId = Number.parseInt(fish?.fishId, 10);
  const name = String(fish?.name || `Fish ${fishId || "?"}`).trim();
  const iconUrl =
    globalThis.window?.__fishystuffResolveFishItemIconUrl?.(fish?.itemId) ||
    globalThis.window?.__fishystuffResolveFishEncyclopediaIconUrl?.(fish?.encyclopediaId) ||
    "";
  const iconMarkup = iconUrl
    ? `<span class="fishy-item-icon-frame is-xs fishy-item-grade-${escapeHtml(grade)}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishy-item-icon-frame is-xs fishy-item-grade-${escapeHtml(grade)}"><span class="fishy-item-icon-fallback fishy-item-grade-${escapeHtml(grade)}">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
  return `<span class="fishy-item-row fishy-item-grade-${escapeHtml(grade)}">${iconMarkup}<span class="fishy-item-label truncate max-w-40">${escapeHtml(name)}</span></span>`;
}

function resolveFishGrade(fish) {
  return resolveItemGrade(fish?.grade, fish?.isPrize === true || fish?.is_prize === true);
}

function zoneIdentityMarkup(zone) {
  const zoneRgb = Number.parseInt(zone?.zoneRgb, 10);
  const name = String(zone?.name || "").trim() || `Zone ${formatZone(zoneRgb)}`;
  return `<span class="truncate max-w-40">${escapeHtml(name)}</span>`;
}

function semanticIdentityMarkup(text) {
  return `<span class="truncate max-w-40">${escapeHtml(text)}</span>`;
}

function formatZone(zoneRgb) {
  const numeric = Number(zoneRgb);
  return Number.isFinite(numeric) ? `#${numeric.toString(16).padStart(6, "0")}` : "none";
}

export function readMapSearchPanelShellSignals(shell) {
  return readMapShellSignals(shell);
}

export function resolveSearchPanelMatches(stateBundle, searchState, zoneCatalog = []) {
  const query = String(stateBundle?.inputState?.filters?.searchText || "").trim();
  if (searchState?.open !== true) {
    return [];
  }
  return query
    ? buildSearchMatches(stateBundle, query, zoneCatalog)
    : buildDefaultFishFilterMatches(stateBundle);
}

function ensureSearchPanelMarkup(host) {
  if (host.querySelector("#fishymap-search-selection-shell")) {
    return;
  }
  host.innerHTML = `
    <div id="fishymap-search-selection-shell" class="not-prose" hidden>
      <div id="fishymap-search-selection" hidden></div>
    </div>
    <div id="fishymap-search-results-shell" class="card card-border bg-base-100 overflow-hidden" hidden>
      <ul id="fishymap-search-results" class="menu menu-sm max-h-64 overflow-auto gap-1 p-2"></ul>
    </div>
  `;
}

export class FishyMapSearchPanelElement extends HTMLElementBase {
  static get observedAttributes() {
    return ["data-search-state"];
  }

  constructor() {
    super();
    this._shell = null;
    this._rafId = 0;
    this._zoneCatalog = [];
    this._elements = null;
    this._dragState = {
      sourcePath: "",
      sourceElement: null,
      dropPath: "",
      dropIndex: -1,
      dropKind: "",
      overElement: null,
    };
    this._handleSignalPatched = (event) => {
      if (patchTouchesSearchPanelSignals(event?.detail || null)) {
        this.scheduleRender();
      }
    };
    this._handleZoneCatalogReady = (event) => {
      this._zoneCatalog = Array.isArray(event?.detail?.zoneCatalog) ? event.detail.zoneCatalog : [];
      if (this._elements) {
        this._elements.zoneCatalog = this._zoneCatalog;
      }
      this.scheduleRender();
    };
    this._handleLiveInit = () => {
      this.scheduleRender();
    };
    this._handleSearchWindowFocusOut = () => {
      globalThis.setTimeout?.(() => {
        const activeElement = globalThis.document?.activeElement;
        if (activeElement instanceof Element && this._elements?.searchWindow?.contains(activeElement)) {
          return;
        }
        this.dispatchPatch({
          _map_ui: {
            search: {
              open: false,
            },
          },
        });
      }, 0);
    };
    this._handleDragStart = (event) => {
      if (event.target.closest("button")) {
        return;
      }
      const draggableNode = event.target.closest("[data-expression-drag-path][draggable='true']");
      if (!draggableNode) {
        return;
      }
      const sourcePath = String(draggableNode.getAttribute("data-expression-drag-path") || "").trim();
      if (!sourcePath) {
        return;
      }
      this.clearExpressionDragState();
      this._dragState.sourcePath = sourcePath;
      this._dragState.sourceElement = draggableNode;
      this.setExpressionDraggingState(true);
      draggableNode.dataset.dragging = "true";
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", sourcePath);
        const dragImage = resolveExpressionDragProxyElement(draggableNode);
        if (dragImage && typeof event.dataTransfer.setDragImage === "function") {
          event.dataTransfer.setDragImage(
            dragImage,
            EXPRESSION_DRAG_PROXY_HOTSPOT,
            EXPRESSION_DRAG_PROXY_HOTSPOT,
          );
        }
      }
    };
    this._handleDragOver = (event) => {
      if (!this._dragState.sourcePath) {
        return;
      }
      const sourcePath = this._dragState.sourcePath;
      const slotTarget = event.target.closest("[data-expression-drop-slot-group-path][data-expression-drop-slot-index]");
      if (slotTarget) {
        const targetGroupPath = String(
          slotTarget.getAttribute("data-expression-drop-slot-group-path") || "",
        ).trim();
        const targetGroupIndex = Number.parseInt(
          slotTarget.getAttribute("data-expression-drop-slot-index"),
          10,
        );
        if (
          targetGroupPath &&
          Number.isInteger(targetGroupIndex) &&
          !expressionPathStartsWith(sourcePath, targetGroupPath)
        ) {
          event.preventDefault();
          if (event.dataTransfer) {
            event.dataTransfer.dropEffect = "move";
          }
          this.applyExpressionDropState(slotTarget, "insert", targetGroupPath, targetGroupIndex);
          return;
        }
      }
      const nodeTarget = event.target.closest("[data-expression-drop-node-path]");
      if (nodeTarget) {
        const targetNodePath = String(nodeTarget.getAttribute("data-expression-drop-node-path") || "").trim();
        if (
          targetNodePath &&
          targetNodePath !== sourcePath &&
          !expressionPathStartsWith(sourcePath, targetNodePath) &&
          !expressionPathStartsWith(targetNodePath, sourcePath)
        ) {
          event.preventDefault();
          if (event.dataTransfer) {
            event.dataTransfer.dropEffect = "move";
          }
          this.applyExpressionDropState(nodeTarget, "group", targetNodePath);
          return;
        }
      }
      const groupTarget = event.target.closest("[data-expression-drop-group-path]");
      if (groupTarget) {
        const targetGroupPath = String(groupTarget.getAttribute("data-expression-drop-group-path") || "").trim();
        if (
          targetGroupPath &&
          targetGroupPath !== sourcePath &&
          targetGroupPath !== parentExpressionPath(sourcePath) &&
          !expressionPathStartsWith(sourcePath, targetGroupPath)
        ) {
          event.preventDefault();
          if (event.dataTransfer) {
            event.dataTransfer.dropEffect = "move";
          }
          this.applyExpressionDropState(groupTarget, "move", targetGroupPath);
          return;
        }
      }
      this.clearExpressionDropState();
    };
    this._handleDrop = (event) => {
      if (!this._dragState.sourcePath || !this._dragState.dropPath || !this._dragState.dropKind) {
        this.clearExpressionDragState();
        return;
      }
      event.preventDefault();
      const patch = buildSearchExpressionDragSignalPatch(this.signals(), {
        sourcePath: this._dragState.sourcePath,
        targetGroupIndex: this._dragState.dropKind === "insert" ? this._dragState.dropIndex : "",
        targetNodePath: this._dragState.dropKind === "group" ? this._dragState.dropPath : "",
        targetGroupPath:
          this._dragState.dropKind === "insert" || this._dragState.dropKind === "move"
            ? this._dragState.dropPath
            : "",
        groupOperator: "and",
      });
      if (patch) {
        this.dispatchPatch(patch);
      }
      this.clearExpressionDragState();
    };
    this._handleDragEnd = () => {
      this.clearExpressionDragState();
    };
    this._handleClick = (event) => {
      const operatorButton = event.target.closest(
        "button.fishy-applied-expression-operator-toggle[data-expression-group-path][data-expression-next-operator]",
      );
      if (operatorButton) {
        this.dispatchPatch(
          buildSearchExpressionOperatorSignalPatch(this.signals(), {
            groupPath: operatorButton.getAttribute("data-expression-group-path"),
            nextOperator: operatorButton.getAttribute("data-expression-next-operator"),
          }),
        );
        return;
      }
      const removeButton = event.target.closest(
        "button.fishymap-selection-remove[data-expression-remove-path], button.fishymap-selection-remove[data-fish-filter-term], button.fishymap-selection-remove[data-fish-id], button.fishymap-selection-remove[data-zone-rgb], button.fishymap-selection-remove[data-semantic-layer-id][data-semantic-field-id]",
      );
      if (removeButton) {
        this.dispatchPatch(
          buildSearchSelectionRemovalSignalPatch(this.signals(), {
            expressionPath: removeButton.getAttribute("data-expression-remove-path"),
            fishFilterTerm: removeButton.getAttribute("data-fish-filter-term"),
            fishId: removeButton.getAttribute("data-fish-id"),
            zoneRgb: removeButton.getAttribute("data-zone-rgb"),
            semanticLayerId: removeButton.getAttribute("data-semantic-layer-id"),
            semanticFieldId: removeButton.getAttribute("data-semantic-field-id"),
          }),
        );
        return;
      }
      const row = event.target.closest(
        "[data-fish-filter-term], [data-fish-id], [data-zone-rgb], [data-semantic-layer-id][data-semantic-field-id]",
      );
      if (!row) {
        return;
      }
      event.preventDefault();
      this.handleSearchResultSelection(row);
    };
    this._handleKeyDown = (event) => {
      if (event.key !== "Enter" && event.key !== " ") {
        return;
      }
      const row = event.target.closest(
        "[data-fish-filter-term], [data-fish-id], [data-zone-rgb], [data-semantic-layer-id][data-semantic-field-id]",
      );
      if (!row) {
        return;
      }
      event.preventDefault();
      this.handleSearchResultSelection(row);
    };
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name !== "data-search-state" || oldValue === newValue) {
      return;
    }
    this.scheduleRender();
  }

  connectedCallback() {
    this._shell = this.closest?.("#map-page-shell") || null;
    ensureSearchPanelMarkup(this);
    this._elements = {
      searchWindow: this._shell?.querySelector?.("#fishymap-search-window") || null,
      searchSelectionShell: this.querySelector("#fishymap-search-selection-shell"),
      searchSelection: this.querySelector("#fishymap-search-selection"),
      searchResultsShell: this.querySelector("#fishymap-search-results-shell"),
      searchResults: this.querySelector("#fishymap-search-results"),
      searchCount: this._shell?.querySelector?.("#fishymap-search-count") || null,
      zoneCatalog: this._zoneCatalog,
    };
    this.addEventListener("click", this._handleClick);
    this.addEventListener("keydown", this._handleKeyDown);
    this.addEventListener("dragstart", this._handleDragStart);
    this.addEventListener("dragover", this._handleDragOver);
    this.addEventListener("drop", this._handleDrop);
    this.addEventListener("dragend", this._handleDragEnd);
    this._elements.searchWindow?.addEventListener?.("focusout", this._handleSearchWindowFocusOut);
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.addEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this.scheduleRender();
  }

  disconnectedCallback() {
    this.removeEventListener("click", this._handleClick);
    this.removeEventListener("keydown", this._handleKeyDown);
    this.removeEventListener("dragstart", this._handleDragStart);
    this.removeEventListener("dragover", this._handleDragOver);
    this.removeEventListener("drop", this._handleDrop);
    this.removeEventListener("dragend", this._handleDragEnd);
    this._elements?.searchWindow?.removeEventListener?.("focusout", this._handleSearchWindowFocusOut);
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    this._rafId = 0;
  }

  clearExpressionDropState() {
    if (this._dragState.overElement?.dataset) {
      delete this._dragState.overElement.dataset.expressionDropMode;
    }
    this._dragState.overElement = null;
    this._dragState.dropKind = "";
    this._dragState.dropPath = "";
    this._dragState.dropIndex = -1;
  }

  clearExpressionDragState() {
    this.clearExpressionDropState();
    if (this._dragState.sourceElement?.dataset) {
      delete this._dragState.sourceElement.dataset.dragging;
    }
    removeExpressionDragProxyElement();
    this.setExpressionDraggingState(false);
    this._dragState.sourceElement = null;
    this._dragState.sourcePath = "";
  }

  setExpressionDraggingState(active) {
    if (!this._elements?.searchSelection?.dataset) {
      return;
    }
    if (active) {
      this._elements.searchSelection.dataset.expressionDragging = "true";
      return;
    }
    delete this._elements.searchSelection.dataset.expressionDragging;
  }

  applyExpressionDropState(element, kind, path, index = -1) {
    if (
      this._dragState.overElement === element &&
      this._dragState.dropKind === kind &&
      this._dragState.dropPath === path &&
      this._dragState.dropIndex === index
    ) {
      return;
    }
    this.clearExpressionDropState();
    this._dragState.overElement = element;
    this._dragState.dropKind = String(kind || "");
    this._dragState.dropPath = String(path || "");
    this._dragState.dropIndex = Number.isInteger(index) ? index : -1;
    if (element?.dataset) {
      element.dataset.expressionDropMode = this._dragState.dropKind;
    }
  }

  signals() {
    return readMapShellSignals(this._shell);
  }

  dispatchPatch(patch) {
    dispatchShellSignalPatch(this._shell, patch);
  }

  handleSearchResultSelection(row) {
    const fishFilterTerm = row.getAttribute("data-fish-filter-term");
    if (fishFilterTerm) {
      this.dispatchPatch(buildSearchMatchSignalPatch(this.signals(), { kind: "fish-filter", term: fishFilterTerm }));
      return;
    }
    const fishId = Number.parseInt(row.getAttribute("data-fish-id"), 10);
    if (Number.isFinite(fishId)) {
      this.dispatchPatch(buildSearchMatchSignalPatch(this.signals(), { kind: "fish", fishId }));
      return;
    }
    const zoneRgb = Number.parseInt(row.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      this.dispatchPatch(buildSearchMatchSignalPatch(this.signals(), { kind: "zone", zoneRgb }));
      return;
    }
    const layerId = String(row.getAttribute("data-semantic-layer-id") || "").trim();
    const fieldId = Number.parseInt(row.getAttribute("data-semantic-field-id"), 10);
    if (layerId && Number.isFinite(fieldId)) {
      this.dispatchPatch(buildSearchMatchSignalPatch(this.signals(), { kind: "semantic", layerId, fieldId }));
    }
  }

  render() {
    this._rafId = 0;
    const signals = this.signals();
    if (!signals || !this._elements?.searchResults) {
      return;
    }
    const bundle = buildSearchPanelStateBundle(signals);
    const matches = resolveSearchPanelMatches(bundle, signals?._map_ui?.search, this._zoneCatalog);

    const fishLookup = new Map((bundle.state?.catalog?.fish || []).map((fish) => [fish.fishId, fish]));
    const fishFilterMetadataByTerm = Object.fromEntries(
      FISH_FILTER_TERM_ORDER.map((term) => [term, fishFilterTermMetadata(term)]),
    );

    this._elements.zoneCatalog = this._zoneCatalog;

    renderSearchSelection(this._elements, bundle, fishLookup, {
      resolveSelectedFishIds,
      resolveSelectedFishFilterTerms,
      resolveSelectedSemanticFieldIdsByLayer,
      resolveSelectedZoneRgbs,
      buildSemanticTermLookup,
      setBooleanProperty,
      setTextContent,
      escapeHtml,
      fishFilterTermIconMarkup,
      fishIdentityMarkup,
      zoneIdentityMarkup,
      semanticIdentityMarkup,
      resolveFishGrade,
      formatZone,
      fishFilterTermMetadata: fishFilterMetadataByTerm,
    });
    renderSearchResults(this._elements, matches, bundle, {
      setBooleanProperty,
      setTextContent,
      escapeHtml,
      fishFilterTermIconMarkup,
      fishIdentityMarkup,
      zoneIdentityMarkup,
      semanticIdentityMarkup,
      formatZone,
    });
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

export function registerFishyMapSearchPanelElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (registry.get(SEARCH_PANEL_TAG_NAME)) {
    return true;
  }
  registry.define(SEARCH_PANEL_TAG_NAME, FishyMapSearchPanelElement);
  return true;
}

registerFishyMapSearchPanelElement();
