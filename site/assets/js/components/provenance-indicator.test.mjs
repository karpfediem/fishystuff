import { test } from "bun:test";
import assert from "node:assert/strict";

import {
    buildProvenanceDetailCards,
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

test("buildProvenanceSegments marks community confirmed presence green and community rate yellow", () => {
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
    assert.equal(rateSegment.sourceIcon, "source-community");
    assert.equal(presenceSegment.sourceIcon, "source-community");
    assert.equal(presenceSegment.sourceTone, "community-presence");
    assert.equal(rateSegment.color, ACTIVE_YELLOW_COLOR);
    assert.equal(presenceSegment.color, ACTIVE_GREEN_COLOR);
    assert.match(provenanceAriaLabel(presenceSegment), /Presence: Community/);
    assert.match(provenanceAriaLabel(rateSegment), /Rate: Community guess/);
});

test("buildProvenanceSegments marks community guessed presence green", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "community",
        presenceDetail: "Community guessed presence · Prize subgroup 11054",
        presenceValueText: "Community guessed presence",
    });

    assert.equal(presenceSegment.sourceTone, "community-presence");
    assert.equal(presenceSegment.color, ACTIVE_GREEN_COLOR);
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
    assert.equal(presenceSegment.sourceIcon, "source-database");
    assert.equal(presenceSegment.detail, "Database-backed presence");
    assert.equal(presenceSegment.color, ACTIVE_GREY_COLOR);
});

test("buildProvenanceSegments marks fully contained ranking rings green", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "ranking",
        presenceDetail: "Ranking ring fully inside zone ×8",
    });

    assert.equal(presenceSegment.sourceLabel, "Ranking ring");
    assert.equal(presenceSegment.sourceIcon, "ring-full");
    assert.equal(presenceSegment.sourceTone, "ranking-full");
    assert.equal(presenceSegment.color, ACTIVE_GREEN_COLOR);
    assert.match(provenanceAriaLabel(presenceSegment), /Presence: Ranking ring/);
});

test("buildProvenanceSegments marks partial-only ranking rings yellow", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "ranking",
        presenceDetail: "Ranking ring overlaps zone edge ×3",
    });

    assert.equal(presenceSegment.color, ACTIVE_YELLOW_COLOR);
    assert.equal(presenceSegment.sourceIcon, "ring-partial");
    assert.equal(presenceSegment.sourceTone, "ranking-partial");
});

test("buildProvenanceSegments prefers fully contained rings when mixed presence sources contribute", () => {
    const [presenceSegment] = buildProvenanceSegments({
        presenceSourceKind: "mixed",
        presenceDetail: "Ranking ring fully inside zone ×8 | Community confirmed ×2",
    });

    assert.equal(presenceSegment.sourceLabel, "Mixed support");
    assert.equal(presenceSegment.sourceTone, "mixed");
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
    assert.equal(databaseRateSegment.sourceIcon, "source-database");
    assert.equal(databaseRateSegment.sourceTone, "database");
    assert.equal(databaseRateSegment.color, ACTIVE_BLUE_COLOR);
    assert.equal(derivedRateSegment.sourceLabel, "Derived");
    assert.equal(derivedRateSegment.color, ACTIVE_GREY_COLOR);
});

test("buildProvenanceSegments can label metric provenance separately from rates", () => {
    const [, metricSegment] = buildProvenanceSegments({
        rateLabel: "Totem EXP",
        rateSourceKind: "derived",
        rateDetail: "Missing source row; imputed from grade rule.",
        rateValueText: "1,875",
    });

    assert.equal(metricSegment.label, "Totem EXP");
    assert.equal(metricSegment.sourceLabel, "Derived");
    assert.match(provenanceAriaLabel(metricSegment), /Totem EXP: Derived/);
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
        provenanceIndicatorColor("presence", "mixed", {
            detail: "Ranking ring overlaps zone edge ×3 | Community confirmed×2 · Prize subgroup",
        }),
        ACTIVE_GREEN_COLOR,
    );
});

test("buildProvenanceDetailCards separates source evidence into toned cards", () => {
    const cards = buildProvenanceDetailCards(
        "Community confirmed×1 · Prize subgroup 11054 · Row import: 2026-04-30 14:00 UTC · Source: Community workbook | Ranking ring overlaps zone edge ×3 · Last seen: 2026-04-29 13:00 UTC",
    );

    assert.equal(cards.length, 2);
    assert.equal(cards[0].tone, "community-presence");
    assert.equal(cards[0].icon, "source-community");
    assert.equal(cards[0].badge, "Community support");
    assert.equal(cards[0].rows[0].label, "Row import");
    assert.equal(cards[0].rows[0].value, "2026-04-30 14:00 UTC");
    assert.equal(cards[0].rows[0].label && cards[0].rows[0].value ? "date-confirmed" : "", "date-confirmed");
    assert.equal(cards[1].tone, "ranking-partial");
    assert.equal(cards[1].icon, "ring-partial");
    assert.equal(cards[1].badge, "Partial ring");
    assert.equal(cards[1].rows[0].label, "Last seen");
});

test("buildProvenanceDetailCards keeps community rate guesses yellow", () => {
    const cards = buildProvenanceDetailCards(
        "Community guess 5% · Community support: community_zone_fish_support",
    );

    assert.equal(cards.length, 1);
    assert.equal(cards[0].tone, "community-rate-guess");
    assert.equal(cards[0].badge, "Community guess");
});
