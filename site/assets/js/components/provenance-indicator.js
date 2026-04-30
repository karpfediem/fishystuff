function trimString(value) {
    const normalized = String(value ?? "").trim();
    return normalized || "";
}

const PROVENANCE_INACTIVE_COLOR =
    "color-mix(in oklab, var(--color-neutral) 28%, var(--color-base-300) 72%)";
const PROVENANCE_NEUTRAL_COLOR =
    "color-mix(in oklab, var(--color-neutral) 62%, var(--color-base-content) 38%)";
const PROVENANCE_RATE_DATABASE_COLOR =
    "color-mix(in oklab, var(--color-info) 76%, var(--color-base-content) 24%)";
const PROVENANCE_RATE_COMMUNITY_COLOR =
    "color-mix(in oklab, var(--color-warning) 80%, var(--color-base-content) 20%)";
const PROVENANCE_RATE_OVERLAY_COLOR =
    "color-mix(in oklab, var(--color-secondary) 78%, var(--color-base-content) 22%)";
const PROVENANCE_PRESENCE_FULL_COLOR =
    "color-mix(in oklab, var(--color-success) 78%, var(--color-base-content) 22%)";
const PROVENANCE_PRESENCE_PARTIAL_COLOR =
    "color-mix(in oklab, var(--color-warning) 80%, var(--color-base-content) 20%)";
const ICON_SPRITE_URL = "/img/icons.svg?v=20260430-2";

function normalizeSourceKind(value) {
    const normalized = trimString(value).toLowerCase();
    if (normalized === "database") {
        return "database";
    }
    if (normalized === "ranking") {
        return "ranking";
    }
    if (normalized === "community") {
        return "community";
    }
    if (normalized === "derived") {
        return "derived";
    }
    if (normalized === "overlay") {
        return "overlay";
    }
    if (normalized === "mixed") {
        return "mixed";
    }
    return "unknown";
}

function sourceLabel(channel, sourceKind) {
    if (sourceKind === "database") {
        return "Database";
    }
    if (sourceKind === "ranking") {
        return channel === "presence" ? "Ranking ring" : "Ranking";
    }
    if (sourceKind === "community") {
        return channel === "rate" ? "Community guess" : "Community";
    }
    if (sourceKind === "derived") {
        return "Derived";
    }
    if (sourceKind === "overlay") {
        return "Personal overlay";
    }
    if (sourceKind === "mixed") {
        return channel === "presence" ? "Mixed support" : "Mixed sources";
    }
    return "Unspecified";
}

function sourceKindFromLabel(value) {
    const normalized = trimString(value).toLowerCase();
    if (normalized.includes("database")) {
        return "database";
    }
    if (normalized.includes("ranking")) {
        return "ranking";
    }
    if (normalized.includes("community")) {
        return "community";
    }
    if (normalized.includes("derived")) {
        return "derived";
    }
    if (normalized.includes("overlay")) {
        return "overlay";
    }
    if (normalized.includes("mixed")) {
        return "mixed";
    }
    return "unknown";
}

function sourceTone(channel, sourceKind, detail, valueText = "") {
    const normalizedSourceKind = normalizeSourceKind(sourceKind);
    if (normalizedSourceKind === "mixed") {
        return "mixed";
    }
    if (normalizedSourceKind === "database") {
        return "database";
    }
    if (normalizedSourceKind === "community") {
        return "community";
    }
    if (channel === "presence" && normalizedSourceKind === "ranking") {
        const ringSupport = presenceRingSupport(detail, valueText);
        if (ringSupport === "full") {
            return "ranking-full";
        }
        if (ringSupport === "partial") {
            return "ranking-partial";
        }
    }
    if (normalizedSourceKind === "ranking") {
        return "ranking";
    }
    if (normalizedSourceKind === "overlay") {
        return "overlay";
    }
    return "neutral";
}

function sourceIconAlias(channel, sourceKind, detail, valueText = "") {
    const normalizedSourceKind = normalizeSourceKind(sourceKind);
    if (normalizedSourceKind === "community") {
        return "source-community";
    }
    if (normalizedSourceKind === "database") {
        return "source-database";
    }
    if (channel === "presence" && normalizedSourceKind === "ranking") {
        const ringSupport = presenceRingSupport(detail, valueText);
        if (ringSupport === "full") {
            return "ring-full";
        }
        if (ringSupport === "partial") {
            return "ring-partial";
        }
    }
    return "";
}

