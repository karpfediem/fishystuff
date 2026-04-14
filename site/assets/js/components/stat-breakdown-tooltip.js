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
    const formulaPart = trimString(row?.formula_part ?? row?.formulaPart);
    const formulaPartOrderRaw = Number(row?.formula_part_order ?? row?.formulaPartOrder);
    const formulaPartOrder = Number.isFinite(formulaPartOrderRaw) ? formulaPartOrderRaw : null;
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
        formulaPart,
        formulaPartOrder,
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

function normalizeFormulaTerm(term = {}) {
    const label = trimString(term?.label);
    const valueText = trimString(term?.value_text ?? term?.valueText);
    const aliases = Array.isArray(term?.aliases)
        ? term.aliases.map((alias) => trimString(alias)).filter(Boolean)
        : [];
    if (!label && !valueText && !aliases.length) {
        return null;
    }
    return {
        label: label || aliases[0] || "Value",
        valueText,
        aliases,
    };
}

export function normalizeStatBreakdownPayload(payload = {}) {
    const sections = Array.isArray(payload?.sections)
        ? payload.sections.map(normalizeBreakdownSection).filter(Boolean)
        : [];
    const formulaTerms = Array.isArray(payload?.formula_terms ?? payload?.formulaTerms)
        ? (payload.formula_terms ?? payload.formulaTerms).map(normalizeFormulaTerm).filter(Boolean)
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
        formulaTerms,
        sections,
    };
}

function statBreakdownMatchKey(value) {
    return trimString(value)
        .toLowerCase()
        .replace(/[().,]/g, "")
        .replace(/\s+/g, " ");
}

function statBreakdownSectionIsResult(section, index = 0) {
    const label = trimString(section?.label).toLowerCase();
    return label === "composition" || (index > 0 && label === "details");
}

function statBreakdownFormulaLeftHandSide(formulaText) {
    const segments = statBreakdownFormulaSegments(formulaText);
    const raw = segments.at(-1) || "";
    if (!raw) {
        return "";
    }
    const equalsIndex = raw.indexOf("=");
    if (equalsIndex < 0) {
        return "";
    }
    return trimString(raw.slice(0, equalsIndex));
}

export function statBreakdownResultKindLabel(payload = {}, section = {}) {
    const formulaText = trimString(payload?.formulaText ?? payload?.formula_text);
    const rows = Array.isArray(section?.rows) ? section.rows : [];
    const resultLabel = trimString(rows.at(-1)?.label);
    const leftHandSide = statBreakdownFormulaLeftHandSide(formulaText);
    const title = trimString(payload?.title);
    const semanticText = [leftHandSide, resultLabel, title]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
    const hasAdditiveFormula = formulaText.includes("+");
    const hasAverageSemantics = /\baverage\b/.test(semanticText);
    const hasTotalSemantics = /\btotal\b/.test(semanticText);

    if (hasAdditiveFormula && hasTotalSemantics) {
        return "Sum Total";
    }
    if (hasAverageSemantics) {
        return "Average";
    }
    if (hasTotalSemantics) {
        return "Total";
    }
    return "Result";
}

export function statBreakdownSectionDisplayLabel(section, index = 0, payload = null) {
    const label = trimString(section?.label) || "Details";
    if (statBreakdownSectionIsResult(section, index)) {
        if (payload) {
            return statBreakdownResultKindLabel(payload, section);
        }
        const resultLabel = trimString(section?.rows?.[0]?.label);
        if (Array.isArray(section?.rows) && section.rows.length === 1 && resultLabel) {
            return resultLabel;
        }
        return "Result";
    }
    return label;
}

function statBreakdownInputSection(section) {
    return trimString(section?.label).toLowerCase() === "inputs";
}

