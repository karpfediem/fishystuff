import * as d3 from "../d3.js";
import {
    FishyDatastarRenderElement,
    readCalculatorSignal,
} from "./datastar-render-element.js";

const DEFAULT_WIDTH = 980;
const MIN_BAR_WIDTH = 84;
const CHART_HEIGHT = 264;
const TOP_PADDING = 26;
const RIGHT_PADDING = 24;
const BOTTOM_PADDING = 56;
const LEFT_PADDING = 24;
const PLOT_TOP = 48;
const PLOT_BOTTOM = CHART_HEIGHT - BOTTOM_PADDING;
const GRID_TICKS = [25, 50, 75, 100];

function chartBars(path) {
    const chart = readCalculatorSignal(path);
    return Array.isArray(chart?.bars) ? chart.bars : [];
}

function expectedValueText(path) {
    const chart = readCalculatorSignal(path);
    return String(chart?.expected_value_text ?? "");
}

class FishyPmfChart extends FishyDatastarRenderElement {
    static get observedAttributes() {
        return ["signal-path", "aria-label"];
    }

    observeChildren() {
        return true;
    }

    renderFromSignals() {
        const path = this.getAttribute("signal-path");
        const bars = chartBars(path);
        if (!bars.length) {
            this.replaceRenderedChildren();
            return;
        }

        const width = Math.max(
            DEFAULT_WIDTH,
            LEFT_PADDING + RIGHT_PADDING + bars.length * MIN_BAR_WIDTH,
        );
        const plotWidth = width - LEFT_PADDING - RIGHT_PADDING;
        const plotHeight = PLOT_BOTTOM - PLOT_TOP;
        const x = d3
            .scaleBand()
            .domain(bars.map((_, index) => String(index)))
            .range([LEFT_PADDING, LEFT_PADDING + plotWidth])
            .paddingInner(0.18)
            .paddingOuter(0.06);
        const maxProbability = Math.max(
            1,
            ...bars.map((bar) => Number(bar.probability_pct) || 0),
        );
        const yMax = maxProbability <= 10
            ? 10
            : Math.min(100, Math.ceil(maxProbability / 10) * 10);
        const y = d3
            .scaleLinear()
            .domain([0, yMax])
            .range([PLOT_BOTTOM, PLOT_TOP]);

        const styles = getComputedStyle(this);
        const axisColor =
            styles.getPropertyValue("--color-base-content").trim() || "#1f2937";
        const gridColor =
            styles.getPropertyValue("--color-base-300").trim() || "#d6d3d1";
        const baseFill =
            styles.getPropertyValue("--color-primary").trim() || "#60a5fa";
        const baseText =
            styles.getPropertyValue("--color-primary-content").trim() || "#0f172a";
        const highlightFill =
            styles.getPropertyValue("--color-secondary").trim() || "#f59e0b";
        const highlightText =
            styles.getPropertyValue("--color-secondary-content").trim() || "#111827";

        const svg = d3
            .create("svg")
            .attr("viewBox", `0 0 ${width} ${CHART_HEIGHT}`)
            .attr("preserveAspectRatio", "xMidYMin meet")
            .attr("role", "img")
            .attr("aria-label", this.getAttribute("aria-label") || "Probability mass chart");

        svg.append("text")
            .attr("x", width - RIGHT_PADDING)
            .attr("y", TOP_PADDING)
            .attr("text-anchor", "end")
            .style("fill", axisColor)
            .style("font-size", "12px")
            .style("font-weight", "600")
            .style("opacity", 0.78)
            .text(`E[X] ${expectedValueText(path)}`);

        const grid = svg.append("g");
        GRID_TICKS.filter((tick) => tick <= yMax).forEach((tick) => {
            grid.append("line")
                .attr("x1", LEFT_PADDING)
                .attr("x2", width - RIGHT_PADDING)
                .attr("y1", y(tick))
                .attr("y2", y(tick))
                .style("stroke", gridColor)
                .style("stroke-opacity", 0.5)
                .style("stroke-width", 1);
            grid.append("text")
                .attr("x", LEFT_PADDING - 6)
                .attr("y", y(tick) + 4)
                .attr("text-anchor", "end")
                .style("fill", axisColor)
                .style("font-size", "10px")
                .style("opacity", 0.6)
                .text(`${tick}%`);
        });

        svg.append("line")
            .attr("x1", LEFT_PADDING)
            .attr("x2", width - RIGHT_PADDING)
            .attr("y1", PLOT_BOTTOM)
            .attr("y2", PLOT_BOTTOM)
            .style("stroke", axisColor)
            .style("stroke-opacity", 0.2)
            .style("stroke-width", 1.2);

        const plot = svg.append("g");
        bars.forEach((bar, index) => {
            const band = x(String(index));
            if (band == null) {
                return;
            }
            const value = Math.max(0, Number(bar.probability_pct) || 0);
            const barTop = y(value);
            const barHeight = Math.max(0, PLOT_BOTTOM - barTop);
            const fill = bar.highlight ? highlightFill : baseFill;
            const textColor = bar.highlight ? highlightText : baseText;

            plot.append("rect")
                .attr("x", band)
                .attr("y", barTop)
                .attr("width", x.bandwidth())
                .attr("height", barHeight)
                .attr("rx", 12)
                .attr("ry", 12)
                .style("fill", fill)
                .style("opacity", 0.9);

            plot.append("text")
                .attr("x", band + x.bandwidth() / 2)
                .attr("y", Math.max(TOP_PADDING + 18, barTop - 8))
                .attr("text-anchor", "middle")
                .style("fill", textColor)
                .style("font-size", "12px")
                .style("font-weight", "700")
                .text(String(bar.value_text ?? ""));

            plot.append("text")
                .attr("x", band + x.bandwidth() / 2)
                .attr("y", PLOT_BOTTOM + 18)
                .attr("text-anchor", "middle")
                .style("fill", axisColor)
                .style("font-size", "11px")
                .style("font-weight", "700")
                .text(String(bar.label ?? ""));
        });

        this.replaceRenderedChildren(svg.node());
    }
}

export function registerPmfChart() {
    if (window.customElements.get("fishy-pmf-chart")) {
        return;
    }
    window.customElements.define("fishy-pmf-chart", FishyPmfChart);
}
