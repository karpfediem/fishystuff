import test from "node:test";
import assert from "node:assert/strict";

import { normalizeStatBreakdownPayload } from "./stat-breakdown-tooltip.js";

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
                        label: "Zone base rate",
                        value_text: "10%",
                        detail_text: "Base group rate from zone data",
                    },
                ],
            },
        ],
    });

    assert.equal(payload.eyebrow, "Computed stat");
    assert.equal(payload.title, "Rare group");
    assert.equal(payload.sections.length, 1);
    assert.equal(payload.sections[0].rows[0].label, "Zone base rate");
    assert.equal(payload.sections[0].rows[0].valueText, "10%");
});

test("normalizeStatBreakdownPayload drops empty sections and returns null for empty payloads", () => {
    assert.equal(normalizeStatBreakdownPayload({ sections: [{ rows: [] }] }), null);
});