export function statBreakdownSectionRowGroups(section = {}) {
    const rows = Array.isArray(section?.rows) ? section.rows : [];
    const groups = [];
    const groupsByKey = new Map();
    rows.forEach((row, rowIndex) => {
        const formulaPart = trimString(row?.formulaPart);
        const order = Number.isFinite(row?.formulaPartOrder)
            ? row.formulaPartOrder
            : Number.MAX_SAFE_INTEGER;
        const key = formulaPart
            ? `part:${order}:${formulaPart}`
            : `row:${rowIndex}`;
        let group = groupsByKey.get(key);
        if (!group) {
            group = {
                label: formulaPart,
                order,
                firstRowIndex: rowIndex,
                rows: [],
            };
            groupsByKey.set(key, group);
            groups.push(group);
        }
        group.rows.push(row);
    });
    groups.sort((left, right) => (
        left.order - right.order
        || left.firstRowIndex - right.firstRowIndex
    ));
    return groups;
}

function statBreakdownInputGroupShowsTitle(group = {}) {
    const groupLabel = trimString(group?.label);
    const firstRowLabel = trimString(group?.rows?.[0]?.label);
    return Boolean(groupLabel) && (
        (Array.isArray(group?.rows) && group.rows.length > 1)
        || firstRowLabel.toLowerCase() !== groupLabel.toLowerCase()
    );
}

function statBreakdownSectionsByType(payload = {}) {
    const inputs = [];
    const other = [];
    const results = [];
    for (const [index, section] of (payload.sections || []).entries()) {
        if (statBreakdownInputSection(section)) {
            inputs.push(section);
            continue;
        }
        if (statBreakdownSectionIsResult(section, index)) {
            results.push(section);
            continue;
        }
        other.push(section);
    }
    return { inputs, other, results };
}

function statBreakdownResultRow(payload = {}) {
    const { results } = statBreakdownSectionsByType(payload);
    const section = results.at(-1);
    const rows = Array.isArray(section?.rows) ? section.rows : [];
    return rows.at(-1) ?? null;
}

function statBreakdownLastResultSectionIndex(payload = {}) {
    let lastResultIndex = -1;
    for (const [index, section] of (payload.sections || []).entries()) {
        if (statBreakdownSectionIsResult(section, index)) {
            lastResultIndex = index;
        }
    }
    return lastResultIndex;
}

function statBreakdownLabelAliases(label) {
    const aliases = new Set();
    const normalized = trimString(label);
    if (!normalized) {
        return [];
    }
    aliases.add(normalized);

    const parenthetical = normalized.match(/^(.*)\s+\((.+)\)$/);
    if (parenthetical) {
        aliases.add(trimString(parenthetical[1]));
        aliases.add(trimString(parenthetical[2]));
    }

    for (const prefix of ["Applied ", "Total ", "Uncapped "]) {
        if (normalized.startsWith(prefix)) {
            const base = trimString(normalized.slice(prefix.length));
            aliases.add(base);
            aliases.add(trimString(`${base} (${trimString(prefix).toLowerCase()})`));
        }
    }

    return Array.from(aliases).filter(Boolean);
}

function statBreakdownResolvedGroupValue(group, summaryRows = []) {
    const groupLabel = trimString(group?.label);
    if (!groupLabel) {
        return "";
    }
    const groupKey = statBreakdownMatchKey(groupLabel);
    const summaryMatch = summaryRows.find((row) => statBreakdownLabelAliases(row.label).some(
        (alias) => statBreakdownMatchKey(alias) === groupKey,
    ));
    if (summaryMatch?.valueText) {
        return summaryMatch.valueText;
    }

    const rows = Array.isArray(group?.rows) ? group.rows : [];
    if (rows.length === 1) {
        return rows[0].valueText;
    }
    const appliedRow = rows.find((row) => /applied\b/i.test(trimString(row?.detailText)));
    if (appliedRow?.valueText) {
        return appliedRow.valueText;
    }
    return rows
        .map((row) => trimString(row?.valueText))
        .filter(Boolean)
        .join(" + ");
}

