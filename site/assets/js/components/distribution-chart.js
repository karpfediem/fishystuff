import * as d3 from "../d3.js";
import {
    FishyDatastarRenderElement,
    readCalculatorSignal,
} from "./datastar-render-element.js";
import { attachStatBreakdownTooltip } from "./stat-breakdown-tooltip.js";

const DEFAULT_VIEWBOX_WIDTH = 1351;
const CHART_HEIGHT = 164;
const CALLOUT_TOP = 8;
const CALLOUT_HEIGHT = 60;
const CALLOUT_RADIUS = 16;
const TRACK_TOP = 132;
const TRACK_HEIGHT = 18;
const TRACK_RADIUS = TRACK_HEIGHT / 2;
const CALLOUT_GAP_PX = 10;

function chartSegments(path) {
    const chart = readCalculatorSignal(path);
    return Array.isArray(chart?.segments) ? chart.segments : [];
}

function serializeBreakdownPayload(breakdown) {
    if (!breakdown || typeof breakdown !== "object") {
        return "";
    }
    try {
        return JSON.stringify(breakdown);
    } catch {
        return "";
    }
}

function estimateCalloutWidthPx(label, valueText, detailText) {
    const longest = Math.max(
        String(label ?? "").length,
        String(valueText ?? "").length,
        String(detailText ?? "").length,
    );
    return Math.max(112, Math.min(248, 42 + longest * 8.6));
}

function neutralSpan(start, end, leftRadius, rightRadius) {
    const width = Math.max(0, end - start);
    const left = start + Math.min(leftRadius, width / 2);
    const right = end - Math.min(rightRadius, width / 2);
    if (left <= right) {
        return [left, right];
    }
    const center = start + width / 2;
    return [center, center];
}

function polygonPath(points) {
    return points
        .map((point, index) => `${index === 0 ? "M" : "L"} ${point[0]} ${point[1]}`)
        .join(" ")
        .concat(" Z");
}

class FishyDistributionChart extends FishyDatastarRenderElement {
    static get observedAttributes() {
        return ["signal-path", "aria-label", "viewbox-width"];
    }

    observeChildren() {
        return true;
    }

