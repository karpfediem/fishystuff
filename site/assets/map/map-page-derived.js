import { parseQuerySignalPatch } from "./map-query-state.js";
import { buildSearchProjectionSignalPatch } from "./map-search-projection.js";
import {
  appendSearchExpressionTerm,
  buildSearchExpressionStatePatch,
  resolveSearchExpression,
  resolveSelectedSearchTerms,
} from "./map-search-contract.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { buildFocusWorldPointSignalPatch } from "./map-selection-actions.js";
import {
  loadTradeNpcMapCatalog,
  tradeNpcFocusTargetForSelectors,
} from "./map-trade-summary.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function mergeProjectionPatch(target, patch, prefix = "") {
  if (!isPlainObject(target) || !isPlainObject(patch)) {
    return target;
  }
  for (const [key, value] of Object.entries(patch)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (path === "_map_ui.search.expression") {
      target[key] = cloneJson(value);
      continue;
    }
    if (Array.isArray(value)) {
      target[key] = cloneJson(value);
      continue;
    }
    if (isPlainObject(value)) {
      const nextTarget = isPlainObject(target[key]) ? target[key] : {};
      target[key] = nextTarget;
      mergeProjectionPatch(nextTarget, value, path);
      continue;
    }
    target[key] = value;
  }
  return target;
}

function currentLocationHref(globalRef = globalThis) {
  return globalRef.location?.href || globalRef.window?.location?.href || "";
}

function normalizePendingQueryFishSelectors(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    const normalized = String(value ?? "").trim();
    if (!normalized) {
      continue;
    }
    const lookupKey = normalized.toLowerCase();
    if (seen.has(lookupKey)) {
      continue;
    }
    seen.add(lookupKey);
    next.push(normalized);
  }
  return next;
}

function normalizePendingQueryNpcSelectors(values) {
  return normalizePendingQueryFishSelectors(values);
}

function normalizeFishLookupKey(value) {
  return String(value ?? "").trim().toLowerCase().replace(/\s+/g, " ");
}

