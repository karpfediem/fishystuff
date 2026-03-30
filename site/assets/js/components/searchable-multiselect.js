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
        this._releaseInputs = [];

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
        this._unbindInputs();
        this._cancelClose();
        this._detachOutsideListener();
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
        this.search(this.searchInputElement()?.value ?? "");
    }

    close() {
        this._cancelClose();
        const panel = this.panelElement();
        if (panel instanceof HTMLElement) {
            panel.hidden = true;
        }
        this.style.zIndex = "";
        this._detachOutsideListener();
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

    _detachOutsideListener() {
        if (!this._outsideListenerAttached) {
            return;
        }
        document.removeEventListener("pointerdown", this._handleDocumentPointerDown, true);
        this._outsideListenerAttached = false;
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
        if (option && this.contains(option)) {
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
        if (!shell || !this.contains(shell)) {
            return;
        }

        const searchInput = this.searchInputElement();
        if (searchInput instanceof HTMLInputElement && event.target !== searchInput) {
            searchInput.focus();
        }
        this.open();
    }

    _handleDocumentPointerDown(event) {
        if (!(event.target instanceof Node) || this.contains(event.target)) {
            return;
        }
        this.close();
    }

    _handleFocusIn(event) {
        if (!this.contains(event.target)) {
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
        if (nextTarget instanceof Node && this.contains(nextTarget)) {
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
        if (!(event.target instanceof Element) || !this.contains(event.target)) {
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
        if (!(event.target instanceof Element) || !this.contains(event.target)) {
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
