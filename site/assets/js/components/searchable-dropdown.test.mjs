import { test as bunTest } from "bun:test";
import assert from "node:assert/strict";

const originalHTMLElement = globalThis.HTMLElement;
const originalElement = globalThis.Element;
const originalNode = globalThis.Node;
const originalDocument = globalThis.document;
const originalWindow = globalThis.window;
const originalCustomElements = globalThis.customElements;
const originalFetch = globalThis.fetch;
const originalGetComputedStyle = globalThis.getComputedStyle;
const originalHTMLInputElement = globalThis.HTMLInputElement;
const originalHTMLTemplateElement = globalThis.HTMLTemplateElement;

function test(name, optionsOrCallback, maybeCallback) {
    const callback = typeof optionsOrCallback === "function" ? optionsOrCallback : maybeCallback;
    const options = typeof optionsOrCallback === "function" ? undefined : optionsOrCallback;
    const wrapped = async () => {
        const cleanups = [];
        const context = {
            after(cleanup) {
                if (typeof cleanup === "function") {
                    cleanups.push(cleanup);
                }
            },
        };
        try {
            return await callback(context);
        } finally {
            for (const cleanup of cleanups.reverse()) {
                await cleanup();
            }
        }
    };
    return options === undefined ? bunTest(name, wrapped) : bunTest(name, options, wrapped);
}

function datasetKeyToAttributeName(key) {
    return `data-${String(key).replace(/[A-Z]/g, (char) => `-${char.toLowerCase()}`)}`;
}

function attributeNameToDatasetKey(name) {
    return String(name).slice(5).replace(/-([a-z])/g, (_, char) => char.toUpperCase());
}

function createDatasetProxy(element, values = {}) {
    const target = { ...values };
    return new Proxy(target, {
        deleteProperty(store, property) {
            delete store[property];
            element.attributes.delete(datasetKeyToAttributeName(property));
            return true;
        },
        set(store, property, value) {
            const normalized = String(value ?? "");
            store[property] = normalized;
            element.attributes.set(datasetKeyToAttributeName(property), normalized);
            return true;
        },
    });
}

class FakeElement extends EventTarget {
    constructor(tagName = "div") {
        super();
        this.attributes = new Map();
        this.childNodes = [];
        this.dataset = createDatasetProxy(this);
        this.hidden = false;
        this.parentNode = null;
        this.tagName = String(tagName || "div").toUpperCase();
        this.style = {};
        this.textContent = "";
        this.clientWidth = 0;
        this.scrollHeight = 0;
        this.scrollLeft = 0;
        this.scrollTop = 0;
        this.clientHeight = 0;
        this.scrollWidth = 0;
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
        if (this._closestMap.has(selector)) {
            return this._closestMap.get(selector) || null;
        }
        let current = this;
        while (current) {
            if (current.matches(selector)) {
                return current;
            }
            current = current.parentNode;
        }
        return null;
    }

