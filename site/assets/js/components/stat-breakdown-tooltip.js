function trimString(value) {
    const normalized = String(value ?? "").trim();
    return normalized || "";
}

function normalizeBreakdownRow(row = {}) {
    const label = trimString(row?.label);
    const valueText = trimString(row?.value_text ?? row?.valueText);
    const detailText = trimString(row?.detail_text ?? row?.detailText);
    const kind = trimString(row?.kind).toLowerCase();
    const iconUrl = trimString(row?.icon_url ?? row?.iconUrl);
    const gradeTone = trimString(row?.grade_tone ?? row?.gradeTone).toLowerCase() || "unknown";
    if (!label && !valueText && !detailText) {
        return null;
    }
    return {
        label: label || "Value",
        valueText,
        detailText,
        kind,
        iconUrl,
        gradeTone,
    };
}

function normalizeBreakdownSection(section = {}) {
    const rows = Array.isArray(section?.rows)
        ? section.rows.map(normalizeBreakdownRow).filter(Boolean)
        : [];
    if (!rows.length) {
        return null;
    }
    return {
        label: trimString(section?.label) || "Details",
        rows,
    };
}

export function normalizeStatBreakdownPayload(payload = {}) {
    const sections = Array.isArray(payload?.sections)
        ? payload.sections.map(normalizeBreakdownSection).filter(Boolean)
        : [];
    const title = trimString(payload?.title);
    const valueText = trimString(payload?.value_text ?? payload?.valueText);
    const summaryText = trimString(payload?.summary_text ?? payload?.summaryText);
    const formulaText = trimString(payload?.formula_text ?? payload?.formulaText);
    if (!title && !valueText && !summaryText && !sections.length) {
        return null;
    }
    return {
        eyebrow: trimString(payload?.kind_label ?? payload?.kindLabel) || "Computed stat",
        title: title || "Breakdown",
        valueText,
        summaryText,
        formulaText,
        sections,
    };
}

function statBreakdownSectionIsResult(section, index = 0) {
    const label = trimString(section?.label).toLowerCase();
    return label === "composition" || (index > 0 && label === "details");
}

export function statBreakdownSectionDisplayLabel(section, index = 0) {
    const label = trimString(section?.label) || "Details";
    if (statBreakdownSectionIsResult(section, index)) {
        const resultLabel = trimString(section?.rows?.[0]?.label);
        if (Array.isArray(section?.rows) && section.rows.length === 1 && resultLabel) {
            return resultLabel;
        }
        return "Result";
    }
    return label;
}

let tooltipElement = null;
let tooltipRefs = null;
const tooltipRoots = new WeakSet();
const payloadCache = new WeakMap();
let activeTooltipRenderState = null;
let activeTooltipAnchor = null;
let activeTooltipAnchorObserver = null;
let activeTooltipPointer = null;

export const STAT_BREAKDOWN_TOOLTIP_ATTRIBUTE_FILTER = [
    "data-fishy-stat-breakdown",
    "data-fishy-stat-color",
];