function sourceBadgeClass(tone) {
    if (tone === "ranking" || tone === "ranking-full") {
        return "badge-success";
    }
    if (tone === "ranking-partial" || tone === "community") {
        return "badge-warning";
    }
    if (tone === "database") {
        return "badge-info";
    }
    if (tone === "overlay") {
        return "badge-secondary";
    }
    if (tone === "mixed") {
        return "badge-accent";
    }
    return "badge-neutral";
}

function defaultDetail(channel, sourceKind, valueText) {
    if (sourceKind === "database") {
        return channel === "rate"
            ? trimString(valueText)
                ? `Database-backed rate. Current value: ${trimString(valueText)}.`
                : "Database-backed rate."
            : "Database-backed presence.";
    }
    if (sourceKind === "ranking") {
        return channel === "presence"
            ? "Ranking ring overlap support from observed player positions."
            : "Ranking-derived provenance.";
    }
    if (sourceKind === "community") {
        return channel === "rate"
            ? "Community-maintained guessed rate."
            : "Community-maintained presence support.";
    }
    if (sourceKind === "derived") {
        return channel === "rate"
            ? trimString(valueText)
                ? `Derived rate. Current value: ${trimString(valueText)}.`
                : "Derived rate."
            : "Derived presence support.";
    }
    if (sourceKind === "overlay") {
        return channel === "rate"
            ? trimString(valueText)
                ? `Personal overlay proposal. Current value: ${trimString(valueText)}.`
                : "Personal overlay proposal."
            : "Personal overlay proposal.";
    }
    if (sourceKind === "mixed") {
        return channel === "presence"
            ? "Multiple presence provenance sources support this row."
            : "Multiple provenance sources support this row.";
    }
    return `No ${channel} provenance recorded yet.`;
}

function presenceRingSupport(detail, valueText) {
    const normalized = `${trimString(detail)} | ${trimString(valueText)}`.toLowerCase();
    if (
        normalized.includes("ring fully inside zone")
        || normalized.includes("fully contained")
        || normalized.includes("ring_full")
    ) {
        return "full";
    }
    if (
        normalized.includes("ring overlaps zone edge")
        || normalized.includes("ring overlaps zone")
        || normalized.includes("ring overlap")
        || normalized.includes("partially contained")
        || normalized.includes("ring_partial")
    ) {
        return "partial";
    }
    return "none";
}

function normalizedRateSourceKind(sourceKind, detail) {
    const normalizedSourceKind = normalizeSourceKind(sourceKind);
    if (normalizedSourceKind === "database" || normalizedSourceKind === "community") {
        return normalizedSourceKind;
    }
    const normalizedDetail = trimString(detail).toLowerCase();
    if (normalizedDetail.includes("database") || normalizedDetail.includes("db ")) {
        return "database";
    }
    if (normalizedDetail.includes("community")) {
        return "community";
    }
    return normalizedSourceKind;
}

export function provenanceIndicatorColor(
    channel,
    sourceKind,
    { active = true, detail = "", valueText = "" } = {},
) {
    if (!active) {
        return PROVENANCE_INACTIVE_COLOR;
    }
    if (channel === "presence") {
        const ringSupport = presenceRingSupport(detail, valueText);
        if (ringSupport === "full") {
            return PROVENANCE_PRESENCE_FULL_COLOR;
        }
        if (ringSupport === "partial") {
            return PROVENANCE_PRESENCE_PARTIAL_COLOR;
        }
        return PROVENANCE_NEUTRAL_COLOR;
    }
    const normalizedSourceKind = normalizedRateSourceKind(sourceKind, detail);
    if (normalizedSourceKind === "database") {
        return PROVENANCE_RATE_DATABASE_COLOR;
    }
    if (normalizedSourceKind === "community") {
        return PROVENANCE_RATE_COMMUNITY_COLOR;
    }
    if (normalizedSourceKind === "overlay") {
        return PROVENANCE_RATE_OVERLAY_COLOR;
    }
    return PROVENANCE_NEUTRAL_COLOR;
}

