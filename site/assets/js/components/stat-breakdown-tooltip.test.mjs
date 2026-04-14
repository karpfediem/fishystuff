import test from "node:test";
import assert from "node:assert/strict";

import {
    STAT_BREAKDOWN_TOOLTIP_ATTRIBUTE_FILTER,
    statBreakdownFormulaTokenRows,
    statBreakdownTooltipPointerForAnchor,
    statBreakdownTooltipAnchorPoint,
    normalizeStatBreakdownPayload,
    statBreakdownFormulaTokens,
    statBreakdownPayloadForAnchor,
    statBreakdownResultKindLabel,
    statBreakdownSectionDisplayLabel,
    statBreakdownSectionRowGroups,
    statBreakdownTooltipRenderKey,
    statBreakdownTooltipShouldReactToMutations,
    statBreakdownTooltipShouldRefresh,
} from "./stat-breakdown-tooltip.js";

test("normalizeStatBreakdownPayload keeps titled sections and rows", () => {
    const payload = normalizeStatBreakdownPayload({
        kind_label: "Computed stat",
        title: "Rare group",
        value_text: "16.53%",
        summary_text: "Normalized share",
        formula_text: "Raw weight divided by all-group total",
        sections: [
            {
                label: "Inputs",
                rows: [
                    {
                        label: "Rare Float",
                        value_text: "+10%",
                        kind: "item",
                        icon_url: "https://cdn.example.test/items/rare-float.webp",
                        grade_tone: "yellow",
                    },
                ],
            },
        ],
    });

    assert.equal(payload.eyebrow, "Computed stat");
    assert.equal(payload.title, "Rare group");
    assert.equal(payload.sections.length, 1);
    assert.equal(payload.sections[0].rows[0].label, "Rare Float");
    assert.equal(payload.sections[0].rows[0].valueText, "+10%");
    assert.equal(payload.sections[0].rows[0].kind, "item");
    assert.equal(payload.sections[0].rows[0].iconUrl, "https://cdn.example.test/items/rare-float.webp");
    assert.equal(payload.sections[0].rows[0].gradeTone, "yellow");
});

test("normalizeStatBreakdownPayload drops empty sections and returns null for empty payloads", () => {
    assert.equal(normalizeStatBreakdownPayload({ sections: [{ rows: [] }] }), null);
});

test("statBreakdownPayloadForAnchor reparses when the bound payload changes", () => {
    const anchor = {
        dataset: {
            fishyStatBreakdown: JSON.stringify({
                title: "Average Total Fishing Time",
                value_text: "18.45",
                sections: [{ label: "Inputs", rows: [{ label: "Average bite time", value_text: "12.00" }] }],
            }),
        },
    };

    const first = statBreakdownPayloadForAnchor(anchor);
    assert.equal(first?.title, "Average Total Fishing Time");
    assert.equal(first?.valueText, "18.45");

    anchor.dataset.fishyStatBreakdown = JSON.stringify({
        title: "Average Total Fishing Time",
        value_text: "16.10",
        sections: [{ label: "Inputs", rows: [{ label: "Average bite time", value_text: "10.00" }] }],
    });

    const second = statBreakdownPayloadForAnchor(anchor);
    assert.equal(second?.title, "Average Total Fishing Time");
    assert.equal(second?.valueText, "16.10");
    assert.equal(second?.sections[0]?.rows[0]?.valueText, "10.00");
});

test("statBreakdownTooltipAnchorPoint prefers pointer coordinates before falling back to the anchor center", () => {
    const anchor = {
        getBoundingClientRect() {
            return {
                left: 100,
                top: 50,
                width: 80,
                height: 40,
            };
        },
    };

    assert.deepEqual(
        statBreakdownTooltipAnchorPoint(anchor, null, { clientX: 320, clientY: 180 }),
        { clientX: 320, clientY: 180 },
    );
    assert.deepEqual(
        statBreakdownTooltipAnchorPoint(anchor),
        { clientX: 140, clientY: 70 },
    );
});