function statBreakdownFormulaTermEntries(payload = {}) {
    const explicitFormulaTerms = Array.isArray(payload?.formulaTerms) ? payload.formulaTerms : [];
    const { inputs, other, results } = statBreakdownSectionsByType(payload);
    const entries = [];
    const entriesByKey = new Map();
    const register = (label, valueText = "", aliases = []) => {
        const labels = [
            ...statBreakdownLabelAliases(label),
            ...aliases.flatMap((alias) => statBreakdownLabelAliases(alias)),
        ];
        for (const alias of labels) {
            const key = statBreakdownMatchKey(alias);
            if (!key || entriesByKey.has(key)) {
                continue;
            }
            const entry = { label: alias, valueText };
            entriesByKey.set(key, entry);
            entries.push(entry);
        }
    };

    for (const term of explicitFormulaTerms) {
        register(term.label, term.valueText, term.aliases);
    }

    if (explicitFormulaTerms.length) {
        const resultRow = statBreakdownResultRow(payload);
        if (resultRow) {
            register(resultRow.label, payload.valueText || resultRow.valueText);
        }
        return entries;
    }

    const summaryRows = [...other, ...results].flatMap((section) => section.rows || []);
    const resultRow = statBreakdownResultRow(payload);
    if (resultRow) {
        register(resultRow.label, payload.valueText || resultRow.valueText);
    }

    for (const section of inputs) {
        for (const group of statBreakdownSectionRowGroups(section)) {
            if (group.label) {
                register(group.label, statBreakdownResolvedGroupValue(group, summaryRows));
                continue;
            }
            for (const row of group.rows || []) {
                register(row.label, row.valueText);
            }
        }
    }

    for (const row of summaryRows) {
        register(row.label, row.valueText);
    }

    return entries;
}

function statBreakdownFormulaSegments(formulaText) {
    return trimString(formulaText)
        .split(/\s*;\s*/u)
        .map((segment) => trimString(segment).replace(/\.$/, ""))
        .filter(Boolean);
}

function statBreakdownTokenizeFormulaSegment(raw, entries = []) {
    const tokens = [];
    let cursor = 0;
    let operatorBuffer = "";

    const flushOperator = () => {
        if (!operatorBuffer) {
            return;
        }
        tokens.push({ kind: "operator", text: operatorBuffer });
        operatorBuffer = "";
    };

    while (cursor < raw.length) {
        const remaining = raw.slice(cursor);
        const match = entries.find((entry) => {
            if (!remaining.toLowerCase().startsWith(entry.matchText.toLowerCase())) {
                return false;
            }
            const previous = raw[cursor - 1] || "";
            const next = raw[cursor + entry.matchText.length] || "";
            const bounded = (!/[a-z0-9%]/i.test(previous)) && (!/[a-z0-9%]/i.test(next));
            return bounded || entry.matchText.includes(" ") || entry.matchText.length === 1;
        });
        if (match) {
            flushOperator();
            tokens.push({
                kind: "term",
                text: raw.slice(cursor, cursor + match.matchText.length),
                valueText: match.valueText,
            });
            cursor += match.matchText.length;
            continue;
        }
        operatorBuffer += raw[cursor];
        cursor += 1;
    }
    flushOperator();
    return tokens.filter((token) => trimString(token.text));
}

export function statBreakdownFormulaTokenRows(formulaText, payload = {}) {
    const segments = statBreakdownFormulaSegments(formulaText);
    if (!segments.length) {
        return [];
    }
    const entries = statBreakdownFormulaTermEntries(payload)
        .map((entry) => ({ ...entry, matchText: entry.label }))
        .sort((left, right) => right.matchText.length - left.matchText.length);
    return segments
        .map((segment) => statBreakdownTokenizeFormulaSegment(segment, entries))
        .filter((tokens) => tokens.length);
}

