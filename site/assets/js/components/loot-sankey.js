import * as d3 from "../d3.js";
import {
    attachProvenanceTooltip,
    buildProvenanceSegments,
    provenanceAriaLabel,
} from "./provenance-indicator.js";
import {
    FishyDatastarRenderElement,
    readCalculatorSignal,
} from "./datastar-render-element.js";

const TOP_PADDING = 20;
const BOTTOM_PADDING = 20;
const GROUP_GAP = 12;
const SPECIES_GAP = 6;
const SPECIES_LABEL_GAP = 8;
const LEFT_X = 8;
const LEFT_WIDTH = 200;
const RIGHT_BAR_WIDTH = 18;
const RIGHT_LABEL_WIDTH = 380;
const RIGHT_LABEL_HEIGHT = 58;
const RIGHT_LABEL_OFFSET = 14;
const SPECIES_METRIC_WIDTH = 72;
const SPECIES_BOX_CONNECTOR_GAP = 8;
const SPECIES_BOX_CONNECTOR_INSET = 8;
const GROUP_TO_SPECIES_GAP = 24;
const SPECIES_TO_SILVER_GAP = 78;
const SILVER_TO_GROUP_GAP = 24;
const SILVER_GROUP_WIDTH = 212;
const RIGHT_MARGIN = 8;
const NODE_RADIUS = 12;
const MIN_SILVER_NODE_HEIGHT = 1.5;
const MIN_INTERNAL_WIDTH =
    LEFT_X
    + LEFT_WIDTH
    + GROUP_TO_SPECIES_GAP
    + RIGHT_BAR_WIDTH
    + RIGHT_LABEL_OFFSET
    + RIGHT_LABEL_WIDTH
    + SPECIES_TO_SILVER_GAP
    + RIGHT_BAR_WIDTH
    + SILVER_TO_GROUP_GAP
    + SILVER_GROUP_WIDTH
    + RIGHT_MARGIN;
const PROVENANCE_RAIL_WIDTH = 7;
const PROVENANCE_RAIL_INSET = 8;
const PROVENANCE_RAIL_GAP = 1.5;
const GROUP_PROVENANCE_RAIL_MAX_HEIGHT = 38;

function gradeRingColor(tone) {
    switch (String(tone || "").trim().toLowerCase()) {
        case "red":
        case "prize":
            return "color-mix(in oklab, var(--color-error) 76%, var(--color-base-content) 24%)";
        case "yellow":
            return "color-mix(in oklab, var(--color-warning) 76%, var(--color-base-content) 24%)";
        case "blue":
            return "color-mix(in oklab, var(--color-info) 76%, var(--color-base-content) 24%)";
        case "green":
            return "color-mix(in oklab, var(--color-success) 76%, var(--color-base-content) 24%)";
        case "white":
            return "var(--color-base-content)";
        default:
            return "color-mix(in oklab, var(--color-neutral) 72%, var(--color-base-content) 28%)";
    }
}

function gradeLabelColor(tone) {
    return `color-mix(in oklab, ${gradeRingColor(tone)} 84%, var(--color-base-content) 16%)`;
}

function gradeSurfaceFill(tone) {
    return `color-mix(in oklab, var(--color-base-100) 90%, ${gradeRingColor(tone)} 10%)`;
}

