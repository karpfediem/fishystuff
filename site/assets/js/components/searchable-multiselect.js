import {
    cloneChildNodes,
    dispatchValueEvents,
    getStringAttribute,
    normalizeSearchText,
    setStringAttribute,
    upgradeProperty,
} from "./searchable-dropdown.js";
import {
    bindBoundSelect,
    boundSelectOptions,
    findBoundSelectOption,
    resolveBoundSelectElement,
} from "./bound-select.js";

const CLOSE_DELAY_MS = 150;
const LOCAL_RESULT_LIMIT = 24;

function uniqueValues(values) {
    const seen = new Set();
    const out = [];
    for (const value of values) {
        const normalized = String(value ?? "").trim();
        if (!normalized || seen.has(normalized)) {
            continue;
        }
        seen.add(normalized);
        out.push(normalized);
    }
    return out;
}

export class FishySearchableMultiselect extends HTMLElement {
    static get observedAttributes() {
        return ["bound-select-id", "placeholder"];
    }

    constructor() {
        super();
        this._closeTimer = 0;
        this._lastSearchKey = "";
        this._outsideListenerAttached = false;
        this._panelAnchor = null;
        this._panelDetached = false;
        this._panelElement = null;
        this._panelEventsAttached = false;
        this._panelPositionFrame = 0;
        this._releaseInputs = [];
        this._viewportListenersAttached = false;

        this._handleBoundInputEvent = this._handleBoundInputEvent.bind(this);
        this._handleClick = this._handleClick.bind(this);
        this._handleDocumentPointerDown = this._handleDocumentPointerDown.bind(this);
        this._handleFocusIn = this._handleFocusIn.bind(this);
        this._handleFocusOut = this._handleFocusOut.bind(this);
        this._handleInput = this._handleInput.bind(this);
        this._handleKeyDown = this._handleKeyDown.bind(this);
        this._handleMouseDown = this._handleMouseDown.bind(this);
        this._handlePointerDown = this._handlePointerDown.bind(this);
        this._handleViewportChange = this._handleViewportChange.bind(this);

        this.addEventListener("click", this._handleClick);
        this.addEventListener("focusin", this._handleFocusIn);
        this.addEventListener("focusout", this._handleFocusOut);
        this.addEventListener("input", this._handleInput);
        this.addEventListener("keydown", this._handleKeyDown);
        this.addEventListener("mousedown", this._handleMouseDown);
        this.addEventListener("pointerdown", this._handlePointerDown);
    }

    get placeholder() {
        return getStringAttribute(this, "placeholder");
    }

    get boundSelectId() {
        return getStringAttribute(this, "bound-select-id");
    }

    set boundSelectId(value) {
        setStringAttribute(this, "bound-select-id", value);
    }

    set placeholder(value) {
        setStringAttribute(this, "placeholder", value);
    }

    connectedCallback() {
        upgradeProperty(this, "boundSelectId");
        upgradeProperty(this, "placeholder");
        this._ensurePanelReference();
        this._bindInputs();
        queueMicrotask(() => {
            if (!this.isConnected) {
                return;
            }
            this._syncBoundInputsFromMarkup();
            this._syncUi();
            this._syncSelection();
            this.search(this.searchInputElement()?.value ?? "");
        });
    }

    disconnectedCallback() {
        this.close();
        this._detachPanelEvents();
        this._unbindInputs();
        this._cancelClose();
        this._detachOutsideListener();
        this._detachViewportListeners();
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (oldValue === newValue) {
            return;
        }
        if (name === "bound-select-id") {
            this._bindInputs();
            this._lastSearchKey = "";
            this._syncSelection();
            this.search(this.searchInputElement()?.value ?? "");
            return;
        }
        if (name === "placeholder") {
            this._syncUi();
        }
    }

    open() {
        const panel = this.panelElement();
        if (!(panel instanceof HTMLElement)) {
            return;
        }
        this._cancelClose();
        panel.hidden = false;
        this.style.zIndex = "60";
        this._attachOutsideListener();
        let detachedPanelWidth = 0;
        if (this._usesDetachedPanel()) {
            detachedPanelWidth = this._measurePanelRect().width;
            this._detachPanel();
            this._attachPanelEvents();
            this._attachViewportListeners();
            this._positionPanel(detachedPanelWidth);
        }
        this.search(this.searchInputElement()?.value ?? "");
        window.requestAnimationFrame(() => {
            if (!this.isConnected || !this.isOpen()) {
                return;
            }
            if (this._usesDetachedPanel()) {
                this._positionPanel(detachedPanelWidth);
            }
        });
    }

