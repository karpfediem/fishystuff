import {
  buildSearchSelectionStatePatch,
  normalizeFishFilterTerms,
} from "./map-search-contract.js";

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function parseIntegerParam(value) {
  if (value == null || value === "") {
    return null;
  }
  const parsed = Number.parseInt(String(value), 10);
  return Number.isInteger(parsed) ? parsed : null;
}

function parseBooleanParam(value) {
  if (value == null) {
    return null;
  }
  const normalized = String(value).trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) {
    return true;
  }
  if (["0", "false", "no", "off"].includes(normalized)) {
    return false;
  }
  return null;
}

function normalizeStringList(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const seen = new Set();
  const next = [];
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

function parseDelimitedTerms(value) {
  if (value == null) {
    return [];
  }
  return normalizeStringList(
    String(value)
      .split(/[\s,]+/g)
      .map((term) => term.trim().toLowerCase())
      .filter(Boolean),
  );
}

function parseFishFilterTermsParam(value) {
  if (value == null) {
    return null;
  }
  return normalizeFishFilterTerms(parseDelimitedTerms(value));
}

function parseLayerSetParam(value) {
  if (value == null) {
    return null;
  }
  return parseDelimitedTerms(value);
}

function parseViewMode(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "2d" || normalized === "3d") {
    return normalized;
  }
  return null;
}

function stripEmptyBranches(patch) {
  if (!isPlainObject(patch)) {
    return null;
  }
  if (isPlainObject(patch._map_ui?.search) && !Object.keys(patch._map_ui.search).length) {
    delete patch._map_ui.search;
  }
  if (isPlainObject(patch._map_ui) && !Object.keys(patch._map_ui).length) {
    delete patch._map_ui;
  }
  if (isPlainObject(patch._map_bridged?.filters) && !Object.keys(patch._map_bridged.filters).length) {
    delete patch._map_bridged.filters;
  }
  if (isPlainObject(patch._map_bridged?.ui) && !Object.keys(patch._map_bridged.ui).length) {
    delete patch._map_bridged.ui;
  }
  if (isPlainObject(patch._map_bridged) && !Object.keys(patch._map_bridged).length) {
    delete patch._map_bridged;
  }
  return Object.keys(patch).length ? patch : null;
}

export function parseQuerySignalPatch(locationHref = globalThis.location?.href) {
  if (!locationHref) {
    return null;
  }

  let params;
  try {
    params = new URL(locationHref, "https://fishystuff.fish").searchParams;
  } catch (_error) {
    return null;
  }

  const fishId =
    parseIntegerParam(params.get("focusFish")) ?? parseIntegerParam(params.get("fish"));
  const patchId = params.get("patch");
  const fromPatchId = params.get("fromPatch") ?? params.get("patchFrom");
  const toPatchId =
    params.get("toPatch") ?? params.get("untilPatch") ?? params.get("patchTo");
  const fishFilterTerms = parseFishFilterTermsParam(params.get("fishTerms"))
    ?? parseFishFilterTermsParam(params.get("fishFilterTerms"))
    ?? [];
  const searchQuery = params.get("search");
  const diagnosticsOpen = parseBooleanParam(params.get("diagnostics"));
  const layers = parseLayerSetParam(params.get("layers"))
    ?? parseLayerSetParam(params.get("layerSet"))
    ?? [];
  const viewMode = parseViewMode(params.get("view") ?? params.get("mode"));

  /** @type {Record<string, unknown>} */
  const patch = {};

  const selectedTerms = [];
  if (fishId != null) {
    selectedTerms.push({ kind: "fish", fishId });
  }
  for (const term of fishFilterTerms) {
    selectedTerms.push({ kind: "fish-filter", term });
  }

  if (searchQuery != null) {
    patch._map_ui = {
      search: {
        query: String(searchQuery),
        ...(String(searchQuery).trim() ? { open: true } : {}),
      },
    };
  }

  if (selectedTerms.length) {
    const searchSelectionPatch = buildSearchSelectionStatePatch(selectedTerms);
    patch._map_ui = {
      ...(isPlainObject(patch._map_ui) ? patch._map_ui : {}),
      ...(isPlainObject(searchSelectionPatch._map_ui) ? searchSelectionPatch._map_ui : {}),
      search: {
        ...(isPlainObject(searchSelectionPatch._map_ui?.search)
          ? searchSelectionPatch._map_ui.search
          : {}),
        ...(isPlainObject(patch._map_ui?.search) ? patch._map_ui.search : {}),
      },
    };
    patch._map_bridged = {
      ...(isPlainObject(searchSelectionPatch._map_bridged)
        ? searchSelectionPatch._map_bridged
        : {}),
    };
  }

  if (
    patchId != null ||
    fromPatchId != null ||
    toPatchId != null ||
    layers.length
  ) {
    patch._map_bridged = patch._map_bridged || {};
    patch._map_bridged.filters = patch._map_bridged.filters || {};
    if (fromPatchId != null || toPatchId != null) {
      if (fromPatchId != null) {
        patch._map_bridged.filters.fromPatchId = fromPatchId || null;
      }
      if (toPatchId != null) {
        patch._map_bridged.filters.toPatchId = toPatchId || null;
      }
    } else if (patchId != null) {
      patch._map_bridged.filters.patchId = patchId || null;
    }
    if (layers.length) {
      patch._map_bridged.filters.layerIdsVisible = layers;
    }
  }

  if (diagnosticsOpen != null || viewMode != null) {
    patch._map_bridged = patch._map_bridged || {};
    patch._map_bridged.ui = {};
    if (diagnosticsOpen != null) {
      patch._map_bridged.ui.diagnosticsOpen = diagnosticsOpen;
    }
    if (viewMode != null) {
      patch._map_bridged.ui.viewMode = viewMode;
    }
  }

  return stripEmptyBranches(patch);
}
