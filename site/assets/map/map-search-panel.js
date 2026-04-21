import { buildAppliedSearchTermsView } from "../js/components/applied-search-terms.js";
import { mapCountText, mapText } from "./map-i18n.js";

function normalizeExpressionOperator(value) {
  return String(value ?? "").trim().toLowerCase() === "and" ? "and" : "or";
}

function patchBoundLabel(bound) {
  return String(bound || "").trim().toLowerCase() === "to"
    ? mapText("search.bound.before")
    : mapText("search.bound.after");
}

function nextPatchBoundLabel(bound) {
  return String(bound || "").trim().toLowerCase() === "to"
    ? mapText("search.bound.after")
    : mapText("search.bound.before");
}

function searchControlId(prefix, path) {
  const normalized = String(path || "root")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${prefix}-${normalized || "root"}`;
}

function patchDropdownValueMarkup(patchId, patch, escapeHtml) {
  const normalizedPatchId = String(patchId || "").trim();
  if (!normalizedPatchId && !patch) {
    return `<span class="truncate font-medium text-base-content/60">${escapeHtml(mapText("search.patch.choose_date"))}</span>`;
  }
  if (!patch) {
    return `<span class="fishymap-date-term-label truncate font-medium">${escapeHtml(normalizedPatchId)}</span>`;
  }
  const label = String(patch.label || normalizedPatchId).trim() || normalizedPatchId;
  if (!normalizedPatchId || label === normalizedPatchId) {
    return `<span class="fishymap-date-term-label truncate font-medium">${escapeHtml(normalizedPatchId || label)}</span>`;
  }
  return `
    <span class="fishymap-date-term-value">
      <span class="fishymap-date-term-label truncate font-medium">${escapeHtml(label)}</span>
      <span class="fishymap-date-term-id">${escapeHtml(normalizedPatchId)}</span>
    </span>
  `;
}

function patchDropdownCatalogMarkup(patches, escapeHtml) {
  return (Array.isArray(patches) ? patches : [])
    .map((patch) => {
      const patchId = String(patch?.patchId || "").trim();
      if (!patchId) {
        return "";
      }
      const label = String(patch?.label || patchId).trim();
      const searchText = [label, patchId].filter(Boolean).join(" ");
      return `
        <template
          data-role="selected-content"
          data-value="${escapeHtml(patchId)}"
          data-label="${escapeHtml(label)}"
          data-search-text="${escapeHtml(searchText)}"
        >
          ${patchDropdownValueMarkup(patchId, { patchId, label }, escapeHtml)}
        </template>
      `;
    })
    .join("");
}

function patchDropdownMarkup(term, context, path, boundLabel, patch) {
  const inputId = searchControlId("fishymap-date-term", path);
  const patchId = String(term?.patchId || "").trim();
  const selectedLabel = patch?.label || patchId || boundLabel;
  return `
    <span class="fishymap-date-term-content flex min-w-0 w-full items-center gap-2">
      <fishy-searchable-dropdown
        class="fishymap-date-term-dropdown relative max-w-full align-middle"
        input-id="${context.escapeHtml(inputId)}"
        label="${context.escapeHtml(selectedLabel)}"
        value="${context.escapeHtml(patchId)}"
        placeholder="${context.escapeHtml(mapText("search.patch.dropdown_placeholder"))}"
        custom-option-mode="iso-date"
        panel-anchor-closest=".fishy-applied-term"
      >
        <input
          id="${context.escapeHtml(inputId)}"
          type="hidden"
          value="${context.escapeHtml(patchId)}"
          data-expression-patch-select-path="${context.escapeHtml(path)}"
        >
        <button
          type="button"
          data-role="trigger"
          class="fishymap-date-term-trigger inline-flex h-7 max-w-56 items-center gap-2 rounded-full border border-base-300 px-2 text-left text-sm"
          aria-haspopup="listbox"
          aria-expanded="false"
        >
          <span data-role="selected-content" class="flex min-w-0 flex-1 items-center gap-2">
            ${patchDropdownValueMarkup(patchId, patch, context.escapeHtml)}
          </span>
          <svg class="fishy-icon size-3.5 shrink-0 opacity-60" viewBox="0 0 24 24" aria-hidden="true">
            <use width="100%" height="100%" href="/img/icons.svg?v=20260419-2#fishy-caret-down"></use>
          </svg>
        </button>
        <div
          data-role="panel"
          class="fishymap-date-term-panel absolute left-0 top-full z-50 mt-2 w-72 max-w-[min(22rem,calc(100vw-3rem))]"
          hidden
        >
          <div class="rounded-box border border-base-300 bg-base-100 p-1">
            <label class="flex items-center gap-2 px-2 py-2 text-sm">
              <svg class="fishy-icon size-4 shrink-0 opacity-60" viewBox="0 0 24 24" aria-hidden="true">
                <use width="100%" height="100%" href="/img/icons.svg?v=20260419-2#fishy-search-field"></use>
              </svg>
              <input
                data-role="search-input"
                type="search"
                class="w-full border-0 bg-transparent p-0 shadow-none outline-none"
                style="outline: none; box-shadow: none;"
                placeholder="${context.escapeHtml(mapText("search.patch.search_placeholder"))}"
                autocomplete="off"
                spellcheck="false"
              >
            </label>
            <ul data-role="results" tabindex="-1" class="menu menu-sm max-h-64 w-full gap-1 overflow-auto p-1"></ul>
          </div>
        </div>
        <div data-role="selected-content-catalog" hidden>
          ${patchDropdownCatalogMarkup(context.patchCatalog, context.escapeHtml)}
        </div>
      </fishy-searchable-dropdown>
    </span>
  `;
}

function buildAppliedSearchTermNode(term, context, path, options = {}) {
  if (!term || typeof term !== "object") {
    return null;
  }
  const negated = options.negated === true;

  if (term.kind === "fish-filter") {
    const metadata = context.fishFilterTermMetadata[term.term];
    const label = metadata?.label || term.term;
    return {
      type: "term",
      key: `fish-filter:${term.term}`,
      path,
      label,
      kindLabel: mapText("search.kind.filter"),
      grade: term.term,
      negated,
      contentMarkup: `
        <span class="inline-flex min-w-0 items-center gap-2">
          ${context.fishFilterTermIconMarkup(term.term)}
          <span class="font-medium">${context.escapeHtml(label)}</span>
        </span>
      `,
      removeLabel: mapText("search.action.remove", { label }),
      removeAttributes: {
        "data-fish-filter-term": term.term,
      },
    };
  }

  if (term.kind === "patch-bound") {
    const patchId = String(term.patchId || "").trim();
    const patch = patchId ? context.patchLookup.get(patchId) || null : null;
    const patchLabel = patch?.label || patchId;
    const boundLabel = patchBoundLabel(term.bound);
    return {
      type: "term",
      key: `patch-bound:${term.bound}:${patchId || "__pending__"}`,
      path,
      label: patchLabel ? `${boundLabel} ${patchLabel}` : boundLabel,
      kindLabel: mapText("search.kind.date"),
      grade: "patch",
      allowNegation: false,
      negated: false,
      description: "",
      contentMarkup: `
        <span class="inline-flex min-w-0 items-center gap-2">
          <svg class="fishy-icon size-4 shrink-0 text-base-content/60" viewBox="0 0 24 24" aria-hidden="true">
            <use width="100%" height="100%" href="/img/icons.svg?v=20260419-2#fishy-calendar-2-fill"></use>
          </svg>
          <button
            class="badge badge-ghost badge-xs cursor-pointer"
            type="button"
            data-expression-patch-toggle-path="${context.escapeHtml(path)}"
            aria-label="${context.escapeHtml(mapText("search.patch.change_bound", { bound: nextPatchBoundLabel(term.bound) }))}"
            title="${context.escapeHtml(mapText("search.patch.change_bound", { bound: nextPatchBoundLabel(term.bound) }))}"
          >${context.escapeHtml(boundLabel)}</button>
          ${patchDropdownMarkup(term, context, path, boundLabel, patch)}
        </span>
      `,
      removeLabel: mapText("search.action.remove", {
        label: patchLabel ? `${boundLabel} ${patchLabel}` : boundLabel,
      }),
      removeAttributes: {
        "data-patch-bound": term.bound,
        ...(patchId ? { "data-patch-id": patchId } : {}),
      },
    };
  }

  if (term.kind === "fish") {
    const fish = context.fishLookup.get(term.fishId);
    const name = fish?.name || mapText("search.fish.fallback", { id: term.fishId });
    return {
      type: "term",
      key: `fish:${term.fishId}`,
      path,
      label: name,
      kindLabel: mapText("search.kind.fish"),
      grade: context.resolveFishGrade(fish),
      negated,
      contentMarkup:
        context.fishIdentityMarkup({ ...(fish || {}), fishId: term.fishId, name }, { interactive: true })
        || `<span class="truncate max-w-36">${context.escapeHtml(name)}</span>`,
      removeLabel: mapText("search.action.remove", { label: name }),
      removeAttributes: {
        "data-fish-id": term.fishId,
      },
    };
  }

  if (term.kind === "zone") {
    const zone = context.zoneLookup.get(term.zoneRgb);
    const name = zone?.name || mapText("search.zone.fallback", { zone: context.formatZone(term.zoneRgb) });
    return {
      type: "term",
      key: `zone:${term.zoneRgb}`,
      path,
      label: name,
      kindLabel: mapText("search.kind.zone"),
      grade: "zone",
      negated,
      description: "",
      contentMarkup:
        context.zoneIdentityMarkup(
          {
            zoneRgb: term.zoneRgb,
            name,
            r: zone?.r,
            g: zone?.g,
            b: zone?.b,
          },
          { interactive: true },
        ) || `<span class="truncate max-w-40">${context.escapeHtml(name)}</span>`,
      removeLabel: mapText("search.action.remove", { label: name }),
      removeAttributes: {
        "data-zone-rgb": term.zoneRgb,
      },
    };
  }

  if (term.kind === "semantic") {
    const semanticTerm =
      context.semanticLookup.get(
        `${String(term.layerId || "").trim()}:${Number.parseInt(term.fieldId, 10)}`,
      ) || null;
    const name = semanticTerm?.label || mapText("search.semantic.field", { id: term.fieldId });
    return {
      type: "term",
      key: `semantic:${term.layerId}:${term.fieldId}`,
      path,
      label: name,
      kindLabel: semanticTerm?.layerName || mapText("search.kind.map"),
      grade: "semantic",
      negated,
      description: semanticTerm?.description || "",
      contentMarkup:
        context.semanticIdentityMarkup(name, { interactive: true })
        || `<span class="truncate max-w-40">${context.escapeHtml(name)}</span>`,
      removeLabel: mapText("search.action.remove", { label: name }),
      removeAttributes: {
        "data-semantic-layer-id": term.layerId,
        "data-semantic-field-id": term.fieldId,
      },
    };
  }

  return null;
}

function buildAppliedSearchExpressionNode(expression, context, path = "root") {
  if (!expression || typeof expression !== "object") {
    return null;
  }

  if (String(expression.type || "").trim().toLowerCase() === "term") {
    return buildAppliedSearchTermNode(expression.term, context, path, {
      negated: expression.negated === true,
    });
  }

  const children = (Array.isArray(expression.children) ? expression.children : [])
    .map((child, index) => buildAppliedSearchExpressionNode(child, context, `${path}.${index}`))
    .filter(Boolean);

  return {
    type: "group",
    key: path,
    path,
    operator: normalizeExpressionOperator(expression.operator),
    negated: expression.negated === true,
    children,
  };
}

export function renderSearchSelection(elements, stateBundle, fishLookup, options = {}) {
  const resolveSearchExpression =
    typeof options.resolveSearchExpression === "function"
      ? options.resolveSearchExpression
      : (bundle) => bundle?.inputState?.search?.expression ?? null;
  const buildSemanticTermLookup =
    typeof options.buildSemanticTermLookup === "function"
      ? options.buildSemanticTermLookup
      : () => new Map();
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
  const resolveFishGrade =
    typeof options.resolveFishGrade === "function" ? options.resolveFishGrade : () => "unknown";
  const formatZone = typeof options.formatZone === "function" ? options.formatZone : (value) => String(value || "");
  const fishFilterTermMetadata = options.fishFilterTermMetadata || {};
  const activeDragPath = String(options.activeDragPath || "").trim();

  const zoneLookup = new Map((elements.zoneCatalog || []).map((zone) => [zone.zoneRgb, zone]));
  const patchLookup = new Map(
    (stateBundle?.state?.catalog?.patches || []).map((patch) => [String(patch.patchId || "").trim(), patch]),
  );
  const patchCatalog = stateBundle?.state?.catalog?.patches || [];
  const semanticLookup = buildSemanticTermLookup(stateBundle);
  const expression = resolveSearchExpression(stateBundle);
  const expressionView = buildAppliedSearchExpressionNode(
    expression,
    {
      escapeHtml,
      fishFilterTermIconMarkup,
      fishFilterTermMetadata,
      fishIdentityMarkup,
      fishLookup,
      formatZone,
      patchCatalog,
      patchLookup,
      resolveFishGrade,
      semanticIdentityMarkup,
      semanticLookup,
      zoneIdentityMarkup,
      zoneLookup,
    },
    "root",
  );

  const { hasContent: hasAnySelection, html, renderKey } = buildAppliedSearchTermsView(expressionView, {
    escapeHtml,
    activeDragPath,
    removeButtonClass: "fishymap-selection-remove",
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

  elements.searchSelection.innerHTML = html;
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
        : match.kind === "patch-bound"
          ? ["patch-bound", match.bound, match.patchId, match.label || "", match.description || ""]
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
      mapCountText("search.results.match_count", matches.length),
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
              aria-label="${escapeHtml(mapText("search.action.add", { label: match.label || match.term }))}"
              title="${escapeHtml(mapText("search.action.add", { label: match.label || match.term }))}"
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
      if (match.kind === "patch-bound") {
        const boundLabel = patchBoundLabel(match.bound);
        const patchId = String(match.patchId || "").trim();
        const promptOnly = !patchId;
        const patchLabel = match.label || patchId || boundLabel;
        return `
          <li>
            <div
              class="flex cursor-pointer items-start gap-3 rounded-box px-3 py-2 text-sm"
              data-patch-bound="${escapeHtml(match.bound)}"
              role="button"
              tabindex="0"
              ${patchId ? `data-patch-id="${escapeHtml(patchId)}"` : ""}
              aria-label="${escapeHtml(mapText("search.action.add", {
                label: promptOnly ? boundLabel : `${boundLabel} ${patchLabel}`,
              }))}"
              title="${escapeHtml(mapText("search.action.add", {
                label: promptOnly ? boundLabel : `${boundLabel} ${patchLabel}`,
              }))}"
            >
              <span class="min-w-0 flex-1 text-left">
                <span class="flex items-center gap-2">
                  <svg class="fishy-icon size-4 shrink-0 text-base-content/60" viewBox="0 0 24 24" aria-hidden="true">
                    <use width="100%" height="100%" href="/img/icons.svg?v=20260419-2#fishy-calendar-2-fill"></use>
                  </svg>
                  <span class="font-semibold">${escapeHtml(patchLabel)}</span>
                </span>
                <span class="mt-1 block truncate text-xs text-base-content/60">
                  ${
                    promptOnly
                      ? escapeHtml(match.description || mapText("search.patch.pick_patch_or_date"))
                      : `<code>${escapeHtml(patchId)}</code>`
                  }
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
            aria-label="${escapeHtml(mapText("search.action.add", { label: match.name }))}"
            title="${escapeHtml(mapText("search.action.add", { label: match.name }))}"
          >
            <span class="min-w-0 flex-1 text-left">
              <span class="flex items-center gap-2">
                ${zoneMarkup}
                <span class="badge badge-outline badge-xs">${escapeHtml(mapText("search.zone.badge"))}</span>
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
        const semanticLabel = match.label || mapText("search.semantic.field", { id: match.fieldId });
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
              aria-label="${escapeHtml(mapText("search.action.add", { label: semanticLabel }))}"
              title="${escapeHtml(mapText("search.action.add", { label: semanticLabel }))}"
            >
              <span class="min-w-0 flex-1 text-left">
                <span class="block">${semanticMarkup}</span>
                <span class="mt-1 block truncate text-xs text-base-content/60">
                  ${escapeHtml(match.description || mapText("search.semantic.field", { id: match.fieldId }))}
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
            aria-label="${escapeHtml(mapText("search.action.add", { label: match.name }))}"
            title="${escapeHtml(mapText("search.action.add", { label: match.name }))}"
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