function slugifyFishLookupKey(value) {
  const normalized = normalizeFishLookupKey(value);
  const ascii =
    typeof normalized.normalize === "function"
      ? normalized.normalize("NFKD").replace(/[\u0300-\u036f]/g, "")
      : normalized;
  return ascii
    .replace(/['"]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function buildFishSelectorLookup(catalogFish) {
  const lookup = new Map();
  for (const fish of Array.isArray(catalogFish) ? catalogFish : []) {
    const fishId = Number.parseInt(fish?.fishId, 10);
    if (!Number.isInteger(fishId) || fishId <= 0) {
      continue;
    }
    for (const key of [normalizeFishLookupKey(fish?.name), slugifyFishLookupKey(fish?.name)]) {
      if (!key || lookup.has(key)) {
        continue;
      }
      lookup.set(key, fishId);
    }
  }
  return lookup;
}

function resolvePendingQueryFishIds(pendingQueryFishSelectors, catalogFish) {
  const fishSelectorLookup = buildFishSelectorLookup(catalogFish);
  const resolvedFishIds = [];
  const seen = new Set();
  for (const selector of normalizePendingQueryFishSelectors(pendingQueryFishSelectors)) {
    const fishId =
      fishSelectorLookup.get(normalizeFishLookupKey(selector))
      ?? fishSelectorLookup.get(slugifyFishLookupKey(selector));
    if (!Number.isInteger(fishId) || seen.has(fishId)) {
      continue;
    }
    seen.add(fishId);
    resolvedFishIds.push(fishId);
  }
  return resolvedFishIds;
}

export function buildSearchProjectionPatchForSignalPatch(signals, patch) {
  if (
    patch?._map_ui?.search?.selectedTerms == null &&
    patch?._map_ui?.search?.expression == null
  ) {
    return null;
  }
  const nextSignals = isPlainObject(signals) ? cloneJson(signals) : {};
  mergeProjectionPatch(nextSignals, patch);
  return buildSearchProjectionSignalPatch(nextSignals);
}

export function buildQueryFishSelectionSignalPatch(signals) {
  const pendingQueryFishSelectors = normalizePendingQueryFishSelectors(
    signals?._map_ui?.search?.pendingQueryFishSelectors,
  );
  if (!pendingQueryFishSelectors.length) {
    return null;
  }
  const catalogFish = Array.isArray(signals?._map_runtime?.catalog?.fish)
    ? signals._map_runtime.catalog.fish
    : [];
  if (!catalogFish.length) {
    return null;
  }

  const currentExpression = resolveSearchExpression(
    signals?._map_ui?.search?.expression,
    signals?._map_ui?.search?.selectedTerms,
  );
  const resolvedFishIds = resolvePendingQueryFishIds(pendingQueryFishSelectors, catalogFish);
  let nextExpression = currentExpression;
  for (const fishId of resolvedFishIds) {
    nextExpression = appendSearchExpressionTerm(nextExpression, { kind: "fish", fishId });
  }

  const currentTermsJson = JSON.stringify(
    resolveSelectedSearchTerms(undefined, currentExpression),
  );
  const nextTermsJson = JSON.stringify(
    resolveSelectedSearchTerms(undefined, nextExpression),
  );
  if (currentTermsJson === nextTermsJson) {
    return {
      _map_ui: {
        search: {
          pendingQueryFishSelectors: [],
        },
      },
    };
  }

  const patch = buildSearchExpressionStatePatch(nextExpression);
  patch._map_ui.search.pendingQueryFishSelectors = [];
  return patch;
}

export async function buildQueryNpcFocusSignalPatch(
  signals,
  { loadTradeNpcMapCatalogImpl = loadTradeNpcMapCatalog } = {},
) {
  const pendingQueryNpcSelectors = normalizePendingQueryNpcSelectors(
    signals?._map_ui?.search?.pendingQueryNpcSelectors,
  );
  if (!pendingQueryNpcSelectors.length) {
    return null;
  }
  let catalog = null;
  try {
    catalog = await loadTradeNpcMapCatalogImpl();
  } catch (_error) {
    return {
      _map_ui: {
        search: {
          pendingQueryNpcSelectors: [],
        },
      },
    };
  }
  const focusWorldPoint = tradeNpcFocusTargetForSelectors(pendingQueryNpcSelectors, catalog);
  const patch = {
    _map_ui: {
      search: {
        pendingQueryNpcSelectors: [],
      },
    },
  };
  const focusPatch = buildFocusWorldPointSignalPatch(focusWorldPoint, signals);
  if (!focusPatch) {
    return patch;
  }
  return mergeProjectionPatch(focusPatch, patch);
}

export function createMapPageDerivedController({
  globalRef = globalThis,
  shell = null,
  readSignals = () => null,
  dispatchPatch = () => {},
} = {}) {
  let boundShell = null;

  function handleSignalPatch(eventOrPatch) {
    const patch = eventOrPatch?.detail ?? eventOrPatch;
    const queryFishSelectionPatch = buildQueryFishSelectionSignalPatch(readSignals());
    if (queryFishSelectionPatch) {
      dispatchPatch(queryFishSelectionPatch);
      if (
        queryFishSelectionPatch?._map_ui?.search?.selectedTerms != null ||
        queryFishSelectionPatch?._map_ui?.search?.expression != null
      ) {
        return true;
      }
    }

    const projectionPatch = buildSearchProjectionPatchForSignalPatch(readSignals(), patch);
    if (!projectionPatch) {
      return Boolean(queryFishSelectionPatch);
    }
    dispatchPatch(projectionPatch);
    return true;
  }

  function applyInitialPatches(locationHref = currentLocationHref(globalRef)) {
    const queryPatch = parseQuerySignalPatch(locationHref);
    if (queryPatch) {
      dispatchPatch(queryPatch);
    }
    const queryFishSelectionPatch = buildQueryFishSelectionSignalPatch(readSignals() || {});
    if (queryFishSelectionPatch) {
      dispatchPatch(queryFishSelectionPatch);
    }
    const projectionPatch = buildSearchProjectionSignalPatch(readSignals() || {});
    if (projectionPatch) {
      dispatchPatch(projectionPatch);
    }
    void buildQueryNpcFocusSignalPatch(readSignals() || {}).then((queryNpcFocusPatch) => {
      if (queryNpcFocusPatch) {
        dispatchPatch(queryNpcFocusPatch);
      }
    });
    return Object.freeze({
      queryPatch,
      queryFishSelectionPatch,
      projectionPatch,
    });
  }

  function start(nextShell = shell) {
    const target = nextShell && typeof nextShell.addEventListener === "function" ? nextShell : null;
    if (!target) {
      return false;
    }
    if (boundShell && boundShell !== target && typeof boundShell.removeEventListener === "function") {
      boundShell.removeEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, handleSignalPatch);
    }
    if (boundShell === target) {
      return true;
    }
    target.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, handleSignalPatch);
    boundShell = target;
    return true;
  }

  return Object.freeze({
    applyInitialPatches,
    handleSignalPatch,
    start,
  });
}
