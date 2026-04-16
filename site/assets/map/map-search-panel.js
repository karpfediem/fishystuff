import { buildAppliedSearchTermsView } from "../js/components/applied-search-terms.js";
import { buildSearchExpressionFromSelectedTerms } from "./map-search-contract.js";

function normalizeExpressionOperator(value) {
  return String(value ?? "").trim().toLowerCase() === "and" ? "and" : "or";
}

function buildFallbackSelectedTerms(stateBundle, resolvers) {
  const selectedFishIds = resolvers.resolveSelectedFishIds(stateBundle);
  const selectedFishFilterTerms = resolvers.resolveSelectedFishFilterTerms(stateBundle);
  const selectedSemanticFieldIdsByLayer = resolvers.resolveSelectedSemanticFieldIdsByLayer(stateBundle);
  const selectedZoneRgbs = resolvers.resolveSelectedZoneRgbs(stateBundle);
  const selectedTerms = [];

  for (const fishFilterTerm of selectedFishFilterTerms) {
    selectedTerms.push({ kind: "fish-filter", term: fishFilterTerm });
  }
  for (const fishId of selectedFishIds) {
    selectedTerms.push({ kind: "fish", fishId });
  }
  for (const zoneRgb of selectedZoneRgbs) {
    selectedTerms.push({ kind: "zone", zoneRgb });
  }
  for (const [layerId, fieldIds] of Object.entries(selectedSemanticFieldIdsByLayer)) {
    if (layerId === "zone_mask") {
      continue;
    }
    for (const fieldId of Array.isArray(fieldIds) ? fieldIds : []) {
      selectedTerms.push({ kind: "semantic", layerId, fieldId });
    }
  }

  return selectedTerms;
}

function buildAppliedSearchTermNode(term, context, path) {
  if (!term || typeof term !== "object") {
    return null;
  }

  if (term.kind === "fish-filter") {
    const metadata = context.fishFilterTermMetadata[term.term];
    const label = metadata?.label || term.term;
    return {
      type: "term",
      key: `fish-filter:${term.term}`,
      path,
      label,
      kindLabel: "Filter",
      grade: term.term,
      contentMarkup: `
        <span class="inline-flex min-w-0 items-center gap-2">
          ${context.fishFilterTermIconMarkup(term.term)}
          <span class="font-medium">${context.escapeHtml(label)}</span>
        </span>
      `,
      removeLabel: `Remove ${label}`,
      removeAttributes: {
        "data-fish-filter-term": term.term,
      },
    };
  }

  if (term.kind === "fish") {
    const fish = context.fishLookup.get(term.fishId);
    const name = fish?.name || `Fish ${term.fishId}`;
    return {
      type: "term",
      key: `fish:${term.fishId}`,
      path,
      label: name,
      kindLabel: "Fish",
      grade: context.resolveFishGrade(fish),
      contentMarkup:
        context.fishIdentityMarkup({ ...(fish || {}), fishId: term.fishId, name }, { interactive: true })
        || `<span class="truncate max-w-36">${context.escapeHtml(name)}</span>`,
      removeLabel: `Remove ${name}`,
      removeAttributes: {
        "data-fish-id": term.fishId,
      },
    };
  }

  if (term.kind === "zone") {
    const zone = context.zoneLookup.get(term.zoneRgb);
    const name = zone?.name || `Zone ${context.formatZone(term.zoneRgb)}`;
    return {
      type: "term",
      key: `zone:${term.zoneRgb}`,
      path,
      label: name,
      kindLabel: "Zone",
      grade: "zone",
      description: context.formatZone(term.zoneRgb),
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
      removeLabel: `Remove ${name}`,
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
    const name = semanticTerm?.label || `Field ${term.fieldId}`;
    return {
      type: "term",
      key: `semantic:${term.layerId}:${term.fieldId}`,
      path,
      label: name,
      kindLabel: semanticTerm?.layerName || "Map",
      grade: "semantic",
      description: semanticTerm?.description || "",
      contentMarkup:
        context.semanticIdentityMarkup(name, { interactive: true })
        || `<span class="truncate max-w-40">${context.escapeHtml(name)}</span>`,
      removeLabel: `Remove ${name}`,
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
    return buildAppliedSearchTermNode(expression.term, context, path);
  }

  const children = (Array.isArray(expression.children) ? expression.children : [])
    .map((child, index) => buildAppliedSearchExpressionNode(child, context, `${path}.${index}`))
    .filter(Boolean);

  return {
    type: "group",
    key: path,
    path,
    operator: normalizeExpressionOperator(expression.operator),
    children,
  };
}

export function renderSearchSelection(elements, stateBundle, fishLookup, options = {}) {
  const resolveSearchExpression =
    typeof options.resolveSearchExpression === "function"
      ? options.resolveSearchExpression
      : (bundle) => bundle?.inputState?.search?.expression ?? null;
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

  const zoneLookup = new Map((elements.zoneCatalog || []).map((zone) => [zone.zoneRgb, zone]));
  const semanticLookup = buildSemanticTermLookup(stateBundle);
  const fallbackSelectedTerms = Array.isArray(stateBundle?.inputState?.search?.selectedTerms)
    ? stateBundle.inputState.search.selectedTerms
    : buildFallbackSelectedTerms(stateBundle, {
        resolveSelectedFishIds,
        resolveSelectedFishFilterTerms,
        resolveSelectedSemanticFieldIdsByLayer,
        resolveSelectedZoneRgbs,
      });
  const expression =
    resolveSearchExpression(stateBundle) || buildSearchExpressionFromSelectedTerms(fallbackSelectedTerms);
  const expressionView = buildAppliedSearchExpressionNode(
    expression,
    {
      escapeHtml,
      fishFilterTermIconMarkup,
      fishFilterTermMetadata,
      fishIdentityMarkup,
      fishLookup,
      formatZone,
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