    close() {
        this._cancelClose();
        const panel = this.panelElement();
        if (panel instanceof HTMLElement) {
            panel.hidden = true;
        }
        this.style.zIndex = "";
        this._detachOutsideListener();
        if (this._usesDetachedPanel()) {
            this._detachViewportListeners();
            this._restorePanel();
            this._detachPanelEvents();
        }
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
        return this.querySelector('[data-role="search-input"]');
    }

    shellElement() {
        return this.querySelector('[data-role="shell"]');
    }

    selectionElement() {
        return this.querySelector('[data-role="selection"]');
    }

    boundSelectElement() {
        return resolveBoundSelectElement(this, this.boundSelectId);
    }

    boundOptionElements() {
        return boundSelectOptions(this.boundSelectElement());
    }

    catalogTemplates() {
        return Array.from(
            this.querySelectorAll('template[data-role="option-template"]'),
        ).filter((element) => element instanceof HTMLTemplateElement);
    }

    search(rawQuery) {
        const query = String(rawQuery ?? "");
        const selectedValues = this.selectedValues();
        const searchKey = `${selectedValues.join("\n")}\n${query}`;
        if (this._lastSearchKey === searchKey) {
            return;
        }
        this._lastSearchKey = searchKey;
        this._renderLocalResults(query, selectedValues);
    }

    selectedValues() {
        return uniqueValues(
            this.boundOptionElements()
                .filter((option) => option.selected)
                .map((option) => option.value),
        );
    }

    select(value) {
        const select = this.boundSelectElement();
        const option = this._findBoundOptionByValue(value);
        if (!(select instanceof HTMLSelectElement) || !(option instanceof HTMLOptionElement)) {
            return;
        }

        let changed = false;
        const categoryKey = getStringAttribute(option, "data-category-key");
        if (categoryKey) {
            for (const candidate of this.boundOptionElements()) {
                if (
                    candidate !== option
                    && candidate.selected
                    && getStringAttribute(candidate, "data-category-key") === categoryKey
                ) {
                    candidate.selected = false;
                    changed = true;
                }
            }
        }

        if (option.selected && !changed) {
            this._clearSearch();
            return;
        }

        if (!option.selected) {
            option.selected = true;
            changed = true;
        }

        if (!changed) {
            this._clearSearch();
            return;
        }

        dispatchValueEvents(select);
        this._clearSearch();
        this._syncSelection();
        this.search("");

        const searchInput = this.searchInputElement();
        if (searchInput instanceof HTMLInputElement) {
            searchInput.focus();
        }
    }

    remove(value) {
        const select = this.boundSelectElement();
        const option = this._findBoundOptionByValue(value);
        if (!(select instanceof HTMLSelectElement) || !(option instanceof HTMLOptionElement) || !option.selected) {
            return;
        }

        option.selected = false;
        dispatchValueEvents(select);
        this._syncSelection();
        this._lastSearchKey = "";
        this.search(this.searchInputElement()?.value ?? "");
    }

    _attachOutsideListener() {
        if (this._outsideListenerAttached) {
            return;
        }
        document.addEventListener("pointerdown", this._handleDocumentPointerDown, true);
        this._outsideListenerAttached = true;
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
        panel.addEventListener("pointerdown", this._handlePointerDown);
        this._panelEventsAttached = true;
    }

    _attachViewportListeners() {
        if (this._viewportListenersAttached) {
            return;
        }
        window.addEventListener("resize", this._handleViewportChange);
        window.addEventListener("scroll", this._handleViewportChange, true);
        window.visualViewport?.addEventListener?.("resize", this._handleViewportChange);
        window.visualViewport?.addEventListener?.("scroll", this._handleViewportChange);
        this._viewportListenersAttached = true;
    }

    _bindInputs() {
        this._unbindInputs();
        const select = this.boundSelectElement();
        this._releaseInputs = [bindBoundSelect(select, this._handleBoundInputEvent)];
    }

    _cancelClose() {
        if (!this._closeTimer) {
            return;
        }
        window.clearTimeout(this._closeTimer);
        this._closeTimer = 0;
    }

