import {
  buildOverviewRowsForLayerSamples,
  preferredOverviewRow,
} from "./map-overview-facts.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizeInteger(value) {
  const number = Number.parseInt(value, 10);
  return Number.isInteger(number) ? number : null;
}

function normalizeCoordinate(value) {
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(number) : null;
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizeLayerSamples(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  return values.filter((sample) => isPlainObject(sample)).map((sample) => cloneJson(sample));
}

function zoneRgbFromLayerSamples(layerSamples) {
  if (!Array.isArray(layerSamples)) {
    return null;
  }
  const zoneSample = layerSamples.find(
    (sample) => trimString(sample?.layerId) === "zone_mask" && Number.isFinite(Number(sample?.rgbU32)),
  );
  return zoneSample ? Number(zoneSample.rgbU32) : null;
}

function preferredSelectionLabel(selection) {
  const pointLabel = trimString(selection?.pointLabel);
  if (pointLabel) {
    return pointLabel;
  }
  const layerSamples = Array.isArray(selection?.layerSamples) ? selection.layerSamples : [];
  const sampleLabel = layerSamples
    .map((sample) => trimString(sample?.label || sample?.name || sample?.fieldLabel))
    .find(Boolean);
  if (sampleLabel) {
    return sampleLabel;
  }
  return "";
}

function nextBookmarkId(existingBookmarks) {
  const existingIds = new Set(
    (Array.isArray(existingBookmarks) ? existingBookmarks : [])
      .map((bookmark) => trimString(bookmark?.id))
      .filter(Boolean),
  );
  if (globalThis.crypto?.randomUUID) {
    let attempt = globalThis.crypto.randomUUID();
    while (existingIds.has(attempt)) {
      attempt = globalThis.crypto.randomUUID();
    }
    return attempt;
  }
  let counter = (Date.now() % 1_000_000_000) || 1;
  let candidate = `bookmark-${counter}`;
  while (existingIds.has(candidate)) {
    counter += 1;
    candidate = `bookmark-${counter}`;
  }
  return candidate;
}

export function normalizeBookmarks(values) {
  if (!Array.isArray(values)) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of values) {
    if (!isPlainObject(value)) {
      continue;
    }
    const id = trimString(value.id);
    const worldX = normalizeCoordinate(value.worldX);
    const worldZ = normalizeCoordinate(value.worldZ);
    if (!id || worldX == null || worldZ == null || seen.has(id)) {
      continue;
    }
    seen.add(id);
    const bookmark = {
      id,
      worldX,
      worldZ,
    };
    const label = trimString(value.label);
    if (label) {
      bookmark.label = label;
    }
    const layerSamples = normalizeLayerSamples(value.layerSamples);
    if (layerSamples.length) {
      bookmark.layerSamples = layerSamples;
    }
    const zoneRgb = normalizeInteger(value.zoneRgb);
    if (zoneRgb != null) {
      bookmark.zoneRgb = zoneRgb;
    }
    const createdAt = trimString(value.createdAt);
    if (createdAt) {
      bookmark.createdAt = createdAt;
    }
    next.push(bookmark);
  }
  return next;
}