test("statBreakdownTooltipPointerForAnchor reuses the last pointer for the same anchor", () => {
    const anchorA = { id: "a" };
    const anchorB = { id: "b" };

    assert.deepEqual(
        statBreakdownTooltipPointerForAnchor(anchorA, { clientX: 25, clientY: 40 }),
        { anchor: anchorA, clientX: 25, clientY: 40 },
    );
    assert.deepEqual(
        statBreakdownTooltipPointerForAnchor(
            anchorA,
            null,
            { anchor: anchorA, clientX: 25, clientY: 40 },
        ),
        { anchor: anchorA, clientX: 25, clientY: 40 },
    );
    assert.equal(
        statBreakdownTooltipPointerForAnchor(
            anchorB,
            null,
            { anchor: anchorA, clientX: 25, clientY: 40 },
        ),
        null,
    );
});

test("statBreakdownTooltipShouldRefresh only refreshes content when tooltip payload or color changes", () => {
    const tooltip = { id: "tooltip-1" };
    const anchor = {
        dataset: {
            fishyStatBreakdown: JSON.stringify({
                title: "Average Total Fishing Time",
                value_text: "18.45",
            }),
            fishyStatColor: "var(--color-info)",
        },
    };

    const first = statBreakdownTooltipShouldRefresh(null, tooltip, anchor);
    assert.equal(first.shouldRefresh, true);
    assert.equal(first.renderKey, statBreakdownTooltipRenderKey(anchor));

    const second = statBreakdownTooltipShouldRefresh(
        { tooltip, renderKey: first.renderKey },
        tooltip,
        anchor,
    );
    assert.equal(second.shouldRefresh, false);
    assert.equal(second.renderKey, first.renderKey);

    anchor.dataset.fishyStatColor = "var(--color-warning)";
    const colorChange = statBreakdownTooltipShouldRefresh(
        { tooltip, renderKey: first.renderKey },
        tooltip,
        anchor,
    );
    assert.equal(colorChange.shouldRefresh, true);

    anchor.dataset.fishyStatColor = "var(--color-info)";
    anchor.dataset.fishyStatBreakdown = JSON.stringify({
        title: "Average Total Fishing Time",
        value_text: "16.10",
    });
    const payloadChange = statBreakdownTooltipShouldRefresh(
        { tooltip, renderKey: first.renderKey },
        tooltip,
        anchor,
    );
    assert.equal(payloadChange.shouldRefresh, true);
});

test("statBreakdownTooltipShouldReactToMutations only reacts to observed tooltip attributes", () => {
    assert.deepEqual(STAT_BREAKDOWN_TOOLTIP_ATTRIBUTE_FILTER, [
        "data-fishy-stat-breakdown",
        "data-fishy-stat-color",
    ]);
    assert.equal(statBreakdownTooltipShouldReactToMutations([
        { type: "attributes", attributeName: "class" },
        { type: "childList" },
    ]), false);
    assert.equal(statBreakdownTooltipShouldReactToMutations([
        { type: "attributes", attributeName: "data-fishy-stat-breakdown" },
    ]), true);
    assert.equal(statBreakdownTooltipShouldReactToMutations([
        { type: "attributes", attributeName: "data-fishy-stat-color" },
    ]), true);
});

test("statBreakdownSectionDisplayLabel uses row labels for single-row results and a generic fallback otherwise", () => {
    assert.equal(statBreakdownSectionDisplayLabel({ label: "Inputs" }, 0), "Inputs");
    assert.equal(statBreakdownSectionDisplayLabel({
        label: "Composition",
        rows: [{ label: "Average total" }],
    }, 1), "Average total");
    assert.equal(statBreakdownSectionDisplayLabel({
        label: "Composition",
        rows: [{ label: "Average casts" }, { label: "Expected catches" }],
    }, 1), "Result");
    assert.equal(statBreakdownSectionDisplayLabel({ label: "Details", rows: [] }, 1), "Result");
});

test("statBreakdownResultKindLabel derives divider titles from the result formula type", () => {
    assert.equal(
        statBreakdownResultKindLabel(
            {
                title: "Average Total Fishing Time",
                formula_text: "Average total = Average bite time + Auto-Fishing Time + AFK catch time.",
            },
            { rows: [{ label: "Average total" }] },
        ),
        "Sum Total",
    );
    assert.equal(
        statBreakdownResultKindLabel(
            {
                title: "Average Bite Time",
                formula_text: "Average bite time = Zone average bite time × Level factor × Abundance factor.",
            },
            { rows: [{ label: "Average bite time" }] },
        ),
        "Average",
    );
    assert.equal(
        statBreakdownResultKindLabel(
            {
                title: "Silver Share",
                formula_text: "Silver share = Group expected silver / All-group expected silver total.",
            },
            { rows: [{ label: "Silver share" }] },
        ),
        "Result",
    );
});

