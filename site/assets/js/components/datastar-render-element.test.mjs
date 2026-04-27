import { test } from "bun:test";
import assert from "node:assert/strict";

async function loadModule() {
    const originalHTMLElement = globalThis.HTMLElement;
    globalThis.HTMLElement = globalThis.HTMLElement ?? class {};
    try {
        return await import(`./datastar-render-element.js?test=${Date.now()}-${Math.random()}`);
    } finally {
        globalThis.HTMLElement = originalHTMLElement;
    }
}

test("cloneSignalValue snapshots existing proxy fields without touching missing fields", async () => {
    const { cloneSignalValue } = await loadModule();
    const accessedMissingFields = [];
    const source = new Proxy(
        {
            rows: [
                {
                    label: "Yellow Corvina",
                    expected_count_raw: 1.25,
                },
            ],
        },
        {
            get(target, property, receiver) {
                if (typeof property === "string" && !(property in target)) {
                    accessedMissingFields.push(property);
                }
                return Reflect.get(target, property, receiver);
            },
        },
    );

    const snapshot = cloneSignalValue(source);

    assert.deepEqual(snapshot, {
        rows: [
            {
                label: "Yellow Corvina",
                expected_count_raw: 1.25,
            },
        ],
    });
    assert.deepEqual(accessedMissingFields, []);
    assert.equal(snapshot.rows[0].presence_text, undefined);
    assert.deepEqual(accessedMissingFields, []);
});

test("cloneSignalValue keeps snapshots detached when a nested proxy field throws", async () => {
    const { cloneSignalValue } = await loadModule();
    const source = {};
    Object.defineProperty(source, "stable", {
        enumerable: true,
        value: { label: "Stable" },
    });
    Object.defineProperty(source, "unstable", {
        enumerable: true,
        get() {
            throw new Error("signal changed while cloning");
        },
    });

    const snapshot = cloneSignalValue(source);

    assert.notEqual(snapshot, source);
    assert.deepEqual(snapshot, { stable: { label: "Stable" } });
});

test("patchTouchesSignalPath only matches patches for the observed signal branch", async () => {
    const { patchTouchesSignalPath } = await loadModule();

    assert.equal(
        patchTouchesSignalPath(
            { _calc: { loot_sankey_chart: { rows: [] } } },
            "_calc.loot_sankey_chart",
        ),
        true,
    );
    assert.equal(
        patchTouchesSignalPath(
            { _calc: { fish_group_distribution_chart: { segments: [] } } },
            "_calc.loot_sankey_chart",
        ),
        false,
    );
    assert.equal(
        patchTouchesSignalPath({ timespanAmount: 1 }, "_calc.loot_sankey_chart"),
        false,
    );
});
