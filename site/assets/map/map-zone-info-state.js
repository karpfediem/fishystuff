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

function normalizePointKind(value) {
  const normalized = trimString(value).toLowerCase();
  if (normalized === "bookmark" || normalized === "waypoint" || normalized === "clicked") {
    return normalized;
  }
  return "";
}

function primitiveDisplayValue(value) {
  if (value == null) {
    return "";
  }
  if (typeof value === "number") {
    return Number.isFinite(value) ? value.toLocaleString("en-US") : "";
  }
  if (typeof value === "boolean") {
    return value ? "Yes" : "No";
  }
  if (typeof value === "string") {
    return trimString(value);
  }
  return "";
}

function titleFromSelection(selection, sample) {
  const pointLabel = trimString(selection?.pointLabel);
  if (pointLabel) {
    return pointLabel;
  }
  const sampleLabel = trimString(sample?.label || sample?.name || sample?.fieldLabel);
  if (sampleLabel) {
    return sampleLabel;
  }
  const pointKind = normalizePointKind(selection?.pointKind);
  const worldX = Number(selection?.worldX);
  const worldZ = Number(selection?.worldZ);
  if (pointKind && Number.isFinite(worldX) && Number.isFinite(worldZ)) {
    return `${Math.round(worldX).toLocaleString("en-US")}, ${Math.round(worldZ).toLocaleString("en-US")}`;
  }
  return "Zone Info";
}

function pointKindStatusText(pointKind, pointLabel) {
  const normalizedLabel = trimString(pointLabel);
  switch (normalizePointKind(pointKind)) {
    case "bookmark":
      return normalizedLabel || "Bookmark";
    case "waypoint":
      return normalizedLabel || "Waypoint";
    case "clicked":
      return "Clicked point";
    default:
      return "no selection";
  }
}

function pointKindIcon(pointKind) {
  switch (normalizePointKind(pointKind)) {
    case "bookmark":
      return "bookmark";
    case "waypoint":
      return "map-pin";
    case "clicked":
      return "hover-zone";
    default:
      return "information-circle";
  }
}

function normalizeSelectionLayerSamples(selection) {
  if (!Array.isArray(selection?.layerSamples)) {
    return [];
  }
  return selection.layerSamples.filter((sample) => isPlainObject(sample)).map((sample) => cloneJson(sample));
}

function buildLayerNameLookup(runtimeLayers) {
  return new Map(
    (Array.isArray(runtimeLayers) ? runtimeLayers : [])
      .filter((layer) => isPlainObject(layer))
      .map((layer) => [trimString(layer.layerId), trimString(layer.name) || trimString(layer.layerId)]),
  );
}

function zoneInfoTabId(sample, fallbackIndex) {
  const layerId = trimString(sample?.layerId);
  if (layerId) {
    return layerId;
  }
  return `sample-${fallbackIndex}`;
}

function zoneInfoTabLabel(sample, layerNameById, fallbackIndex) {
  const layerId = trimString(sample?.layerId);
  return layerNameById.get(layerId) || trimString(sample?.layerName) || `Layer ${fallbackIndex + 1}`;
}

function zoneInfoFacts(selection, sample, layerNameById) {
  const facts = [];
  const layerId = trimString(sample?.layerId);
  const layerName = layerNameById.get(layerId) || layerId;
  if (layerName) {
    facts.push({ icon: "squares-2x2", label: "Layer", value: layerName });
  }
  const label = trimString(sample?.label || sample?.name || sample?.fieldLabel);
  if (label) {
    facts.push({ icon: "information-circle", label: "Label", value: label });
  }
  const worldX = Number(selection?.worldX);
  const worldZ = Number(selection?.worldZ);
  if (Number.isFinite(worldX) && Number.isFinite(worldZ)) {
    facts.push({
      icon: "map-pin",
      label: "World",
      value: `${Math.round(worldX).toLocaleString("en-US")}, ${Math.round(worldZ).toLocaleString("en-US")}`,
    });
  }
  for (const [key, value] of Object.entries(sample || {})) {
    if (["layerId", "label", "name", "fieldLabel"].includes(key)) {
      continue;
    }
    const displayValue = primitiveDisplayValue(value);
    if (!displayValue) {
      continue;
    }
    const labelText = key.replace(/_/g, " ");
    if (facts.some((fact) => fact.label.toLowerCase() === labelText.toLowerCase())) {
      continue;
    }
    facts.push({
      icon: "information-circle",
      label: labelText,
      value: displayValue,
    });
  }
  return facts;
}

export function patchTouchesZoneInfoSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return Boolean(
    patch._map_runtime?.selection != null ||
      patch._map_runtime?.catalog?.layers != null ||
      patch._map_ui?.windowUi?.zoneInfo != null,
  );
}

export function buildZoneInfoViewModel(signals) {
  const selection = isPlainObject(signals?._map_runtime?.selection)
    ? cloneJson(signals._map_runtime.selection)
    : {};
  const layerSamples = normalizeSelectionLayerSamples(selection);
  const layerNameById = buildLayerNameLookup(signals?._map_runtime?.catalog?.layers);
  const tabs = layerSamples.map((sample, index) => ({
    id: zoneInfoTabId(sample, index),
    label: zoneInfoTabLabel(sample, layerNameById, index),
    sample,
  }));
  const requestedTabId = trimString(signals?._map_ui?.windowUi?.zoneInfo?.tab);
  const activeTabId =
    requestedTabId && tabs.some((tab) => tab.id === requestedTabId)
      ? requestedTabId
      : tabs[0]?.id || "";
  const activeTab = tabs.find((tab) => tab.id === activeTabId) || null;
  const pointKind = normalizePointKind(selection?.pointKind);
  return {
    descriptor: {
      title: titleFromSelection(selection, activeTab?.sample),
      titleIcon: pointKindIcon(pointKind),
      statusIcon: pointKindIcon(pointKind),
      statusText: pointKindStatusText(pointKind, selection?.pointLabel),
      pointKind,
    },
    tabs: tabs.map((tab) => ({
      id: tab.id,
      label: tab.label,
    })),
    activeTabId,
    empty: tabs.length === 0,
    facts: activeTab ? zoneInfoFacts(selection, activeTab.sample, layerNameById) : [],
  };
}
