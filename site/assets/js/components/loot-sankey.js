import * as d3 from "../d3.js";

const MIN_INTERNAL_WIDTH = 1320;
const TOP_PADDING = 20;
const BOTTOM_PADDING = 20;
const GROUP_GAP = 12;
const SPECIES_GAP = 6;
const SPECIES_LABEL_GAP = 8;
const LEFT_X = 24;
const LEFT_WIDTH = 200;
const RIGHT_BAR_WIDTH = 18;
const RIGHT_LABEL_WIDTH = 248;
const RIGHT_LABEL_HEIGHT = 54;
const RIGHT_LABEL_OFFSET = 14;
const GROUP_TO_SPECIES_GAP = 110;
const SPECIES_TO_SILVER_GAP = 78;
const SILVER_TO_GROUP_GAP = 76;
const SILVER_GROUP_WIDTH = 212;
const NODE_RADIUS = 12;

function provenanceDotColor(kind) {
    if (kind === "database") {
        return "color-mix(in oklab, var(--color-info) 72%, var(--color-base-content) 28%)";
    }
    if (kind === "community") {
        return "color-mix(in oklab, var(--color-warning) 78%, var(--color-base-content) 22%)";
    }
    return "color-mix(in oklab, var(--color-neutral) 72%, var(--color-base-content) 28%)";
}

function positiveNumber(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) && numeric > 0 ? numeric : 0;
}

function stackedHeight(rows, scale, gap) {
    if (!rows.length) {
        return 0;
    }
    const total = d3.sum(rows, (row) => positiveNumber(row.expected_count_raw) * scale);
    return total + gap * Math.max(0, rows.length - 1);
}

function readChartSignal(path) {
    return window.__fishystuffCalculator?.readSignal?.(path) ?? null;
}

function compactSilverText(valueText) {
    const numeric = Number(String(valueText ?? "").replaceAll(",", ""));
    if (!Number.isFinite(numeric)) {
        return String(valueText ?? "");
    }
    if (numeric < 1000) {
        return Math.round(numeric).toString();
    }
    return new Intl.NumberFormat("en-US", {
        notation: "compact",
        maximumFractionDigits: 1,
    }).format(numeric);
}

function sankeyPath(x1, y1, x2, y2, h1, h2) {
    const c1 = x1 + 120;
    const c2 = x2 - 120;
    return [
        `M ${x1} ${y1}`,
        `C ${c1} ${y1}, ${c2} ${y2}, ${x2} ${y2}`,
        `L ${x2} ${y2 + h2}`,
        `C ${c2} ${y2 + h2}, ${c1} ${y1 + h1}, ${x1} ${y1 + h1}`,
        "Z",
    ].join(" ");
}

class FishyLootSankey extends HTMLElement {
    #resizeObserver = null;
    #rafId = 0;
    _handleSignalPatchBound = null;

    constructor() {
        super();
        this._handleSignalPatchBound = () => this.#handleSignalPatch();
    }

    static get observedAttributes() {
        return ["signal-path", "aria-label"];
    }

    connectedCallback() {
        this.#scheduleRender();
        this.#resizeObserver = new ResizeObserver(() => this.#scheduleRender());
        this.#resizeObserver.observe(this);
        document.addEventListener(
            "datastar-patch-signals",
            this._handleSignalPatchBound,
        );
    }

    disconnectedCallback() {
        if (this.#resizeObserver) {
            this.#resizeObserver.disconnect();
            this.#resizeObserver = null;
        }
        if (this.#rafId) {
            cancelAnimationFrame(this.#rafId);
            this.#rafId = 0;
        }
        document.removeEventListener(
            "datastar-patch-signals",
            this._handleSignalPatchBound,
        );
    }

    attributeChangedCallback() {
        this.#scheduleRender();
    }

    #handleSignalPatch() {
        this.#scheduleRender();
    }

