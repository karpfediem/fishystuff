import test from "node:test";
import assert from "node:assert/strict";

const originalHTMLElement = globalThis.HTMLElement;
const originalElement = globalThis.Element;
const originalNode = globalThis.Node;
const originalDocument = globalThis.document;
const originalWindow = globalThis.window;
const originalCustomElements = globalThis.customElements;
const originalHTMLInputElement = globalThis.HTMLInputElement;
const originalHTMLTemplateElement = globalThis.HTMLTemplateElement;

class FakeElement extends EventTarget {
    constructor() {
        super();
        this.attributes = new Map();
        this.childNodes = [];
        this.dataset = {};
        this.hidden = false;
        this.parentNode = null;
        this.style = {};
        this.textContent = "";
        this._queryMap = new Map();
        this._queryAllMap = new Map();
        this._closestMap = new Map();
        this._rect = {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            right: 0,
            bottom: 0,
        };
    }

    append(...nodes) {
        for (const node of nodes) {
            this.appendChild(node);
        }
    }

    appendChild(child) {
        if (!child) {
            return child;
        }
        if (child.parentNode) {
            child.parentNode.removeChild(child);
        }
        this.childNodes.push(child);
        child.parentNode = this;
        return child;
    }

    closest(selector) {
        return this._closestMap.get(selector) || null;
    }

    contains(node) {
        if (node === this) {
            return true;
        }
        return this.childNodes.some((child) => typeof child.contains === "function" && child.contains(node));
    }

    getBoundingClientRect() {
        return { ...this._rect };
    }

    getAttribute(name) {
        return this.attributes.has(name) ? this.attributes.get(name) : null;
    }

    insertBefore(child, referenceNode) {
        if (!child) {
            return child;
        }
        if (child.parentNode) {
            child.parentNode.removeChild(child);
        }
        const index = referenceNode ? this.childNodes.indexOf(referenceNode) : -1;
        if (index < 0) {
            this.childNodes.push(child);
        } else {
            this.childNodes.splice(index, 0, child);
        }
        child.parentNode = this;
        return child;
    }

    querySelector(selector) {
        return this._queryMap.get(selector) || null;
    }

    querySelectorAll(selector) {
        return this._queryAllMap.get(selector) || [];
    }

    removeAttribute(name) {
        this.attributes.delete(String(name));
    }

    removeChild(child) {
        const index = this.childNodes.indexOf(child);
        if (index >= 0) {
            this.childNodes.splice(index, 1);
            child.parentNode = null;
        }
        return child;
    }

    setAttribute(name, value) {
        this.attributes.set(String(name), String(value ?? ""));
    }

    setQuery(selector, element) {
        this._queryMap.set(selector, element);
    }

    setQueryAll(selector, elements) {
        this._queryAllMap.set(selector, Array.isArray(elements) ? elements : []);
    }

    setClosest(selector, element) {
        this._closestMap.set(selector, element);
    }

    setRect({ left = 0, top = 0, width = 0, height = 0 }) {
        this._rect = {
            left,
            top,
            width,
            height,
            right: left + width,
            bottom: top + height,
        };
    }

    get nextSibling() {
        if (!this.parentNode) {
            return null;
        }
        const index = this.parentNode.childNodes.indexOf(this);
        return index >= 0 ? this.parentNode.childNodes[index + 1] ?? null : null;
    }
}

class FakeComment extends FakeElement {}

function createEnvironment() {
    const document = new EventTarget();
    document.body = new FakeElement();
    document.documentElement = new FakeElement();
    document.createComment = () => new FakeComment();
    document.createElement = () => new FakeElement();
    document.getElementById = () => null;

    const customElementsRegistry = {
        get() {
            return null;
        },
        define() {},
    };

    const window = new EventTarget();
    window.innerWidth = 1280;
    window.innerHeight = 900;
    window.location = { href: "http://localhost/" };
    window.customElements = customElementsRegistry;
    window.requestAnimationFrame = (callback) => {
        callback();
        return 1;
    };
    window.cancelAnimationFrame = () => {};
    return { document, window, customElementsRegistry };
}

async function loadModule() {
    const { document, window, customElementsRegistry } = createEnvironment();
    globalThis.HTMLElement = FakeElement;
    globalThis.Element = FakeElement;
    globalThis.Node = FakeElement;
    globalThis.document = document;
    globalThis.window = window;
    globalThis.customElements = customElementsRegistry;
    globalThis.HTMLInputElement = FakeElement;
    globalThis.HTMLTemplateElement = FakeElement;
    return import(`./searchable-dropdown.js?test=${Date.now()}-${Math.random()}`);
}

