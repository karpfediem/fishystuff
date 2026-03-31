export function renderSearchSelection(elements, stateBundle, fishLookup, options = {}) {
  const resolveSelectedFishIds =
    typeof options.resolveSelectedFishIds === "function"
      ? options.resolveSelectedFishIds
      : () => [];
  const resolveSelectedFishFilterTerms =
    typeof options.resolveSelectedFishFilterTerms === "function"
      ? options.resolveSelectedFishFilterTerms
      : () => [];
  const resolveSelectedSemanticFieldIdsByLayer =
    typeof options.resolveSelectedSemanticFieldIdsByLayer === "function"
      ? options.resolveSelectedSemanticFieldIdsByLayer
      : () => ({});
  const resolveSelectedZoneRgbs =
    typeof options.resolveSelectedZoneRgbs === "function"
      ? options.resolveSelectedZoneRgbs
      : () => [];
  const buildSemanticTermLookup =
    typeof options.buildSemanticTermLookup === "function"
      ? options.buildSemanticTermLookup
      : () => new Map();
  const setBooleanProperty =
    typeof options.setBooleanProperty === "function" ? options.setBooleanProperty : () => {};
  const setTextContent =
    typeof options.setTextContent === "function" ? options.setTextContent : () => {};
  const escapeHtml = typeof options.escapeHtml === "function" ? options.escapeHtml : (value) => String(value || "");
  const fishFilterTermIconMarkup =
    typeof options.fishFilterTermIconMarkup === "function"
      ? options.fishFilterTermIconMarkup
      : () => "";
  const fishIdentityMarkup =
    typeof options.fishIdentityMarkup === "function" ? options.fishIdentityMarkup : () => "";
  const zoneIdentityMarkup =
    typeof options.zoneIdentityMarkup === "function" ? options.zoneIdentityMarkup : () => "";
  const semanticIdentityMarkup =
    typeof options.semanticIdentityMarkup === "function"
      ? options.semanticIdentityMarkup
      : () => "";
  const formatZone = typeof options.formatZone === "function" ? options.formatZone : (value) => String(value || "");
  const fishFilterTermMetadata = options.fishFilterTermMetadata || {};

  const selectedFishIds = resolveSelectedFishIds(stateBundle);
  const selectedFishFilterTerms = resolveSelectedFishFilterTerms(stateBundle);
  const selectedSemanticFieldIdsByLayer = resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const selectedZoneRgbs = resolveSelectedZoneRgbs(stateBundle);
  const hasSelection =
    selectedFishIds.length > 0 ||
    selectedFishFilterTerms.length > 0 ||
    selectedZoneRgbs.length > 0;
  const zoneLookup = new Map((elements.zoneCatalog || []).map((zone) => [zone.zoneRgb, zone]));
  const semanticLookup = buildSemanticTermLookup(stateBundle);
  const selectedSemanticEntries = Object.entries(selectedSemanticFieldIdsByLayer)
    .filter(([layerId]) => layerId !== "zone_mask")
    .flatMap(([layerId, fieldIds]) =>
      fieldIds.map((fieldId) => ({
        layerId,
        fieldId,
        term: semanticLookup.get(`${String(layerId || "").trim()}:${Number.parseInt(fieldId, 10)}`) || null,
      })),
    );
  const hasSemanticSelection = selectedSemanticEntries.length > 0;
  const hasAnySelection = hasSelection || hasSemanticSelection;
  const renderKey = JSON.stringify({
    selectedFishFilterTerms,
    selectedFishIds,
    selectedZoneRgbs,
    selectedSemantic: selectedSemanticEntries.map(({ layerId, fieldId, term }) => [
      layerId,
      fieldId,
      term?.label || "",
      term?.description || "",
      term?.layerName || "",
    ]),
    selectedFish: selectedFishIds.map((fishId) => {
      const fish = fishLookup.get(fishId);
      return [
        fishId,
        fish?.name || "",
        fish?.itemId || null,
        fish?.encyclopediaId || null,
        fish?.grade || "",
        fish?.isPrize === true ? 1 : 0,
      ];
    }),
    selectedZones: selectedZoneRgbs.map((zoneRgb) => {
      const zone = zoneLookup.get(zoneRgb);
      return [zoneRgb, zone?.name || "", zone?.rgbKey || ""];
    }),
  });
  if (elements.searchSelection.dataset.renderKey === renderKey) {
    elements.searchSelection.hidden = !hasAnySelection;
    if (elements.searchSelectionShell) {
      elements.searchSelectionShell.hidden = !hasAnySelection;
    }
    if (elements.searchWindow) {
      elements.searchWindow.dataset.hasSelection = hasAnySelection ? "true" : "false";
    }
    return;
  }
  elements.searchSelection.dataset.renderKey = renderKey;

  if (!hasAnySelection) {
    elements.searchSelection.innerHTML = "";
    elements.searchSelection.hidden = true;
    if (elements.searchSelectionShell) {
      elements.searchSelectionShell.hidden = true;
    }
    if (elements.searchWindow) {
      elements.searchWindow.dataset.hasSelection = "false";
    }
    return;
  }

  elements.searchSelection.hidden = false;
  if (elements.searchSelectionShell) {
    elements.searchSelectionShell.hidden = false;
  }
  if (elements.searchWindow) {
    elements.searchWindow.dataset.hasSelection = "true";
  }

  elements.searchSelection.innerHTML = selectedFishFilterTerms
    .map((fishFilterTerm) => {
      const metadata = fishFilterTermMetadata[fishFilterTerm];
      const label = metadata?.label || fishFilterTerm;
      return `
        <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
          <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">
            ${fishFilterTermIconMarkup(fishFilterTerm)}
            <span class="font-medium">${escapeHtml(label)}</span>
          </span>
          <button
            class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
            data-fish-filter-term="${escapeHtml(fishFilterTerm)}"
            type="button"
            aria-label="Remove ${escapeHtml(label)}"
          >
            ×
          </button>
        </div>
      `;
    })
    .concat(
      selectedFishIds.map((fishId) => {
        const fish = fishLookup.get(fishId);
        const name = fish?.name || `Fish ${fishId}`;
        const fishMarkup =
          fishIdentityMarkup({ ...fish, fishId, name }, { interactive: true }) ||
          `<span class="truncate max-w-36">${escapeHtml(name)}</span>`;
        return `
        <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
          <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">${fishMarkup}</span>
          <button
            class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
            data-fish-id="${fishId}"
            type="button"
            aria-label="Remove ${escapeHtml(name)}"
          >
            ×
          </button>
        </div>
      `;
      }),
    )
    .concat(
      selectedZoneRgbs.map((zoneRgb) => {
        const zone = zoneLookup.get(zoneRgb);
        const name = zone?.name || `Zone ${formatZone(zoneRgb)}`;
        const zoneMarkup =
          zoneIdentityMarkup(
            {
              zoneRgb,
              name,
              r: zone?.r,
              g: zone?.g,
              b: zone?.b,
            },
            { interactive: true },
          ) || `<span class="truncate max-w-40">${escapeHtml(name)}</span>`;
        return `
          <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
            <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">${zoneMarkup}</span>
            <button
              class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
              data-zone-rgb="${zoneRgb}"
              type="button"
              aria-label="Remove ${escapeHtml(name)}"
            >
              ×
            </button>
          </div>
        `;
      }),
    )
    .concat(
      selectedSemanticEntries.map(({ layerId, fieldId, term }) => {
        const name = term?.label || `Field ${fieldId}`;
        const description = term?.description || "";
        const semanticMarkup =
          semanticIdentityMarkup(name, { interactive: true }) ||
          `<span class="truncate max-w-40">${escapeHtml(name)}</span>`;
        return `
          <div class="join items-center rounded-full border border-base-300 bg-base-100 p-1 text-base-content">
            <span class="inline-flex min-w-0 items-center gap-2 px-2 text-sm">
              <span class="min-w-0">${semanticMarkup}</span>
              ${
                description
                  ? `<span class="truncate max-w-40 text-xs text-base-content/55">${escapeHtml(description)}</span>`
                  : ""
              }
            </span>
            <button
              class="fishymap-selection-remove btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70"
              data-semantic-layer-id="${escapeHtml(layerId)}"
              data-semantic-field-id="${fieldId}"
              type="button"
              aria-label="Remove ${escapeHtml(name)}"
            >
              ×
            </button>
          </div>
        `;
      }),
    )
    .join("");
}