    #scheduleRender() {
        if (this.#rafId) {
            cancelAnimationFrame(this.#rafId);
        }
        this.#rafId = requestAnimationFrame(() => {
            this.#rafId = 0;
            this.#render();
        });
    }

    #render() {
        const chart = readChartSignal(this.getAttribute("signal-path"));
        const rows = Array.isArray(chart?.rows) ? chart.rows : [];
        const speciesRows = Array.isArray(chart?.species_rows) ? chart.species_rows : [];
        const showSilverAmounts = Boolean(chart?.show_silver_amounts);
        if (!rows.length || !speciesRows.length) {
            this.replaceChildren();
            return;
        }

        const totalCount = Math.max(
            Number.EPSILON,
            d3.sum(speciesRows, (row) => positiveNumber(row.expected_count_raw)),
        );
        const totalProfit = Math.max(
            Number.EPSILON,
            d3.sum(speciesRows, (row) => positiveNumber(row.expected_profit_raw)),
        );
        const labelStackHeight = speciesRows.length
            ? speciesRows.length * RIGHT_LABEL_HEIGHT
                + Math.max(0, speciesRows.length - 1) * SPECIES_LABEL_GAP
            : 0;
        const innerHeight = Math.max(labelStackHeight, 340);
        const countBarX = LEFT_X + LEFT_WIDTH + GROUP_TO_SPECIES_GAP;
        const labelX = countBarX + RIGHT_BAR_WIDTH + RIGHT_LABEL_OFFSET;
        const silverBarX = labelX + RIGHT_LABEL_WIDTH + SPECIES_TO_SILVER_GAP;
        const silverGroupX = silverBarX + RIGHT_BAR_WIDTH + SILVER_TO_GROUP_GAP;
        const width = Math.max(
            this.clientWidth || 0,
            Math.max(MIN_INTERNAL_WIDTH, silverGroupX + SILVER_GROUP_WIDTH + 24),
        );
        const leftScale = Math.max(
            0,
            (innerHeight - GROUP_GAP * Math.max(0, rows.length - 1)) / totalCount,
        );
        const rightScale = Math.max(
            0,
            (innerHeight - SPECIES_GAP * Math.max(0, speciesRows.length - 1)) / totalCount,
        );
        const profitScale = Math.max(
            0,
            (innerHeight - SPECIES_GAP * Math.max(0, speciesRows.length - 1)) / totalProfit,
        );
        const height = innerHeight + TOP_PADDING + BOTTOM_PADDING;

        const groupTop = new Map();
        let leftCursor = TOP_PADDING;
        rows.forEach((row) => {
            groupTop.set(row.label, leftCursor);
            leftCursor += positiveNumber(row.expected_count_raw) * leftScale + GROUP_GAP;
        });

        const speciesTop = [];
        let rightCursor = TOP_PADDING;
        speciesRows.forEach((row) => {
            speciesTop.push(rightCursor);
            rightCursor += positiveNumber(row.expected_count_raw) * rightScale + SPECIES_GAP;
        });

        const profitSpeciesTop = [];
        let profitSpeciesCursor = TOP_PADDING;
        speciesRows.forEach((row) => {
            profitSpeciesTop.push(profitSpeciesCursor);
            profitSpeciesCursor += positiveNumber(row.expected_profit_raw) * profitScale + SPECIES_GAP;
        });

        const profitGroupTop = new Map();
        let profitGroupCursor = TOP_PADDING;
        rows.forEach((row) => {
            profitGroupTop.set(row.label, profitGroupCursor);
            profitGroupCursor += positiveNumber(row.expected_profit_raw) * profitScale + GROUP_GAP;
        });

        const speciesLabelTop = [];
        let labelCursor = TOP_PADDING;
        speciesRows.forEach(() => {
            speciesLabelTop.push(labelCursor);
            labelCursor += RIGHT_LABEL_HEIGHT + SPECIES_LABEL_GAP;
        });

        const leftFlowCursor = new Map(groupTop);
        const profitFlowCursor = new Map(profitGroupTop);

        const svg = d3
            .create("svg")
            .attr("viewBox", `0 0 ${width} ${height}`)
            .attr("role", "img")
            .attr(
                "aria-label",
                this.getAttribute("aria-label") || "Expected loot flows from groups to species",
            );

        const countFlows = svg.append("g");
        speciesRows.forEach((row, index) => {
            const leftTop = leftFlowCursor.get(row.group_label) ?? TOP_PADDING;
            const leftHeight = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * leftScale,
            );
            const rightTop = speciesTop[index];
            const rightHeight = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * rightScale,
            );

            countFlows.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        LEFT_X + LEFT_WIDTH,
                        leftTop,
                        countBarX,
                        rightTop,
                        leftHeight,
                        rightHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.42);

            leftFlowCursor.set(row.group_label, leftTop + leftHeight);
        });

        const silverFlows = svg.append("g");
        speciesRows.forEach((row, index) => {
            const countTop = speciesTop[index];
            const countHeight = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * rightScale,
            );
            const profitRaw = positiveNumber(row.expected_profit_raw);
            const profitHeight = profitRaw * profitScale;
            if (profitHeight <= 0) {
                return;
            }
            const speciesProfitTop = profitSpeciesTop[index];
            silverFlows.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        countBarX + RIGHT_BAR_WIDTH,
                        countTop,
                        silverBarX,
                        speciesProfitTop,
                        countHeight,
                        profitHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.28);

            const groupProfitTop = profitFlowCursor.get(row.group_label) ?? TOP_PADDING;
            silverFlows.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        silverBarX + RIGHT_BAR_WIDTH,
                        speciesProfitTop,
                        silverGroupX,
                        groupProfitTop,
                        profitHeight,
                        profitHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.44);

            profitFlowCursor.set(row.group_label, groupProfitTop + profitHeight);
        });

        const leftNodes = svg.append("g");
        rows.forEach((row) => {
            const top = groupTop.get(row.label) ?? TOP_PADDING;
            const heightValue = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * leftScale,
            );
            const mid = top + heightValue / 2;
            const valueLabel = showSilverAmounts
                ? `${row.expected_count_text} | ${compactSilverText(row.expected_profit_text)}`
                : row.expected_count_text;
            const evidenceLabel = String(row.evidence_text ?? "");

            leftNodes.append("rect")
                .attr("x", LEFT_X)
                .attr("y", top)
                .attr("width", LEFT_WIDTH)
                .attr("height", heightValue)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            leftNodes.append("text")
                .attr("x", LEFT_X + 10)
                .attr("y", mid - 8)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "13px")
                .style("font-weight", "700")
                .text(row.label);

            leftNodes.append("text")
                .attr("x", LEFT_X + 10)
                .attr("y", mid + 10)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11.5px")
                .style("font-weight", "600")
                .text(valueLabel);
        });

        const rightNodes = svg.append("g");
        speciesRows.forEach((row, index) => {
            const barTop = speciesTop[index];
            const barHeight = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * rightScale,
            );
            const labelTop = speciesLabelTop[index];
            const labelMid = labelTop + RIGHT_LABEL_HEIGHT / 2;
            const barMid = barTop + barHeight / 2;
            const valueLabel = showSilverAmounts
                ? `${row.expected_count_text} | ${compactSilverText(row.expected_profit_text)}`
                : row.expected_count_text;
            const rateLabel = String(row.rate_text ?? "");
            const rateTooltip = String(row.rate_tooltip ?? "");
            const dotColor = provenanceDotColor(String(row.rate_source_kind ?? ""));
            const hasIcon = Boolean(row.icon_url);

            rightNodes.append("rect")
                .attr("x", countBarX)
                .attr("y", barTop)
                .attr("width", RIGHT_BAR_WIDTH)
                .attr("height", barHeight)
                .attr("rx", Math.min(NODE_RADIUS, RIGHT_BAR_WIDTH / 2))
                .attr("ry", Math.min(NODE_RADIUS, RIGHT_BAR_WIDTH / 2))
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.25);

            rightNodes.append("path")
                .attr(
                    "d",
                    [
                        `M ${countBarX + RIGHT_BAR_WIDTH} ${barMid}`,
                        `C ${countBarX + RIGHT_BAR_WIDTH + 16} ${barMid}, ${labelX - 16} ${labelMid}, ${labelX} ${labelMid}`,
                    ].join(" "),
                )
                .style("fill", "none")
                .style("stroke", row.stroke_color)
                .style("stroke-opacity", 0.75)
                .style("stroke-width", 1.5);

            const card = rightNodes.append("foreignObject")
                .attr("x", labelX)
                .attr("y", labelTop)
                .attr("width", RIGHT_LABEL_WIDTH)
                .attr("height", RIGHT_LABEL_HEIGHT);

            const cardRoot = card.append("xhtml:div")
                .attr("class", "loot-sankey-card")
                .style("--loot-card-fill", row.fill_color)
                .style("--loot-card-stroke", row.stroke_color)
                .style("--loot-card-text", row.text_color)
                .style("--loot-card-dot", dotColor);
            const infoDot = cardRoot.append("xhtml:span")
                .attr("class", "loot-sankey-card__info-dot")
                .attr("aria-hidden", "true");
            if (rateTooltip) {
                infoDot.attr("title", rateTooltip);
            }

            const cardBody = cardRoot.append("xhtml:div")
                .attr("class", "loot-sankey-card__body");

            if (hasIcon) {
                cardBody.append("xhtml:img")
                    .attr("class", "loot-sankey-card__icon")
                    .attr("src", row.icon_url)
                    .attr("alt", "")
                    .attr("aria-hidden", "true");
            }

            const textStack = cardBody.append("xhtml:div")
                .attr("class", "loot-sankey-card__text");

            textStack.append("xhtml:div")
                .attr("class", "loot-sankey-card__label")
                .text(String(row.label ?? ""));

            textStack.append("xhtml:div")
                .attr("class", "loot-sankey-card__value")
                .text(valueLabel);

            const rateColumn = cardBody.append("xhtml:div")
                .attr("class", "loot-sankey-card__rate-column");

            rateColumn.append("xhtml:div")
                .attr("class", "loot-sankey-card__rate")
                .text(rateLabel);
        });

        const silverNodes = svg.append("g");
        speciesRows.forEach((row, index) => {
            const profitRaw = positiveNumber(row.expected_profit_raw);
            const top = profitSpeciesTop[index];
            const heightValue = profitRaw * profitScale;
            if (heightValue <= 0) {
                return;
            }

            silverNodes.append("rect")
                .attr("x", silverBarX)
                .attr("y", top)
                .attr("width", RIGHT_BAR_WIDTH)
                .attr("height", heightValue)
                .attr("rx", Math.min(NODE_RADIUS, RIGHT_BAR_WIDTH / 2))
                .attr("ry", Math.min(NODE_RADIUS, RIGHT_BAR_WIDTH / 2))
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.25);
        });

        const profitGroups = svg.append("g");
        rows.forEach((row) => {
            const top = profitGroupTop.get(row.label) ?? TOP_PADDING;
            const heightValue = positiveNumber(row.expected_profit_raw) * profitScale;
            if (heightValue <= 0) {
                return;
            }
            const mid = top + heightValue / 2;

            profitGroups.append("rect")
                .attr("x", silverGroupX)
                .attr("y", top)
                .attr("width", SILVER_GROUP_WIDTH)
                .attr("height", heightValue)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            profitGroups.append("text")
                .attr("x", silverGroupX + 10)
                .attr("y", mid - 8)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "13px")
                .style("font-weight", "700")
                .text(row.label);

            if (showSilverAmounts) {
                profitGroups.append("text")
                    .attr("x", silverGroupX + 10)
                    .attr("y", mid + 10)
                    .attr("dominant-baseline", "middle")
                    .style("fill", row.text_color)
                    .style("font-size", "11.5px")
                    .style("font-weight", "600")
                    .text(compactSilverText(row.expected_profit_text));
            }
        });

        this.replaceChildren(svg.node());
    }
}

export function registerLootSankey() {
    if (window.customElements.get("fishy-loot-sankey")) {
        return;
    }
    window.customElements.define("fishy-loot-sankey", FishyLootSankey);
}