function gradeSurfaceStroke(tone) {
    return `color-mix(in oklab, ${gradeRingColor(tone)} 62%, var(--color-base-300) 38%)`;
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

function distributedDisplayHeights(values, totalSpan, gap, minimumHeight) {
    const count = values.length;
    if (!count) {
        return [];
    }

    const usableSpan = Math.max(0, totalSpan - gap * Math.max(0, count - 1));
    if (usableSpan <= 0) {
        return Array.from({ length: count }, () => 0);
    }

    const positiveValues = values.map((value) => positiveNumber(value));
    const totalValue = d3.sum(positiveValues);
    if (totalValue <= 0) {
        return Array.from({ length: count }, () => usableSpan / count);
    }

    const baseHeights = positiveValues.map((value) => (usableSpan * value) / totalValue);
    const boostedHeights = baseHeights.map((height) => Math.max(minimumHeight, height));
    const boostedTotal = d3.sum(boostedHeights);

    if (boostedTotal <= usableSpan) {
        const extra = usableSpan - boostedTotal;
        return boostedHeights.map((height, index) =>
            height + (positiveValues[index] / totalValue) * extra,
        );
    }

    const floorTotal = minimumHeight * count;
    if (floorTotal >= usableSpan) {
        return Array.from({ length: count }, () => usableSpan / count);
    }

    const scalableBudget = usableSpan - floorTotal;
    const scalableValues = positiveValues.map((value, index) =>
        baseHeights[index] > minimumHeight ? value : 0,
    );
    const scalableTotal = d3.sum(scalableValues);

    if (scalableTotal <= 0) {
        return Array.from({ length: count }, () => usableSpan / count);
    }

    return positiveValues.map((_, index) => {
        const scaledShare = scalableValues[index] / scalableTotal;
        return minimumHeight + scaledShare * scalableBudget;
    });
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

function truncateText(value, maxChars) {
    const text = String(value ?? "");
    if (text.length <= maxChars) {
        return text;
    }
    if (maxChars <= 1) {
        return "…";
    }
    return `${text.slice(0, maxChars - 1)}…`;
}

function applyStatBreakdownAttrs(selection, rawBreakdown, color) {
    const breakdown = String(rawBreakdown ?? "").trim();
    if (!breakdown) {
        selection
            .attr("data-fishy-stat-breakdown", null)
            .attr("data-fishy-stat-color", null)
            .attr("tabindex", null)
            .style("cursor", null);
        return selection;
    }
    selection
        .attr("data-fishy-stat-breakdown", breakdown)
        .attr("data-fishy-stat-color", color)
        .attr("tabindex", 0)
        .style("cursor", "help");
    return selection;
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

class FishyLootSankey extends FishyDatastarRenderElement {
    static get observedAttributes() {
        return ["signal-path", "aria-label"];
    }

    observeChildren() {
        return true;
    }

    observeResize() {
        return true;
    }

    renderFromSignals() {
        attachProvenanceTooltip(this);
        const chart = readCalculatorSignal(this.getAttribute("signal-path"));
        const rows = Array.isArray(chart?.rows) ? chart.rows : [];
        const speciesRows = Array.isArray(chart?.species_rows) ? chart.species_rows : [];
        if (!rows.length || !speciesRows.length) {
            this.replaceRenderedChildren();
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
        const width = Math.max(this.clientWidth || 0, MIN_INTERNAL_WIDTH);
        const countBarX = LEFT_X + LEFT_WIDTH + GROUP_TO_SPECIES_GAP;
        const silverGroupX = width - RIGHT_MARGIN - SILVER_GROUP_WIDTH;
        const silverBarX = silverGroupX - SILVER_TO_GROUP_GAP - RIGHT_BAR_WIDTH;
        const leftSpeciesX = countBarX + RIGHT_BAR_WIDTH + RIGHT_LABEL_OFFSET;
        const rightSpeciesX = silverBarX - SPECIES_TO_SILVER_GAP - RIGHT_LABEL_WIDTH;
        const labelX = (leftSpeciesX + rightSpeciesX) / 2;
        const labelWidth = RIGHT_LABEL_WIDTH;
        const speciesCenterWidth =
            labelWidth - SPECIES_METRIC_WIDTH * 2 - SPECIES_BOX_CONNECTOR_GAP * 2;
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

        const profitSpeciesHeights = distributedDisplayHeights(
            speciesRows.map((row) => row.expected_profit_raw),
            innerHeight,
            SPECIES_GAP,
            MIN_SILVER_NODE_HEIGHT,
        );
        const profitSpeciesTop = [];
        let profitSpeciesCursor = TOP_PADDING;
        profitSpeciesHeights.forEach((heightValue) => {
            profitSpeciesTop.push(profitSpeciesCursor);
            profitSpeciesCursor += heightValue + SPECIES_GAP;
        });

        const profitGroupHeightsList = distributedDisplayHeights(
            rows.map((row) => row.expected_profit_raw),
            innerHeight,
            GROUP_GAP,
            MIN_SILVER_NODE_HEIGHT,
        );
        const profitGroupTop = new Map();
        const profitGroupHeights = new Map();
        let profitGroupCursor = TOP_PADDING;
        rows.forEach((row, index) => {
            const heightValue = profitGroupHeightsList[index] ?? 0;
            profitGroupTop.set(row.label, profitGroupCursor);
            profitGroupHeights.set(row.label, heightValue);
            profitGroupCursor += heightValue + GROUP_GAP;
        });

        const speciesLabelTop = [];
        let labelCursor = TOP_PADDING;
        speciesRows.forEach(() => {
            speciesLabelTop.push(labelCursor);
            labelCursor += RIGHT_LABEL_HEIGHT + SPECIES_LABEL_GAP;
        });

        const leftFlowCursor = new Map(groupTop);
        const groupedProfitSpeciesHeights = d3.rollup(
            speciesRows.map((row, index) => ({
                groupLabel: row.group_label,
                heightValue: profitSpeciesHeights[index] ?? 0,
            })),
            (entries) => d3.sum(entries, (entry) => entry.heightValue),
            (entry) => entry.groupLabel,
        );
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
            const profitHeight = Math.max(
                MIN_SILVER_NODE_HEIGHT,
                profitSpeciesHeights[index] ?? 0,
            );
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
            const groupHeight = profitGroupHeights.get(row.group_label) ?? profitHeight;
            const groupedHeightTotal =
                groupedProfitSpeciesHeights.get(row.group_label) ?? profitHeight;
            const groupSliceHeight = groupedHeightTotal > 0
                ? (profitHeight / groupedHeightTotal) * groupHeight
                : profitHeight;
            silverFlows.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        silverBarX + RIGHT_BAR_WIDTH,
                        speciesProfitTop,
                        silverGroupX,
                        groupProfitTop,
                        profitHeight,
                        groupSliceHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.44);

            profitFlowCursor.set(row.group_label, groupProfitTop + groupSliceHeight);
        });

        const leftNodes = svg.append("g");
        rows.forEach((row) => {
            const top = groupTop.get(row.label) ?? TOP_PADDING;
            const heightValue = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * leftScale,
            );
            const mid = top + heightValue / 2;
            const valueLabel = `${row.count_share_text} · ${row.expected_count_text}`;
            const provenanceSegments = buildProvenanceSegments({
                rateSourceKind: String(row.drop_rate_source_kind ?? ""),
                rateDetail: String(row.drop_rate_tooltip ?? ""),
                rateValueText: String(row.count_share_text ?? ""),
            });
            const provenanceRailX =
                LEFT_X + LEFT_WIDTH - PROVENANCE_RAIL_INSET - PROVENANCE_RAIL_WIDTH;
            const availableRailHeight = Math.max(0, heightValue - 8);
            const provenanceRailHeight = Math.min(
                GROUP_PROVENANCE_RAIL_MAX_HEIGHT,
                availableRailHeight,
            );
            const provenanceRailY = top + Math.max(0, (heightValue - provenanceRailHeight) / 2);
            const provenanceSegmentHeight = provenanceSegments.length
                ? Math.max(
                    0,
                    provenanceRailHeight - PROVENANCE_RAIL_GAP * (provenanceSegments.length - 1),
                ) / provenanceSegments.length
                : 0;

            const groupNode = applyStatBreakdownAttrs(
                leftNodes.append("g"),
                row.count_breakdown,
                "var(--color-info)",
            );

            groupNode.append("rect")
                .attr("x", LEFT_X)
                .attr("y", top)
                .attr("width", LEFT_WIDTH)
                .attr("height", heightValue)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            groupNode.append("text")
                .attr("x", LEFT_X + 10)
                .attr("y", mid - 8)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "13px")
                .style("font-weight", "700")
                .text(row.label);

            groupNode.append("text")
                .attr("x", LEFT_X + 10)
                .attr("y", mid + 10)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11.5px")
                .style("font-weight", "600")
                .text(valueLabel);

            if (provenanceSegmentHeight > 0.5) {
                const provenanceRail = leftNodes.append("g")
                    .attr("aria-label", "Fact provenance");
                provenanceSegments.forEach((segment, segmentIndex) => {
                    const segmentY = provenanceRailY
                        + segmentIndex * (provenanceSegmentHeight + PROVENANCE_RAIL_GAP);
                    provenanceRail.append("rect")
                        .attr("x", provenanceRailX)
                        .attr("y", segmentY)
                        .attr("width", PROVENANCE_RAIL_WIDTH)
                        .attr("height", provenanceSegmentHeight)
                        .attr("rx", Math.min(PROVENANCE_RAIL_WIDTH / 2, 3))
                        .attr("ry", Math.min(PROVENANCE_RAIL_WIDTH / 2, 3))
                        .attr("tabindex", 0)
                        .attr("aria-label", provenanceAriaLabel(segment))
                        .attr("data-fishy-provenance-label", segment.label)
                        .attr("data-fishy-provenance-source", segment.sourceLabel)
                        .attr("data-fishy-provenance-detail", segment.detail)
                        .attr("data-fishy-provenance-color", segment.color)
                        .style("fill", segment.color)
                        .style("opacity", segment.active ? 1 : 0.65);
                });
            }
        });

        const speciesConnectors = svg.append("g");
        const rightNodes = svg.append("g");
        speciesRows.forEach((row, index) => {
            const barTop = speciesTop[index];
            const barHeight = Math.max(
                1.5,
                positiveNumber(row.expected_count_raw) * rightScale,
            );
            const labelTop = speciesLabelTop[index];
            const connectorTop = labelTop + SPECIES_BOX_CONNECTOR_INSET;
            const connectorHeight = RIGHT_LABEL_HEIGHT - SPECIES_BOX_CONNECTOR_INSET * 2;
            const dropMetricText = String(row.drop_rate_text ?? "");
            const dropValueText = String(row.expected_count_text ?? "");
            const silverMetricText = String(row.silver_share_text ?? "");
            const silverValueText = compactSilverText(row.expected_profit_text);
            const iconRing = gradeRingColor(row.icon_grade_tone);
            const itemLabelColor = gradeLabelColor(row.icon_grade_tone);
            const itemSurfaceFill = gradeSurfaceFill(row.icon_grade_tone);
            const itemSurfaceStroke = gradeSurfaceStroke(row.icon_grade_tone);
            const hasIcon = Boolean(row.icon_url);
            const leftBoxX = labelX;
            const centerBoxX = leftBoxX + SPECIES_METRIC_WIDTH + SPECIES_BOX_CONNECTOR_GAP;
            const rightBoxX =
                centerBoxX + speciesCenterWidth + SPECIES_BOX_CONNECTOR_GAP;
            const leftBoxMid = labelTop + RIGHT_LABEL_HEIGHT / 2;
            const rightBoxMid = leftBoxMid;
            const iconFrameSize = 28;
            const iconFrameX = centerBoxX + 12;
            const iconFrameY = labelTop + (RIGHT_LABEL_HEIGHT - iconFrameSize) / 2;
            const labelTextX = hasIcon ? iconFrameX + iconFrameSize + 10 : centerBoxX + 12;
            const labelTextMaxChars = hasIcon ? 16 : 22;
            const speciesProfitTop = profitSpeciesTop[index];
            const profitHeight = Math.max(
                MIN_SILVER_NODE_HEIGHT,
                profitSpeciesHeights[index] ?? 0,
            );
            const provenanceSegments = buildProvenanceSegments({
                rateSourceKind: String(row.drop_rate_source_kind ?? ""),
                rateDetail: String(row.drop_rate_tooltip ?? ""),
                rateValueText: dropMetricText,
                presenceSourceKind: String(row.presence_source_kind ?? ""),
                presenceDetail: String(row.presence_tooltip ?? ""),
                presenceValueText: String(row.presence_text ?? ""),
            });
            const provenanceRailX =
                centerBoxX + speciesCenterWidth - PROVENANCE_RAIL_INSET - PROVENANCE_RAIL_WIDTH;
            const provenanceRailY = labelTop + 4;
            const provenanceRailHeight = RIGHT_LABEL_HEIGHT - 8;
            const provenanceSegmentHeight =
                Math.max(0, provenanceRailHeight - PROVENANCE_RAIL_GAP) / provenanceSegments.length;

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

            speciesConnectors.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        countBarX + RIGHT_BAR_WIDTH,
                        barTop,
                        leftBoxX,
                        connectorTop,
                        barHeight,
                        connectorHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.38);

            speciesConnectors.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        leftBoxX + SPECIES_METRIC_WIDTH,
                        connectorTop,
                        centerBoxX,
                        connectorTop,
                        connectorHeight,
                        connectorHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.34);

            speciesConnectors.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        centerBoxX + speciesCenterWidth,
                        connectorTop,
                        rightBoxX,
                        connectorTop,
                        connectorHeight,
                        connectorHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.34);

            speciesConnectors.append("path")
                .attr(
                    "d",
                    sankeyPath(
                        rightBoxX + SPECIES_METRIC_WIDTH,
                        connectorTop,
                        silverBarX,
                        speciesProfitTop,
                        connectorHeight,
                        profitHeight,
                    ),
                )
                .style("fill", row.connector_color)
                .style("opacity", 0.34);

            const countMetric = applyStatBreakdownAttrs(
                rightNodes.append("g"),
                row.count_breakdown,
                "var(--color-info)",
            );

            countMetric.append("rect")
                .attr("x", leftBoxX)
                .attr("y", labelTop)
                .attr("width", SPECIES_METRIC_WIDTH)
                .attr("height", RIGHT_LABEL_HEIGHT)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            countMetric.append("text")
                .attr("x", leftBoxX + SPECIES_METRIC_WIDTH / 2)
                .attr("y", leftBoxMid - 6)
                .attr("text-anchor", "middle")
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "13px")
                .style("font-weight", "800")
                .style("font-variant-numeric", "tabular-nums")
                .text(dropMetricText);

            countMetric.append("text")
                .attr("x", leftBoxX + SPECIES_METRIC_WIDTH / 2)
                .attr("y", leftBoxMid + 10)
                .attr("text-anchor", "middle")
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11px")
                .style("font-weight", "700")
                .style("font-variant-numeric", "tabular-nums")
                .text(dropValueText);

            rightNodes.append("rect")
                .attr("x", centerBoxX)
                .attr("y", labelTop)
                .attr("width", speciesCenterWidth)
                .attr("height", RIGHT_LABEL_HEIGHT)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", itemSurfaceFill)
                .style("stroke", itemSurfaceStroke)
                .style("stroke-width", 1.5)
                .append("title")
                .text(String(row.label ?? ""));

            if (hasIcon) {
                const iconFrameRadius = Math.round(iconFrameSize * 0.34);
                rightNodes.append("rect")
                    .attr("x", iconFrameX)
                    .attr("y", iconFrameY)
                    .attr("width", iconFrameSize)
                    .attr("height", iconFrameSize)
                    .attr("rx", iconFrameRadius)
                    .attr("ry", iconFrameRadius)
                    .style("fill", "color-mix(in oklab, var(--color-base-100) 94%, transparent)")
                    .style("stroke", iconRing)
                    .style("stroke-width", 2.5);

                rightNodes.append("image")
                    .attr("x", iconFrameX + 5)
                    .attr("y", iconFrameY + 5)
                    .attr("width", iconFrameSize - 10)
                    .attr("height", iconFrameSize - 10)
                    .attr("href", row.icon_url)
                    .attr("preserveAspectRatio", "xMidYMid meet");
            }

            rightNodes.append("text")
                .attr("x", labelTextX)
                .attr("y", labelTop + RIGHT_LABEL_HEIGHT / 2 + 1)
                .attr("dominant-baseline", "middle")
                .attr("text-anchor", "start")
                .style("fill", itemLabelColor)
                .style("font-size", "13px")
                .style("font-weight", "800")
                .text(truncateText(row.label, labelTextMaxChars))
                .append("title")
                .text(String(row.label ?? ""));

            const provenanceRail = rightNodes.append("g")
                .attr("aria-label", "Fact provenance");
            provenanceSegments.forEach((segment, segmentIndex) => {
                const segmentY = provenanceRailY
                    + segmentIndex * (provenanceSegmentHeight + PROVENANCE_RAIL_GAP);
                provenanceRail.append("rect")
                    .attr("x", provenanceRailX)
                    .attr("y", segmentY)
                    .attr("width", PROVENANCE_RAIL_WIDTH)
                    .attr("height", provenanceSegmentHeight)
                    .attr("rx", Math.min(PROVENANCE_RAIL_WIDTH / 2, 3))
                    .attr("ry", Math.min(PROVENANCE_RAIL_WIDTH / 2, 3))
                    .attr("tabindex", 0)
                    .attr("aria-label", provenanceAriaLabel(segment))
                    .attr("data-fishy-provenance-label", segment.label)
                    .attr("data-fishy-provenance-source", segment.sourceLabel)
                    .attr("data-fishy-provenance-detail", segment.detail)
                    .attr("data-fishy-provenance-color", segment.color)
                    .style("fill", segment.color)
                    .style("opacity", segment.active ? 1 : 0.65);
            });

            const silverMetric = applyStatBreakdownAttrs(
                rightNodes.append("g"),
                row.silver_breakdown,
                "var(--color-success)",
            );

            silverMetric.append("rect")
                .attr("x", rightBoxX)
                .attr("y", labelTop)
                .attr("width", SPECIES_METRIC_WIDTH)
                .attr("height", RIGHT_LABEL_HEIGHT)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            silverMetric.append("text")
                .attr("x", rightBoxX + SPECIES_METRIC_WIDTH / 2)
                .attr("y", rightBoxMid - 6)
                .attr("text-anchor", "middle")
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "12.5px")
                .style("font-weight", "800")
                .style("font-variant-numeric", "tabular-nums")
                .text(silverMetricText);

            silverMetric.append("text")
                .attr("x", rightBoxX + SPECIES_METRIC_WIDTH / 2)
                .attr("y", rightBoxMid + 10)
                .attr("text-anchor", "middle")
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11px")
                .style("font-weight", "700")
                .style("font-variant-numeric", "tabular-nums")
                .text(silverValueText);
        });

        const silverNodes = svg.append("g");
        speciesRows.forEach((row, index) => {
            const top = profitSpeciesTop[index];
            const heightValue = Math.max(
                MIN_SILVER_NODE_HEIGHT,
                profitSpeciesHeights[index] ?? 0,
            );

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
            const heightValue = Math.max(
                MIN_SILVER_NODE_HEIGHT,
                profitGroupHeights.get(row.label) ?? 0,
            );
            const mid = top + heightValue / 2;
            const valueLabel = `${row.silver_share_text} · ${compactSilverText(row.expected_profit_text)}`;

            const groupNode = applyStatBreakdownAttrs(
                profitGroups.append("g"),
                row.silver_breakdown,
                "var(--color-success)",
            );

            groupNode.append("rect")
                .attr("x", silverGroupX)
                .attr("y", top)
                .attr("width", SILVER_GROUP_WIDTH)
                .attr("height", heightValue)
                .attr("rx", NODE_RADIUS)
                .attr("ry", NODE_RADIUS)
                .style("fill", row.fill_color)
                .style("stroke", row.stroke_color)
                .style("stroke-width", 1.5);

            groupNode.append("text")
                .attr("x", silverGroupX + 10)
                .attr("y", mid - 8)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "13px")
                .style("font-weight", "700")
                .text(row.label);

            groupNode.append("text")
                .attr("x", silverGroupX + 10)
                .attr("y", mid + 10)
                .attr("dominant-baseline", "middle")
                .style("fill", row.text_color)
                .style("font-size", "11.5px")
                .style("font-weight", "600")
                .text(valueLabel);
        });

        this.replaceRenderedChildren(svg.node());
    }
}

export function registerLootSankey() {
    if (window.customElements.get("fishy-loot-sankey")) {
        return;
    }
    window.customElements.define("fishy-loot-sankey", FishyLootSankey);
}