export function statBreakdownFormulaTokens(formulaText, payload = {}) {
    return statBreakdownFormulaTokenRows(formulaText, payload)[0] || [];
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
    const formulaRows = documentRef.createElement("div");
    formulaRows.className = "fishy-stat-breakdown-tooltip__formula-rows";
    formula.append(formulaLabel, formulaRows);

    const sections = documentRef.createElement("div");
    sections.className = "fishy-stat-breakdown-tooltip__sections";

    tooltip.append(eyebrow, header, summary, formula, sections);
    documentRef.body.appendChild(tooltip);

    tooltipElement = tooltip;
    tooltipRefs = { eyebrowLabel, title, value, summary, formula, formulaRows, sections };
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

export function statBreakdownTooltipPointerForAnchor(anchor, event, activePointer = null) {
    const pointer = tooltipPointerSnapshot(event);
    if (pointer) {
        return { ...pointer, anchor };
    }
    if (activePointer?.anchor === anchor) {
        return activePointer;
    }
    return null;
}

export function statBreakdownTooltipAnchorPoint(anchor, position = null, fallbackPosition = null) {
    let clientX = Number(position?.clientX);
    let clientY = Number(position?.clientY);
    if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) {
        clientX = Number(fallbackPosition?.clientX);
        clientY = Number(fallbackPosition?.clientY);
    }
    if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) {
        const rect = typeof anchor?.getBoundingClientRect === "function"
            ? anchor.getBoundingClientRect()
            : null;
        clientX = Number(rect?.left ?? 0) + Number(rect?.width ?? 0) / 2;
        clientY = Number(rect?.top ?? 0) + Number(rect?.height ?? 0) / 2;
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

function buildFormulaToken(documentRef, token, variant = "symbolic") {
    if (token.kind !== "term") {
        const operator = documentRef.createElement("span");
        operator.className = "fishy-stat-breakdown-tooltip__formula-operator";
        operator.textContent = trimString(token.text);
        return operator;
    }

    const term = documentRef.createElement("span");
    term.className = [
        "fishy-stat-breakdown-tooltip__formula-term",
        variant === "resolved" ? "fishy-stat-breakdown-tooltip__formula-term--resolved" : "",
    ].filter(Boolean).join(" ");

    const label = documentRef.createElement("span");
    label.className = "fishy-stat-breakdown-tooltip__formula-term-label";
    label.textContent = token.text;
    term.appendChild(label);

    const resolvedValue = trimString(token.valueText);
    if (variant === "resolved" && resolvedValue && statBreakdownMatchKey(resolvedValue) !== statBreakdownMatchKey(token.text)) {
        const value = documentRef.createElement("span");
        value.className = "fishy-stat-breakdown-tooltip__formula-term-value";
        value.textContent = resolvedValue;
        term.appendChild(value);
    }

    return term;
}

function buildFormulaRow(documentRef, labelText, tokens, variant = "symbolic") {
    const row = documentRef.createElement("div");
    row.className = "fishy-stat-breakdown-tooltip__formula-row";

    const tokensElement = documentRef.createElement("div");
    tokensElement.className = "fishy-stat-breakdown-tooltip__formula-tokens";
    tokensElement.append(...tokens.map((token) => buildFormulaToken(documentRef, token, variant)));

    if (trimString(labelText)) {
        const label = documentRef.createElement("div");
        label.className = "fishy-stat-breakdown-tooltip__formula-row-label";
        label.textContent = labelText;
        row.appendChild(label);
    }

    row.appendChild(tokensElement);
    return row;
}

function buildSectionFormula(documentRef, labelText, tokenRows = [], variant = "resolved") {
    const container = documentRef.createElement("div");
    container.className = "fishy-stat-breakdown-tooltip__section-formula";
    container.append(
        ...tokenRows.map((tokens, rowIndex) => buildFormulaRow(
            documentRef,
            rowIndex === 0 ? labelText : "",
            tokens,
            variant,
        )),
    );
    return container;
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

function buildSection(documentRef, section, index = 0, { payload = null, resolvedFormulaRows = [] } = {}) {
    const displayLabel = statBreakdownSectionDisplayLabel(section, index, payload);
    const isSecondarySection = index > 0;
    const isResultSection = statBreakdownSectionIsResult(section, index);
    const isInputSection = statBreakdownInputSection(section);
    const rowGroups = statBreakdownSectionRowGroups(section);
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

    if (isResultSection && resolvedFormulaRows.length) {
        sectionElement.appendChild(
            buildSectionFormula(documentRef, "", resolvedFormulaRows, "resolved"),
        );
    }

    for (const group of rowGroups) {
        const groupElement = documentRef.createElement("div");
        const showsGroupTitle = isInputSection && statBreakdownInputGroupShowsTitle(group);
        groupElement.className = [
            "fishy-stat-breakdown-tooltip__group",
            isInputSection ? "fishy-stat-breakdown-tooltip__group--input" : "",
            isResultSection ? "fishy-stat-breakdown-tooltip__group--result" : "",
            showsGroupTitle ? "fishy-stat-breakdown-tooltip__group--titled" : "",
        ].filter(Boolean).join(" ");

        if (showsGroupTitle) {
            const groupTitle = documentRef.createElement("div");
            groupTitle.className = "fishy-stat-breakdown-tooltip__group-title";
            groupTitle.textContent = group.label;
            groupElement.appendChild(groupTitle);
        }

        for (const [rowIndex, row] of group.rows.entries()) {
            const rowElement = documentRef.createElement("div");
            const isLastSectionRow = group === rowGroups[rowGroups.length - 1]
                && rowIndex === group.rows.length - 1;
            const isEmphasisRow = isResultSection && isLastSectionRow;
            rowElement.className = [
                "fishy-stat-breakdown-tooltip__row",
                isEmphasisRow ? "fishy-stat-breakdown-tooltip__row--emphasis" : "",
            ].filter(Boolean).join(" ");

            const main = buildRowMain(documentRef, row, { showDetail: !isResultSection });

            const value = documentRef.createElement("div");
            value.className = "fishy-stat-breakdown-tooltip__row-value";
            value.textContent = row.valueText;

            rowElement.append(main, value);
            groupElement.appendChild(rowElement);
        }

        sectionElement.appendChild(groupElement);
    }

    return sectionElement;
}

function updateTooltipPosition(tooltip, anchor, position = null, fallbackPosition = null) {
    const windowRef = globalThis.window;
    if (!tooltip || !windowRef) {
        return;
    }
    const anchorPoint = statBreakdownTooltipAnchorPoint(anchor, position, fallbackPosition);
    const clientX = anchorPoint.clientX;
    const clientY = anchorPoint.clientY;
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
    const pointer = statBreakdownTooltipPointerForAnchor(anchor, event, activeTooltipPointer);
    if (pointer) {
        activeTooltipPointer = pointer;
    }
    observeActiveTooltipAnchor(anchor);
    const refreshState = statBreakdownTooltipShouldRefresh(activeTooltipRenderState, tooltip, anchor);
    if (refreshState.shouldRefresh) {
        refs.eyebrowLabel.textContent = payload.eyebrow;
        refs.title.textContent = payload.title;
        refs.value.textContent = payload.valueText;
        refs.value.hidden = !payload.valueText;
        refs.summary.textContent = payload.summaryText;
        refs.summary.hidden = !payload.summaryText || payload.sections.length > 0;
        const documentRef = globalThis.document;
        const formulaTokenRows = statBreakdownFormulaTokenRows(payload.formulaText, payload);
        const resultSectionIndex = statBreakdownLastResultSectionIndex(payload);
        const topFormulaRows = formulaTokenRows.map((tokens) => buildFormulaRow(
            documentRef,
            "",
            tokens,
            "symbolic",
        ));
        if (formulaTokenRows.length && resultSectionIndex < 0) {
            topFormulaRows.push(...formulaTokenRows.map((tokens) => buildFormulaRow(
                documentRef,
                "",
                tokens,
                "resolved",
            )));
        }
        refs.formulaRows.replaceChildren(...topFormulaRows);
        refs.formula.hidden = topFormulaRows.length === 0;

        refs.sections.replaceChildren(
            ...payload.sections.map((section, index) => buildSection(documentRef, section, index, {
                payload,
                resolvedFormulaRows: index === resultSectionIndex ? formulaTokenRows : [],
            })),
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
    updateTooltipPosition(tooltip, anchor, pointer, event);
}

function hideTooltip() {
    if (!tooltipElement) {
        return;
    }
    tooltipElement.hidden = true;
    activeTooltipRenderState = null;
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
