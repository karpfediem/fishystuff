import { renderSearchResults, renderSearchSelection } from "./map-search-panel.js";
import {
  buildDefaultFishFilterMatches,
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

function spriteIcon(name, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${name}"></use></svg>`;
}

function itemGradeTone(grade, isPrize) {
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
  const fishId = Number.parseInt(fish?.fishId, 10);
  const name = String(fish?.name || `Fish ${fishId || "?"}`).trim();
  const gradeTone = itemGradeTone(fish?.grade, fish?.isPrize === true || fish?.is_prize === true);
  const iconUrl =
    globalThis.window?.__fishystuffResolveFishItemIconUrl?.(fish?.itemId) ||
    globalThis.window?.__fishystuffResolveFishEncyclopediaIconUrl?.(fish?.encyclopediaId) ||
    "";
  const iconMarkup = iconUrl
    ? `<span class="fishy-item-icon-frame is-xs fishy-item-grade-${escapeHtml(gradeTone)}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishy-item-icon-frame is-xs fishy-item-grade-${escapeHtml(gradeTone)}"><span class="fishy-item-icon-fallback fishy-item-grade-${escapeHtml(gradeTone)}">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
  return `<span class="fishy-item-row fishy-item-grade-${escapeHtml(gradeTone)}">${iconMarkup}<span class="fishy-item-label truncate max-w-40">${escapeHtml(name)}</span></span>`;
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
  constructor() {
    super();
    this._shell = null;
    this._rafId = 0;
    this._zoneCatalog = [];
    this._elements = null;
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
    this._handleClick = (event) => {
      const removeButton = event.target.closest(
        "button.fishymap-selection-remove[data-fish-filter-term], button.fishymap-selection-remove[data-fish-id], button.fishymap-selection-remove[data-zone-rgb], button.fishymap-selection-remove[data-semantic-layer-id][data-semantic-field-id]",
      );
      if (removeButton) {
        this.dispatchPatch(
          buildSearchSelectionRemovalSignalPatch(this.signals(), {
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
    this._elements.searchWindow?.addEventListener?.("focusout", this._handleSearchWindowFocusOut);
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.addEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this.scheduleRender();
  }

  disconnectedCallback() {
    this.removeEventListener("click", this._handleClick);
    this.removeEventListener("keydown", this._handleKeyDown);
    this._elements?.searchWindow?.removeEventListener?.("focusout", this._handleSearchWindowFocusOut);
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    this._rafId = 0;
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
    const query = String(bundle.inputState?.filters?.searchText || "").trim();
    const searchOpen = signals?._map_ui?.search?.open === true;
    const matches =
      bundle.state.ready === true && searchOpen
        ? query
          ? buildSearchMatches(bundle, query, this._zoneCatalog)
          : buildDefaultFishFilterMatches(bundle)
        : [];

    const fishLookup = new Map((bundle.state?.catalog?.fish || []).map((fish) => [fish.fishId, fish]));

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
      formatZone,
      fishFilterTermMetadata: {
        favourite: fishFilterTermMetadata("favourite"),
        missing: fishFilterTermMetadata("missing"),
      },
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
