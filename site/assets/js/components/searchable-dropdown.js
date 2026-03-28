const CLOSE_DELAY_MS = 150;
const LOCAL_RESULT_LIMIT = 24;
const SEARCH_QUERY_PARAM = "q";
const SELECTED_QUERY_PARAM = "selected";
const URL_SCOPE_RESOLVERS = Object.freeze({
    api: "__fishystuffResolveApiUrl",
    site: "__fishystuffResolveSiteUrl",
    cdn: "__fishystuffResolveCdnUrl",
});

export function getStringAttribute(element, name) {
    return String(element.getAttribute(name) ?? "").trim();
}

export function setStringAttribute(element, name, value) {
    const normalized = String(value ?? "");
    if (normalized) {
        element.setAttribute(name, normalized);
        return;
    }
    element.removeAttribute(name);
}

export function upgradeProperty(element, property) {
    if (!Object.prototype.hasOwnProperty.call(element, property)) {
        return;
    }
    const value = element[property];
    delete element[property];
    element[property] = value;
}

export function dispatchValueEvents(element) {
    element.dispatchEvent(new Event("input", { bubbles: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
}

export function cloneChildNodes(source) {
    return Array.from(source.childNodes, (node) => node.cloneNode(true));
}

export function findPropertyDescriptor(target, property) {
    let current = target;
    while (current) {
        const descriptor = Object.getOwnPropertyDescriptor(current, property);
        if (descriptor) {
            return descriptor;
        }
        current = Object.getPrototypeOf(current);
    }
    return null;
}

export function normalizeSearchText(value) {
    return String(value ?? "").trim().toLowerCase();
}

export function resolvePublicAssetUrl(rawUrl) {
    const normalizedUrl = String(rawUrl ?? "").trim();
    if (!normalizedUrl) {
        return "";
    }
    if (
        normalizedUrl.startsWith("http://")
        || normalizedUrl.startsWith("https://")
        || normalizedUrl.startsWith("data:")
    ) {
        return normalizedUrl;
    }
    if (
        normalizedUrl.startsWith("/images/")
        || normalizedUrl.startsWith("images/")
        || normalizedUrl.startsWith("/fields/")
        || normalizedUrl.startsWith("fields/")
        || normalizedUrl.startsWith("/region_groups/")
        || normalizedUrl.startsWith("region_groups/")
    ) {
        return resolveScopedUrl(normalizedUrl, "cdn");
    }
    return normalizedUrl;
}

export function materializePublicAssetUrls(root) {
    if (!(root instanceof Element || root instanceof DocumentFragment)) {
        return;
    }

    if (root instanceof Element && root.matches("img[data-public-src]")) {
        const resolvedSrc = resolvePublicAssetUrl(root.getAttribute("data-public-src"));
        if (resolvedSrc) {
            root.setAttribute("src", resolvedSrc);
        }
    }

    for (const image of root.querySelectorAll("img[data-public-src]")) {
        const resolvedSrc = resolvePublicAssetUrl(image.getAttribute("data-public-src"));
        if (resolvedSrc) {
            image.setAttribute("src", resolvedSrc);
        }
    }
}

function parseHtmlFragment(html) {
    const template = document.createElement("template");
    template.innerHTML = String(html ?? "");
    materializePublicAssetUrls(template.content);
    return template.content;
}

export function resolveScopedUrl(rawUrl, scope) {
    const normalizedUrl = String(rawUrl ?? "").trim();
    if (!normalizedUrl) {
        return "";
    }
    if (
        normalizedUrl.startsWith("http://")
        || normalizedUrl.startsWith("https://")
        || normalizedUrl.startsWith("data:")
    ) {
        return normalizedUrl;
    }

    const resolverName = URL_SCOPE_RESOLVERS[String(scope ?? "").trim()];
    if (resolverName && typeof window[resolverName] === "function") {
        return window[resolverName](normalizedUrl);
    }

    return new URL(normalizedUrl, window.location.href).toString();
}

export class FishySearchableDropdown extends HTMLElement {
    static get observedAttributes() {
        return [
            "input-id",
            "label",
            "placeholder",
            "search-url",
            "search-url-root",
            "value",
        ];
    }

    constructor() {
        super();
        this._boundInput = null;
        this._closeTimer = 0;
        this._lastSearchKey = "";
        this._outsideListenerAttached = false;
        this._isReflectingBoundInputValue = false;
        this._isWritingBoundInputValue = false;
        this._releaseBoundInput = null;
        this._searchController = null;

        this._handleBoundInputEvent = this._handleBoundInputEvent.bind(this);
        this._handleClick = this._handleClick.bind(this);
        this._handleDocumentPointerDown = this._handleDocumentPointerDown.bind(this);
        this._handleFocusIn = this._handleFocusIn.bind(this);
        this._handleFocusOut = this._handleFocusOut.bind(this);
        this._handleInput = this._handleInput.bind(this);
        this._handleKeyDown = this._handleKeyDown.bind(this);
        this._handleMouseDown = this._handleMouseDown.bind(this);

        this.addEventListener("click", this._handleClick);
        this.addEventListener("focusin", this._handleFocusIn);
        this.addEventListener("focusout", this._handleFocusOut);
        this.addEventListener("input", this._handleInput);
        this.addEventListener("keydown", this._handleKeyDown);
        this.addEventListener("mousedown", this._handleMouseDown);
    }

    get inputId() {
        return getStringAttribute(this, "input-id");
    }

    set inputId(value) {
        setStringAttribute(this, "input-id", value);
    }

    get label() {
        return getStringAttribute(this, "label");
    }

    set label(value) {
        setStringAttribute(this, "label", value);
    }

    get placeholder() {
        return getStringAttribute(this, "placeholder");
    }

    set placeholder(value) {
        setStringAttribute(this, "placeholder", value);
    }

    get searchUrl() {
        return getStringAttribute(this, "search-url");
    }

    set searchUrl(value) {
        setStringAttribute(this, "search-url", value);
    }

    get searchUrlRoot() {
        return getStringAttribute(this, "search-url-root");
    }

    set searchUrlRoot(value) {
        setStringAttribute(this, "search-url-root", value);
    }

    get value() {
        return getStringAttribute(this, "value");
    }

    set value(value) {
        setStringAttribute(this, "value", value);
    }

    connectedCallback() {
        upgradeProperty(this, "inputId");
        upgradeProperty(this, "label");
        upgradeProperty(this, "placeholder");
        upgradeProperty(this, "searchUrl");
        upgradeProperty(this, "searchUrlRoot");
        upgradeProperty(this, "value");
        this._bindBoundInput();

        queueMicrotask(() => {
            if (!this.isConnected) {
                return;
            }
            this._syncUi();
            this._syncBoundInputValue(this.value, false);
            this._syncFromBoundInputValue(this.boundInputElement()?.value ?? this.value);
        });
    }

    disconnectedCallback() {
        this._unbindBoundInput();
        this._cancelClose();
        this._abortSearch();
        this._detachOutsideListener();
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (oldValue === newValue) {
            return;
        }
        if (name === "input-id") {
            this._bindBoundInput();
        }
        if (name === "value") {
            this._lastSearchKey = "";
            if (!this._isReflectingBoundInputValue) {
                this._syncBoundInputValue(this.value, false);
            }
        }
        this._syncUi();
    }

    open() {
        const panel = this.panelElement();
        const input = this.searchInputElement();
        if (!(panel instanceof HTMLElement) || !(input instanceof HTMLInputElement)) {
            return;
        }

        this._cancelClose();
        panel.hidden = false;
        this.style.zIndex = "60";
        this._setExpanded(true);
        this._attachOutsideListener();
        input.value = "";
        this._lastSearchKey = "";

        window.requestAnimationFrame(() => {
            if (!this.isConnected || !this.isOpen()) {
                return;
            }
            input.focus();
            this.search("");
        });
    }

    close() {
        this._cancelClose();

        const panel = this.panelElement();
        if (panel instanceof HTMLElement) {
            panel.hidden = true;
        }

        this.style.zIndex = "";
        this._abortSearch();
        this._detachOutsideListener();
        this._setExpanded(false);
    }

    refreshResults() {
        this._lastSearchKey = "";
        this.search(this.searchInputElement()?.value ?? "");
    }

    toggle() {
        if (this.isOpen()) {
            this.close();
            return;
        }
        this.open();
    }

    search(rawQuery) {
        const query = String(rawQuery ?? "");
        const results = this.resultsElement();
        if (!(results instanceof HTMLElement)) {
            return;
        }

        const selectedValue = String(
            this.boundInputElement()?.value ?? this.value,
        );
        const searchKey = `${selectedValue}\n${query}`;
        if (this._lastSearchKey === searchKey) {
            return;
        }

        this._cancelClose();
        this._lastSearchKey = searchKey;
        this._abortSearch();

        const searchUrl = this._buildSearchUrl(query);
        if (!searchUrl) {
            this._renderLocalResults(query, selectedValue);
            return;
        }

        const controller = new AbortController();
        this._searchController = controller;
        fetch(searchUrl, {
            cache: "no-store",
            headers: {
                Accept: "text/html",
            },
            signal: controller.signal,
        })
            .then((response) => {
                if (!response.ok) {
                    throw new Error(`Search request failed: ${response.status}`);
                }
                return response.text();
            })
            .then((html) => {
                if (controller.signal.aborted) {
                    return;
                }

                const currentResults = this.resultsElement();
                if (!(currentResults instanceof HTMLElement)) {
                    return;
                }

                const fragment = parseHtmlFragment(html);
                const nextResults = fragment.firstElementChild;
                if (!(nextResults instanceof HTMLElement)) {
                    return;
                }
                currentResults.replaceWith(nextResults);
            })
            .catch((error) => {
                if (error?.name === "AbortError") {
                    return;
                }
                console.error("Error loading searchable dropdown results:", error);
            })
            .finally(() => {
                if (this._searchController === controller) {
                    this._searchController = null;
                }
            });
    }

    select(value, label) {
        this.value = value;
        this.label = label;
        this._syncBoundInputValue(value, true);

        const input = this.searchInputElement();
        if (input instanceof HTMLInputElement) {
            input.value = "";
        }

        this._lastSearchKey = "";
        this.dispatchEvent(new Event("input", { bubbles: true }));
        this.dispatchEvent(new Event("change", { bubbles: true }));
        this.close();
    }

    boundInputElement() {
        if (!this.inputId) {
            return null;
        }
        return document.getElementById(this.inputId);
    }

    isOpen() {
        const panel = this.panelElement();
        return panel instanceof HTMLElement ? !panel.hidden : false;
    }

    panelElement() {
        return this.querySelector('[data-role="panel"]');
    }

    resultsElement() {
        return this.querySelector('[data-role="results"]');
    }

    searchInputElement() {
        return this.querySelector('[data-role="search-input"]');
    }

    selectedLabelElement() {
        return this.querySelector('[data-role="selected-label"]');
    }

    selectedContentElement() {
        return this.querySelector('[data-role="selected-content"]');
    }

    selectedContentCatalogElement() {
        return this.querySelector('[data-role="selected-content-catalog"]');
    }

    triggerElement() {
        return this.querySelector('[data-role="trigger"]');
    }

    _bindBoundInput() {
        const input = this.boundInputElement();
        if (this._boundInput === input) {
            return;
        }

        this._unbindBoundInput();
        if (!(input instanceof HTMLInputElement)) {
            return;
        }

        input.addEventListener("input", this._handleBoundInputEvent);
        input.addEventListener("change", this._handleBoundInputEvent);

        let releaseValueObserver = () => {};
        if (!Object.prototype.hasOwnProperty.call(input, "value")) {
            const descriptor = findPropertyDescriptor(input, "value");
            if (descriptor?.get && descriptor?.set) {
                Object.defineProperty(input, "value", {
                    configurable: true,
                    enumerable: descriptor.enumerable ?? true,
                    get() {
                        return descriptor.get.call(this);
                    },
                    set: (nextValue) => {
                        const previousValue = descriptor.get.call(input);
                        descriptor.set.call(input, nextValue);
                        const currentValue = descriptor.get.call(input);
                        if (
                            this._isWritingBoundInputValue
                            || currentValue === previousValue
                        ) {
                            return;
                        }
                        this._syncFromBoundInputValue(currentValue);
                    },
                });
                releaseValueObserver = () => {
                    delete input.value;
                };
            }
        }

        this._boundInput = input;
        this._releaseBoundInput = () => {
            input.removeEventListener("input", this._handleBoundInputEvent);
            input.removeEventListener("change", this._handleBoundInputEvent);
            releaseValueObserver();
            this._boundInput = null;
            this._releaseBoundInput = null;
        };
    }

    _abortSearch() {
        if (!this._searchController) {
            return;
        }
        this._searchController.abort();
        this._searchController = null;
    }

    _attachOutsideListener() {
        if (this._outsideListenerAttached) {
            return;
        }
        document.addEventListener("pointerdown", this._handleDocumentPointerDown, true);
        this._outsideListenerAttached = true;
    }

    _buildSearchUrl(query) {
        const resolved = resolveScopedUrl(this.searchUrl, this.searchUrlRoot);
        if (!resolved) {
            return "";
        }

        let url;
        try {
            url = new URL(resolved, window.location.href);
        } catch (_) {
            return "";
        }

        const normalizedQuery = String(query ?? "").trim();
        const selectedValue = String(
            this.boundInputElement()?.value ?? this.value ?? "",
        ).trim();

        if (normalizedQuery) {
            url.searchParams.set(SEARCH_QUERY_PARAM, normalizedQuery);
        } else {
            url.searchParams.delete(SEARCH_QUERY_PARAM);
        }

        if (selectedValue) {
            url.searchParams.set(SELECTED_QUERY_PARAM, selectedValue);
        } else {
            url.searchParams.delete(SELECTED_QUERY_PARAM);
        }

        return url.toString();
    }

    _cancelClose() {
        if (!this._closeTimer) {
            return;
        }
        window.clearTimeout(this._closeTimer);
        this._closeTimer = 0;
    }

    _detachOutsideListener() {
        if (!this._outsideListenerAttached) {
            return;
        }
        document.removeEventListener("pointerdown", this._handleDocumentPointerDown, true);
        this._outsideListenerAttached = false;
    }

    _findCatalogTemplateByValue(value) {
        const catalog = this.selectedContentCatalogElement();
        if (!(catalog instanceof HTMLElement)) {
            return null;
        }
        return Array.from(
            catalog.querySelectorAll('template[data-role="selected-content"]'),
        ).find((template) => template.getAttribute("data-value") === value) ?? null;
    }

    _findOptionByValue(value) {
        return Array.from(
            this.querySelectorAll("[data-searchable-dropdown-option]"),
        ).find((option) => option.getAttribute("data-value") === value) ?? null;
    }

    _catalogTemplates() {
        const catalog = this.selectedContentCatalogElement();
        if (!(catalog instanceof HTMLElement)) {
            return [];
        }
        return Array.from(catalog.querySelectorAll('template[data-role="selected-content"]'));
    }

    _handleBoundInputEvent() {
        this._syncFromBoundInputValue(this.boundInputElement()?.value ?? "");
    }

    _handleClick(event) {
        if (!(event.target instanceof Element)) {
            return;
        }

        const trigger = event.target.closest('[data-role="trigger"]');
        if (trigger && this.contains(trigger)) {
            event.preventDefault();
            this.toggle();
            return;
        }

        const option = event.target.closest("[data-searchable-dropdown-option]");
        if (!option || !this.contains(option)) {
            return;
        }

        event.preventDefault();
        const value = String(option.getAttribute("data-value") ?? "");
        const label = String(option.getAttribute("data-label") ?? option.textContent ?? "").trim();
        this._syncSelectedContentFromOption(option, label);
        this.select(value, label);
    }

    _handleDocumentPointerDown(event) {
        if (!(event.target instanceof Node) || this.contains(event.target)) {
            return;
        }
        this.close();
    }

    _handleFocusIn(event) {
        if (this.contains(event.target)) {
            this._cancelClose();
        }
    }

    _handleFocusOut(event) {
        if (!this.isOpen()) {
            return;
        }

        const nextTarget = event.relatedTarget;
        if (nextTarget instanceof Node && this.contains(nextTarget)) {
            return;
        }

        this._scheduleClose();
    }

    _handleInput(event) {
        if (event.target !== this.searchInputElement()) {
            return;
        }
        this.search(event.target.value);
    }

    _handleKeyDown(event) {
        if (event.key !== "Escape" || !this.contains(event.target)) {
            return;
        }

        event.preventDefault();
        this.close();

        const trigger = this.triggerElement();
        if (trigger instanceof HTMLElement) {
            trigger.focus();
        }
    }

    _handleMouseDown(event) {
        if (!(event.target instanceof Element) || !this.contains(event.target)) {
            return;
        }

        this._cancelClose();

        const option = event.target.closest("[data-searchable-dropdown-option]");
        if (option && this.contains(option)) {
            event.preventDefault();
        }
    }

    _scheduleClose() {
        this._cancelClose();
        this._closeTimer = window.setTimeout(() => {
            this._closeTimer = 0;
            this.close();
        }, CLOSE_DELAY_MS);
    }

    _setExpanded(isExpanded) {
        const trigger = this.triggerElement();
        if (trigger instanceof HTMLElement) {
            trigger.setAttribute("aria-expanded", isExpanded ? "true" : "false");
        }
    }

    _localTemplateMatches(template, normalizedQuery) {
        if (!normalizedQuery) {
            return true;
        }
        const haystack = normalizeSearchText(
            [
                template.getAttribute("data-search-text"),
                template.getAttribute("data-label"),
                template.textContent,
            ]
                .filter(Boolean)
                .join(" "),
        );
        return normalizedQuery
            .split(/\s+/)
            .filter(Boolean)
            .every((part) => haystack.includes(part));
    }

    _renderLocalResults(rawQuery, selectedValue) {
        const results = this.resultsElement();
        if (!(results instanceof HTMLElement)) {
            return;
        }

        const templates = this._catalogTemplates();
        if (!templates.length) {
            return;
        }

        const normalizedQuery = normalizeSearchText(rawQuery);
        const matches = templates
            .filter((template) => this._localTemplateMatches(template, normalizedQuery))
            .sort((left, right) => {
                const leftValue = String(left.getAttribute("data-value") ?? "");
                const rightValue = String(right.getAttribute("data-value") ?? "");
                const leftSelected = leftValue === selectedValue ? 0 : 1;
                const rightSelected = rightValue === selectedValue ? 0 : 1;
                return leftSelected - rightSelected;
            })
            .slice(0, LOCAL_RESULT_LIMIT);

        if (!matches.length) {
            const item = document.createElement("li");
            item.className = "menu-disabled";
            const label = document.createElement("span");
            label.textContent = "No matching options";
            item.append(label);
            results.replaceChildren(item);
            return;
        }

        results.replaceChildren(
            ...matches.map((template) => {
                const value = String(template.getAttribute("data-value") ?? "");
                const label = String(template.getAttribute("data-label") ?? value).trim();
                const item = document.createElement("li");
                const button = document.createElement("button");
                button.type = "button";
                button.className = `justify-between gap-3 text-left${value === selectedValue ? " menu-active" : ""}`;
                button.dataset.searchableDropdownOption = "";
                button.setAttribute("data-value", value);
                button.setAttribute("data-label", label);

                const optionContent = document.createElement("span");
                optionContent.dataset.role = "option-content";
                optionContent.className = "flex min-w-0 flex-1 items-center gap-3";
                optionContent.replaceChildren(...cloneChildNodes(template.content));
                materializePublicAssetUrls(optionContent);
                button.append(optionContent);

                if (value === selectedValue) {
                    const badge = document.createElement("span");
                    badge.className = "badge badge-soft badge-primary badge-xs";
                    badge.textContent = "Selected";
                    button.append(badge);
                }

                item.append(button);
                return item;
            }),
        );
    }

    _syncFromBoundInputValue(rawValue) {
        const value = String(rawValue ?? "");
        this._isReflectingBoundInputValue = true;
        this.value = value;
        this._isReflectingBoundInputValue = false;

        const option = this._findOptionByValue(value);
        if (option instanceof HTMLElement) {
            const label = String(
                option.getAttribute("data-label") ?? option.textContent ?? "",
            ).trim();
            if (label) {
                this.label = label;
            }
            this._syncSelectedContentFromOption(option, label);
            return;
        }

        const template = this._findCatalogTemplateByValue(value);
        if (template instanceof HTMLTemplateElement) {
            const label = String(template.getAttribute("data-label") ?? value).trim();
            if (label) {
                this.label = label;
            }
            const container = this.selectedContentElement();
            if (container instanceof HTMLElement) {
                container.replaceChildren(...cloneChildNodes(template.content));
                materializePublicAssetUrls(container);
            }
        }
    }

    _syncSelectedContentFromOption(option, fallbackLabel) {
        const container = this.selectedContentElement();
        if (!(container instanceof HTMLElement)) {
            const labelNode = this.selectedLabelElement();
            if (labelNode instanceof HTMLElement) {
                labelNode.textContent = fallbackLabel;
            }
            return;
        }

        const selectedTemplate = option.querySelector('template[data-role="selected-content"]');
        if (selectedTemplate instanceof HTMLTemplateElement) {
            container.replaceChildren(...cloneChildNodes(selectedTemplate.content));
            materializePublicAssetUrls(container);
            return;
        }

        const optionContent = option.querySelector('[data-role="option-content"]');
        if (optionContent instanceof HTMLElement) {
            container.replaceChildren(...cloneChildNodes(optionContent));
            materializePublicAssetUrls(container);
            return;
        }

        container.textContent = fallbackLabel;
    }

    _syncBoundInputValue(value, emitEvents) {
        const input = this.boundInputElement();
        if (!(input instanceof HTMLInputElement)) {
            return;
        }

        this._isWritingBoundInputValue = true;
        input.value = String(value ?? "");
        if (emitEvents) {
            dispatchValueEvents(input);
        }
        this._isWritingBoundInputValue = false;
    }

    _syncUi() {
        const selectedContent = this.selectedContentElement();
        if (
            selectedContent instanceof HTMLElement
            && !selectedContent.childNodes.length
            && this.hasAttribute("label")
        ) {
            selectedContent.textContent = this.label;
        }

        const labelNode = this.selectedLabelElement();
        if (labelNode instanceof HTMLElement && this.hasAttribute("label")) {
            labelNode.textContent = this.label;
        }

        const searchInput = this.searchInputElement();
        if (searchInput instanceof HTMLInputElement && this.hasAttribute("placeholder")) {
            searchInput.placeholder = this.placeholder;
        }

        this._setExpanded(this.isOpen());
        materializePublicAssetUrls(this);
    }

    _unbindBoundInput() {
        if (typeof this._releaseBoundInput === "function") {
            this._releaseBoundInput();
        }
    }
}

export function registerSearchableDropdown() {
    if (window.customElements.get("fishy-searchable-dropdown")) {
        return;
    }
    window.customElements.define(
        "fishy-searchable-dropdown",
        FishySearchableDropdown,
    );
}