test("statBreakdownSectionRowGroups sorts inputs by formula part order and groups shared terms", () => {
    const groups = statBreakdownSectionRowGroups({
        label: "Inputs",
        rows: [
            { label: "Pet 1", formulaPart: "Item DRR", formulaPartOrder: 2 },
            { label: "Brandstone factor", formulaPart: "Brandstone factor", formulaPartOrder: 1 },
            { label: "Pet 2", formulaPart: "Item DRR", formulaPartOrder: 2 },
            { label: "Guru 20", formulaPart: "Lifeskill DRR", formulaPartOrder: 3 },
        ],
    });

    assert.deepEqual(
        groups.map((group) => ({
            label: group.label,
            rows: group.rows.map((row) => row.label),
        })),
        [
            { label: "Brandstone factor", rows: ["Brandstone factor"] },
            { label: "Item DRR", rows: ["Pet 1", "Pet 2"] },
            { label: "Lifeskill DRR", rows: ["Guru 20"] },
        ],
    );
});

test("statBreakdownFormulaTokens keeps the symbolic formula order and attaches resolved values", () => {
    const payload = normalizeStatBreakdownPayload({
        value_text: "105.00",
        formula_text: "Average total = Average bite time + Auto-Fishing Time + AFK catch time.",
        sections: [
            {
                label: "Inputs",
                rows: [
                    {
                        label: "Average bite time",
                        value_text: "12.00",
                        formula_part: "Average bite time",
                        formula_part_order: 1,
                    },
                    {
                        label: "Auto-Fishing Time",
                        value_text: "90.00",
                        formula_part: "Auto-Fishing Time",
                        formula_part_order: 2,
                    },
                    {
                        label: "AFK catch time",
                        value_text: "3.00",
                        formula_part: "AFK catch time",
                        formula_part_order: 3,
                    },
                ],
            },
            {
                label: "Composition",
                rows: [{ label: "Average total", value_text: "105.00" }],
            },
        ],
    });

    const tokens = statBreakdownFormulaTokens(payload.formulaText, payload);

    assert.deepEqual(
        tokens.filter((token) => token.kind === "term").map((token) => ({
            text: token.text,
            valueText: token.valueText,
        })),
        [
            { text: "Average total", valueText: "105.00" },
            { text: "Average bite time", valueText: "12.00" },
            { text: "Auto-Fishing Time", valueText: "90.00" },
            { text: "AFK catch time", valueText: "3.00" },
        ],
    );
});

test("statBreakdownFormulaTokenRows splits semicolon-separated formulas into separate rows", () => {
    const payload = normalizeStatBreakdownPayload({
        value_text: "50.00%",
        formula_text: "AFR (uncapped) = highest pet AFR + additive item AFR; Applied AFR = min(AFR, 66.67%).",
        sections: [
            {
                label: "Inputs",
                rows: [
                    {
                        label: "Pet AFR",
                        value_text: "35.00%",
                        formula_part: "highest pet AFR",
                        formula_part_order: 1,
                    },
                    {
                        label: "Food buff",
                        value_text: "15.00%",
                        formula_part: "additive item AFR",
                        formula_part_order: 2,
                    },
                ],
            },
            {
                label: "Composition",
                rows: [
                    { label: "Uncapped AFR", value_text: "50.00%" },
                    { label: "Timing cap", value_text: "66.67%" },
                    { label: "Applied AFR", value_text: "50.00%" },
                ],
            },
        ],
    });

    const tokenRows = statBreakdownFormulaTokenRows(payload.formulaText, payload);

    assert.equal(tokenRows.length, 2);
    assert.deepEqual(
        tokenRows.map((row) => row.filter((token) => token.kind === "term").map((token) => token.text)),
        [
            ["AFR (uncapped)", "highest pet AFR", "additive item AFR"],
            ["Applied AFR", "AFR"],
        ],
    );
});
