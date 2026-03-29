import * as d3 from "../d3.js";

const MIN_INTERNAL_WIDTH = 980;
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
const NODE_RADIUS = 12;
const LABEL_ICON_SIZE = 20;

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

function truncateLabel(label, maxChars) {
    const chars = Array.from(String(label ?? ""));
    if (chars.length <= maxChars) {
        return chars.join("");
    }
    return chars.slice(0, Math.max(0, maxChars - 1)).join("") + "…";
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
        const labelStackHeight = speciesRows.length
            ? speciesRows.length * RIGHT_LABEL_HEIGHT
                + Math.max(0, speciesRows.length - 1) * SPECIES_LABEL_GAP
            : 0;
        const innerHeight = Math.max(labelStackHeight, 340);
        const width = Math.max(this.clientWidth || 0, MIN_INTERNAL_WIDTH);
        const labelX = width - RIGHT_LABEL_WIDTH - 24;
        const rightBarX = labelX - RIGHT_LABEL_OFFSET - RIGHT_BAR_WIDTH;
        const leftScale = Math.max(
            0,
            (innerHeight - GROUP_GAP * Math.max(0, rows.length - 1)) / totalCount,
        );
        const rightScale = Math.max(
            0,
            (innerHeight - SPECIES_GAP * Math.max(0, speciesRows.length - 1)) / totalCount,
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

        const speciesLabelTop = [];
        let labelCursor = TOP_PADDING;
        speciesRows.forEach(() => {
            speciesLabelTop.push(labelCursor);
            labelCursor += RIGHT_LABEL_HEIGHT + SPECIES_LABEL_GAP;
        });

        const leftFlowCursor = new Map(groupTop);

        const svg = d3
            .create("svg")
            .attr("viewBox", `0 0 ${width} ${height}`)
            .attr("role", "img")
            .attr(
                "aria-label",
                this.getAttribute("aria-label") || "Expected loot flows from groups to species",
            );

        const flows = svg.append("g");
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

            flows.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        LEFT_X + LEFT_WIDTH,
                        leftTop,
                        rightBarX,
                        rightTop,
                        leftHeight,
                        rightHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.42);

            leftFlowCursor.set(row.group_label, leftTop + leftHeight);
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
                ? `${row.expected_count_text} | ${row.expected_profit_text}`
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
                .attr("y", mid - 6)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11px")
                .style("font-weight", "700")
                .text(row.label);

            leftNodes.append("text")
                .attr("x", LEFT_X + 10)
                .attr("y", mid + 8)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "10px")
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
                ? `${row.expected_count_text} | ${row.expected_profit_text}`
                : row.expected_count_text;
            const evidenceLabel = String(row.evidence_text ?? "");
            const hasIcon = Boolean(row.icon_url);
            const textX = labelX + 10 + (hasIcon ? LABEL_ICON_SIZE + 8 : 0);

            rightNodes.append("rect")
                .attr("x", rightBarX)
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
                        `M ${rightBarX + RIGHT_BAR_WIDTH} ${barMid}`,
                        `C ${rightBarX + RIGHT_BAR_WIDTH + 16} ${barMid}, ${labelX - 16} ${labelMid}, ${labelX} ${labelMid}`,
                    ].join(" "),
                )
                .style("fill", "none")
                .style("stroke", row.stroke_color)
                .style("stroke-opacity", 0.75)
                .style("stroke-width", 1.5);

            rightNodes.append("rect")
                .attr("x", labelX)
                .attr("y", labelTop)
                .attr("width", RIGHT_LABEL_WIDTH)
                .attr("height", RIGHT_LABEL_HEIGHT)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            if (hasIcon) {
                rightNodes.append("image")
                    .attr("x", labelX + 10)
                    .attr("y", labelTop + (RIGHT_LABEL_HEIGHT - LABEL_ICON_SIZE) / 2)
                    .attr("width", LABEL_ICON_SIZE)
                    .attr("height", LABEL_ICON_SIZE)
                    .attr("href", row.icon_url)
                    .attr("preserveAspectRatio", "xMidYMid meet");
            }

            rightNodes.append("text")
                .attr("x", textX)
                .attr("y", labelTop + 15)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11px")
                .style("font-weight", "700")
                .text(truncateLabel(row.label, hasIcon ? 24 : 28));

            rightNodes.append("text")
                .attr("x", textX)
                .attr("y", labelTop + 29)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "10px")
                .text(valueLabel);

            rightNodes.append("text")
                .attr("x", textX)
                .attr("y", labelTop + 43)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "9px")
                .style("opacity", 0.88)
                .text(truncateLabel(evidenceLabel, 44));
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
