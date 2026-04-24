const CLOSE_DELAY_MS = 150;
const RESULTS_PAGE_SIZE = 24;
const LOAD_MORE_THRESHOLD_PX = 96;
const LOAD_MORE_THRESHOLD_RATIO = 0.25;
const DEFAULT_MORE_RESULTS_LABEL = "Load more results";
const SEARCH_QUERY_PARAM = "q";
const OFFSET_QUERY_PARAM = "offset";
const SELECTED_QUERY_PARAM = "selected";
export const SEARCHABLE_DROPDOWN_OPEN_EVENT = "fishystuff:searchable-dropdown-open";
export const SEARCHABLE_DROPDOWN_CLOSE_EVENT = "fishystuff:searchable-dropdown-close";
const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;
const HTMLElementBase = globalThis.HTMLElement ?? class {};
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

export function normalizeIsoDateValue(value) {
    const normalized = String(value ?? "").trim();
    if (!ISO_DATE_PATTERN.test(normalized)) {
        return "";
    }
    const [yearRaw, monthRaw, dayRaw] = normalized.split("-");
    const year = Number.parseInt(yearRaw, 10);
    const month = Number.parseInt(monthRaw, 10);
    const day = Number.parseInt(dayRaw, 10);
    if (!Number.isInteger(year) || !Number.isInteger(month) || !Number.isInteger(day)) {
        return "";
    }
    const date = new Date(Date.UTC(year, month - 1, day));
    if (Number.isNaN(date.getTime())) {
        return "";
    }
    if (
        date.getUTCFullYear() !== year
        || date.getUTCMonth() !== month - 1
        || date.getUTCDate() !== day
    ) {
        return "";
    }
    return normalized;
}

function loadMoreThreshold(maxScrollTop) {
    return Math.min(LOAD_MORE_THRESHOLD_PX, Math.max(0, maxScrollTop) * LOAD_MORE_THRESHOLD_RATIO);
}

function scrollMetrics(element) {
    const maxScrollTop = Math.max(0, Number(element?.scrollHeight || 0) - Number(element?.clientHeight || 0));
    const maxScrollLeft = Math.max(0, Number(element?.scrollWidth || 0) - Number(element?.clientWidth || 0));
    return {
        maxScrollTop,
        maxScrollLeft,
        scrollTop: Math.max(0, Number(element?.scrollTop || 0)),
        scrollLeft: Math.max(0, Number(element?.scrollLeft || 0)),
    };
}

function hasScrollableRange(metrics) {
    return metrics.maxScrollTop > 0 || metrics.maxScrollLeft > 0;
}

function hasScrollProgress(metrics) {
    return metrics.scrollTop > 0 || metrics.scrollLeft > 0;
}

function isNearScrollEnd(metrics) {
    const remainingY = metrics.maxScrollTop - metrics.scrollTop;
    const remainingX = metrics.maxScrollLeft - metrics.scrollLeft;
    return (
        (metrics.maxScrollTop > 0 && remainingY <= loadMoreThreshold(metrics.maxScrollTop))
        || (metrics.maxScrollLeft > 0 && remainingX <= loadMoreThreshold(metrics.maxScrollLeft))
    );
}

function isScrollableOverflowValue(value) {
    const normalized = String(value ?? "").trim().toLowerCase();
    return normalized === "auto" || normalized === "scroll" || normalized === "overlay";
}

