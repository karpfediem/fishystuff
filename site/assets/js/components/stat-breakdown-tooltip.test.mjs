import test from "node:test";
import assert from "node:assert/strict";

import {
    normalizeStatBreakdownPayload,
    statBreakdownPayloadForAnchor,
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