function buildSegment({
    channel,
    label,
    sourceKind,
    detail,
    valueText = "",
}) {
    const normalizedSourceKind = normalizeSourceKind(sourceKind);
    const normalizedDetail = trimString(detail);
    const normalizedValueText = trimString(valueText);
    const active =
        normalizedSourceKind !== "unknown"
        || normalizedDetail.length > 0
        || normalizedValueText.length > 0;
    const source = sourceLabel(channel, normalizedSourceKind);
    const resolvedDetail =
        normalizedDetail
        || (channel === "presence" && normalizedValueText ? normalizedValueText : "")
        || defaultDetail(channel, normalizedSourceKind, normalizedValueText);
    return {
        channel,
        label,
        sourceKind: normalizedSourceKind,
        sourceLabel: source,
        sourceTone: sourceTone(channel, normalizedSourceKind, resolvedDetail, normalizedValueText),
        sourceIcon: sourceIconAlias(channel, normalizedSourceKind, resolvedDetail, normalizedValueText),
        detail: resolvedDetail,
        color: provenanceIndicatorColor(channel, normalizedSourceKind, {
            active,
            detail: resolvedDetail,
            valueText: normalizedValueText,
        }),
        active,
    };
}

export function provenanceAriaLabel(segment) {
    const label = trimString(segment?.label) || "Provenance";
    const source = trimString(segment?.sourceLabel) || "Unspecified";
    const detail = trimString(segment?.detail);
    return detail ? `${label}: ${source}. ${detail}` : `${label}: ${source}.`;
}

export function buildProvenanceSegments({
    rateSourceKind,
    rateDetail,
    rateValueText,
    presenceSourceKind,
    presenceDetail,
    presenceValueText,
} = {}) {
    return [
        buildSegment({
            channel: "presence",
            label: "Presence",
            sourceKind: presenceSourceKind,
            detail: presenceDetail,
            valueText: presenceValueText,
        }),
        buildSegment({
            channel: "rate",
            label: "Rate",
            sourceKind: rateSourceKind,
            detail: rateDetail,
            valueText: rateValueText,
        }),
    ];
}

let tooltipElement = null;
const tooltipRoots = new WeakSet();

function ensureTooltipElement() {
    if (tooltipElement?.isConnected) {
        return tooltipElement;
    }
    const documentRef = globalThis.document;
    if (!documentRef?.body || typeof documentRef.createElement !== "function") {
        return null;
    }
    tooltipElement = documentRef.createElement("div");
    tooltipElement.className =
        "fishy-provenance-tooltip card card-compact border border-base-300 bg-base-100 shadow-xl";
    tooltipElement.hidden = true;
    tooltipElement.setAttribute("role", "tooltip");
    tooltipElement.innerHTML = `
        <div class="fishy-provenance-tooltip__header">
            <span class="fishy-provenance-tooltip__title">
                <span class="fishy-provenance-tooltip__source-icon-shell" aria-hidden="true"></span>
                <span class="fishy-provenance-tooltip__label"></span>
            </span>
            <span class="fishy-provenance-tooltip__source badge badge-sm badge-soft badge-neutral"></span>
        </div>
        <div class="fishy-provenance-tooltip__details"></div>
    `;
    documentRef.body.appendChild(tooltipElement);
    return tooltipElement;
}

function detailRows(detail) {
    return trimString(detail)
        .split(/\s*(?:\n+|\s+\|\s+|\s+·\s+)\s*/u)
        .map((part) => part.trim())
        .filter(Boolean)
        .map((part) => {
            const separatorIndex = part.indexOf(":");
            if (separatorIndex > 0 && separatorIndex <= 28) {
                return {
                    label: part.slice(0, separatorIndex).trim(),
                    value: part.slice(separatorIndex + 1).trim(),
                };
            }
            return { label: "", value: part };
        })
        .filter((row) => row.label || row.value);
}

function detailIconAlias(row) {
    const normalized = `${trimString(row.label)} ${trimString(row.value)}`.toLowerCase();
    if (normalized.includes("ring fully inside zone") || normalized.includes("ring_full")) {
        return "ring-full";
    }
    if (
        normalized.includes("ring overlaps zone edge")
        || normalized.includes("ring overlaps zone")
        || normalized.includes("ring_partial")
    ) {
        return "ring-partial";
    }
    if (normalized.includes("community") || normalized.includes("workbook")) {
        return "source-community";
    }
    if (normalized.includes("database") || normalized.includes("db ")) {
        return "source-database";
    }
    return "";
}