    cloneNode(deep = false) {
        const clone = new FakeElement(this.tagName);
        clone.hidden = this.hidden;
        clone.textContent = this.textContent;
        clone.clientWidth = this.clientWidth;
        clone.scrollHeight = this.scrollHeight;
        clone.scrollLeft = this.scrollLeft;
        clone.scrollTop = this.scrollTop;
        clone.clientHeight = this.clientHeight;
        clone.scrollWidth = this.scrollWidth;
        clone.style = { ...this.style };
        for (const [name, value] of this.attributes) {
            clone.setAttribute(name, value);
        }
        if (deep) {
            for (const child of this.childNodes) {
                clone.appendChild(typeof child.cloneNode === "function" ? child.cloneNode(true) : child);
            }
        }
        return clone;
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

    matches(selector) {
        if (!selector) {
            return false;
        }

        const attributeSelector = selector.match(
            /^(?:(?<tag>[a-zA-Z0-9-]+))?\[(?<attr>[^\]=]+)(?:=\"(?<value>[^\"]*)\")?\]$/,
        );
        if (attributeSelector?.groups) {
            const { tag, attr, value } = attributeSelector.groups;
            if (tag && this.tagName !== String(tag).toUpperCase()) {
                return false;
            }
            if (!this.hasAttribute(attr)) {
                return false;
            }
            return value === undefined || this.getAttribute(attr) === value;
        }

        return this.tagName === String(selector).toUpperCase();
    }

    querySelector(selector) {
        if (this._queryMap.has(selector)) {
            return this._queryMap.get(selector) || null;
        }
        return this._walk().find((node) => node !== this && node.matches(selector)) || null;
    }

    querySelectorAll(selector) {
        if (this._queryAllMap.has(selector)) {
            return this._queryAllMap.get(selector) || [];
        }
        return this._walk().filter((node) => node !== this && node.matches(selector));
    }

    removeAttribute(name) {
        const normalized = String(name);
        this.attributes.delete(normalized);
        if (normalized.startsWith("data-")) {
            delete this.dataset[attributeNameToDatasetKey(normalized)];
        }
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
        const normalizedName = String(name);
        const normalizedValue = String(value ?? "");
        this.attributes.set(normalizedName, normalizedValue);
        if (normalizedName.startsWith("data-")) {
            this.dataset[attributeNameToDatasetKey(normalizedName)] = normalizedValue;
        }
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

    get firstElementChild() {
        return this.childNodes.find((child) => child instanceof FakeElement) || null;
    }

    hasAttribute(name) {
        return this.attributes.has(String(name));
    }

    replaceChildren(...nodes) {
        for (const child of [...this.childNodes]) {
            child.parentNode = null;
        }
        this.childNodes = [];
        for (const node of nodes) {
            this.appendChild(node);
        }
    }

    replaceWith(node) {
        if (!this.parentNode) {
            return;
        }
        const siblings = this.parentNode.childNodes;
        const index = siblings.indexOf(this);
        if (index < 0) {
            return;
        }
        if (node.parentNode) {
            node.parentNode.removeChild(node);
        }
        siblings.splice(index, 1, node);
        node.parentNode = this.parentNode;
        this.parentNode = null;
    }

    _walk() {
        const nodes = [this];
        for (const child of this.childNodes) {
            if (child instanceof FakeElement) {
                nodes.push(...child._walk());
            }
        }
        return nodes;
    }
}

class FakeComment extends FakeElement {}

class FakeTemplateElement extends FakeElement {
    constructor(fragmentRegistry = null) {
        super("template");
        this._fragmentRegistry = fragmentRegistry;
        this._innerHTML = "";
        this.content = new FakeElement("fragment");
    }

    set innerHTML(value) {
        this._innerHTML = String(value ?? "");
        const mapped = this._fragmentRegistry?.get(this._innerHTML) || null;
        this.content.replaceChildren(...(mapped ? [mapped] : []));
    }

    get innerHTML() {
        return this._innerHTML;
    }

    cloneNode(deep = false) {
        const clone = new FakeTemplateElement(this._fragmentRegistry);
        clone.content = this.content.cloneNode(true);
        clone._innerHTML = this._innerHTML;
        if (deep) {
            for (const child of this.childNodes) {
                clone.appendChild(typeof child.cloneNode === "function" ? child.cloneNode(true) : child);
            }
        }
        for (const [name, value] of this.attributes) {
            clone.setAttribute(name, value);
        }
        return clone;
    }
}

function createEnvironment(options = {}) {
    const fragmentRegistry = options.fragmentRegistry || null;
    const document = new EventTarget();
    document.body = new FakeElement("body");
    document.documentElement = new FakeElement("html");
    document.createComment = () => new FakeComment();
    document.createElement = (tagName = "div") => (
        String(tagName).toLowerCase() === "template"
            ? new FakeTemplateElement(fragmentRegistry)
            : new FakeElement(tagName)
    );
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
    window.getComputedStyle = (element) => ({
        overflow: element?.style?.overflow ?? "",
        overflowX: element?.style?.overflowX ?? element?.style?.overflow ?? "",
        overflowY: element?.style?.overflowY ?? element?.style?.overflow ?? "",
    });
    window.requestAnimationFrame = (callback) => {
        callback();
        return 1;
    };
    window.cancelAnimationFrame = () => {};
    return { document, window, customElementsRegistry };
}

async function loadModule(options = {}) {
    const { document, window, customElementsRegistry } = createEnvironment(options);
    globalThis.HTMLElement = FakeElement;
    globalThis.Element = FakeElement;
    globalThis.Node = FakeElement;
    globalThis.document = document;
    globalThis.window = window;
    globalThis.customElements = customElementsRegistry;
    globalThis.getComputedStyle = window.getComputedStyle;
    globalThis.HTMLInputElement = FakeElement;
    globalThis.HTMLTemplateElement = FakeTemplateElement;
    return {
        ...(await import(`./searchable-dropdown.js?test=${Date.now()}-${Math.random()}`)),
        document,
        window,
        customElementsRegistry,
    };
}

function restoreGlobals() {
    globalThis.HTMLElement = originalHTMLElement;
    globalThis.Element = originalElement;
    globalThis.Node = originalNode;
    globalThis.document = originalDocument;
    globalThis.window = originalWindow;
    globalThis.customElements = originalCustomElements;
    globalThis.fetch = originalFetch;
    globalThis.getComputedStyle = originalGetComputedStyle;
    globalThis.HTMLInputElement = originalHTMLInputElement;
    globalThis.HTMLTemplateElement = originalHTMLTemplateElement;
}

function createDropdownOption(value, label) {
    const item = new FakeElement("li");
    const button = new FakeElement("button");
    const content = new FakeElement("span");

    button.setAttribute("data-searchable-dropdown-option", "");
    button.setAttribute("data-value", value);
    button.setAttribute("data-label", label);
    content.setAttribute("data-role", "option-content");
    content.textContent = label;
    button.append(content);
    item.append(button);
    return item;
}

function createMoreResultsRow(nextOffset, label = "Load more results") {
    const item = new FakeElement("li");
    const content = new FakeElement("span");

    item.setAttribute("data-searchable-dropdown-more", "");
    item.setAttribute("data-next-offset", String(nextOffset));
    content.textContent = label;
    item.append(content);
    return item;
}

function createResultsPage({
    count,
    start = 0,
    nextOffset = null,
    id = "results",
}) {
    const results = new FakeElement("ul");
    results.setAttribute("id", id);
    results.setAttribute("data-role", "results");
    results.clientHeight = 100;
    results.scrollHeight = 400;
    for (let index = 0; index < count; index += 1) {
        const value = `value-${start + index}`;
        results.append(createDropdownOption(value, `Option ${start + index}`));
    }
    if (nextOffset !== null) {
        results.setAttribute("data-next-offset", String(nextOffset));
        results.append(createMoreResultsRow(nextOffset));
    }
    return results;
}

function configureAutoSizingDropdownResults(results, { clientHeight, columns = 1, rowHeight = 24 }) {
    results.clientHeight = clientHeight;

    const recomputeScrollHeight = () => {
        const optionCount = results.querySelectorAll("[data-searchable-dropdown-option]").length;
        const moreRowCount = results.querySelectorAll("[data-searchable-dropdown-more]").length;
        const optionRows = optionCount > 0 ? Math.ceil(optionCount / Math.max(1, columns)) : 0;
        results.scrollHeight = Math.max(clientHeight, (optionRows + moreRowCount) * rowHeight);
    };

    const appendChild = results.appendChild.bind(results);
    results.appendChild = (child) => {
        const appended = appendChild(child);
        recomputeScrollHeight();
        return appended;
    };

    const replaceChildren = results.replaceChildren.bind(results);
    results.replaceChildren = (...nodes) => {
        replaceChildren(...nodes);
        recomputeScrollHeight();
    };

    recomputeScrollHeight();
    return results;
}

async function flushAsyncWork() {
    await Promise.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await Promise.resolve();
}

test("FishySearchableDropdown keeps its panel attached by default", async (t) => {
    t.after(restoreGlobals);

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
    await flushAsyncWork();

    dropdown.open();

    assert.equal(panel.parentNode, dropdown);
    assert.equal(panel.style.position, undefined);
    assert.equal(panel.hidden, false);
    assert.equal(dropdown.searchInputElement(), searchInput);
    assert.equal(dropdown.resultsElement(), results);

    dropdown.close();

    assert.equal(panel.parentNode, dropdown);
    assert.equal(panel.hidden, true);
});

test("FishySearchableDropdown detaches its panel to the document body in detached mode and restores it on close", async (t) => {
    t.after(restoreGlobals);

    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    dropdown.setAttribute("panel-mode", "detached");
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
    assert.equal(panel.hasAttribute("data-searchable-dropdown-panel"), true);
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

test("detached fixed panel uses viewport coordinates when the document is scrolled", async (t) => {
    t.after(restoreGlobals);

    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    document.documentElement.setRect({ left: 0, top: -420, width: 1280, height: 1800 });
    dropdown.setAttribute("panel-mode", "detached");
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

    assert.equal(panel.style.position, "fixed");
    assert.equal(panel.style.left, "240px");
    assert.equal(panel.style.top, "160px");
});

test("detached fixed panel ignores scroll events from inside its own results", async (t) => {
    t.after(restoreGlobals);

    const { FishySearchableDropdown, window } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    dropdown.setAttribute("panel-mode", "detached");
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

    let scheduledFrames = 0;
    window.requestAnimationFrame = (callback) => {
        scheduledFrames += 1;
        callback();
        return scheduledFrames;
    };

    dropdown._handleViewportChange({ type: "scroll", target: results });

    assert.equal(scheduledFrames, 0);

    dropdown._handleViewportChange({ type: "scroll", target: document });

    assert.equal(scheduledFrames, 1);
});

test("detached fixed panel stays inside the viewport when neither side fully fits", async (t) => {
    t.after(restoreGlobals);

    const { FishySearchableDropdown } = await loadModule();

    window.innerWidth = 390;
    window.innerHeight = 844;
    document.documentElement.clientWidth = 390;
    document.documentElement.clientHeight = 844;

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    dropdown.setAttribute("panel-mode", "detached");
    trigger.setRect({ left: 12, top: 459, width: 366, height: 45 });
    panel.setRect({ left: 0, top: 0, width: 366, height: 481 });
    results.setRect({ left: 0, top: 97, width: 366, height: 384 });
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

    assert.equal(panel.style.position, "fixed");
    assert.equal(panel.style.left, "12px");
    assert.equal(panel.style.top, "12px");
    assert.equal(panel.style.maxHeight, "439px");
    assert.equal(results.style.maxHeight, "342px");
    assert.equal(results.style.overflowY, "auto");
    assert.equal(results.style.overscrollBehavior, "contain");

    dropdown.close();

    assert.equal(panel.style.maxHeight, "");
    assert.equal(results.style.maxHeight, "");
    assert.equal(results.style.overflowY, "");
    assert.equal(results.style.overscrollBehavior, "");
});

test("searchable dropdown accepts valid ISO date custom options", async (t) => {
    t.after(restoreGlobals);

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
    t.after(restoreGlobals);

    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const anchor = new FakeElement();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    dropdown.setAttribute("panel-mode", "detached");
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

test("searchable dropdown can keep its panel minimum width when the anchor is narrower", async (t) => {
    t.after(restoreGlobals);

    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const anchor = new FakeElement();
    const trigger = new FakeElement();
    const panel = new FakeElement();
    const searchInput = new FakeElement();
    const results = new FakeElement();

    dropdown.setAttribute("panel-mode", "detached");
    dropdown.setAttribute("panel-anchor-closest", ".fishymap-date-term-content");
    dropdown.setAttribute("panel-min-width", "panel");
    dropdown.setClosest(".fishymap-date-term-content", anchor);
    anchor.setRect({ left: 180, top: 132, width: 220, height: 40 });
    trigger.setRect({ left: 248, top: 136, width: 156, height: 32 });
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
    assert.equal(panel.style.width, "288px");
});

test("searchable dropdown paginates local catalog results instead of silently truncating", async (t) => {
    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const results = new FakeElement("ul");
    const catalog = new FakeElement("div");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");
    results.clientHeight = 100;
    results.scrollHeight = 400;

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (let index = 0; index < 30; index += 1) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = `Option ${index}`;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", `value-${index}`);
        template.setAttribute("data-label", `Option ${index}`);
        template.setAttribute("data-search-text", `Option ${index}`);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 24);
    assert.equal(results.getAttribute("data-next-offset"), "24");
    assert.ok(results.querySelector("[data-searchable-dropdown-more]"));
    const firstOption = results.querySelector("[data-searchable-dropdown-option]");

    results.scrollTop = 320;
    dropdown._handleResultsScroll();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(results.querySelector("[data-searchable-dropdown-option]"), firstOption);
    assert.equal(results.getAttribute("data-next-offset"), null);
    assert.equal(results.querySelector("[data-searchable-dropdown-more]"), null);
});

test("searchable dropdown excludes values selected in sibling inputs from local results", async (t) => {
    const { FishySearchableDropdown, document } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const results = new FakeElement("ul");
    const catalog = new FakeElement("div");
    const ownInput = new FakeElement("input");
    const siblingInput = new FakeElement("input");

    ownInput.value = "skill-a";
    siblingInput.value = "skill-b";
    document.getElementById = (id) => (id === "skill-slot-one" ? ownInput : null);
    document.querySelectorAll = (selector) => (
        selector === '[data-pet-skill-input-group="pet1"]'
            ? [ownInput, siblingInput]
            : []
    );

    dropdown.setAttribute("input-id", "skill-slot-one");
    dropdown.setAttribute("exclude-selected-inputs", '[data-pet-skill-input-group="pet1"]');
    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (const [value, label] of [
        ["skill-a", "Skill A"],
        ["skill-b", "Skill B"],
        ["skill-c", "Skill C"],
    ]) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = label;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", value);
        template.setAttribute("data-label", label);
        template.setAttribute("data-search-text", label);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");

    assert.deepEqual(
        Array.from(results.querySelectorAll("[data-searchable-dropdown-option]"), (option) => option.getAttribute("data-value")),
        ["skill-a", "skill-c"],
    );

    siblingInput.value = "skill-c";
    dropdown.search("");

    assert.deepEqual(
        Array.from(results.querySelectorAll("[data-searchable-dropdown-option]"), (option) => option.getAttribute("data-value")),
        ["skill-a", "skill-b"],
    );
});

test("searchable dropdown loads more when an ancestor is the actual scroll host", async (t) => {
    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const scrollShell = new FakeElement("div");
    const results = new FakeElement("ul");
    const catalog = new FakeElement("div");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");
    scrollShell.style.overflowY = "auto";
    scrollShell.clientHeight = 100;
    scrollShell.scrollHeight = 400;
    results.clientHeight = 100;
    results.scrollHeight = 100;

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(scrollShell);
    scrollShell.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (let index = 0; index < 30; index += 1) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = `Option ${index}`;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", `value-${index}`);
        template.setAttribute("data-label", `Option ${index}`);
        template.setAttribute("data-search-text", `Option ${index}`);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 24);
    assert.equal(results.getAttribute("data-next-offset"), "24");

    scrollShell.scrollTop = 320;
    dropdown._handleResultsScroll();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(results.getAttribute("data-next-offset"), null);
});

test("searchable dropdown loads more when horizontal overflow reaches the end", async (t) => {
    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const results = new FakeElement("ul");
    const catalog = new FakeElement("div");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");
    results.style.overflowX = "auto";
    results.clientHeight = 100;
    results.scrollHeight = 100;
    results.clientWidth = 200;
    results.scrollWidth = 520;

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (let index = 0; index < 30; index += 1) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = `Option ${index}`;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", `value-${index}`);
        template.setAttribute("data-label", `Option ${index}`);
        template.setAttribute("data-search-text", `Option ${index}`);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 24);
    assert.equal(results.getAttribute("data-next-offset"), "24");

    results.scrollLeft = 320;
    dropdown._handleResultsScroll();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(results.getAttribute("data-next-offset"), null);
});

test("searchable dropdown auto-loads one extra local page when the first page does not scroll", async (t) => {
    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const results = new FakeElement("ul");
    const catalog = new FakeElement("div");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");
    results.clientHeight = 400;
    results.scrollHeight = 400;

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (let index = 0; index < 30; index += 1) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = `Option ${index}`;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", `value-${index}`);
        template.setAttribute("data-label", `Option ${index}`);
        template.setAttribute("data-search-text", `Option ${index}`);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");
    await flushAsyncWork();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(results.getAttribute("data-next-offset"), null);
    assert.equal(results.querySelector("[data-searchable-dropdown-more]"), null);
});

test("searchable dropdown stops after one extra auto-fill when results still do not scroll", async (t) => {
    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const results = configureAutoSizingDropdownResults(new FakeElement("ul"), {
        clientHeight: 390,
        columns: 4,
        rowHeight: 30,
    });
    const catalog = new FakeElement("div");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (let index = 0; index < 90; index += 1) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = `Option ${index}`;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", `value-${index}`);
        template.setAttribute("data-label", `Option ${index}`);
        template.setAttribute("data-search-text", `Option ${index}`);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");
    await flushAsyncWork();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 48);
    assert.equal(results.getAttribute("data-next-offset"), "48");
    assert.equal(results.scrollHeight, results.clientHeight);
    assert.ok(results.querySelector("[data-searchable-dropdown-more]"));
});

test("searchable dropdown does not treat the top of a short scroll range as near-end", async (t) => {
    const { FishySearchableDropdown } = await loadModule();

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const results = new FakeElement("ul");
    const catalog = new FakeElement("div");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    results.setAttribute("data-role", "results");
    catalog.setAttribute("data-role", "selected-content-catalog");
    results.clientHeight = 360;
    results.scrollHeight = 420;

    dropdown.append(trigger);
    dropdown.append(panel);
    dropdown.append(catalog);
    panel.append(searchInput);
    panel.append(results);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    for (let index = 0; index < 30; index += 1) {
        const template = new FakeTemplateElement();
        const content = new FakeElement("span");
        content.textContent = `Option ${index}`;
        template.setAttribute("data-role", "selected-content");
        template.setAttribute("data-value", `value-${index}`);
        template.setAttribute("data-label", `Option ${index}`);
        template.setAttribute("data-search-text", `Option ${index}`);
        template.content.append(content);
        catalog.append(template);
    }

    dropdown.connectedCallback();
    await Promise.resolve();

    dropdown.search("");
    dropdown._handleResultsScroll();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 24);
    assert.equal(results.getAttribute("data-next-offset"), "24");

    results.scrollTop = 50;
    dropdown._handleResultsScroll();

    assert.equal(results.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(results.getAttribute("data-next-offset"), null);
});

test("searchable dropdown loads the next remote page when scrolling near the end", async (t) => {
    const fragmentRegistry = new Map();
    const { FishySearchableDropdown } = await loadModule({ fragmentRegistry });

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const initialResults = new FakeElement("ul");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    initialResults.setAttribute("data-role", "results");
    initialResults.clientHeight = 100;
    initialResults.scrollHeight = 400;

    dropdown.setAttribute("search-url", "/api/search");
    dropdown.append(trigger);
    dropdown.append(panel);
    panel.append(searchInput);
    panel.append(initialResults);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    const page1 = createResultsPage({
        count: 24,
        nextOffset: 24,
        id: "calculator-rod-picker-results",
    });
    const page2 = createResultsPage({
        count: 6,
        start: 24,
        id: "calculator-rod-picker-results",
    });
    fragmentRegistry.set("PAGE1", page1);
    fragmentRegistry.set("PAGE2", page2);

    const fetchCalls = [];
    globalThis.fetch = async (url) => {
        fetchCalls.push(String(url));
        return {
            ok: true,
            async text() {
                return fetchCalls.length === 1 ? "PAGE1" : "PAGE2";
            },
        };
    };

    dropdown.connectedCallback();
    await flushAsyncWork();

    dropdown.search("rod");
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 1);
    assert.match(fetchCalls[0], /q=rod/);
    assert.doesNotMatch(fetchCalls[0], /offset=/);
    assert.equal(dropdown.resultsElement(), page1);
    assert.equal(page1.querySelectorAll("[data-searchable-dropdown-option]").length, 24);
    assert.equal(page1.getAttribute("data-next-offset"), "24");
    assert.ok(page1.querySelector("[data-searchable-dropdown-more]"));
    const firstOption = page1.querySelector("[data-searchable-dropdown-option]");

    page1.scrollTop = 320;
    dropdown._handleResultsScroll();
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 2);
    assert.match(fetchCalls[1], /q=rod/);
    assert.match(fetchCalls[1], /offset=24/);
    assert.equal(page1.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(page1.querySelector("[data-searchable-dropdown-option]"), firstOption);
    assert.equal(page1.getAttribute("data-next-offset"), null);
    assert.equal(page1.querySelector("[data-searchable-dropdown-more]"), null);
});

test("searchable dropdown reuses rendered remote results for the blank query", async (t) => {
    const fragmentRegistry = new Map();
    const { FishySearchableDropdown } = await loadModule({ fragmentRegistry });

    const dropdown = new FishySearchableDropdown();
    dropdown.isConnected = true;
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const page1 = createResultsPage({
        count: 24,
        nextOffset: 24,
        id: "calculator-zone-picker-results",
    });

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");

    dropdown.setAttribute("search-url", "/api/search");
    dropdown.append(trigger);
    dropdown.append(panel);
    panel.append(searchInput);
    panel.append(page1);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    const page2 = createResultsPage({
        count: 6,
        start: 24,
        id: "calculator-zone-picker-results",
    });
    fragmentRegistry.set("PAGE2", page2);

    const fetchCalls = [];
    globalThis.fetch = async (url) => {
        fetchCalls.push(String(url));
        return {
            ok: true,
            async text() {
                return "PAGE2";
            },
        };
    };

    dropdown.connectedCallback();
    await flushAsyncWork();

    dropdown.search("");
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 0);
    assert.equal(dropdown.resultsElement(), page1);
    assert.equal(page1.querySelectorAll("[data-searchable-dropdown-option]").length, 24);

    page1.scrollTop = 320;
    dropdown._handleResultsScroll();
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 1);
    assert.match(fetchCalls[0], /offset=24/);
    assert.equal(page1.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(page1.getAttribute("data-next-offset"), null);
});

test("searchable dropdown auto-loads one extra remote page when the first page does not scroll", async (t) => {
    const fragmentRegistry = new Map();
    const { FishySearchableDropdown } = await loadModule({ fragmentRegistry });

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const initialResults = new FakeElement("ul");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    initialResults.setAttribute("data-role", "results");
    initialResults.clientHeight = 400;
    initialResults.scrollHeight = 400;

    dropdown.setAttribute("search-url", "/api/search");
    dropdown.append(trigger);
    dropdown.append(panel);
    panel.append(searchInput);
    panel.append(initialResults);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    const page1 = createResultsPage({
        count: 24,
        nextOffset: 24,
        id: "calculator-rod-picker-results",
    });
    page1.clientHeight = 400;
    page1.scrollHeight = 400;
    const page2 = createResultsPage({
        count: 6,
        start: 24,
        id: "calculator-rod-picker-results",
    });
    fragmentRegistry.set("PAGE1", page1);
    fragmentRegistry.set("PAGE2", page2);

    const fetchCalls = [];
    globalThis.fetch = async (url) => {
        fetchCalls.push(String(url));
        return {
            ok: true,
            async text() {
                return fetchCalls.length === 1 ? "PAGE1" : "PAGE2";
            },
        };
    };

    dropdown.connectedCallback();
    await flushAsyncWork();

    await flushAsyncWork();
    dropdown.search("rod");
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 2);
    assert.equal(page1.querySelectorAll("[data-searchable-dropdown-option]").length, 30);
    assert.equal(page1.getAttribute("data-next-offset"), null);
    assert.equal(page1.querySelector("[data-searchable-dropdown-more]"), null);
});

