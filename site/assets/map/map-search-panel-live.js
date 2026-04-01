import { renderSearchResults, renderSearchSelection } from "./map-search-panel.js";
import { dispatchShellSignalPatch } from "./map-signal-patch.js";
import { normalizeZoneCatalog } from "./map-zone-catalog.js";
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
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

export { patchTouchesSearchPanelSignals } from "./map-search-state.js";

const ICON_SPRITE_URL = "/img/icons.svg";

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

export function createMapSearchPanelController({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  zoneCatalog = [],
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  listenToSignalPatches = true,
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapSearchPanelController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapSearchPanelController requires getSignals()");
  }

  const elements = {
    searchWindow: shell.querySelector("#fishymap-search-window"),
    searchInput: shell.querySelector("#fishymap-search"),
    searchSelectionShell: shell.querySelector("#fishymap-search-selection-shell"),
    searchSelection: shell.querySelector("#fishymap-search-selection"),
    searchResultsShell: shell.querySelector("#fishymap-search-results-shell"),
    searchResults: shell.querySelector("#fishymap-search-results"),
    searchCount: shell.querySelector("#fishymap-search-count"),
    zoneCatalog,
  };
  if (!(elements.searchWindow instanceof HTMLElement) || !(elements.searchResults instanceof HTMLElement)) {
    throw new Error("createMapSearchPanelController requires live search elements");
  }

  const state = {
    frameId: 0,
  };
  let currentZoneCatalog = normalizeZoneCatalog(zoneCatalog);
  elements.zoneCatalog = currentZoneCatalog;

  function signals() {
    return getSignals() || null;
  }

  function buildBundle() {
    return buildSearchPanelStateBundle(signals());
  }

  function fishLookup(bundle) {
    return new Map((bundle.state?.catalog?.fish || []).map((fish) => [fish.fishId, fish]));
  }

  function render() {
    state.frameId = 0;
    const bundle = buildBundle();
    const query = String(bundle.inputState?.filters?.searchText || "").trim();
    const searchOpen = signals()?._map_ui?.search?.open === true;
    const matches =
      bundle.state.ready === true && searchOpen
        ? query
          ? buildSearchMatches(bundle, query, currentZoneCatalog)
          : buildDefaultFishFilterMatches(bundle)
        : [];

    renderSearchSelection(elements, bundle, fishLookup(bundle), {
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
    renderSearchResults(elements, matches, bundle, {
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

  function applyPatch(patch) {
    dispatchPatch(shell, patch);
    scheduleRender();
  }

  function handleSearchResultSelection(row) {
    if (!row) {
      return;
    }
    const fishFilterTerm = row.getAttribute("data-fish-filter-term");
    if (fishFilterTerm) {
      applyPatch(buildSearchMatchSignalPatch(signals(), { kind: "fish-filter", term: fishFilterTerm }));
      return;
    }
    const fishId = Number.parseInt(row.getAttribute("data-fish-id"), 10);
    if (Number.isFinite(fishId)) {
      applyPatch(buildSearchMatchSignalPatch(signals(), { kind: "fish", fishId }));
      return;
    }
    const zoneRgb = Number.parseInt(row.getAttribute("data-zone-rgb"), 10);
    if (Number.isFinite(zoneRgb)) {
      applyPatch(buildSearchMatchSignalPatch(signals(), { kind: "zone", zoneRgb }));
      return;
    }
    const layerId = String(row.getAttribute("data-semantic-layer-id") || "").trim();
    const fieldId = Number.parseInt(row.getAttribute("data-semantic-field-id"), 10);
    if (layerId && Number.isFinite(fieldId)) {
      applyPatch(buildSearchMatchSignalPatch(signals(), { kind: "semantic", layerId, fieldId }));
    }
  }

  elements.searchResults.addEventListener("click", (event) => {
    const row = event.target.closest(
      "[data-fish-filter-term], [data-fish-id], [data-zone-rgb], [data-semantic-layer-id][data-semantic-field-id]",
    );
    if (!row) {
      return;
    }
    event.preventDefault();
    handleSearchResultSelection(row);
  });

  elements.searchResults.addEventListener("keydown", (event) => {
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
    handleSearchResultSelection(row);
  });

  elements.searchSelection.addEventListener("click", (event) => {
    const removeButton = event.target.closest(
      "button.fishymap-selection-remove[data-fish-filter-term], button.fishymap-selection-remove[data-fish-id], button.fishymap-selection-remove[data-zone-rgb], button.fishymap-selection-remove[data-semantic-layer-id][data-semantic-field-id]",
    );
    if (!removeButton) {
      return;
    }
    applyPatch(
      buildSearchSelectionRemovalSignalPatch(signals(), {
        fishFilterTerm: removeButton.getAttribute("data-fish-filter-term"),
        fishId: removeButton.getAttribute("data-fish-id"),
        zoneRgb: removeButton.getAttribute("data-zone-rgb"),
        semanticLayerId: removeButton.getAttribute("data-semantic-layer-id"),
        semanticFieldId: removeButton.getAttribute("data-semantic-field-id"),
      }),
    );
  });

  if (listenToSignalPatches) {
    shell.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, (event) => {
      if (patchTouchesSearchPanelSignals(event?.detail || null)) {
        scheduleRender();
      }
    });
    shell.addEventListener(FISHYMAP_ZONE_CATALOG_READY_EVENT, (event) => {
      currentZoneCatalog = normalizeZoneCatalog(event?.detail?.zoneCatalog);
      elements.zoneCatalog = currentZoneCatalog;
      scheduleRender();
    });
  }

  elements.searchWindow.addEventListener("focusout", () => {
    globalThis.setTimeout?.(() => {
      const activeElement = globalThis.document?.activeElement;
      if (activeElement instanceof Element && elements.searchWindow.contains(activeElement)) {
        return;
      }
      applyPatch({
        _map_ui: {
          search: {
            open: false,
          },
        },
      });
    }, 0);
  });

  return Object.freeze({
    render,
    scheduleRender,
  });
}