function ensureTooltipElement() {
    if (tooltipElement?.isConnected && tooltipRefs) {
        return { tooltip: tooltipElement, refs: tooltipRefs };
    }
    const documentRef = globalThis.document;
    if (!documentRef?.body || typeof documentRef.createElement !== "function") {
        return null;
    }

    const tooltip = documentRef.createElement("div");
    tooltip.className = "fishy-stat-breakdown-tooltip";
    tooltip.hidden = true;
    tooltip.setAttribute("role", "tooltip");

    const eyebrow = documentRef.createElement("div");
    eyebrow.className = "fishy-stat-breakdown-tooltip__eyebrow";
    const swatch = documentRef.createElement("span");
    swatch.className = "fishy-stat-breakdown-tooltip__swatch";
    swatch.setAttribute("aria-hidden", "true");
    const eyebrowLabel = documentRef.createElement("span");
    eyebrowLabel.className = "fishy-stat-breakdown-tooltip__eyebrow-label";
    eyebrow.append(swatch, eyebrowLabel);

    const header = documentRef.createElement("div");
    header.className = "fishy-stat-breakdown-tooltip__header";
    const title = documentRef.createElement("div");
    title.className = "fishy-stat-breakdown-tooltip__title";
    const value = documentRef.createElement("div");
    value.className = "fishy-stat-breakdown-tooltip__value";
    header.append(title, value);

    const summary = documentRef.createElement("div");
    summary.className = "fishy-stat-breakdown-tooltip__summary";

    const formula = documentRef.createElement("div");
    formula.className = "fishy-stat-breakdown-tooltip__formula";
    const formulaLabel = documentRef.createElement("div");
    formulaLabel.className = "fishy-stat-breakdown-tooltip__formula-label";
    formulaLabel.textContent = "Formula";
    const formulaBody = documentRef.createElement("div");
    formulaBody.className = "fishy-stat-breakdown-tooltip__formula-body";
    formula.append(formulaLabel, formulaBody);

    const sections = documentRef.createElement("div");
    sections.className = "fishy-stat-breakdown-tooltip__sections";

    tooltip.append(eyebrow, header, summary, formula, sections);
    documentRef.body.appendChild(tooltip);

    tooltipElement = tooltip;
    tooltipRefs = { eyebrowLabel, title, value, summary, formula, formulaBody, sections };
    return { tooltip, refs: tooltipRefs };
}

function statBreakdownTargetFromEvent(eventTarget) {
    if (!eventTarget || typeof eventTarget.closest !== "function") {
        return null;
    }
    return eventTarget.closest("[data-fishy-stat-breakdown]");
}

export function statBreakdownPayloadForAnchor(anchor) {
    if (!anchor) {
        return null;
    }
    const raw = trimString(anchor.dataset?.fishyStatBreakdown);
    const cached = payloadCache.get(anchor);
    if (cached && cached.raw === raw) {
        return cached.payload;
    }
    let payload = null;
    if (raw) {
        try {
            payload = normalizeStatBreakdownPayload(JSON.parse(raw));
        } catch {
            payload = null;
        }
    }
    payloadCache.set(anchor, { raw, payload });
    return payload;
}

export function statBreakdownTooltipRenderKey(anchor) {
    const raw = trimString(anchor?.dataset?.fishyStatBreakdown);
    if (!raw) {
        return "";
    }
    const color = trimString(anchor?.dataset?.fishyStatColor) || "var(--color-info)";
    return `${raw}\u0000${color}`;
}

export function statBreakdownTooltipShouldRefresh(renderState, tooltip, anchor) {
    const renderKey = statBreakdownTooltipRenderKey(anchor);
    return {
        renderKey,
        shouldRefresh: Boolean(renderKey)
            && (renderState?.tooltip !== tooltip || renderState?.renderKey !== renderKey),
    };
}

export function statBreakdownTooltipShouldReactToMutations(mutations = []) {
    return mutations.some((mutation) => (
        mutation?.type === "attributes"
            && STAT_BREAKDOWN_TOOLTIP_ATTRIBUTE_FILTER.includes(mutation.attributeName)
    ));
}

function tooltipPointerSnapshot(event) {
    const clientX = Number(event?.clientX);
    const clientY = Number(event?.clientY);
    if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) {
        return null;
    }
    return { clientX, clientY };
}

function disconnectActiveTooltipAnchorObserver() {
    activeTooltipAnchorObserver?.disconnect?.();
    activeTooltipAnchorObserver = null;
    activeTooltipAnchor = null;
}

