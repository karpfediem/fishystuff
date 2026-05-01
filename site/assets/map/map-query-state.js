import {
  buildSearchSelectionStatePatch,
  normalizeFishFilterTerms,
  normalizePatchId,
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

function parseDelimitedParamValues(values, pattern = /,/g) {
  if (!Array.isArray(values) || !values.length) {
    return [];
  }
  return normalizeStringList(
    values.flatMap((value) =>
      String(value ?? "")
        .split(pattern)
        .map((term) => term.trim())
        .filter(Boolean),
    ),
  );
}

function parseDelimitedTerms(value) {
  if (value == null) {
    return [];
  }
  return parseDelimitedParamValues([value], /[\s,]+/g).map((term) => term.toLowerCase());
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

function parseFishSelectors(params, keys) {
  const values = Array.isArray(keys)
    ? keys.flatMap((key) => params.getAll(key))
    : [];
  if (!values.length) {
    return null;
  }
  return parseDelimitedParamValues(values, /,/g);
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

  const fishSelectors = parseFishSelectors(
    params,
    params.has("focusFish") ? ["focusFish"] : ["fish"],
  ) || [];
  const npcSelectors = parseFishSelectors(params, ["npc"]) || [];
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
  const pendingQueryFishSelectors = [];
  for (const fishSelector of fishSelectors) {
    const fishId = parseIntegerParam(fishSelector);
    if (fishId != null) {
      selectedTerms.push({ kind: "fish", fishId });
      continue;
    }
    pendingQueryFishSelectors.push(fishSelector);
  }
  for (const term of fishFilterTerms) {
    selectedTerms.push({ kind: "fish-filter", term });
  }
  const exactPatchId = normalizePatchId(patchId);
  const normalizedFromPatchId = normalizePatchId(fromPatchId) || exactPatchId;
  const normalizedToPatchId = normalizePatchId(toPatchId) || exactPatchId;
  if (normalizedFromPatchId) {
    selectedTerms.push({ kind: "patch-bound", bound: "from", patchId: normalizedFromPatchId });
  }
  if (normalizedToPatchId) {
    selectedTerms.push({ kind: "patch-bound", bound: "to", patchId: normalizedToPatchId });
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

  if (pendingQueryFishSelectors.length) {
    patch._map_ui = patch._map_ui || {};
    patch._map_ui.search = {
      ...(isPlainObject(patch._map_ui.search) ? patch._map_ui.search : {}),
      pendingQueryFishSelectors,
    };
  }

  if (npcSelectors.length) {
    patch._map_ui = patch._map_ui || {};
    patch._map_ui.search = {
      ...(isPlainObject(patch._map_ui.search) ? patch._map_ui.search : {}),
      pendingQueryNpcSelectors: npcSelectors,
    };
  }

  if (
    layers.length
  ) {
    patch._map_bridged = patch._map_bridged || {};
    patch._map_bridged.filters = patch._map_bridged.filters || {};
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
