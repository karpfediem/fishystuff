function trimString(value) {
    const normalized = String(value ?? "").trim();
    return normalized || "";
}

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
    if (sourceKind === "mixed") {
        return channel === "presence"
            ? "Multiple presence provenance sources support this row."
            : "Multiple provenance sources support this row.";
    }
    return `No ${channel} provenance recorded yet.`;
}

export function provenanceIndicatorColor(channel, sourceKind, { active = true } = {}) {
    const databaseColor =
        "color-mix(in oklab, var(--color-info) 76%, var(--color-base-content) 24%)";
    const rankingColor =
        "color-mix(in oklab, var(--color-info) 46%, var(--color-success) 54%)";
    const communityPresenceColor =
        "color-mix(in oklab, var(--color-success) 78%, var(--color-base-content) 22%)";
    const communityRateColor =
        "color-mix(in oklab, var(--color-warning) 80%, var(--color-base-content) 20%)";
    if (!active) {
        return "color-mix(in oklab, var(--color-neutral) 28%, var(--color-base-300) 72%)";
    }
    if (sourceKind === "database") {
        return databaseColor;
    }
    if (sourceKind === "ranking") {
        return rankingColor;
    }
    if (sourceKind === "community") {
        if (channel === "presence") {
            return communityPresenceColor;
        }
        return communityRateColor;
    }
    if (sourceKind === "mixed") {
        if (channel === "presence") {
            return `linear-gradient(180deg, ${rankingColor} 0 50%, ${communityPresenceColor} 50% 100%)`;
        }
        return `linear-gradient(180deg, ${databaseColor} 0 50%, ${communityRateColor} 50% 100%)`;
    }
    return "color-mix(in oklab, var(--color-neutral) 62%, var(--color-base-content) 38%)";
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
    return {
        channel,
        label,
        sourceKind: normalizedSourceKind,
        sourceLabel: source,
        detail:
            normalizedDetail
            || (channel === "presence" && normalizedValueText ? normalizedValueText : "")
            || defaultDetail(channel, normalizedSourceKind, normalizedValueText),
        color: provenanceIndicatorColor(channel, normalizedSourceKind, { active }),
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