export function normalizeSelectedBookmarkIds(bookmarks, selectedIds) {
  const bookmarkIds = new Set(normalizeBookmarks(bookmarks).map((bookmark) => bookmark.id));
  if (!Array.isArray(selectedIds) || !bookmarkIds.size) {
    return [];
  }
  const next = [];
  const seen = new Set();
  for (const value of selectedIds) {
    const normalized = trimString(value);
    if (!normalized || seen.has(normalized) || !bookmarkIds.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

export function bookmarkDisplayLabel(bookmark, fallbackIndex = 0, stateBundle = null) {
  const explicitLabel = trimString(bookmark?.label);
  if (explicitLabel) {
    return explicitLabel;
  }
  const fallbackLabel =
    preferredOverviewRow(bookmark?.layerSamples, {
      zoneCatalog: stateBundle?.zoneCatalog,
      runtimeLayers: stateBundle?.state?.catalog?.layers,
    })?.value || preferredSelectionLabel(bookmark);
  if (fallbackLabel) {
    return fallbackLabel;
  }
  return `Bookmark ${fallbackIndex + 1}`;
}

export function buildBookmarkOverviewRows(bookmark, fallbackIndex = 0, stateBundle = null) {
  const displayLabel = bookmarkDisplayLabel(bookmark, fallbackIndex, stateBundle);
  const overviewRows = buildOverviewRowsForLayerSamples(bookmark?.layerSamples, {
    zoneCatalog: stateBundle?.zoneCatalog,
    runtimeLayers: stateBundle?.state?.catalog?.layers,
  }).filter(
    (row) =>
      !(
        trimString(row?.key) === "zone" &&
        trimString(row?.value).toLowerCase() === trimString(displayLabel).toLowerCase()
      ),
  );
  const rows = [
    {
      icon: "bookmark",
      label: "Bookmark",
      value: displayLabel,
      hideLabel: true,
    },
  ];
  return rows.concat(overviewRows);
}

export function createBookmarkFromSelection(selection, existingBookmarks = []) {
  const worldX = normalizeCoordinate(selection?.worldX);
  const worldZ = normalizeCoordinate(selection?.worldZ);
  if (worldX == null || worldZ == null) {
    return null;
  }
  const layerSamples = normalizeLayerSamples(selection?.layerSamples);
  const label = preferredSelectionLabel(selection);
  return {
    id: nextBookmarkId(existingBookmarks),
    worldX,
    worldZ,
    ...(label ? { label } : {}),
    ...(layerSamples.length ? { layerSamples } : {}),
    ...(zoneRgbFromLayerSamples(layerSamples) != null
      ? { zoneRgb: zoneRgbFromLayerSamples(layerSamples) }
      : {}),
    createdAt: new Date().toISOString(),
  };
}

export function renameBookmark(bookmarks, bookmarkId, nextLabel) {
  const normalizedId = trimString(bookmarkId);
  const normalizedLabel = trimString(nextLabel);
  return normalizeBookmarks(bookmarks).map((bookmark) =>
    bookmark.id === normalizedId
      ? {
          ...bookmark,
          ...(normalizedLabel ? { label: normalizedLabel } : {}),
        }
      : bookmark,
  );
}

export function moveBookmarkBefore(bookmarks, movingBookmarkId, targetBookmarkId, position = "before") {
  const normalizedBookmarks = normalizeBookmarks(bookmarks);
  const sourceId = trimString(movingBookmarkId);
  const targetId = trimString(targetBookmarkId);
  const sourceIndex = normalizedBookmarks.findIndex((bookmark) => bookmark.id === sourceId);
  const targetIndex = normalizedBookmarks.findIndex((bookmark) => bookmark.id === targetId);
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) {
    return normalizedBookmarks;
  }
  const next = normalizedBookmarks.slice();
  const [moved] = next.splice(sourceIndex, 1);
  const adjustedTargetIndex = sourceIndex < targetIndex ? targetIndex - 1 : targetIndex;
  const insertIndex = position === "after" ? adjustedTargetIndex + 1 : adjustedTargetIndex;
  next.splice(insertIndex, 0, moved);
  return next;
}

export function patchTouchesBookmarkSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return Boolean(
    patch._map_bookmarks?.entries != null ||
      patch._map_ui?.bookmarks != null ||
      patch._map_runtime?.ready != null ||
      patch._map_runtime?.view != null ||
      patch._map_runtime?.selection != null ||
      patch._map_runtime?.catalog?.layers != null,
  );
}

export function buildBookmarkPanelStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const bookmarks = normalizeBookmarks(signals?._map_bookmarks?.entries);
  const bookmarkUi = isPlainObject(signals?._map_ui?.bookmarks) ? signals._map_ui.bookmarks : {};
  return {
    state: {
      ready: runtime.ready === true,
      view: cloneJson(runtime.view || {}),
      selection: cloneJson(runtime.selection || {}),
      catalog: {
        layers: Array.isArray(runtime.catalog?.layers) ? cloneJson(runtime.catalog.layers) : [],
      },
    },
    bookmarks,
    bookmarkUi: {
      placing: bookmarkUi.placing === true,
      selectedIds: normalizeSelectedBookmarkIds(bookmarks, bookmarkUi.selectedIds),
    },
  };
}

export function selectionBookmarkKey(selection) {
  const worldX = normalizeCoordinate(selection?.worldX);
  const worldZ = normalizeCoordinate(selection?.worldZ);
  if (worldX == null || worldZ == null) {
    return "";
  }
  return JSON.stringify({
    worldX,
    worldZ,
    pointKind: trimString(selection?.pointKind),
    pointLabel: trimString(selection?.pointLabel),
  });
}
