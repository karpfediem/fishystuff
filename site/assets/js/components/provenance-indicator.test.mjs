import test from "node:test";
import assert from "node:assert/strict";

import {
    buildProvenanceSegments,
    provenanceAriaLabel,
    provenanceIndicatorColor,
} from "./provenance-indicator.js";

test("buildProvenanceSegments distinguishes database, community presence, and community rate colors", () => {
    const [presenceSegment, rateSegment] = buildProvenanceSegments({
        rateSourceKind: "community",
        rateDetail: "Community guess · Prize subgroup 11054",
        rateValueText: "1.00%",
        presenceSourceKind: "community",
        presenceDetail: "Community confirmed×2 · Prize subgroup 11054",
        presenceValueText: "Community confirmed×2",
    });

    assert.equal(rateSegment.sourceLabel, "Community guess");
    assert.equal(presenceSegment.sourceLabel, "Community");
    assert.equal(rateSegment.color, provenanceIndicatorColor("rate", "community"));
    assert.equal(presenceSegment.color, provenanceIndicatorColor("presence", "community"));
    assert.match(provenanceAriaLabel(presenceSegment), /Presence: Community/);
    assert.match(provenanceAriaLabel(rateSegment), /Rate: Community guess/);
});

test("buildProvenanceSegments falls back to neutral inactive facts when provenance is missing", () => {
    const [presenceSegment, rateSegment] = buildProvenanceSegments({});

    assert.equal(rateSegment.active, false);
    assert.equal(presenceSegment.active, false);
    assert.match(rateSegment.detail, /No rate provenance recorded yet\./);
    assert.match(presenceSegment.detail, /No presence provenance recorded yet\./);
});

test("buildProvenanceSegments keeps database presence blue and preserves presence text fallback", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "database",
        presenceValueText: "Ranking presence",
    });

    assert.equal(presenceSegment.sourceLabel, "Database");
    assert.equal(presenceSegment.detail, "Ranking presence");
    assert.equal(presenceSegment.color, provenanceIndicatorColor("presence", "database"));
});

test("buildProvenanceSegments recognizes ranking presence provenance", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "ranking",
        presenceDetail: "Ranking ring fully inside zone ×8",
    });

    assert.equal(presenceSegment.sourceLabel, "Ranking ring");
    assert.equal(presenceSegment.color, provenanceIndicatorColor("presence", "ranking"));
    assert.match(provenanceAriaLabel(presenceSegment), /Presence: Ranking ring/);
});

test("buildProvenanceSegments uses mixed presence provenance when multiple sources contribute", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "mixed",
        presenceDetail: "Ranking ring fully inside zone ×8 | Community confirmed ×2",
    });

    assert.equal(presenceSegment.sourceLabel, "Mixed support");
    assert.match(
        presenceSegment.color,
        /linear-gradient\(180deg,/,
    );
});

test("buildProvenanceSegments returns presence before rate", () => {
    const [firstSegment, secondSegment] = buildProvenanceSegments({
        presenceSourceKind: "community",
        rateSourceKind: "database",
    });

    assert.equal(firstSegment.label, "Presence");
    assert.equal(secondSegment.label, "Rate");
});