function observeActiveTooltipAnchor(anchor) {
    if (activeTooltipAnchor === anchor && activeTooltipAnchorObserver) {
        return;
    }
    disconnectActiveTooltipAnchorObserver();
    const Observer = globalThis.MutationObserver;
    if (!anchor || typeof Observer !== "function") {
        return;
    }
    activeTooltipAnchor = anchor;
    activeTooltipAnchorObserver = new Observer((mutations) => {
        if (anchor !== activeTooltipAnchor || !tooltipElement || tooltipElement.hidden) {
            return;
        }
        if (!statBreakdownTooltipShouldReactToMutations(mutations)) {
            return;
        }
        if (!statBreakdownPayloadForAnchor(anchor)) {
            hideTooltip();
            return;
        }
        showTooltip(anchor, activeTooltipPointer);
    });
    activeTooltipAnchorObserver.observe(anchor, {
        attributes: true,
        attributeFilter: STAT_BREAKDOWN_TOOLTIP_ATTRIBUTE_FILTER,
    });
}

function itemToneClass(gradeTone) {
    const normalized = trimString(gradeTone).toLowerCase() || "unknown";
    return `fishy-item-grade-${normalized}`;
}

function itemFallbackLabel(label) {
    return trimString(label).charAt(0).toUpperCase() || "?";
}

function buildRowMain(documentRef, row, { showDetail = true } = {}) {
    const main = documentRef.createElement("div");
    main.className = "fishy-stat-breakdown-tooltip__row-main";

    if (row.kind === "item") {
        const toneClass = itemToneClass(row.gradeTone);
        const itemRow = documentRef.createElement("span");
        itemRow.className = `fishy-stat-breakdown-tooltip__item-row fishy-item-row ${toneClass}`;

        const iconFrame = documentRef.createElement("span");
        iconFrame.className = `fishy-stat-breakdown-tooltip__item-icon-frame fishy-item-icon-frame is-xs ${toneClass}`;
        if (row.iconUrl) {
            const icon = documentRef.createElement("img");
            icon.className = "fishy-stat-breakdown-tooltip__item-icon fishy-item-icon item-icon";
            icon.src = row.iconUrl;
            icon.alt = `${row.label} icon`;
            icon.loading = "lazy";
            icon.decoding = "async";
            iconFrame.appendChild(icon);
        } else {
            const fallback = documentRef.createElement("span");
            fallback.className = `fishy-item-icon-fallback ${toneClass}`;
            fallback.textContent = itemFallbackLabel(row.label);
            iconFrame.appendChild(fallback);
        }

        const copy = documentRef.createElement("span");
        copy.className = "fishy-stat-breakdown-tooltip__item-copy";
        const label = documentRef.createElement("span");
        label.className = `fishy-stat-breakdown-tooltip__item-label fishy-item-label ${toneClass}`;
        label.textContent = row.label;
        copy.appendChild(label);

        itemRow.append(iconFrame, copy);
        main.appendChild(itemRow);
    } else {
        const label = documentRef.createElement("div");
        label.className = "fishy-stat-breakdown-tooltip__row-label";
        label.textContent = row.label;
        main.appendChild(label);
    }

    if (showDetail && row.detailText) {
        const detail = documentRef.createElement("div");
        detail.className = row.kind === "item"
            ? "fishy-stat-breakdown-tooltip__item-detail fishy-stat-breakdown-tooltip__row-detail"
            : "fishy-stat-breakdown-tooltip__row-detail";
        detail.textContent = row.detailText;
        main.appendChild(detail);
    }

    return main;
}

