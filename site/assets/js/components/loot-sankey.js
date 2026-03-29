import * as d3 from "../d3.js";

const MIN_INTERNAL_WIDTH = 980;
const TOP_PADDING = 20;
const BOTTOM_PADDING = 20;
const GROUP_GAP = 12;
const SPECIES_GAP = 6;
const LEFT_X = 24;
const LEFT_WIDTH = 200;
const RIGHT_WIDTH = 220;
const NODE_RADIUS = 12;

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
            d3.sum(speciesRows, (row) => Number(row.expected_count_raw) || 0),
        );
        const innerHeight = Math.max(speciesRows.length * 28, 340);
        const width = Math.max(this.clientWidth || 0, MIN_INTERNAL_WIDTH);
        const rightX = width - RIGHT_WIDTH - 24;
        const height = innerHeight + TOP_PADDING + BOTTOM_PADDING;
        const leftScale = Math.max(
            0,
            (innerHeight - GROUP_GAP * Math.max(0, rows.length - 1)) / totalCount,
        );
        const rightScale = Math.max(
            0,
            (innerHeight - SPECIES_GAP * Math.max(0, speciesRows.length - 1)) / totalCount,
        );

        const groupTop = new Map();
        let leftCursor = TOP_PADDING;
        rows.forEach((row) => {
            groupTop.set(row.label, leftCursor);
            leftCursor += (Number(row.expected_count_raw) || 0) * leftScale + GROUP_GAP;
        });

        const speciesTop = [];
        let rightCursor = TOP_PADDING;
        speciesRows.forEach((row) => {
            speciesTop.push(rightCursor);
            rightCursor += (Number(row.expected_count_raw) || 0) * rightScale + SPECIES_GAP;
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
                (Number(row.expected_count_raw) || 0) * leftScale,
            );
            const rightTop = speciesTop[index];
            const rightHeight = Math.max(
                1.5,
                (Number(row.expected_count_raw) || 0) * rightScale,
            );

            flows.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        LEFT_X + LEFT_WIDTH,
                        leftTop,
                        rightX,
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
                (Number(row.expected_count_raw) || 0) * leftScale,
            );
            const mid = top + heightValue / 2;
            const valueLabel = showSilverAmounts
                ? `${row.expected_count_text} | ${row.expected_profit_text}`
                : row.expected_count_text;

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
            const top = speciesTop[index];
            const heightValue = Math.max(
                1.5,
                (Number(row.expected_count_raw) || 0) * rightScale,
            );
            const mid = top + heightValue / 2;
            const valueLabel = showSilverAmounts
                ? `${row.expected_count_text} | ${row.expected_profit_text}`
                : row.expected_count_text;

            rightNodes.append("rect")
                .attr("x", rightX)
                .attr("y", top)
                .attr("width", RIGHT_WIDTH)
                .attr("height", heightValue)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            rightNodes.append("text")
                .attr("x", rightX + 10)
                .attr("y", mid - 6)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11px")
                .style("font-weight", "700")
                .text(truncateLabel(row.label, 28));

            rightNodes.append("text")
                .attr("x", rightX + 10)
                .attr("y", mid + 8)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "10px")
                .text(valueLabel);
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