    renderFromSignals() {
        attachStatBreakdownTooltip(this);
        const segments = chartSegments(this.getAttribute("signal-path"));
        if (!segments.length) {
            this.replaceRenderedChildren();
            return;
        }

        const requestedWidth = Number(this.getAttribute("viewbox-width"));
        const width = Number.isFinite(requestedWidth) && requestedWidth > 0
            ? requestedWidth
            : DEFAULT_VIEWBOX_WIDTH;
        const trackWidth = width;
        const totalWidthPct = Math.max(
            100,
            segments.reduce(
                (sum, segment) => sum + Math.max(0, Number(segment.width_pct) || 0),
                0,
            ),
        );
        const x = d3.scaleLinear().domain([0, totalWidthPct]).range([0, trackWidth]);
        const styles = getComputedStyle(this);
        const trackBackground =
            styles.getPropertyValue("--color-base-300").trim() || "#d6d3d1";
        const trackBorder =
            styles.getPropertyValue("--color-base-content").trim() || "#1f2937";
        const clipId = `distribution-track-${crypto.randomUUID()}`;

        let startPct = 0;
        const provisional = segments.map((segment) => {
            const widthPct = Math.max(0, Number(segment.width_pct) || 0);
            const endPct = startPct + widthPct;
            const calloutWidthPx = Math.min(
                width - 8,
                estimateCalloutWidthPx(
                    segment.label,
                    segment.value_text,
                    segment.detail_text,
                ),
            );
            const startX = x(startPct);
            const endX = x(endPct);
            const segmentMidX = startX + (endX - startX) / 2;
            const preferredLeftPx = Math.max(
                0,
                Math.min(width - calloutWidthPx, segmentMidX - calloutWidthPx / 2),
            );
            const current = {
                segment,
                startPct,
                endPct,
                startX,
                endX,
                calloutWidthPx,
                preferredLeftPx,
                calloutLeftPx: preferredLeftPx,
            };
            startPct = endPct;
            return current;
        });

        for (let index = 1; index < provisional.length; index += 1) {
            const previous = provisional[index - 1];
            const current = provisional[index];
            const minimumLeft =
                previous.calloutLeftPx + previous.calloutWidthPx + CALLOUT_GAP_PX;
            current.calloutLeftPx = Math.max(current.preferredLeftPx, minimumLeft);
        }

        let nextLeft = width;
        for (let index = provisional.length - 1; index >= 0; index -= 1) {
            const current = provisional[index];
            const maximumLeft =
                nextLeft - current.calloutWidthPx - (index === provisional.length - 1 ? 0 : CALLOUT_GAP_PX);
            current.calloutLeftPx = Math.max(
                0,
                Math.min(current.calloutLeftPx, maximumLeft),
            );
            nextLeft = current.calloutLeftPx;
        }

        const layout = provisional.map((entry) => ({
            ...entry,
            calloutLeftPx: Math.max(
                0,
                Math.min(width - entry.calloutWidthPx, entry.calloutLeftPx),
            ),
        }));

        const svg = d3
            .create("svg")
            .attr("viewBox", `0 0 ${width} ${CHART_HEIGHT}`)
            .attr("preserveAspectRatio", "xMidYMin meet")
            .attr("role", "img")
            .attr("aria-label", this.getAttribute("aria-label") || "Distribution chart");

        const defs = svg.append("defs");
        defs.append("clipPath")
            .attr("id", clipId)
            .append("rect")
            .attr("x", 0)
            .attr("y", TRACK_TOP)
            .attr("width", trackWidth)
            .attr("height", TRACK_HEIGHT)
            .attr("rx", TRACK_RADIUS)
            .attr("ry", TRACK_RADIUS);

        const connectors = svg.append("g");
        const callouts = svg.append("g");
        const track = svg.append("g");

        layout.forEach((entry, index) => {
            const startX = entry.startX;
            const endX = entry.endX;
            const calloutX = entry.calloutLeftPx;
            const calloutWidth = entry.calloutWidthPx;
            const [segmentNeutralLeft, segmentNeutralRight] = neutralSpan(
                startX,
                endX,
                index === 0 ? TRACK_RADIUS : 0,
                index === layout.length - 1 ? TRACK_RADIUS : 0,
            );
            const [calloutNeutralLeft, calloutNeutralRight] = neutralSpan(
                calloutX,
                calloutX + calloutWidth,
                CALLOUT_RADIUS,
                CALLOUT_RADIUS,
            );

            connectors.append("path")
                .attr(
                    "d",
                    polygonPath([
                        [segmentNeutralLeft, TRACK_TOP],
                        [segmentNeutralRight, TRACK_TOP],
                        [calloutNeutralRight, CALLOUT_TOP + CALLOUT_HEIGHT],
                        [calloutNeutralLeft, CALLOUT_TOP + CALLOUT_HEIGHT],
                    ]),
                )
                .style("fill", entry.segment.connector_color);

            const callout = callouts.append("g");
            callout.append("rect")
                .attr("x", calloutX)
                .attr("y", CALLOUT_TOP)
                .attr("width", calloutWidth)
                .attr("height", CALLOUT_HEIGHT)
                .attr("rx", CALLOUT_RADIUS)
                .attr("ry", CALLOUT_RADIUS)
                .style("fill", entry.segment.fill_color)
                .style("stroke", entry.segment.stroke_color)
                .style("stroke-width", 1.5);

            callout.append("text")
                .attr("x", calloutX + calloutWidth / 2)
                .attr("y", CALLOUT_TOP + 18)
                .attr("text-anchor", "middle")
                .style("fill", entry.segment.text_color)
                .style("font-size", "11px")
                .style("font-weight", "600")
                .text(entry.segment.label);

            callout.append("text")
                .attr("x", calloutX + calloutWidth / 2)
                .attr("y", CALLOUT_TOP + 36)
                .attr("text-anchor", "middle")
                .style("fill", entry.segment.text_color)
                .style("font-size", "15px")
                .style("font-weight", "700")
                .text(entry.segment.value_text);

            callout.append("text")
                .attr("x", calloutX + calloutWidth / 2)
                .attr("y", CALLOUT_TOP + 51)
                .attr("text-anchor", "middle")
                .style("fill", entry.segment.text_color)
                .style("font-size", "11.5px")
                .style("font-weight", "600")
                .text(entry.segment.detail_text);

            const breakdownPayload = serializeBreakdownPayload(entry.segment.breakdown);
            if (breakdownPayload) {
                callout.append("rect")
                    .attr("x", calloutX)
                    .attr("y", CALLOUT_TOP)
                    .attr("width", calloutWidth)
                    .attr("height", CALLOUT_HEIGHT)
                    .attr("rx", CALLOUT_RADIUS)
                    .attr("ry", CALLOUT_RADIUS)
                    .attr("class", "distribution-chart__hotspot")
                    .attr("tabindex", 0)
                    .attr("focusable", true)
                    .attr(
                        "aria-label",
                        `${String(entry.segment.label ?? "Segment")} ${String(entry.segment.value_text ?? "")}. Show composition.`,
                    )
                    .attr("data-fishy-stat-breakdown", breakdownPayload)
                    .attr("data-fishy-stat-color", String(entry.segment.fill_color ?? ""))
                    .style("fill", "rgba(255, 255, 255, 0)")
                    .style("pointer-events", "all");
            }
        });

        track.append("rect")
            .attr("x", 0)
            .attr("y", TRACK_TOP)
            .attr("width", trackWidth)
            .attr("height", TRACK_HEIGHT)
            .attr("rx", TRACK_RADIUS)
            .attr("ry", TRACK_RADIUS)
            .style("fill", trackBackground);

        const segmentsGroup = track.append("g").attr("clip-path", `url(#${clipId})`);
        layout.forEach((entry) => {
            const startX = x(entry.startPct);
            const endX = x(entry.endPct);
            segmentsGroup.append("rect")
                .attr("x", startX)
                .attr("y", TRACK_TOP)
                .attr("width", Math.max(0, endX - startX))
                .attr("height", TRACK_HEIGHT)
                .style("fill", entry.segment.fill_color);
        });

        track.append("rect")
            .attr("x", 0)
            .attr("y", TRACK_TOP)
            .attr("width", trackWidth)
            .attr("height", TRACK_HEIGHT)
            .attr("rx", TRACK_RADIUS)
            .attr("ry", TRACK_RADIUS)
            .style("fill", "none")
            .style("stroke", trackBorder)
            .style("stroke-opacity", 0.1)
            .style("stroke-width", 1.2);

        this.replaceRenderedChildren(svg.node());
    }
}

export function registerDistributionChart() {
    if (window.customElements.get("fishy-distribution-chart")) {
        return;
    }
    window.customElements.define(
        "fishy-distribution-chart",
        FishyDistributionChart,
    );
}
