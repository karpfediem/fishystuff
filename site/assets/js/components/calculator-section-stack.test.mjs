import { test } from "bun:test";
import assert from "node:assert/strict";

async function loadModule() {
    const originalHTMLElement = globalThis.HTMLElement;
    const originalCustomElements = globalThis.customElements;
    globalThis.HTMLElement = globalThis.HTMLElement ?? class {};
    globalThis.customElements = globalThis.customElements ?? {
        define() {},
        get() {
            return null;
        },
    };
    try {
        return await import(`./calculator-section-stack.js?test=${Date.now()}-${Math.random()}`);
    } finally {
        globalThis.HTMLElement = originalHTMLElement;
        globalThis.customElements = originalCustomElements;
    }
}

test("buildCalculatorSectionRenderOrder keeps custom rows first", async () => {
    const { buildCalculatorSectionRenderOrder } = await loadModule();

    assert.deepEqual(
        buildCalculatorSectionRenderOrder(
            ["overview", "zone", "distribution", "gear"],
            [[["overview"], ["distribution"]]],
        ),
        ["overview", "distribution", "zone", "gear"],
    );
});

test("flattenCustomLayout preserves row, column, and stack order while removing duplicates", async () => {
    const { flattenCustomLayout } = await loadModule();

    assert.deepEqual(
        flattenCustomLayout([
            [["overview"], ["distribution"]],
            [["distribution", "gear"]],
            [["missing"], ["food"]],
        ], ["overview", "distribution", "gear", "food"]),
        ["overview", "distribution", "gear", "food"],
    );
});

test("normalizeCustomLayout keeps rows and columns while filtering unknown sections", async () => {
    const { normalizeCustomLayout } = await loadModule();

    assert.deepEqual(
        normalizeCustomLayout(
            [
                [["overview", "missing"]],
                [["distribution"], ["gear", "distribution"]],
            ],
            ["overview", "distribution", "gear", "food"],
            ["food"],
        ),
        [
            [["overview"]],
            [["distribution"], ["gear"]],
        ],
    );
});

test("normalizeCustomLayout falls back to one-item rows for custom sections", async () => {
    const { normalizeCustomLayout } = await loadModule();

    assert.deepEqual(
        normalizeCustomLayout(
            undefined,
            ["overview", "zone", "distribution"],
            ["overview", "distribution"],
        ),
        [
            [["overview"]],
            [["distribution"]],
        ],
    );
});

test("normalizeCustomLayout rejects row-only layout values as non-current UI state", async () => {
    const { normalizeCustomLayout } = await loadModule();

    assert.deepEqual(
        normalizeCustomLayout(
            [
                ["overview", "distribution"],
            ],
            ["overview", "distribution"],
            ["overview"],
        ),
        [],
    );
});

test("normalizeCustomLayout preserves an explicit empty layout", async () => {
    const { normalizeCustomLayout } = await loadModule();

    assert.deepEqual(
        normalizeCustomLayout(
            [],
            ["overview", "zone", "distribution"],
            ["overview", "distribution"],
        ),
        [],
    );
});

test("patchTouchesCalculatorSectionLayout ignores non-layout signal patches", async () => {
    const { patchTouchesCalculatorSectionLayout } = await loadModule();

    assert.equal(patchTouchesCalculatorSectionLayout({ _resources: 50 }), false);
    assert.equal(patchTouchesCalculatorSectionLayout({ _calc: { zone_name: "Ahrmo Sea" } }), false);
    assert.equal(patchTouchesCalculatorSectionLayout({ _user_presets: { version: 8 } }), false);
    assert.equal(
        patchTouchesCalculatorSectionLayout({ _calculator_ui: { distribution_tab: "loot_flow" } }),
        false,
    );
});

test("patchTouchesCalculatorSectionLayout matches order-affecting layout patches", async () => {
    const { patchTouchesCalculatorSectionLayout } = await loadModule();

    assert.equal(
        patchTouchesCalculatorSectionLayout({ _calculator_ui: { workspace_tab: "loadout" } }),
        true,
    );
    assert.equal(
        patchTouchesCalculatorSectionLayout({ _calculator_ui: { custom_sections: ["overview"] } }),
        true,
    );
    assert.equal(
        patchTouchesCalculatorSectionLayout({ _calculator_ui: { custom_layout: [[["overview"]]] } }),
        true,
    );
    assert.equal(patchTouchesCalculatorSectionLayout({ _calculator_ui: null }), true);
});
