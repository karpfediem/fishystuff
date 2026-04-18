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
        return channel === "presence" ? "Mixed support" : "Mixed provenance";
    }
    return "Unspecified";
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
    tooltipElement.className = "fishy-provenance-tooltip";
    tooltipElement.hidden = true;
    tooltipElement.setAttribute("role", "tooltip");
    tooltipElement.innerHTML = `
        <div class="fishy-provenance-tooltip__eyebrow">
            <span class="fishy-provenance-tooltip__swatch" aria-hidden="true"></span>
            <span class="fishy-provenance-tooltip__label"></span>
        </div>
        <div class="fishy-provenance-tooltip__source"></div>
        <div class="fishy-provenance-tooltip__detail"></div>
    `;
    documentRef.body.appendChild(tooltipElement);
    return tooltipElement;
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
    const detail = trimString(anchor.dataset.fishyProvenanceDetail);
    const color = trimString(anchor.dataset.fishyProvenanceColor);
    tooltip.querySelector(".fishy-provenance-tooltip__label").textContent = label;
    tooltip.querySelector(".fishy-provenance-tooltip__source").textContent = source;
    tooltip.querySelector(".fishy-provenance-tooltip__detail").textContent = detail;
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