function detailRowTone(row) {
    const normalized = `${trimString(row.label)} ${trimString(row.value)}`.toLowerCase();
    if (normalized.includes("ring fully inside zone") || normalized.includes("ring_full")) {
        return "ranking-full";
    }
    if (
        normalized.includes("ring overlaps zone edge")
        || normalized.includes("ring overlaps zone")
        || normalized.includes("ring_partial")
    ) {
        return "ranking-partial";
    }
    if (normalized.includes("community") || normalized.includes("workbook")) {
        return "community";
    }
    if (normalized.includes("database") || normalized.includes("db ")) {
        return "database";
    }
    if (normalized.includes("overlay") || normalized.includes("personal")) {
        return "overlay";
    }
    return "";
}

function createSpriteIcon(documentRef, className, alias) {
    if (!documentRef || !alias) {
        return null;
    }
    const svg = documentRef.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", className);
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    svg.setAttribute("focusable", "false");
    const use = documentRef.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("width", "100%");
    use.setAttribute("height", "100%");
    use.setAttribute("href", `${ICON_SPRITE_URL}#fishy-${alias}`);
    svg.appendChild(use);
    return svg;
}

function appendDetailIcon(rowElement, alias) {
    const documentRef = globalThis.document;
    if (!documentRef || !alias) {
        return;
    }
    rowElement.classList.add("has-icon");
    const svg = createSpriteIcon(documentRef, "fishy-provenance-tooltip__detail-icon", alias);
    rowElement.appendChild(svg);
}

function renderSourceIcon(tooltip, alias) {
    const documentRef = globalThis.document;
    const shell = tooltip.querySelector(".fishy-provenance-tooltip__source-icon-shell");
    if (!documentRef || !shell) {
        return;
    }
    shell.replaceChildren();
    const svg = createSpriteIcon(documentRef, "fishy-provenance-tooltip__source-icon", alias);
    if (svg) {
        shell.appendChild(svg);
        return;
    }
    const status = documentRef.createElement("span");
    status.className = "fishy-provenance-tooltip__source-status status status-sm";
    status.setAttribute("aria-hidden", "true");
    shell.appendChild(status);
}

function renderTooltipDetails(tooltip, detail) {
    const documentRef = globalThis.document;
    const container = tooltip.querySelector(".fishy-provenance-tooltip__details");
    if (!container || !documentRef) {
        return;
    }
    container.replaceChildren();
    const rows = detailRows(detail);
    container.hidden = rows.length === 0;
    for (const row of rows) {
        const rowElement = documentRef.createElement("div");
        rowElement.className = row.label
            ? "fishy-provenance-tooltip__detail-row fishy-provenance-tooltip__detail-row--keyed"
            : "fishy-provenance-tooltip__detail-row";
        const rowTone = detailRowTone(row);
        if (rowTone) {
            rowElement.dataset.detailTone = rowTone;
        }
        if (row.label) {
            const keyElement = documentRef.createElement("span");
            keyElement.className = "fishy-provenance-tooltip__detail-key";
            keyElement.textContent = row.label;
            rowElement.appendChild(keyElement);
        }
        appendDetailIcon(rowElement, detailIconAlias(row));
        const valueElement = documentRef.createElement("span");
        valueElement.className = "fishy-provenance-tooltip__detail-value";
        valueElement.textContent = row.value;
        rowElement.appendChild(valueElement);
        container.appendChild(rowElement);
    }
}

function provenanceTargetFromEvent(eventTarget) {
    if (!eventTarget || typeof eventTarget.closest !== "function") {
        return null;
    }
    return eventTarget.closest("[data-fishy-provenance-detail]");
}