test("searchable dropdown does not treat the top of a short remote scroll range as near-end", async (t) => {
    const fragmentRegistry = new Map();
    const { FishySearchableDropdown } = await loadModule({ fragmentRegistry });

    const dropdown = new FishySearchableDropdown();
    const trigger = new FakeElement("button");
    const panel = new FakeElement("div");
    const searchInput = new FakeElement("input");
    const initialResults = new FakeElement("ul");

    trigger.setAttribute("data-role", "trigger");
    panel.setAttribute("data-role", "panel");
    searchInput.setAttribute("data-role", "search-input");
    initialResults.setAttribute("data-role", "results");
    initialResults.clientHeight = 360;
    initialResults.scrollHeight = 420;

    dropdown.setAttribute("search-url", "/api/search");
    dropdown.append(trigger);
    dropdown.append(panel);
    panel.append(searchInput);
    panel.append(initialResults);
    t.after(() => {
        dropdown.disconnectedCallback();
        restoreGlobals();
    });

    const page1 = createResultsPage({
        count: 24,
        nextOffset: 24,
        id: "calculator-rod-picker-results",
    });
    page1.clientHeight = 360;
    page1.scrollHeight = 420;
    const page2 = createResultsPage({
        count: 6,
        start: 24,
        id: "calculator-rod-picker-results",
    });
    fragmentRegistry.set("PAGE1", page1);
    fragmentRegistry.set("PAGE2", page2);

    const fetchCalls = [];
    globalThis.fetch = async (url) => {
        fetchCalls.push(String(url));
        return {
            ok: true,
            async text() {
                return fetchCalls.length === 1 ? "PAGE1" : "PAGE2";
            },
        };
    };

    dropdown.connectedCallback();
    await flushAsyncWork();

    dropdown.search("rod");
    await flushAsyncWork();
    dropdown._handleResultsScroll();
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 1);

    page1.scrollTop = 50;
    dropdown._handleResultsScroll();
    await flushAsyncWork();

    assert.equal(fetchCalls.length, 2);
    assert.match(fetchCalls[1], /offset=24/);
});