function parseHtmlFragment(html) {
    const template = document.createElement("template");
    template.innerHTML = String(html ?? "");
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

export class FishySearchableDropdown extends HTMLElementBase {
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
        this._viewportListenersAttached = false;
        this._isReflectingBoundInputValue = false;
        this._isWritingBoundInputValue = false;
        this._panelAnchor = null;
        this._panelDetached = false;
        this._panelElement = null;
        this._panelEventsAttached = false;
        this._panelPositionFrame = 0;
        this._autoFilledSearchKey = "";
        this._moreResultsObserver = null;
        this._pendingAutoFillSearchKey = "";
        this._releaseBoundInput = null;
        this._resultsElement = null;
        this._resultsScrollHost = null;
        this._activeSearchMode = "";
        this._activeSearchQuery = "";
        this._activeSearchSelectedValue = "";
        this._loadMoreController = null;
        this._localSearchState = null;
        this._searchController = null;

        this._handleBoundInputEvent = this._handleBoundInputEvent.bind(this);
        this._handleClick = this._handleClick.bind(this);
        this._handleDocumentPointerDown = this._handleDocumentPointerDown.bind(this);
        this._handleFocusIn = this._handleFocusIn.bind(this);
        this._handleFocusOut = this._handleFocusOut.bind(this);
        this._handleInput = this._handleInput.bind(this);
        this._handleKeyDown = this._handleKeyDown.bind(this);
        this._handleMouseDown = this._handleMouseDown.bind(this);
        this._handleResultsScroll = this._handleResultsScroll.bind(this);
        this._handleViewportChange = this._handleViewportChange.bind(this);

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
        this._ensurePanelReference();
        this._attachPanelEvents();
        this._attachResultsEvents();
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
        this.close();
        this._detachPanelEvents();
        this._detachResultsEvents();
        this._unbindBoundInput();
        this._cancelClose();
        this._abortSearch();
        this._abortLoadMore();
        this._disconnectMoreResultsObserver();
        this._detachOutsideListener();
        this._detachViewportListeners();
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
        const wasOpen = this.isOpen();

        this._cancelClose();
        panel.hidden = false;
        this.style.zIndex = "60";
        this._setExpanded(true);
        this._attachOutsideListener();
        let detachedPanelWidth = 0;
        if (this._usesDetachedPanel()) {
            detachedPanelWidth = this._measurePanelRect().width;
            this._detachPanel();
            this._attachViewportListeners();
            this._positionPanel(detachedPanelWidth);
        }
        if (!wasOpen) {
            this.dispatchEvent(new CustomEvent(SEARCHABLE_DROPDOWN_OPEN_EVENT, {
                bubbles: true,
                detail: { dropdown: this },
            }));
        }
        input.value = "";
        this._lastSearchKey = "";

        window.requestAnimationFrame(() => {
            if (!this.isConnected || !this.isOpen()) {
                return;
            }
            if (this._usesDetachedPanel()) {
                this._positionPanel(detachedPanelWidth);
            }
            input.focus();
            this.search("");
        });
    }

    close() {
        this._cancelClose();

        const panel = this.panelElement();
        const wasOpen = this.isOpen();
        if (panel instanceof HTMLElement) {
            panel.hidden = true;
        }

        this.style.zIndex = "";
        this._abortSearch();
        this._abortLoadMore();
        this._detachOutsideListener();
        if (this._usesDetachedPanel()) {
            this._detachViewportListeners();
        }
        this._setExpanded(false);
        if (this._usesDetachedPanel()) {
            this._restorePanel();
        }
        if (wasOpen) {
            this.dispatchEvent(new CustomEvent(SEARCHABLE_DROPDOWN_CLOSE_EVENT, {
                bubbles: true,
                detail: { dropdown: this },
            }));
        }
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
        this._autoFilledSearchKey = "";
        this._pendingAutoFillSearchKey = "";
        this._abortSearch();
        this._abortLoadMore();
        this._setResultsNextOffset(results, null);
        results.removeAttribute("aria-busy");
        this._activeSearchQuery = query;
        this._activeSearchSelectedValue = selectedValue;
        this._localSearchState = null;

        const searchUrl = this._buildSearchUrl(query);
        if (!searchUrl) {
            this._activeSearchMode = "local";
            this._renderLocalResults(query, selectedValue);
            return;
        }

        this._activeSearchMode = "remote";
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
                this._attachResultsEvents();
                this._syncMoreResultsObserver();
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
                    this._scheduleAutoFillIfNeeded();
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
        this._ensurePanelReference();
        return this._panelElement;
    }

    resultsElement() {
        return this.panelElement()?.querySelector?.('[data-role="results"]') ?? null;
    }

    searchInputElement() {
        return this.panelElement()?.querySelector?.('[data-role="search-input"]') ?? null;
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

    _buildCustomOption(rawQuery, selectedValue, templates) {
        if (getStringAttribute(this, "custom-option-mode") !== "iso-date") {
            return null;
        }
        const queryValue = normalizeIsoDateValue(rawQuery);
        const selectedDateValue = normalizeIsoDateValue(selectedValue);
        const customValue = queryValue || (!String(rawQuery ?? "").trim() ? selectedDateValue : "");
        if (!customValue) {
            return null;
        }
        const templateValues = new Set(
            (Array.isArray(templates) ? templates : [])
                .map((template) => String(template.getAttribute("data-value") ?? "").trim())
                .filter(Boolean),
        );
        if (templateValues.has(customValue)) {
            return null;
        }

        const item = document.createElement("li");
        const button = document.createElement("button");
        button.type = "button";
        button.className = `justify-between gap-3 text-left${customValue === selectedValue ? " menu-active" : ""}`;
        button.dataset.searchableDropdownOption = "";
        button.setAttribute("data-value", customValue);
        button.setAttribute("data-label", customValue);

        const optionContent = document.createElement("span");
        optionContent.dataset.role = "option-content";
        optionContent.className = "flex min-w-0 flex-1 items-center gap-3";

        const label = document.createElement("span");
        label.className = "truncate font-medium";
        label.textContent = `Use date ${customValue}`;
        optionContent.append(label);
        button.append(optionContent);

        const selectedTemplate = document.createElement("template");
        selectedTemplate.dataset.role = "selected-content";
        selectedTemplate.innerHTML = `<span class="truncate font-medium">${customValue}</span>`;
        button.append(selectedTemplate);

        if (customValue === selectedValue) {
            const badge = document.createElement("span");
            badge.className = "badge badge-soft badge-primary badge-xs";
            badge.textContent = "Selected";
            button.append(badge);
        }

        item.append(button);
        return item;
    }

    _attachPanelEvents() {
        const panel = this.panelElement();
        if (!(panel instanceof HTMLElement) || this._panelEventsAttached) {
            return;
        }
        panel.addEventListener("click", this._handleClick);
        panel.addEventListener("focusin", this._handleFocusIn);
        panel.addEventListener("focusout", this._handleFocusOut);
        panel.addEventListener("input", this._handleInput);
        panel.addEventListener("keydown", this._handleKeyDown);
        panel.addEventListener("mousedown", this._handleMouseDown);
        this._panelEventsAttached = true;
    }

    _attachResultsEvents() {
        const results = this.resultsElement();
        const scrollHost = this._resolveResultsScrollHost(results);
        if (this._resultsElement === results && this._resultsScrollHost === scrollHost) {
            return;
        }
        this._detachResultsEvents();
        if (!(results instanceof HTMLElement) || !(scrollHost instanceof HTMLElement)) {
            return;
        }
        scrollHost.addEventListener("scroll", this._handleResultsScroll, { passive: true });
        this._resultsElement = results;
        this._resultsScrollHost = scrollHost;
    }

    _resolveResultsScrollHost(results = this.resultsElement()) {
        if (!(results instanceof HTMLElement)) {
            return null;
        }
        let fallback = results;
        let current = results;
        while (current instanceof HTMLElement) {
            const style = typeof window.getComputedStyle === "function" ? window.getComputedStyle(current) : null;
            const overflowY = style?.overflowY ?? style?.overflow ?? "";
            const overflowX = style?.overflowX ?? style?.overflow ?? "";
            if (isScrollableOverflowValue(overflowY) || isScrollableOverflowValue(overflowX)) {
                fallback = current;
                const metrics = scrollMetrics(current);
                if (hasScrollableRange(metrics)) {
                    return current;
                }
            }
            if (current === this.panelElement()) {
                break;
            }
            current = current.parentElement ?? (current.parentNode instanceof HTMLElement ? current.parentNode : null);
        }
        return fallback;
    }

    _attachViewportListeners() {
        if (this._viewportListenersAttached) {
            return;
        }
        window.addEventListener("resize", this._handleViewportChange);
        if (!this._usesOverlayAnchorPlacement()) {
            window.addEventListener("scroll", this._handleViewportChange, true);
        }
        this._viewportListenersAttached = true;
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

    _abortLoadMore() {
        if (!this._loadMoreController) {
            return;
        }
        this._loadMoreController.abort();
        this._loadMoreController = null;
    }

    _attachOutsideListener() {
        if (this._outsideListenerAttached) {
            return;
        }
        document.addEventListener("pointerdown", this._handleDocumentPointerDown, true);
        this._outsideListenerAttached = true;
    }

    _clearDetachedPanelStyles() {
        const panel = this.panelElement();
        if (!(panel instanceof HTMLElement)) {
            return;
        }
        panel.style.position = "";
        panel.style.left = "";
        panel.style.top = "";
        panel.style.width = "";
        panel.style.minWidth = "";
        panel.style.maxWidth = "";
        panel.style.zIndex = "";
        panel.style.margin = "";
    }

    _configuredPanelWidthStyle() {
        const width = getStringAttribute(this, "panel-width");
        if (!width) {
            return "";
        }
        const edgeInset = this._usesOverlayAnchorPlacement() ? "0px" : "24px";
        return `min(${width}, calc(100vw - ${edgeInset}))`;
    }

    _buildSearchUrl(query, offset = null) {
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

        if (Number.isInteger(offset) && offset > 0) {
            url.searchParams.set(OFFSET_QUERY_PARAM, String(offset));
        } else {
            url.searchParams.delete(OFFSET_QUERY_PARAM);
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

    _detachPanel() {
        const panel = this.panelElement();
        const body = document.body || document.documentElement;
        if (!(panel instanceof HTMLElement) || !(body instanceof HTMLElement) || this._panelDetached) {
            return;
        }
        if (!(this._panelAnchor instanceof Node) && panel.parentNode) {
            this._panelAnchor = document.createComment("fishy-searchable-dropdown-panel");
            panel.parentNode.insertBefore(this._panelAnchor, panel);
        }
        body.appendChild(panel);
        this._panelDetached = true;
    }

    _detachPanelEvents() {
        const panel = this.panelElement();
        if (!(panel instanceof HTMLElement) || !this._panelEventsAttached) {
            return;
        }
        panel.removeEventListener("click", this._handleClick);
        panel.removeEventListener("focusin", this._handleFocusIn);
        panel.removeEventListener("focusout", this._handleFocusOut);
        panel.removeEventListener("input", this._handleInput);
        panel.removeEventListener("keydown", this._handleKeyDown);
        panel.removeEventListener("mousedown", this._handleMouseDown);
        this._panelEventsAttached = false;
    }

    _detachResultsEvents() {
        if (!(this._resultsScrollHost instanceof HTMLElement)) {
            this._resultsElement = null;
            this._resultsScrollHost = null;
            return;
        }
        this._resultsScrollHost.removeEventListener("scroll", this._handleResultsScroll);
        this._resultsElement = null;
        this._resultsScrollHost = null;
    }

    _detachViewportListeners() {
        if (!this._viewportListenersAttached) {
            return;
        }
        window.removeEventListener("resize", this._handleViewportChange);
        window.removeEventListener("scroll", this._handleViewportChange, true);
        this._viewportListenersAttached = false;
        if (this._panelPositionFrame) {
            window.cancelAnimationFrame?.(this._panelPositionFrame);
            this._panelPositionFrame = 0;
        }
    }

    _ensurePanelReference() {
        if (!(this._panelElement instanceof HTMLElement)) {
            this._panelElement = this.querySelector('[data-role="panel"]');
        }
        return this._panelElement;
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

    _findCatalogOptionContentTemplateByValue(value) {
        const catalog = this.selectedContentCatalogElement();
        if (!(catalog instanceof HTMLElement)) {
            return null;
        }
        return Array.from(
            catalog.querySelectorAll('template[data-role="option-content"]'),
        ).find((template) => template.getAttribute("data-value") === value) ?? null;
    }

    _findOptionByValue(value) {
        return Array.from(
            this.panelElement()?.querySelectorAll?.("[data-searchable-dropdown-option]") ?? [],
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
        if (trigger && this._ownsNode(trigger)) {
            event.preventDefault();
            this.toggle();
            return;
        }

        const moreIndicator = event.target.closest("[data-searchable-dropdown-more]");
        if (moreIndicator && this._ownsNode(moreIndicator)) {
            event.preventDefault();
            this._loadMore();
            return;
        }

        const option = event.target.closest("[data-searchable-dropdown-option]");
        if (!option || !this._ownsNode(option)) {
            return;
        }

        event.preventDefault();
        const value = String(option.getAttribute("data-value") ?? "");
        const label = String(option.getAttribute("data-label") ?? option.textContent ?? "").trim();
        this._syncSelectedContentFromOption(option, label);
        this.select(value, label);
    }

    _handleDocumentPointerDown(event) {
        if (!(event.target instanceof Node) || this._ownsNode(event.target)) {
            return;
        }
        this.close();
    }

    _handleFocusIn(event) {
        if (event.target instanceof Node && this._ownsNode(event.target)) {
            this._cancelClose();
        }
    }

    _handleFocusOut(event) {
        if (!this.isOpen()) {
            return;
        }

        const nextTarget = event.relatedTarget;
        if (nextTarget instanceof Node && this._ownsNode(nextTarget)) {
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
        if (!(event.target instanceof Node) || event.key !== "Escape" || !this._ownsNode(event.target)) {
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
        if (!(event.target instanceof Element) || !this._ownsNode(event.target)) {
            return;
        }

        this._cancelClose();

        const moreIndicator = event.target.closest("[data-searchable-dropdown-more]");
        if (moreIndicator && this._ownsNode(moreIndicator)) {
            event.preventDefault();
            return;
        }

        const option = event.target.closest("[data-searchable-dropdown-option]");
        if (option && this._ownsNode(option)) {
            event.preventDefault();
        }
    }

    _handleResultsScroll() {
        this._maybeLoadMore();
    }

    _handleViewportChange() {
        if (!this.isOpen() || this._panelPositionFrame) {
            return;
        }
        this._panelPositionFrame = window.requestAnimationFrame(() => {
            this._panelPositionFrame = 0;
            this._positionPanel();
        }) || 0;
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
            this._setResultsNextOffset(results, null);
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
            });
        const customOption = this._buildCustomOption(rawQuery, selectedValue, templates);
        this._localSearchState = {
            matches,
            nextOffset: 0,
            selectedValue,
        };

        if (!matches.length && !customOption) {
            const item = document.createElement("li");
            item.className = "menu-disabled";
            const label = document.createElement("span");
            label.textContent = "No matching options";
            item.append(label);
            results.replaceChildren(item);
            this._setResultsNextOffset(results, null);
            return;
        }

        results.replaceChildren(...(customOption ? [customOption] : []));
        this._appendLocalResultsPage();
    }

    _appendLocalResultsPage() {
        const results = this.resultsElement();
        const state = this._localSearchState;
        if (!(results instanceof HTMLElement) || !state) {
            return;
        }

        const start = state.nextOffset;
        const end = Math.min(start + RESULTS_PAGE_SIZE, state.matches.length);
        const pageMatches = state.matches.slice(start, end);
        state.nextOffset = end;

        const existingNodes = Array.from(results.childNodes).filter(
            (node) => !this._isMoreResultsNode(node),
        );
        results.replaceChildren(
            ...existingNodes,
            ...pageMatches.map((template) => this._buildLocalResultItem(template, state.selectedValue)),
        );

        const nextOffset = end < state.matches.length ? end : null;
        this._setResultsNextOffset(results, nextOffset);
        if (nextOffset !== null) {
            results.append(this._buildMoreResultsIndicator(nextOffset));
        }
        this._syncMoreResultsObserver();
        this._scheduleAutoFillIfNeeded();
    }

    _buildLocalResultItem(template, selectedValue) {
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
        const optionTemplate = this._findCatalogOptionContentTemplateByValue(value);
        optionContent.replaceChildren(
            ...cloneChildNodes(
                optionTemplate instanceof HTMLTemplateElement ? optionTemplate.content : template.content,
            ),
        );
        button.append(optionContent, template.cloneNode(true));

        if (value === selectedValue) {
            const badge = document.createElement("span");
            badge.className = "badge badge-soft badge-primary badge-xs";
            badge.textContent = "Selected";
            button.append(badge);
        }

        item.append(button);
        return item;
    }

    _buildMoreResultsIndicator(nextOffset) {
        const item = document.createElement("li");
        const button = document.createElement("button");
        button.type = "button";
        button.className = "justify-start gap-3 text-left text-base-content/70";
        button.setAttribute("data-searchable-dropdown-more", "");
        button.setAttribute("data-next-offset", String(nextOffset));

        const label = document.createElement("span");
        label.textContent = this._moreResultsLabel();
        button.append(label);
        item.append(button);
        return item;
    }

    _isMoreResultsNode(node) {
        return (
            node instanceof Element
            && (
                node.hasAttribute("data-searchable-dropdown-more")
                || node.querySelector?.("[data-searchable-dropdown-more]")
            )
        );
    }

    _loadMore() {
        if (this._searchController || this._loadMoreController) {
            return;
        }
        if (this._activeSearchMode === "local") {
            if (!this._localSearchState || this._resultsNextOffset() === null) {
                return;
            }
            this._appendLocalResultsPage();
            return;
        }
        if (this._activeSearchMode === "remote") {
            this._loadMoreRemote();
        }
    }

    _loadMoreRemote() {
        const offset = this._resultsNextOffset();
        if (offset === null) {
            return;
        }

        const searchUrl = this._buildSearchUrl(this._activeSearchQuery, offset);
        if (!searchUrl) {
            return;
        }

        const results = this.resultsElement();
        if (results instanceof HTMLElement) {
            results.setAttribute("aria-busy", "true");
        }

        const controller = new AbortController();
        this._loadMoreController = controller;
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

                const mergedChildren = [
                    ...Array.from(currentResults.childNodes).filter(
                        (node) => !this._isMoreResultsNode(node),
                    ),
                    ...Array.from(nextResults.childNodes),
                ];
                currentResults.replaceChildren(...mergedChildren);
                this._setResultsNextOffset(currentResults, this._resultsNextOffset(nextResults));
                this._syncMoreResultsObserver();
            })
            .catch((error) => {
                if (error?.name === "AbortError") {
                    return;
                }
                console.error("Error loading more searchable dropdown results:", error);
            })
            .finally(() => {
                if (results instanceof HTMLElement) {
                    results.removeAttribute("aria-busy");
                }
                if (this._loadMoreController === controller) {
                    this._loadMoreController = null;
                    this._scheduleAutoFillIfNeeded();
                }
            });
    }

    _scheduleAutoFillIfNeeded() {
        if (
            !this._lastSearchKey
            || this._autoFilledSearchKey === this._lastSearchKey
            || this._pendingAutoFillSearchKey === this._lastSearchKey
        ) {
            return;
        }
        this._pendingAutoFillSearchKey = this._lastSearchKey;
        queueMicrotask(() => {
            if (this._pendingAutoFillSearchKey !== this._lastSearchKey) {
                return;
            }
            this._pendingAutoFillSearchKey = "";
            this._maybeAutoFillUntilScrollable();
        });
    }

    _disconnectMoreResultsObserver() {
        if (
            typeof IntersectionObserver !== "function"
            || !(this._moreResultsObserver instanceof IntersectionObserver)
        ) {
            this._moreResultsObserver = null;
            return;
        }
        this._moreResultsObserver.disconnect();
        this._moreResultsObserver = null;
    }

    _maybeAutoFillUntilScrollable() {
        const results = this.resultsElement();
        if (!(results instanceof HTMLElement) || this._searchController || this._loadMoreController) {
            return;
        }
        if (this._resultsNextOffset(results) === null) {
            return;
        }
        const scrollHost = this._resultsScrollHost ?? this._resolveResultsScrollHost(results);
        if (!(scrollHost instanceof HTMLElement)) {
            return;
        }
        if (hasScrollableRange(scrollMetrics(scrollHost))) {
            return;
        }
        this._autoFilledSearchKey = this._lastSearchKey;
        this._loadMore();
    }

    _syncMoreResultsObserver() {
        this._attachResultsEvents();
        this._disconnectMoreResultsObserver();
        if (typeof IntersectionObserver !== "function") {
            return;
        }
        const results = this.resultsElement();
        const moreButton = results?.querySelector?.("[data-searchable-dropdown-more]");
        const scrollHost = this._resultsScrollHost ?? this._resolveResultsScrollHost(results);
        if (
            !(results instanceof HTMLElement)
            || !(moreButton instanceof HTMLElement)
            || !(scrollHost instanceof HTMLElement)
        ) {
            return;
        }
        this._moreResultsObserver = new IntersectionObserver(
            (entries) => {
                if (!entries.some((entry) => entry.isIntersecting)) {
                    return;
                }
                const currentScrollHost = this._resultsScrollHost ?? this._resolveResultsScrollHost();
                if (!(currentScrollHost instanceof HTMLElement)) {
                    return;
                }
                const metrics = scrollMetrics(currentScrollHost);
                if (!hasScrollableRange(metrics) || !hasScrollProgress(metrics)) {
                    return;
                }
                this._maybeLoadMore();
            },
            {
                root: scrollHost,
                threshold: 1,
            },
        );
        this._moreResultsObserver.observe(moreButton);
    }

    _maybeLoadMore() {
        const results = this.resultsElement();
        if (!(results instanceof HTMLElement) || this._searchController || this._loadMoreController) {
            return;
        }
        if (this._resultsNextOffset(results) === null) {
            return;
        }

        const scrollHost = this._resultsScrollHost ?? this._resolveResultsScrollHost(results);
        if (!(scrollHost instanceof HTMLElement)) {
            return;
        }
        const metrics = scrollMetrics(scrollHost);
        if (!hasScrollableRange(metrics)) {
            return;
        }
        if (isNearScrollEnd(metrics)) {
            this._loadMore();
        }
    }

    _moreResultsLabel() {
        return getStringAttribute(this, "more-results-label") || DEFAULT_MORE_RESULTS_LABEL;
    }

    _resultsNextOffset(results = this.resultsElement()) {
        const raw = String(results?.getAttribute?.("data-next-offset") ?? "").trim();
        if (!raw) {
            return null;
        }
        const value = Number.parseInt(raw, 10);
        return Number.isInteger(value) && value >= 0 ? value : null;
    }

    _setResultsNextOffset(results, nextOffset) {
        if (!(results instanceof HTMLElement)) {
            return;
        }
        if (Number.isInteger(nextOffset) && nextOffset >= 0) {
            results.setAttribute("data-next-offset", String(nextOffset));
            return;
        }
        results.removeAttribute("data-next-offset");
    }

    _measurePanelRect() {
        const panel = this.panelElement();
        if (!(panel instanceof HTMLElement)) {
            return { width: 0, height: 0 };
        }
        const previousHidden = panel.hidden;
        const previousVisibility = panel.style.visibility;
        const previousWidth = panel.style.width;
        const previousMinWidth = panel.style.minWidth;
        const previousMaxWidth = panel.style.maxWidth;
        const configuredWidth = this._configuredPanelWidthStyle();
        panel.hidden = false;
        panel.style.visibility = "hidden";
        if (configuredWidth) {
            panel.style.width = configuredWidth;
            panel.style.minWidth = "0";
            panel.style.maxWidth = this._usesOverlayAnchorPlacement()
                ? "100vw"
                : "calc(100vw - 24px)";
        }
        const rect = panel.getBoundingClientRect();
        panel.hidden = previousHidden;
        panel.style.visibility = previousVisibility;
        panel.style.width = previousWidth;
        panel.style.minWidth = previousMinWidth;
        panel.style.maxWidth = previousMaxWidth;
        return rect;
    }

    _resolvePanelAnchorElement() {
        const selector = getStringAttribute(this, "panel-anchor-closest");
        if (selector) {
            try {
                const anchor = this.closest(selector);
                if (anchor instanceof HTMLElement) {
                    return anchor;
                }
            } catch (_) {
                // Ignore invalid selectors and fall back to the trigger element.
            }
        }
        return this.triggerElement();
    }

    _usesDetachedPanel() {
        return getStringAttribute(this, "panel-mode") === "detached";
    }

    _usesOverlayAnchorPlacement() {
        return getStringAttribute(this, "panel-placement") === "overlay-anchor";
    }

    _ownsNode(node) {
        if (!(node instanceof Node)) {
            return false;
        }
        const panel = this.panelElement();
        return this.contains(node) || (panel instanceof Node && panel.contains(node));
    }

    _positionPanel(measuredWidth = 0) {
        const panel = this.panelElement();
        const anchor = this._resolvePanelAnchorElement();
        if (!(panel instanceof HTMLElement) || !(anchor instanceof HTMLElement)) {
            return;
        }
        const viewportWidth = Math.max(window.innerWidth || 0, 320);
        const viewportHeight = Math.max(window.innerHeight || 0, 240);
        const visibleViewportRight = Math.max(
            320,
            Math.min(
                window.innerWidth || Number.POSITIVE_INFINITY,
                document.documentElement.clientWidth || Number.POSITIVE_INFINITY,
                window.visualViewport
                    ? window.visualViewport.offsetLeft + window.visualViewport.width
                    : Number.POSITIVE_INFINITY,
            ),
        );
        const anchorRect = anchor.getBoundingClientRect();
        const panelWidth = Math.round(measuredWidth || panel.getBoundingClientRect().width || 0);
        const anchorWidth = Math.round(anchorRect.width || 0);
        const overlayAnchor = this._usesOverlayAnchorPlacement();
        const edgeInset = overlayAnchor ? 0 : 12;
        const widthSource = getStringAttribute(this, "panel-min-width");
        const maxWidth = overlayAnchor
            ? visibleViewportRight - edgeInset - anchorRect.left
            : visibleViewportRight - edgeInset * 2;
        const width = Math.max(
            0,
            Math.min(
                widthSource === "panel" ? Math.max(anchorWidth, panelWidth) : (anchorWidth || panelWidth),
                Math.max(0, maxWidth),
            ),
        );

        panel.style.position = overlayAnchor ? "absolute" : "fixed";
        panel.style.margin = "0";
        panel.style.zIndex = "70";
        panel.style.width = width ? `${width}px` : "";
        panel.style.minWidth = "0";
        panel.style.maxWidth = `${Math.max(visibleViewportRight - edgeInset * 2, 160)}px`;
        panel.style.left = `${edgeInset}px`;
        panel.style.top = `${edgeInset}px`;

        const panelRect = panel.getBoundingClientRect();
        if (overlayAnchor) {
            const originRect = document.documentElement.getBoundingClientRect();
            const originLeft = Number.isFinite(originRect.left) ? originRect.left : 0;
            const originTop = Number.isFinite(originRect.top) ? originRect.top : 0;
            panel.style.left = `${anchorRect.left - originLeft}px`;
            panel.style.top = `${anchorRect.top - originTop}px`;
            return;
        }

        const minLeft = edgeInset;
        let left = anchorRect.left;
        if (left + panelRect.width > visibleViewportRight - edgeInset) {
            left = Math.max(
                minLeft,
                visibleViewportRight - panelRect.width - edgeInset,
            );
        }

        const belowTop = Math.round(anchorRect.bottom + 8);
        const aboveTop = Math.round(anchorRect.top - panelRect.height - 8);
        const topInViewport =
            belowTop + panelRect.height <= viewportHeight - 12 || aboveTop < 12
                ? belowTop
                : aboveTop;

        panel.style.left = `${left}px`;
        panel.style.top = `${Math.max(edgeInset, topInViewport)}px`;
    }

    _restorePanel() {
        const panel = this.panelElement();
        if (!(panel instanceof HTMLElement) || !this._panelDetached) {
            return;
        }
        this._clearDetachedPanelStyles();
        if (this._panelAnchor?.parentNode) {
            this._panelAnchor.parentNode.insertBefore(panel, this._panelAnchor.nextSibling);
        } else {
            this.appendChild(panel);
        }
        this._panelDetached = false;
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
            }
            return;
        }

        const customIsoDate = normalizeIsoDateValue(value);
        if (customIsoDate) {
            this.label = customIsoDate;
            const container = this.selectedContentElement();
            if (container instanceof HTMLElement) {
                const text = document.createElement("span");
                text.className = "truncate font-medium";
                text.textContent = customIsoDate;
                container.replaceChildren(text);
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
            return;
        }

        const optionContent = option.querySelector('[data-role="option-content"]');
        if (optionContent instanceof HTMLElement) {
            container.replaceChildren(...cloneChildNodes(optionContent));
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