function updateTooltipPosition(tooltip, anchor, event) {
    const windowRef = globalThis.window;
    if (!tooltip || !windowRef) {
        return;
    }
    let clientX = Number(event?.clientX);
    let clientY = Number(event?.clientY);
    if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) {
        const rect = typeof anchor?.getBoundingClientRect === "function"
            ? anchor.getBoundingClientRect()
            : null;
        clientX = Number(rect?.left ?? 0) + Number(rect?.width ?? 0) / 2;
        clientY = Number(rect?.top ?? 0) + Number(rect?.height ?? 0) / 2;
    }
    const offsetX = 14;
    const offsetY = 18;
    tooltip.style.left = "0";
    tooltip.style.top = "0";
    tooltip.style.transform = `translate3d(${clientX + offsetX}px, ${clientY + offsetY}px, 0)`;
    const tooltipRect = typeof tooltip.getBoundingClientRect === "function"
        ? tooltip.getBoundingClientRect()
        : null;
    if (!tooltipRect) {
        return;
    }
    const viewportWidth = Number(windowRef.innerWidth ?? 0);
    const viewportHeight = Number(windowRef.innerHeight ?? 0);
    let nextX = clientX + offsetX;
    let nextY = clientY + offsetY;
    if (viewportWidth > 0 && nextX + tooltipRect.width + 12 > viewportWidth) {
        nextX = Math.max(12, clientX - tooltipRect.width - 12);
    }
    if (viewportHeight > 0 && nextY + tooltipRect.height + 12 > viewportHeight) {
        nextY = Math.max(12, clientY - tooltipRect.height - 12);
    }
    tooltip.style.transform = `translate3d(${nextX}px, ${nextY}px, 0)`;
}

function showTooltip(anchor, event) {
    const tooltip = ensureTooltipElement();
    if (!tooltip || !anchor?.dataset) {
        return;
    }
    const label = trimString(anchor.dataset.fishyProvenanceLabel) || "Provenance";
    const source = trimString(anchor.dataset.fishyProvenanceSource) || "Unspecified";
    let sourceKind = normalizeSourceKind(anchor.dataset.fishyProvenanceSourceKind);
    if (sourceKind === "unknown") {
        sourceKind = sourceKindFromLabel(source);
    }
    const detail = trimString(anchor.dataset.fishyProvenanceDetail);
    const color = trimString(anchor.dataset.fishyProvenanceColor);
    const tone = trimString(anchor.dataset.fishyProvenanceSourceTone)
        || sourceTone(label.toLowerCase(), sourceKind, detail);
    const icon = trimString(anchor.dataset.fishyProvenanceSourceIcon)
        || sourceIconAlias(label.toLowerCase(), sourceKind, detail);
    tooltip.querySelector(".fishy-provenance-tooltip__label").textContent = label;
    const sourceElement = tooltip.querySelector(".fishy-provenance-tooltip__source");
    sourceElement.className =
        `fishy-provenance-tooltip__source badge badge-sm badge-soft ${sourceBadgeClass(tone)}`;
    sourceElement.textContent = source;
    tooltip.dataset.sourceKind = sourceKind;
    tooltip.dataset.sourceTone = tone;
    renderSourceIcon(tooltip, icon);
    renderTooltipDetails(tooltip, detail);
    tooltip.style.setProperty("--fishy-provenance-tooltip-color", color);
    tooltip.hidden = false;
    updateTooltipPosition(tooltip, anchor, event);
}

function hideTooltip() {
    if (!tooltipElement) {
        return;
    }
    tooltipElement.hidden = true;
}

export function attachProvenanceTooltip(root) {
    if (!root || tooltipRoots.has(root) || typeof root.addEventListener !== "function") {
        return;
    }
    tooltipRoots.add(root);

    root.addEventListener("mouseover", (event) => {
        const target = provenanceTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        showTooltip(target, event);
    });

    root.addEventListener("mousemove", (event) => {
        const target = provenanceTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        showTooltip(target, event);
    });

    root.addEventListener("mouseout", (event) => {
        const target = provenanceTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        const nextTarget = provenanceTargetFromEvent(event.relatedTarget);
        if (nextTarget === target) {
            return;
        }
        hideTooltip();
    });

    root.addEventListener("focusin", (event) => {
        const target = provenanceTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        showTooltip(target);
    });

    root.addEventListener("focusout", (event) => {
        const target = provenanceTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        const nextTarget = provenanceTargetFromEvent(event.relatedTarget);
        if (nextTarget === target) {
            return;
        }
        hideTooltip();
    });
}
