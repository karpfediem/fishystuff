import FishyMapBridge, { FISHYMAP_EVENTS } from "./map-host.js";

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

function buildFishLookup(catalogFish) {
  const map = new Map();
  for (const fish of catalogFish || []) {
    map.set(fish.fishId, fish);
  }
  return map;
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

function findFishMatches(catalogFish, searchText, prizeOnly) {
  const query = String(searchText || "").trim().toLowerCase();
  const terms = query ? query.split(/\s+/g).filter(Boolean) : [];
  if (!terms.length && !prizeOnly) {
    return [];
  }
  const filtered = [];
  for (const fish of catalogFish || []) {
    if (prizeOnly && !fish.isPrize) {
      continue;
    }
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
    select.innerHTML = `<option value="">${emptyLabel}</option>`;
    select.value = "";
    return;
  }

  const options = orderedPatches.map((patch) => {
    const name = patch.patchName || patch.patchId;
    const date = formatPatchDate(patch.startTsUtc);
    const label = date ? `${name} (${date})` : name;
    return `<option value="${patch.patchId.replace(/"/g, "&quot;")}">${label}</option>`;
  });

  select.innerHTML = options.join("");
  select.value = selectedPatchId || orderedPatches[0].patchId;
}

function renderLayerToggles(container, layers, visibleLayerIds) {
  const visible = new Set(visibleLayerIds || []);
  if (!layers || !layers.length) {
    container.innerHTML =
      '<p class="text-xs text-base-content/60">Layer registry is loading…</p>';
    return;
  }
  container.innerHTML = layers
    .map((layer) => {
      const checked = visible.has(layer.layerId);
      const opacity = Math.round((layer.opacity ?? 1) * 100);
      return `
        <label class="label cursor-pointer justify-start gap-3 rounded-box px-0 py-1.5">
          <input
            class="fishymap-layer-toggle checkbox checkbox-sm checkbox-primary"
            data-layer-id="${layer.layerId.replace(/"/g, "&quot;")}"
            type="checkbox"
            ${checked ? "checked" : ""}
          />
          <span class="label-text flex-1">${layer.name}</span>
          <span class="text-[11px] uppercase tracking-[0.18em] text-base-content/45">${opacity}%</span>
        </label>
      `;
    })
    .join("");
}

function renderSearchResults(elements, matches, stateBundle) {
  const query = String(stateBundle.inputState?.filters?.searchText || "").trim();
  const prizeOnly = Boolean(stateBundle.inputState?.filters?.prizeOnly);
  const activeMatches = matches.slice(0, 12);
  elements.searchCount.textContent = `${matches.length} fish`;
  if (!matches.length) {
    elements.searchResults.innerHTML = `<div class="px-2 py-3 text-xs text-base-content/60">${
      query || prizeOnly ? "No fish match the current filter." : "Start typing to filter fish."
    }</div>`;
    return;
  }
  elements.searchResults.innerHTML = activeMatches
    .map(
      (fish) => `
        <button
          class="btn btn-ghost btn-sm w-full justify-start rounded-xl px-3"
          data-fish-id="${fish.fishId}"
          type="button"
        >
          <span class="truncate">${fish.name}</span>
          <span class="ml-auto text-[11px] text-base-content/45">#${fish.fishId}</span>
        </button>
      `,
    )
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
  container.innerHTML = lines.map((line) => `<p>${line}</p>`).join("");
}

function renderPanel(elements, stateBundle) {
  const state = stateBundle.state || {};
  const inputState = stateBundle.inputState || {};
  const catalogFish = state.catalog?.fish || [];
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
  const visibleLayers =
    inputState.filters?.layerIdsVisible || state.filters?.layerIdsVisible || [];
  const searchText = inputState.filters?.searchText || "";
  const prizeOnly = Boolean(inputState.filters?.prizeOnly);
  const fishLookup = buildFishLookup(catalogFish);

  applyThemeToShell(elements.shell);

  elements.readyPill.textContent = state.ready ? "Ready" : "Loading";
  elements.readyPill.className = `badge badge-sm ${
    state.ready ? "badge-success" : "badge-outline"
  }`;
  elements.viewReadout.textContent = state.view?.viewMode === "3d" ? "3D" : "2D";
  elements.viewMode.value = state.view?.viewMode === "3d" ? "3d" : "2d";

  if (elements.search.value !== searchText) {
    elements.search.value = searchText;
  }
  elements.prizeOnly.checked = prizeOnly;

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
  renderLayerToggles(elements.layers, state.catalog?.layers || [], visibleLayers);

  const matches = findFishMatches(catalogFish, searchText, prizeOnly);
  renderSearchResults(elements, matches, stateBundle);

  elements.legend.open = Boolean(inputState.ui?.legendOpen);
  elements.diagnostics.open = Boolean(inputState.ui?.diagnosticsOpen);

  const panelOpen = inputState.ui?.leftPanelOpen !== false;
  elements.panel.hidden = !panelOpen;
  elements.panelOpen.hidden = panelOpen;

  const selectedFish =
    fishLookup.get(state.selection?.fishId) ||
    fishLookup.get(state.filters?.fishIds?.[state.filters?.fishIds?.length - 1]);
  const zoneName =
    state.selection?.zoneName ||
    (state.selection?.zoneRgb != null ? `Zone ${formatZone(state.selection.zoneRgb)}` : null);
  const fishName = selectedFish?.name || null;
  if (zoneName && fishName) {
    elements.selectionSummary.textContent = `${zoneName} with ${fishName}.`;
  } else if (zoneName) {
    elements.selectionSummary.textContent = zoneName;
  } else if (fishName) {
    elements.selectionSummary.textContent = `Fish filter focused on ${fishName}.`;
  } else {
    elements.selectionSummary.textContent = "No zone selected.";
  }

  if (state.hover?.zoneRgb != null) {
    const worldX = Number.isFinite(state.hover.worldX)
      ? Math.round(state.hover.worldX)
      : "n/a";
    const worldZ = Number.isFinite(state.hover.worldZ)
      ? Math.round(state.hover.worldZ)
      : "n/a";
    elements.hoverSummary.textContent = `${
      state.hover.zoneName || formatZone(state.hover.zoneRgb)
    } at world ${worldX}, ${worldZ}.`;
  } else {
    elements.hoverSummary.textContent = "Hover a zone to inspect it.";
  }

  renderStatusLines(elements.statusLines, state.statuses || {});
  elements.diagnosticJson.textContent = JSON.stringify(
    state.lastDiagnostic || state.statuses || {},
    null,
    2,
  );
}

function collectVisibleLayerIds(layersRoot) {
  return Array.from(
    layersRoot.querySelectorAll(".fishymap-layer-toggle:checked"),
    (input) => input.getAttribute("data-layer-id"),
  ).filter(Boolean);
}

function bindUi(shell, elements) {
  let isRendering = false;

  function renderCurrentState(stateBundle = requestBridgeState(shell)) {
    isRendering = true;
    try {
      renderPanel(elements, stateBundle);
    } finally {
      isRendering = false;
    }
  }

  function pushSearchPatch() {
    const current = requestBridgeState(shell);
    const catalogFish = current.state.catalog?.fish || [];
    const searchText = elements.search.value;
    const prizeOnly = elements.prizeOnly.checked;
    const matches = findFishMatches(catalogFish, searchText, prizeOnly);
    const fishIds = matches.slice(0, 128).map((fish) => fish.fishId);
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText,
        prizeOnly,
        fishIds,
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
      elements.prizeOnly.checked,
    );
    const top = matches[0];
    if (!top) {
      return;
    }
    event.preventDefault();
    elements.search.value = top.name;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText: top.name,
        prizeOnly: elements.prizeOnly.checked,
        fishIds: [top.fishId],
      },
      commands: {
        focusFishId: top.fishId,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.prizeOnly.addEventListener("change", () => {
    if (isRendering) {
      return;
    }
    pushSearchPatch();
  });

  elements.searchResults.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-fish-id]");
    if (!button) {
      return;
    }
    const fishId = Number.parseInt(button.getAttribute("data-fish-id"), 10);
    const label = button.querySelector("span")?.textContent?.trim() || String(fishId);
    elements.search.value = label;
    dispatchMapState(shell, {
      version: 1,
      filters: {
        searchText: label,
        prizeOnly: elements.prizeOnly.checked,
        fishIds: [fishId],
      },
      commands: {
        focusFishId: fishId,
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

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

  elements.viewMode.addEventListener("change", () => {
    dispatchMapCommand(shell, {
      setViewMode: elements.viewMode.value === "3d" ? "3d" : "2d",
    });
  });

  elements.layers.addEventListener("change", (event) => {
    if (isRendering || !event.target.classList.contains("fishymap-layer-toggle")) {
      return;
    }
    dispatchMapState(shell, {
      version: 1,
      filters: {
        layerIdsVisible: collectVisibleLayerIds(elements.layers),
      },
    });
    renderCurrentState(requestBridgeState(shell));
  });

  elements.resetView.addEventListener("click", () => {
    dispatchMapCommand(shell, { resetView: true });
  });

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

  for (const type of [
    FISHYMAP_EVENTS.ready,
    FISHYMAP_EVENTS.viewChanged,
    FISHYMAP_EVENTS.selectionChanged,
    FISHYMAP_EVENTS.hoverChanged,
    FISHYMAP_EVENTS.diagnostic,
  ]) {
    shell.addEventListener(type, (event) => {
      renderCurrentState({
        state: event.detail?.state || FishyMapBridge.getCurrentState(),
        inputState:
          event.detail?.inputState ||
          (typeof FishyMapBridge.getCurrentInputState === "function"
            ? FishyMapBridge.getCurrentInputState()
            : {}),
      });
    });
  }

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
    panel: document.getElementById("fishymap-panel"),
    panelOpen: document.getElementById("fishymap-panel-open"),
    panelClose: document.getElementById("fishymap-panel-close"),
    readyPill: document.getElementById("fishymap-ready-pill"),
    search: document.getElementById("fishymap-search"),
    searchResults: document.getElementById("fishymap-search-results"),
    searchCount: document.getElementById("fishymap-search-count"),
    prizeOnly: document.getElementById("fishymap-prize-only"),
    patchFrom: document.getElementById("fishymap-patch-from"),
    patchTo: document.getElementById("fishymap-patch-to"),
    viewMode: document.getElementById("fishymap-view-mode"),
    layers: document.getElementById("fishymap-layers"),
    resetView: document.getElementById("fishymap-reset-view"),
    legend: document.getElementById("fishymap-legend"),
    diagnostics: document.getElementById("fishymap-diagnostics"),
    statusLines: document.getElementById("fishymap-status-lines"),
    diagnosticJson: document.getElementById("fishymap-diagnostic-json"),
    selectionSummary: document.getElementById("fishymap-selection-summary"),
    hoverSummary: document.getElementById("fishymap-hover-summary"),
    viewReadout: document.getElementById("fishymap-view-readout"),
    errorOverlay: document.getElementById("fishymap-error-overlay"),
    errorMessage: document.getElementById("fishymap-error-message"),
    canvas,
  };

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
