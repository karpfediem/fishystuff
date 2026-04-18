import test from "node:test";
import assert from "node:assert/strict";

import {
    buildProvenanceSegments,
    provenanceAriaLabel,
    provenanceIndicatorColor,
} from "./provenance-indicator.js";

const ACTIVE_GREY_COLOR =
    "color-mix(in oklab, var(--color-neutral) 62%, var(--color-base-content) 38%)";
const ACTIVE_BLUE_COLOR =
    "color-mix(in oklab, var(--color-info) 76%, var(--color-base-content) 24%)";
const ACTIVE_GREEN_COLOR =
    "color-mix(in oklab, var(--color-success) 78%, var(--color-base-content) 22%)";
const ACTIVE_PURPLE_COLOR =
    "color-mix(in oklab, var(--color-secondary) 78%, var(--color-base-content) 22%)";
const ACTIVE_YELLOW_COLOR =
    "color-mix(in oklab, var(--color-warning) 80%, var(--color-base-content) 20%)";
const INACTIVE_GREY_COLOR =
    "color-mix(in oklab, var(--color-neutral) 28%, var(--color-base-300) 72%)";

test("buildProvenanceSegments keeps community-only presence grey and community rate yellow", () => {
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
    assert.equal(rateSegment.color, ACTIVE_YELLOW_COLOR);
    assert.equal(presenceSegment.color, ACTIVE_GREY_COLOR);
    assert.match(provenanceAriaLabel(presenceSegment), /Presence: Community/);
    assert.match(provenanceAriaLabel(rateSegment), /Rate: Community guess/);
});

test("buildProvenanceSegments falls back to neutral inactive facts when provenance is missing", () => {
    const [presenceSegment, rateSegment] = buildProvenanceSegments({});

    assert.equal(rateSegment.active, false);
    assert.equal(presenceSegment.active, false);
    assert.equal(rateSegment.color, INACTIVE_GREY_COLOR);
    assert.equal(presenceSegment.color, INACTIVE_GREY_COLOR);
    assert.match(rateSegment.detail, /No rate provenance recorded yet\./);
    assert.match(presenceSegment.detail, /No presence provenance recorded yet\./);
});

test("buildProvenanceSegments keeps non-ring database presence grey and preserves presence text fallback", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "database",
        presenceValueText: "Database-backed presence",
    });

    assert.equal(presenceSegment.sourceLabel, "Database");
    assert.equal(presenceSegment.detail, "Database-backed presence");
    assert.equal(presenceSegment.color, ACTIVE_GREY_COLOR);
});

test("buildProvenanceSegments marks fully contained ranking rings green", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "ranking",
        presenceDetail: "Ranking ring fully inside zone ×8",
    });

    assert.equal(presenceSegment.sourceLabel, "Ranking ring");
    assert.equal(presenceSegment.color, ACTIVE_GREEN_COLOR);
    assert.match(provenanceAriaLabel(presenceSegment), /Presence: Ranking ring/);
});

test("buildProvenanceSegments marks partial-only ranking rings yellow", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "ranking",
        presenceDetail: "Ranking ring overlaps zone edge ×3",
    });

    assert.equal(presenceSegment.color, ACTIVE_YELLOW_COLOR);
});

test("buildProvenanceSegments prefers fully contained rings when mixed presence sources contribute", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "mixed",
        presenceDetail: "Ranking ring fully inside zone ×8 | Community confirmed ×2",
    });

    assert.equal(presenceSegment.sourceLabel, "Mixed support");
    assert.equal(presenceSegment.color, ACTIVE_GREEN_COLOR);
});

test("buildProvenanceSegments recognizes database and derived rate provenance labels", () => {
    const [, databaseRateSegment] = buildProvenanceSegments({
        rateSourceKind: "database",
        rateDetail: "DB 80%",
        rateValueText: "80%",
    });
    const [, derivedRateSegment] = buildProvenanceSegments({
        rateSourceKind: "derived",
        rateDetail: "Derived 12.5% from current group weights",
        rateValueText: "12.5%",
    });

    assert.equal(databaseRateSegment.sourceLabel, "Database");
    assert.equal(databaseRateSegment.color, ACTIVE_BLUE_COLOR);
    assert.equal(derivedRateSegment.sourceLabel, "Derived");
    assert.equal(derivedRateSegment.color, ACTIVE_GREY_COLOR);
});

test("buildProvenanceSegments recognizes personal overlay provenance", () => {
    const [, overlayRateSegment] = buildProvenanceSegments({
        rateSourceKind: "overlay",
        rateDetail: "Personal overlay final share 82%.",
        rateValueText: "82%",
    });

    assert.equal(overlayRateSegment.sourceLabel, "Personal overlay");
    assert.equal(overlayRateSegment.color, ACTIVE_PURPLE_COLOR);
    assert.match(provenanceAriaLabel(overlayRateSegment), /Personal overlay/);
});

test("buildProvenanceSegments returns presence before rate", () => {
    const [firstSegment, secondSegment] = buildProvenanceSegments({
        presenceSourceKind: "community",
        rateSourceKind: "database",
    });

    assert.equal(firstSegment.label, "Presence");
    assert.equal(secondSegment.label, "Rate");
});

test("provenanceIndicatorColor derives presence color from resolved ring evidence", () => {
    assert.equal(
        provenanceIndicatorColor("presence", "mixed", {
            detail: "Ranking ring fully inside zone ×8 | Community confirmed×2",
        }),
        ACTIVE_GREEN_COLOR,
    );
    assert.equal(
        provenanceIndicatorColor("presence", "ranking", {
            detail: "Ranking ring overlaps zone edge ×3",
        }),
        ACTIVE_YELLOW_COLOR,
    );
    assert.equal(
        provenanceIndicatorColor("presence", "community", {
            detail: "Community confirmed×2 · Prize subgroup",
        }),
        ACTIVE_GREY_COLOR,
    );
});
