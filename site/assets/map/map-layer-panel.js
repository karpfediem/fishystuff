import {
  FISHYMAP_POINT_ICON_SCALE_MAX,
  FISHYMAP_POINT_ICON_SCALE_MIN,
} from "./map-host.js";
import {
  clampLayerOpacity,
  clampPointIconScale,
  flattenLayerClipMasks,
  isFixedGroundLayer,
  layerOpacityLabel,
  layerOpacityValue,
  pointIconScaleLabel,
  pointIconScaleValue,
  resolveLayerEntries,
} from "./map-layer-state.js";

function layerKindLabel(kind) {
  if (kind === "fish-evidence") {
    return "Evidence";
  }
  if (kind === "vector-geojson") {
    return "Vector";
  }
  if (kind === "waypoints") {
    return "Waypoints";
  }
  if (kind === "tiled-raster") {
    return "Raster";
  }
  return "Layer";
}

function escapeAttribute(value) {
  return String(value || "").replace(/"/g, "&quot;");
}

export function renderLayerStack(container, stateBundle, options = {}) {
  const layers = resolveLayerEntries(stateBundle);
  const expandedLayerIds =
    options.expandedLayerIds instanceof Set ? options.expandedLayerIds : new Set();
  const renderLoadingPanelMarkup =
    typeof options.renderLoadingPanelMarkup === "function"
      ? options.renderLoadingPanelMarkup
      : (label) => String(label || "");
  const escapeHtml =
    typeof options.escapeHtml === "function" ? options.escapeHtml : (value) => String(value || "");
  const dragHandleIcon =
    typeof options.dragHandleIcon === "function" ? options.dragHandleIcon : () => "";
  const layerSettingsIcon =
    typeof options.layerSettingsIcon === "function" ? options.layerSettingsIcon : () => "";
  const eyeIcon = typeof options.eyeIcon === "function" ? options.eyeIcon : () => "";

  if (!layers.length) {
    const loadingKey = "__loading__";
    if (container.dataset.renderKey !== loadingKey) {
      container.dataset.renderKey = loadingKey;
      container.innerHTML = renderLoadingPanelMarkup("Layer registry is loading…");
    }
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
      layer.clipMaskLayerId || "",
      layer.supportsWaypointConnections ? 1 : 0,
      layer.waypointConnectionsVisible ? 1 : 0,
      layer.supportsWaypointLabels ? 1 : 0,
      layer.waypointLabelsVisible ? 1 : 0,
      layer.supportsPointIcons ? 1 : 0,
      layer.pointIconsVisible ? 1 : 0,
      Math.round(clampPointIconScale(layer.pointIconScale) * 1000),
      Math.round(clampPointIconScale(layer.pointIconScaleDefault) * 1000),
      Number.isFinite(layer.displayOrder) ? layer.displayOrder : 0,
      layer.locked ? 1 : 0,
      expandedLayerIds.has(layer.layerId) ? 1 : 0,
    ]),
  );
  if (container.dataset.renderKey === renderKey) {
    return;
  }
  container.dataset.renderKey = renderKey;

  const layerNameById = new Map(layers.map((layer) => [layer.layerId, layer.name]));
  const clipMasks = {};
  for (const layer of layers) {
    const clipMaskLayerId = String(layer.clipMaskLayerId || "").trim();
    if (
      !clipMaskLayerId ||
      clipMaskLayerId === layer.layerId ||
      !layerNameById.has(clipMaskLayerId) ||
      isFixedGroundLayer(clipMaskLayerId)
    ) {
      continue;
    }
    clipMasks[layer.layerId] = clipMaskLayerId;
  }
  const flatClipMasks = flattenLayerClipMasks(clipMasks);
  const clippedLayersByMask = new Map();
  for (const layer of layers) {
    const clipMaskLayerId = String(flatClipMasks[layer.layerId] || "").trim();
    if (!clipMaskLayerId) {
      continue;
    }
    const clippedLayers = clippedLayersByMask.get(clipMaskLayerId) || [];
    clippedLayers.push({
      layer,
      indentLevel: 1,
    });
    clippedLayersByMask.set(clipMaskLayerId, clippedLayers);
  }
  const displayedLayers = [];
  const displayedLayerIds = new Set();
  for (const layer of layers) {
    if (flatClipMasks[layer.layerId]) {
      continue;
    }
    displayedLayers.push({ layer, indentLevel: 0 });
    displayedLayerIds.add(layer.layerId);
    for (const child of clippedLayersByMask.get(layer.layerId) || []) {
      displayedLayers.push(child);
      displayedLayerIds.add(child.layer.layerId);
    }
  }
  for (const layer of layers) {
    if (displayedLayerIds.has(layer.layerId)) {
      continue;
    }
    displayedLayers.push({ layer, indentLevel: 0 });
  }

  container.innerHTML = displayedLayers
    .map(({ layer, indentLevel }) => {
      const visible = Boolean(layer.visible);
      const locked = Boolean(layer.locked);
      const settingsExpanded = expandedLayerIds.has(layer.layerId);
      const kind = layerKindLabel(layer.kind);
      const visibilityLabel = visible ? "Hide" : "Show";
      const clipMaskValue = String(flatClipMasks[layer.layerId] || "").trim();
      const clipMaskName = clipMaskValue ? layerNameById.get(clipMaskValue) || clipMaskValue : "";
      const clippedLayers = clippedLayersByMask.get(layer.layerId) || [];
      const clippedLayerNames = clippedLayers.map((candidate) => candidate.layer.name);
      const relationBadges = [];
      if (clipMaskName) {
        relationBadges.push(
          `<span class="badge badge-soft badge-xs">Clipped by ${escapeHtml(clipMaskName)}</span>`,
        );
      }
      if (clippedLayers.length) {
        relationBadges.push(
          `<span class="badge badge-soft badge-xs">Masks ${clippedLayers.length}</span>`,
        );
      }
      const waypointControls = [];
      if (layer.supportsWaypointConnections) {
        waypointControls.push(`
          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input
              class="toggle toggle-xs toggle-primary"
              data-layer-waypoint-connections="${escapeAttribute(layer.layerId)}"
              type="checkbox"
              ${layer.waypointConnectionsVisible ? "checked" : ""}
            >
            <span class="label-text text-xs text-base-content/70">Connections</span>
          </label>
        `);
      }
      if (layer.supportsWaypointLabels) {
        waypointControls.push(`
          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input
              class="toggle toggle-xs toggle-primary"
              data-layer-waypoint-labels="${escapeAttribute(layer.layerId)}"
              type="checkbox"
              ${layer.waypointLabelsVisible ? "checked" : ""}
            >
            <span class="label-text text-xs text-base-content/70">Names</span>
          </label>
        `);
      }
      const pointControls = [];
      if (layer.supportsPointIcons) {
        pointControls.push(`
          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input
              class="toggle toggle-xs toggle-primary"
              data-layer-point-icons="${escapeAttribute(layer.layerId)}"
              type="checkbox"
              ${layer.pointIconsVisible ? "checked" : ""}
            >
            <span class="label-text text-xs text-base-content/70">Icons</span>
          </label>
        `);
      }
      return `
        <article
          class="fishymap-layer-card card card-border bg-base-200"
          data-layer-id="${escapeAttribute(layer.layerId)}"
          data-indent-level="${indentLevel > 0 ? "1" : "0"}"
          data-locked="${locked ? "true" : "false"}"
          data-settings-expanded="${settingsExpanded ? "true" : "false"}"
          data-clip-mask-source="${locked ? "false" : "true"}"
          style="--fishymap-layer-indent:${indentLevel};"
        >
          <button
            class="fishymap-layer-drag btn btn-sm btn-circle btn-ghost"
            data-layer-drag="${escapeAttribute(layer.layerId)}"
            type="button"
            aria-label="${locked ? `${layer.name} is pinned to the ground layer` : `Drag ${layer.name}`}"
            draggable="${locked ? "false" : "true"}"
            ${locked ? "disabled" : ""}
            tabindex="-1"
          >
            ${dragHandleIcon()}
          </button>
          <div class="fishymap-layer-body min-w-0">
            <div class="fishymap-layer-header">
              <span class="truncate text-sm font-semibold">${escapeHtml(layer.name)}</span>
            </div>
            ${
              settingsExpanded
                ? `
                  <div class="fishymap-layer-controls">
                    <div class="fishymap-layer-relations">
                      <span class="badge badge-ghost badge-xs">${kind}</span>
                      ${locked ? '<span class="badge badge-outline badge-xs">Ground</span>' : ""}
                      ${relationBadges.join("")}
                    </div>
                    ${
                      clippedLayerNames.length
                        ? `
                          <p class="text-[11px] text-base-content/45">
                            Masking ${escapeHtml(clippedLayerNames.join(", "))}
                          </p>
                        `
                        : ""
                    }
                    ${
                      locked
                        ? ""
                        : `
                          <fieldset class="fishymap-layer-opacity-control fieldset">
                            <div class="flex items-center justify-between gap-3">
                              <span class="fieldset-legend m-0 px-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Opacity</span>
                              <span class="text-xs font-semibold text-base-content/60" data-layer-opacity-value>${layerOpacityLabel(layer.opacity)}</span>
                            </div>
                            <input
                              class="fishymap-layer-opacity-range range range-primary range-xs"
                              data-layer-opacity="${escapeAttribute(layer.layerId)}"
                              type="range"
                              min="0"
                              max="1"
                              step="0.05"
                              value="${layerOpacityValue(layer.opacity)}"
                              aria-label="Opacity for ${escapeHtml(layer.name)}"
                            >
                          </fieldset>
                        `
                    }
                    ${
                      waypointControls.length
                        ? `
                          <fieldset class="fieldset">
                            <span class="fieldset-legend m-0 px-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Waypoints</span>
                            <div class="flex flex-wrap items-center gap-x-4 gap-y-1">
                              ${waypointControls.join("")}
                            </div>
                          </fieldset>
                        `
                        : ""
                    }
                    ${
                      pointControls.length
                        ? `
                          <fieldset class="fieldset">
                            <span class="fieldset-legend m-0 px-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Fish Evidence</span>
                            <div class="space-y-2">
                              <div class="flex flex-wrap items-center gap-x-4 gap-y-1">
                                ${pointControls.join("")}
                              </div>
                              <div class="space-y-2">
                                <div class="flex items-center justify-between gap-3">
                                  <span class="text-xs font-semibold text-base-content/70">Fish icon size</span>
                                  <span class="text-xs font-semibold text-base-content/60" data-layer-point-icon-scale-value>${pointIconScaleLabel(layer.pointIconScale)}</span>
                                </div>
                                <input
                                  class="range range-primary range-xs"
                                  data-layer-point-icon-scale="${escapeAttribute(layer.layerId)}"
                                  type="range"
                                  min="${FISHYMAP_POINT_ICON_SCALE_MIN}"
                                  max="${FISHYMAP_POINT_ICON_SCALE_MAX}"
                                  step="0.05"
                                  value="${pointIconScaleValue(layer.pointIconScale)}"
                                  aria-label="Fish icon size for ${escapeHtml(layer.name)}"
                                >
                              </div>
                            </div>
                          </fieldset>
                        `
                        : ""
                    }
                  </div>
                `
                : ""
            }
          </div>
          <button
            class="fishymap-layer-settings btn btn-sm btn-circle ${
              settingsExpanded ? "btn-soft btn-primary" : "btn-ghost"
            }"
            data-layer-settings-toggle="${escapeAttribute(layer.layerId)}"
            type="button"
            aria-label="${settingsExpanded ? "Hide" : "Show"} settings for ${escapeHtml(layer.name)}"
            aria-expanded="${settingsExpanded ? "true" : "false"}"
            title="${settingsExpanded ? "Hide" : "Show"} settings for ${escapeHtml(layer.name)}"
          >
            ${layerSettingsIcon()}
          </button>
          <button
            class="fishymap-layer-visibility btn btn-sm btn-circle ${
              visible ? "btn-soft btn-primary" : "btn-ghost"
            }"
            data-layer-visibility="${escapeAttribute(layer.layerId)}"
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