function buildSection(documentRef, section, index = 0) {
    const displayLabel = statBreakdownSectionDisplayLabel(section, index);
    const isSecondarySection = index > 0;
    const isResultSection = statBreakdownSectionIsResult(section, index);
    const sectionElement = documentRef.createElement("section");
    sectionElement.className = [
        "fishy-stat-breakdown-tooltip__section",
        isResultSection ? "fishy-stat-breakdown-tooltip__section--result" : "",
    ].filter(Boolean).join(" ");

    if (isSecondarySection) {
        const divider = documentRef.createElement("div");
        divider.className = "divider divider-neutral fishy-stat-breakdown-tooltip__section-divider";
        divider.textContent = displayLabel;
        sectionElement.appendChild(divider);
    } else {
        const title = documentRef.createElement("div");
        title.className = "fishy-stat-breakdown-tooltip__section-title";
        title.textContent = displayLabel;
        sectionElement.appendChild(title);
    }

    for (const [rowIndex, row] of section.rows.entries()) {
        const rowElement = documentRef.createElement("div");
        const isEmphasisRow = isResultSection && rowIndex === section.rows.length - 1;
        rowElement.className = [
            "fishy-stat-breakdown-tooltip__row",
            isEmphasisRow ? "fishy-stat-breakdown-tooltip__row--emphasis" : "",
        ].filter(Boolean).join(" ");

        const main = buildRowMain(documentRef, row, { showDetail: !isResultSection });

        const value = documentRef.createElement("div");
        value.className = "fishy-stat-breakdown-tooltip__row-value";
        value.textContent = row.valueText;

        rowElement.append(main, value);
        sectionElement.appendChild(rowElement);
    }

    return sectionElement;
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
    const tooltipData = ensureTooltipElement();
    if (!tooltipData || !anchor?.dataset) {
        return;
    }
    const payload = statBreakdownPayloadForAnchor(anchor);
    if (!payload) {
        return;
    }
    const { tooltip, refs } = tooltipData;
    activeTooltipPointer = tooltipPointerSnapshot(event) ?? activeTooltipPointer;
    observeActiveTooltipAnchor(anchor);
    const refreshState = statBreakdownTooltipShouldRefresh(activeTooltipRenderState, tooltip, anchor);
    if (refreshState.shouldRefresh) {
        refs.eyebrowLabel.textContent = payload.eyebrow;
        refs.title.textContent = payload.title;
        refs.value.textContent = payload.valueText;
        refs.value.hidden = !payload.valueText;
        refs.summary.textContent = payload.summaryText;
        refs.summary.hidden = !payload.summaryText || payload.sections.length > 0;
        refs.formulaBody.textContent = payload.formulaText;
        refs.formula.hidden = !payload.formulaText;

        const documentRef = globalThis.document;
        refs.sections.replaceChildren(
            ...payload.sections.map((section, index) => buildSection(documentRef, section, index)),
        );
        refs.sections.hidden = payload.sections.length === 0;

        tooltip.style.setProperty(
            "--fishy-stat-breakdown-color",
            trimString(anchor.dataset.fishyStatColor) || "var(--color-info)",
        );
        activeTooltipRenderState = {
            tooltip,
            renderKey: refreshState.renderKey,
        };
    }
    tooltip.hidden = false;
    updateTooltipPosition(tooltip, anchor, event);
}

function hideTooltip() {
    if (!tooltipElement) {
        return;
    }
    tooltipElement.hidden = true;
    activeTooltipRenderState = null;
    activeTooltipPointer = null;
    disconnectActiveTooltipAnchorObserver();
}

export function attachStatBreakdownTooltip(root) {
    if (!root || tooltipRoots.has(root) || typeof root.addEventListener !== "function") {
        return;
    }
    tooltipRoots.add(root);

    root.addEventListener("mouseover", (event) => {
        const target = statBreakdownTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        showTooltip(target, event);
    });

    root.addEventListener("mousemove", (event) => {
        const target = statBreakdownTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        showTooltip(target, event);
    });

    root.addEventListener("mouseout", (event) => {
        const target = statBreakdownTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        const nextTarget = statBreakdownTargetFromEvent(event.relatedTarget);
        if (nextTarget === target) {
            return;
        }
        hideTooltip();
    });

    root.addEventListener("focusin", (event) => {
        const target = statBreakdownTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        showTooltip(target);
    });

    root.addEventListener("focusout", (event) => {
        const target = statBreakdownTargetFromEvent(event.target);
        if (!target) {
            return;
        }
        const nextTarget = statBreakdownTargetFromEvent(event.relatedTarget);
        if (nextTarget === target) {
            return;
        }
        hideTooltip();
    });
}