    _clearSearch() {
        const searchInput = this.searchInputElement();
        if (searchInput instanceof HTMLInputElement) {
            searchInput.value = "";
        }
        this._lastSearchKey = "";
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
        panel.style.maxHeight = "";
        panel.style.overscrollBehavior = "";
        panel.style.zIndex = "";
        panel.style.margin = "";
        this._clearDetachedResultsStyles();
    }

    _clearDetachedResultsStyles() {
        const results = this.resultsElement();
        if (!(results instanceof HTMLElement)) {
            return;
        }
        results.style.maxHeight = "";
        results.style.overflowY = "";
        results.style.overscrollBehavior = "";
    }

    _configuredPanelWidthStyle() {
        const width = getStringAttribute(this, "panel-width");
        if (!width) {
            return "";
        }
        return `min(${width}, calc(100vw - 24px))`;
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
            this._panelAnchor = document.createComment("fishy-searchable-multiselect-panel");
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
        panel.removeEventListener("pointerdown", this._handlePointerDown);
        this._panelEventsAttached = false;
    }

    _detachViewportListeners() {
        if (!this._viewportListenersAttached) {
            return;
        }
        window.removeEventListener("resize", this._handleViewportChange);
        window.removeEventListener("scroll", this._handleViewportChange, true);
        window.visualViewport?.removeEventListener?.("resize", this._handleViewportChange);
        window.visualViewport?.removeEventListener?.("scroll", this._handleViewportChange);
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
        if (
            this._panelElement instanceof HTMLElement
            && !this._panelElement.hasAttribute("data-searchable-multiselect-panel")
        ) {
            this._panelElement.setAttribute("data-searchable-multiselect-panel", "");
        }
        return this._panelElement;
    }

    _findBoundOptionByValue(value) {
        return findBoundSelectOption(this.boundSelectElement(), value);
    }

    _findCatalogTemplateByValue(value) {
        return this.catalogTemplates().find(
            (template) => template.getAttribute("data-value") === value,
        ) ?? null;
    }

    _findFirstAddableResultButton() {
        return (
            this.resultsElement()?.querySelector(
                "button[data-searchable-multiselect-option]:not([data-selected='true'])",
            ) ?? null
        );
    }

    _handleBoundInputEvent() {
        this._syncSelection();
        this._lastSearchKey = "";
        if (this.isOpen()) {
            this.search(this.searchInputElement()?.value ?? "");
        }
    }

    _handleClick(event) {
        if (!(event.target instanceof Element)) {
            return;
        }

        const removeButton = event.target.closest(
            "button[data-searchable-multiselect-remove]",
        );
        if (removeButton && this.contains(removeButton)) {
            event.preventDefault();
            this.remove(String(removeButton.getAttribute("data-value") ?? ""));
            return;
        }

        const option = event.target.closest(
            "button[data-searchable-multiselect-option]",
        );
        if (option && this._ownsNode(option)) {
            event.preventDefault();
            if (option.getAttribute("data-selected") === "true") {
                this._clearSearch();
                this.search("");
                return;
            }
            this.select(String(option.getAttribute("data-value") ?? ""));
            return;
        }

        const shell = event.target.closest('[data-role="shell"]');
        if (!shell || !this._ownsNode(shell)) {
            return;
        }

        const searchInput = this.searchInputElement();
        if (searchInput instanceof HTMLInputElement && event.target !== searchInput) {
            searchInput.focus();
        }
        this.open();
    }

    _handleDocumentPointerDown(event) {
        if (!(event.target instanceof Node) || this._ownsNode(event.target)) {
            return;
        }
        this.close();
    }

