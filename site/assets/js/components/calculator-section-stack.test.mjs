import test from "node:test";
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

test("buildCalculatorSectionRenderOrder keeps pinned rows first and selected section next", async () => {
    const { buildCalculatorSectionRenderOrder } = await loadModule();

    assert.deepEqual(
        buildCalculatorSectionRenderOrder(
            ["overview", "inputs", "distribution", "gear"],
            "gear",
            [["overview", "distribution"]],
        ),
        ["overview", "distribution", "gear", "inputs"],
    );
});

test("flattenPinnedLayout preserves row order and removes duplicates", async () => {
    const { flattenPinnedLayout } = await loadModule();

    assert.deepEqual(
        flattenPinnedLayout([
            ["overview", "distribution"],
            ["distribution", "gear"],
            ["missing", "food"],
        ], ["overview", "distribution", "gear", "food"]),
        ["overview", "distribution", "gear", "food"],
    );
});

test("normalizePinnedLayout keeps rows while filtering unknown sections", async () => {
    const { normalizePinnedLayout } = await loadModule();

    assert.deepEqual(
        normalizePinnedLayout(
            [
                ["overview", "missing"],
                ["distribution", "gear", "distribution"],
            ],
            ["overview", "distribution", "gear", "food"],
            ["food"],
        ),
        [
            ["overview"],
            ["distribution", "gear"],
        ],
    );
});

test("normalizePinnedLayout falls back to one-item rows for legacy pinned sections", async () => {
    const { normalizePinnedLayout } = await loadModule();

    assert.deepEqual(
        normalizePinnedLayout(
            undefined,
            ["overview", "inputs", "distribution"],
            ["overview", "distribution"],
        ),
        [
            ["overview"],
            ["distribution"],
        ],
    );
});

test("normalizePinnedLayout preserves an explicit empty layout", async () => {
    const { normalizePinnedLayout } = await loadModule();

    assert.deepEqual(
        normalizePinnedLayout(
            [],
            ["overview", "inputs", "distribution"],
            ["overview", "distribution"],
        ),
        [],
    );
});
