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

test("buildCalculatorSectionRenderOrder keeps pinned sections first and selected section next", async () => {
    const { buildCalculatorSectionRenderOrder } = await loadModule();

    assert.deepEqual(
        buildCalculatorSectionRenderOrder(
            ["overview", "inputs", "distribution", "gear"],
            "gear",
            ["overview", "distribution"],
        ),
        ["overview", "distribution", "gear", "inputs"],
    );
});

test("buildCalculatorSectionRenderOrder filters duplicates and unknown pinned ids", async () => {
    const { buildCalculatorSectionRenderOrder } = await loadModule();

    assert.deepEqual(
        buildCalculatorSectionRenderOrder(
            ["overview", "inputs", "distribution"],
            "inputs",
            ["distribution", "distribution", "missing"],
        ),
        ["distribution", "inputs", "overview"],
    );
});

test("projectPinnedSlotIndex projects the dragged center into the nearest slot band", async () => {
    const { projectPinnedSlotIndex } = await loadModule();

    assert.equal(projectPinnedSlotIndex([100, 200, 300], 20), 0);
    assert.equal(projectPinnedSlotIndex([100, 200, 300], 150), 1);
    assert.equal(projectPinnedSlotIndex([100, 200, 300], 260), 2);
    assert.equal(projectPinnedSlotIndex([100, 200, 300], 360), 3);
});

test("buildPinnedSlots derives slot thresholds from card rect midpoints", async () => {
    const { buildPinnedSlots } = await loadModule();

    assert.deepEqual(
        buildPinnedSlots([
            { top: 20, height: 100 },
            { top: 160, height: 80 },
        ]),
        [
            { index: 0, thresholdY: 70 },
            { index: 1, thresholdY: 200 },
        ],
    );
});

test("projectPinnedSlotIndex accepts slot objects from buildPinnedSlots", async () => {
    const { buildPinnedSlots, projectPinnedSlotIndex } = await loadModule();
    const slots = buildPinnedSlots([
        { top: 40, height: 80 },
        { top: 160, height: 120 },
    ]);

    assert.equal(projectPinnedSlotIndex(slots, 10), 0);
    assert.equal(projectPinnedSlotIndex(slots, 110), 1);
    assert.equal(projectPinnedSlotIndex(slots, 300), 2);
});