export function renderSearchResults(elements, matches, stateBundle, options = {}) {
  const setBooleanProperty =
    typeof options.setBooleanProperty === "function" ? options.setBooleanProperty : () => {};
  const setTextContent =
    typeof options.setTextContent === "function" ? options.setTextContent : () => {};
  const escapeHtml = typeof options.escapeHtml === "function" ? options.escapeHtml : (value) => String(value || "");
  const fishFilterTermIconMarkup =
    typeof options.fishFilterTermIconMarkup === "function"
      ? options.fishFilterTermIconMarkup
      : () => "";
  const fishIdentityMarkup =
    typeof options.fishIdentityMarkup === "function" ? options.fishIdentityMarkup : () => "";
  const zoneIdentityMarkup =
    typeof options.zoneIdentityMarkup === "function" ? options.zoneIdentityMarkup : () => "";
  const semanticIdentityMarkup =
    typeof options.semanticIdentityMarkup === "function"
      ? options.semanticIdentityMarkup
      : () => "";
  const formatZone = typeof options.formatZone === "function" ? options.formatZone : (value) => String(value || "");

  const query = String(stateBundle.inputState?.filters?.searchText || "").trim();
  const showResults = matches.length > 0;
  const activeMatches = matches.slice(0, 12);
  const renderKey = JSON.stringify({
    query,
    results: activeMatches.map((match) =>
      match.kind === "fish-filter"
        ? ["fish-filter", match.term, match.label || "", match.description || ""]
        : match.kind === "zone"
          ? ["zone", match.zoneRgb, match.name, match.rgbKey]
          : match.kind === "semantic"
            ? [
                "semantic",
                match.layerId,
                match.fieldId,
                match.label || "",
                match.description || "",
                match.layerName || "",
              ]
            : [
                "fish",
                match.fishId,
                match.itemId ?? null,
                match.encyclopediaId ?? null,
                match.grade || "",
                match.isPrize === true ? 1 : 0,
              ],
    ),
    total: matches.length,
  });
  if (elements.searchResultsShell) {
    setBooleanProperty(elements.searchResultsShell, "hidden", !showResults);
  }
  if (elements.searchCount) {
    setTextContent(
      elements.searchCount,
      `${matches.length} ${matches.length === 1 ? "match" : "matches"}`,
    );
    setBooleanProperty(elements.searchCount, "hidden", !query);
  }
  if (elements.searchResults.dataset.renderKey === renderKey) {
    return;
  }
  elements.searchResults.dataset.renderKey = renderKey;
  if (!showResults) {
    elements.searchResults.innerHTML = "";
    return;
  }
  elements.searchResults.innerHTML = activeMatches
    .map((match) => {
      if (match.kind === "fish-filter") {
        return `
          <li>
            <div
              class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
              data-fish-filter-term="${escapeHtml(match.term)}"
              role="button"
              tabindex="0"
              aria-label="Add ${escapeHtml(match.label || match.term)}"
              title="Add ${escapeHtml(match.label || match.term)}"
            >
              <span class="min-w-0 flex-1 text-left">
                <span class="flex items-center gap-2">
                  ${fishFilterTermIconMarkup(match.term)}
                  <span class="font-semibold">${escapeHtml(match.label || match.term)}</span>
                </span>
                <span class="mt-1 block truncate text-xs text-base-content/60">
                  ${escapeHtml(match.description || "")}
                </span>
              </span>
            </div>
          </li>
        `;
      }
      if (match.kind === "zone") {
        const zoneMarkup =
          zoneIdentityMarkup(match, { interactive: true }) ||
          `<span class="truncate">${escapeHtml(match.name)}</span>`;
        return `
        <li>
          <div
            class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
            data-zone-rgb="${match.zoneRgb}"
            role="button"
            tabindex="0"
            aria-label="Add ${escapeHtml(match.name)}"
            title="Add ${escapeHtml(match.name)}"
          >
            <span class="min-w-0 flex-1 text-left">
              <span class="flex items-center gap-2">
                ${zoneMarkup}
                <span class="badge badge-outline badge-xs">Zone</span>
              </span>
              <span class="block truncate text-xs text-base-content/60">
                <code>${escapeHtml(match.rgbKey)}</code>
                <span class="ml-2">${escapeHtml(formatZone(match.zoneRgb))}</span>
              </span>
            </span>
          </div>
        </li>
      `;
      }
      if (match.kind === "semantic") {
        const semanticLabel = match.label || `Field ${match.fieldId}`;
        const semanticMarkup =
          semanticIdentityMarkup(semanticLabel, { interactive: true }) ||
          `<span class="truncate">${escapeHtml(semanticLabel)}</span>`;
        return `
          <li>
            <div
              class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
              data-semantic-layer-id="${escapeHtml(match.layerId)}"
              data-semantic-field-id="${match.fieldId}"
              data-semantic-label="${escapeHtml(semanticLabel)}"
              role="button"
              tabindex="0"
              aria-label="Add ${escapeHtml(semanticLabel)}"
              title="Add ${escapeHtml(semanticLabel)}"
            >
              <span class="min-w-0 flex-1 text-left">
                <span class="block">${semanticMarkup}</span>
                <span class="mt-1 block truncate text-xs text-base-content/60">
                  ${escapeHtml(match.description || `Field ${match.fieldId}`)}
                </span>
              </span>
            </div>
          </li>
        `;
      }
      return `
        <li>
          <div
            class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
            data-fish-id="${match.fishId}"
            role="button"
            tabindex="0"
            aria-label="Add ${escapeHtml(match.name)}"
            title="Add ${escapeHtml(match.name)}"
          >
            <span class="min-w-0 flex-1 text-left">
              ${fishIdentityMarkup(match, { interactive: true })}
            </span>
          </div>
        </li>
      `;
    })
    .join("");
}