test("FishySearchableDropdown detaches its panel to the document body and restores it on close", async (t) => {
    t.after(() => {
        globalThis.HTMLElement = originalHTMLElement;
        globalThis.Element = originalElement;
        globalThis.Node = originalNode;
        globalThis.document = originalDocument;
        globalThis.window = originalWindow;
        globalThis.customElements = originalCustomElements;
        globalThis.HTMLInputElement = originalHTMLInputElement;
        globalThis.HTMLTemplateElement = originalHTMLTemplateElement;
    });

    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    trigger.setRect({ left: 240, top: 120, width: 156, height: 32 });
    panel.setRect({ left: 0, top: 0, width: 288, height: 220 });
    dropdown.setQuery('[data-role="trigger"]', trigger);
    dropdown.setQuery('[data-role="panel"]', panel);
    panel.setQuery('[data-role="search-input"]', searchInput);
    panel.setQuery('[data-role="results"]', results);
    dropdown.append(trigger);
    dropdown.append(panel);
    panel.append(searchInput);
    panel.append(results);

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.open();

    assert.equal(panel.parentNode, document.body);
    assert.equal(panel.style.position, "fixed");
    assert.equal(panel.hidden, false);
    assert.equal(dropdown.searchInputElement(), searchInput);
    assert.equal(dropdown.resultsElement(), results);

    dropdown._handleDocumentPointerDown({ target: searchInput });
    assert.equal(dropdown.isOpen(), true);

    dropdown.close();

    assert.equal(panel.parentNode, dropdown);
    assert.equal(panel.style.position, "");
    assert.equal(panel.hidden, true);
});

test("searchable dropdown accepts valid ISO date custom options", async (t) => {
    t.after(() => {
        globalThis.HTMLElement = originalHTMLElement;
        globalThis.Element = originalElement;
        globalThis.Node = originalNode;
        globalThis.document = originalDocument;
        globalThis.window = originalWindow;
        globalThis.customElements = originalCustomElements;
        globalThis.HTMLInputElement = originalHTMLInputElement;
        globalThis.HTMLTemplateElement = originalHTMLTemplateElement;
    });

    const { FishySearchableDropdown, normalizeIsoDateValue } = await loadModule();
    const dropdown = new FishySearchableDropdown();
    dropdown.setAttribute("custom-option-mode", "iso-date");

    assert.equal(normalizeIsoDateValue("2026-04-16"), "2026-04-16");
    assert.equal(normalizeIsoDateValue("2026-02-30"), "");

    const customOption = dropdown._buildCustomOption("2026-04-16", "", []);
    const button = customOption?.childNodes?.[0] ?? null;

    assert.ok(customOption);
    assert.equal(button?.getAttribute?.("data-value"), "2026-04-16");
    assert.equal(button?.getAttribute?.("data-label"), "2026-04-16");
});

test("searchable dropdown can anchor its detached panel to a wider ancestor", async (t) => {
    t.after(() => {
        globalThis.HTMLElement = originalHTMLElement;
        globalThis.Element = originalElement;
        globalThis.Node = originalNode;
        globalThis.document = originalDocument;
        globalThis.window = originalWindow;
        globalThis.customElements = originalCustomElements;
        globalThis.HTMLInputElement = originalHTMLInputElement;
        globalThis.HTMLTemplateElement = originalHTMLTemplateElement;
    });

    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const anchor = new FakeElement();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    dropdown.setAttribute("panel-anchor-closest", ".fishymap-date-term-content");
    dropdown.setClosest(".fishymap-date-term-content", anchor);
    anchor.setRect({ left: 180, top: 132, width: 420, height: 40 });
    trigger.setRect({ left: 332, top: 136, width: 172, height: 32 });
    panel.setRect({ left: 0, top: 0, width: 288, height: 220 });
    dropdown.setQuery('[data-role="trigger"]', trigger);
    dropdown.setQuery('[data-role="panel"]', panel);
    panel.setQuery('[data-role="search-input"]', searchInput);
    panel.setQuery('[data-role="results"]', results);
    dropdown.append(trigger);
    dropdown.append(panel);
    panel.append(searchInput);
    panel.append(results);

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.open();

    assert.equal(panel.style.left, "180px");
    assert.equal(panel.style.top, "180px");
    assert.equal(panel.style.width, "420px");
});