    _handleFocusIn(event) {
        if (!this._ownsNode(event.target)) {
            return;
        }
        this._cancelClose();
        if (event.target === this.searchInputElement()) {
            this.open();
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
        this.open();
        this.search(event.target.value);
    }

    _handleKeyDown(event) {
        if (!(event.target instanceof Element) || !this._ownsNode(event.target)) {
            return;
        }

        if (event.key === "Escape") {
            event.preventDefault();
            this.close();
            this.searchInputElement()?.blur();
            return;
        }

        if (event.key !== "Enter" || event.target !== this.searchInputElement()) {
            return;
        }

        const option = this._findFirstAddableResultButton();
        if (!(option instanceof HTMLButtonElement)) {
            return;
        }
        event.preventDefault();
        this.select(String(option.getAttribute("data-value") ?? ""));
    }

    _handleMouseDown(event) {
        this._handlePressStart(event);
    }

    _handlePointerDown(event) {
        this._handlePressStart(event);
    }

    _handlePressStart(event) {
        if (!(event.target instanceof Element) || !this._ownsNode(event.target)) {
            return;
        }

        this._cancelClose();

        if (
            event.target.closest("button[data-searchable-multiselect-option]")
            || event.target.closest("button[data-searchable-multiselect-remove]")
        ) {
            event.preventDefault();
        }
    }

    _handleViewportChange(event = null) {
        if (event?.type === "scroll" && event.target instanceof Node) {
            const panel = this.panelElement();
            if (panel instanceof Node && panel.contains(event.target)) {
                return;
            }
        }
        if (!this.isOpen() || this._panelPositionFrame) {
            return;
        }
        this._panelPositionFrame = window.requestAnimationFrame(() => {
            this._panelPositionFrame = 0;
            this._positionPanel();
        }) || 0;
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

    _renderLocalResults(rawQuery, selectedValues) {
        const results = this.resultsElement();
        if (!(results instanceof HTMLElement)) {
            return;
        }

        const selectedValueSet = new Set(selectedValues);
        const normalizedQuery = normalizeSearchText(rawQuery);
        const matches = this.catalogTemplates()
            .filter((template) => this._localTemplateMatches(template, normalizedQuery))
            .sort((left, right) => {
                const leftValue = String(left.getAttribute("data-value") ?? "");
                const rightValue = String(right.getAttribute("data-value") ?? "");
                const leftSelected = selectedValueSet.has(leftValue) ? 1 : 0;
                const rightSelected = selectedValueSet.has(rightValue) ? 1 : 0;
                if (leftSelected !== rightSelected) {
                    return leftSelected - rightSelected;
                }
                const leftLabel = String(left.getAttribute("data-label") ?? leftValue);
                const rightLabel = String(right.getAttribute("data-label") ?? rightValue);
                return leftLabel.localeCompare(rightLabel);
            })
            .slice(0, LOCAL_RESULT_LIMIT);

        if (!matches.length) {
            const item = document.createElement("li");
            item.className = "menu-disabled";
            const label = document.createElement("span");
            label.textContent = "No matching options";
            item.append(label);
            results.replaceChildren(item);
            this._positionOpenDetachedPanel();
            return;
        }

        results.replaceChildren(
            ...matches.map((template) => {
                const value = String(template.getAttribute("data-value") ?? "");
                const label = String(template.getAttribute("data-label") ?? value).trim();
                const isSelected = selectedValueSet.has(value);
                const item = document.createElement("li");
                const button = document.createElement("button");
                button.type = "button";
                button.className = `justify-between gap-3 text-left${isSelected ? " opacity-75" : ""}`;
                button.dataset.searchableMultiselectOption = "";
                button.setAttribute("data-value", value);
                button.setAttribute("data-label", label);
                button.setAttribute("data-selected", isSelected ? "true" : "false");

                const optionContent = document.createElement("span");
                optionContent.className = "flex min-w-0 flex-1 items-center gap-3";
                optionContent.append(...cloneChildNodes(template.content));
                button.append(optionContent);

                if (isSelected) {
                    const badge = document.createElement("span");
                    badge.className = "badge badge-soft badge-primary badge-xs";
                    badge.textContent = "Added";
                    button.append(badge);
                }

                item.append(button);
                return item;
            }),
        );
        this._positionOpenDetachedPanel();
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
        const previousMaxHeight = panel.style.maxHeight;
        const previousOverscrollBehavior = panel.style.overscrollBehavior;
        const results = this.resultsElement();
        const previousResultsMaxHeight = results instanceof HTMLElement
            ? results.style.maxHeight
            : "";
        const previousResultsOverflowY = results instanceof HTMLElement
            ? results.style.overflowY
            : "";
        const previousResultsOverscrollBehavior = results instanceof HTMLElement
            ? results.style.overscrollBehavior
            : "";
        const configuredWidth = this._configuredPanelWidthStyle();
        panel.hidden = false;
        panel.style.visibility = "hidden";
        panel.style.maxHeight = "";
        panel.style.overscrollBehavior = "";
        if (results instanceof HTMLElement) {
            results.style.maxHeight = "";
            results.style.overflowY = "";
            results.style.overscrollBehavior = "";
        }
        if (configuredWidth) {
            panel.style.width = configuredWidth;
            panel.style.minWidth = "0";
            panel.style.maxWidth = "calc(100vw - 24px)";
        }
        const rect = panel.getBoundingClientRect();
        panel.hidden = previousHidden;
        panel.style.visibility = previousVisibility;
        panel.style.width = previousWidth;
        panel.style.minWidth = previousMinWidth;
        panel.style.maxWidth = previousMaxWidth;
        panel.style.maxHeight = previousMaxHeight;
        panel.style.overscrollBehavior = previousOverscrollBehavior;
        if (results instanceof HTMLElement) {
            results.style.maxHeight = previousResultsMaxHeight;
            results.style.overflowY = previousResultsOverflowY;
            results.style.overscrollBehavior = previousResultsOverscrollBehavior;
        }
        return rect;
    }

    _ownsNode(node) {
        if (!(node instanceof Node)) {
            return false;
        }
        const panel = this.panelElement();
        return this.contains(node) || (panel instanceof Node && panel.contains(node));
    }

    _positionOpenDetachedPanel() {
        if (!this.isOpen() || !this._usesDetachedPanel()) {
            return;
        }
        this._positionPanel();
    }

    _positionPanel(measuredWidth = 0) {
        const panel = this.panelElement();
        const anchor = this.shellElement();
        if (!(panel instanceof HTMLElement) || !(anchor instanceof HTMLElement)) {
            return;
        }
        const viewportHeight = Math.max(window.innerHeight || 0, 240);
        const visualViewportLeft = window.visualViewport
            ? Math.max(0, window.visualViewport.offsetLeft || 0)
            : 0;
        const visualViewportTop = window.visualViewport
            ? Math.max(0, window.visualViewport.offsetTop || 0)
            : 0;
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
        const visibleViewportBottom = Math.max(
            240,
            Math.min(
                viewportHeight,
                document.documentElement.clientHeight || Number.POSITIVE_INFINITY,
                window.visualViewport
                    ? window.visualViewport.offsetTop + window.visualViewport.height
                    : Number.POSITIVE_INFINITY,
            ),
        );
        const anchorRect = anchor.getBoundingClientRect();
        const panelWidth = Math.round(measuredWidth || panel.getBoundingClientRect().width || 0);
        const anchorWidth = Math.round(anchorRect.width || 0);
        const edgeInset = 12;
        const widthSource = getStringAttribute(this, "panel-min-width");
        const maxWidth = visibleViewportRight - visualViewportLeft - edgeInset * 2;
        const width = Math.max(
            0,
            Math.min(
                widthSource === "panel" ? Math.max(anchorWidth, panelWidth) : (anchorWidth || panelWidth),
                Math.max(0, maxWidth),
            ),
        );
        this._clearDetachedResultsStyles();

        panel.style.position = "fixed";
        panel.style.margin = "0";
        panel.style.zIndex = "70";
        panel.style.width = width ? `${width}px` : "";
        panel.style.minWidth = "0";
        panel.style.maxWidth = `${Math.max(maxWidth, 160)}px`;
        panel.style.maxHeight = "";
        panel.style.overscrollBehavior = "";
        panel.style.left = `${visualViewportLeft + edgeInset}px`;
        panel.style.top = `${visualViewportTop + edgeInset}px`;

        const panelRect = panel.getBoundingClientRect();
        const minLeft = visualViewportLeft + edgeInset;
        let left = Math.max(minLeft, anchorRect.left);
        if (left + panelRect.width > visibleViewportRight - edgeInset) {
            left = Math.max(
                minLeft,
                visibleViewportRight - panelRect.width - edgeInset,
            );
        }

        const viewportBottom = Math.max(
            visualViewportTop + edgeInset + 1,
            visibleViewportBottom - edgeInset,
        );
        const minTop = visualViewportTop + edgeInset;
        const gap = 8;
        const belowTop = Math.round(anchorRect.bottom + gap);
        const naturalHeight = Math.round(panelRect.height || 0);
        const spaceBelow = Math.max(0, viewportBottom - belowTop);
        const spaceAbove = Math.max(0, anchorRect.top - gap - minTop);
        const placeBelow =
            spaceBelow >= naturalHeight
            || (spaceAbove < naturalHeight && spaceBelow >= spaceAbove);
        let topInViewport;
        let availableHeight;
        if (placeBelow) {
            topInViewport = Math.max(minTop, belowTop);
            availableHeight = Math.max(0, viewportBottom - topInViewport);
        } else {
            availableHeight = spaceAbove;
            const panelHeight = naturalHeight
                ? Math.min(naturalHeight, availableHeight)
                : availableHeight;
            topInViewport = Math.max(minTop, Math.round(anchorRect.top - gap - panelHeight));
        }

        panel.style.left = `${left}px`;
        panel.style.top = `${topInViewport}px`;
        this._constrainPanelHeight(
            panel,
            Math.min(naturalHeight || availableHeight, availableHeight),
        );
    }

    _constrainPanelHeight(panel, maxHeight) {
        const constrainedHeight = Math.floor(Number(maxHeight) || 0);
        if (!(panel instanceof HTMLElement) || constrainedHeight <= 0) {
            return;
        }

        const results = this.resultsElement();
        const panelRect = panel.getBoundingClientRect();
        if (results instanceof HTMLElement) {
            const resultsRect = results.getBoundingClientRect();
            const reservedHeight = Math.max(
                0,
                Math.round((panelRect.height || 0) - (resultsRect.height || 0)),
            );
            const resultsMaxHeight = Math.max(48, constrainedHeight - reservedHeight);
            results.style.maxHeight = `${resultsMaxHeight}px`;
            results.style.overflowY = "auto";
            results.style.overscrollBehavior = "contain";
        }

        panel.style.maxHeight = `${constrainedHeight}px`;
        panel.style.overscrollBehavior = "contain";
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

    _usesDetachedPanel() {
        return getStringAttribute(this, "panel-mode") === "detached";
    }

    _scheduleClose() {
        this._cancelClose();
        this._closeTimer = window.setTimeout(() => {
            this._closeTimer = 0;
            this.close();
        }, CLOSE_DELAY_MS);
    }

    _syncSelection() {
        const selection = this.selectionElement();
        if (!(selection instanceof HTMLElement)) {
            return;
        }

        const selectedValues = this.selectedValues();
        selection.hidden = selectedValues.length === 0;

        selection.replaceChildren(
            ...selectedValues.map((value) => {
                const template = this._findCatalogTemplateByValue(value);
                const label = String(
                    template?.getAttribute("data-label")
                        ?? this._findBoundOptionByValue(value)?.getAttribute("data-label")
                        ?? value,
                ).trim();

                const chip = document.createElement("div");
                chip.className = "join items-stretch rounded-box border border-base-300 bg-base-100 p-1 text-base-content shadow-sm";

                const content = document.createElement("span");
                content.className = "inline-flex min-w-0 items-center px-2 py-1 text-sm";
                if (template instanceof HTMLTemplateElement) {
                    content.append(...cloneChildNodes(template.content));
                } else {
                    content.textContent = label;
                }
                chip.append(content);

                const removeButton = document.createElement("button");
                removeButton.type = "button";
                removeButton.className = "btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70";
                removeButton.dataset.searchableMultiselectRemove = "";
                removeButton.setAttribute("data-value", value);
                removeButton.setAttribute("aria-label", `Remove ${label}`);
                removeButton.textContent = "×";
                chip.append(removeButton);
                return chip;
            }),
        );
        this._positionOpenDetachedPanel();
    }

    _syncBoundInputsFromMarkup() {
        for (const option of this.boundOptionElements()) {
            option.selected = option.hasAttribute("selected");
        }
    }

    _syncUi() {
        const searchInput = this.searchInputElement();
        if (searchInput instanceof HTMLInputElement && this.hasAttribute("placeholder")) {
            searchInput.placeholder = this.placeholder;
        }
    }

    _unbindInputs() {
        for (const release of this._releaseInputs) {
            release();
        }
        this._releaseInputs = [];
    }
}

export function registerSearchableMultiselect() {
    if (window.customElements.get("fishy-searchable-multiselect")) {
        return;
    }
    window.customElements.define(
        "fishy-searchable-multiselect",
        FishySearchableMultiselect,
    );
}
