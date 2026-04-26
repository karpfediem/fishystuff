import {
    dispatchValueEvents,
    getStringAttribute,
    setStringAttribute,
    upgradeProperty,
} from "./searchable-dropdown.js";
import {
    bindBoundSelect,
    boundSelectOptions,
    findBoundSelectOption,
    resolveBoundSelectElement,
} from "./bound-select.js";

export class FishyCheckboxGroup extends HTMLElement {
    static get observedAttributes() {
        return ["bound-select-id", "max-selected"];
    }

    constructor() {
        super();
        this._releaseBoundSelect = null;

        this._handleBoundInputEvent = this._handleBoundInputEvent.bind(this);
        this._handleChange = this._handleChange.bind(this);

        this.addEventListener("change", this._handleChange);
    }

    get boundSelectId() {
        return getStringAttribute(this, "bound-select-id");
    }

    set boundSelectId(value) {
        setStringAttribute(this, "bound-select-id", value);
    }

    get maxSelected() {
        const value = Number.parseInt(this.getAttribute("max-selected") || "", 10);
        return Number.isFinite(value) && value > 0 ? value : null;
    }

    set maxSelected(value) {
        if (value === null || value === undefined || value === "") {
            this.removeAttribute("max-selected");
            return;
        }
        this.setAttribute("max-selected", String(value));
    }

    connectedCallback() {
        upgradeProperty(this, "boundSelectId");
        upgradeProperty(this, "maxSelected");
        this._bindInputs();
        queueMicrotask(() => {
            if (!this.isConnected) {
                return;
            }
            this._syncSelection();
        });
    }

    disconnectedCallback() {
        this._unbindInputs();
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (oldValue === newValue) {
            return;
        }
        if (name === "bound-select-id") {
            this._bindInputs();
            this._syncSelection();
        } else if (name === "max-selected") {
            this._syncSelection();
        }
    }

    boundSelectElement() {
        return resolveBoundSelectElement(this, this.boundSelectId);
    }

    checkboxElements() {
        return Array.from(this.querySelectorAll("input[data-checkbox-group-option]")).filter(
            (element) => element instanceof HTMLInputElement,
        );
    }

    boundOptionElements() {
        return boundSelectOptions(this.boundSelectElement());
    }

    _bindInputs() {
        this._unbindInputs();
        const select = this.boundSelectElement();
        this._releaseBoundSelect = bindBoundSelect(select, this._handleBoundInputEvent);
    }

    _unbindInputs() {
        if (typeof this._releaseBoundSelect === "function") {
            this._releaseBoundSelect();
        }
        this._releaseBoundSelect = null;
    }

    _handleBoundInputEvent() {
        this._syncSelection();
    }

    _handleChange(event) {
        const target = event.target;
        if (!(target instanceof HTMLInputElement) || !target.matches("input[data-checkbox-group-option]")) {
            return;
        }

        const select = this.boundSelectElement();
        const option = this._findBoundOptionByValue(target.value);
        if (!(select instanceof HTMLSelectElement) || !(option instanceof HTMLOptionElement)) {
            return;
        }

        const maxSelected = this.maxSelected;
        if (
            target.checked &&
            maxSelected !== null &&
            this.checkboxElements().filter((checkbox) => checkbox.checked).length > maxSelected
        ) {
            target.checked = false;
            option.selected = false;
            dispatchValueEvents(select);
            return;
        }

        option.selected = target.checked;
        dispatchValueEvents(select);
    }

    _findBoundOptionByValue(value) {
        return findBoundSelectOption(this.boundSelectElement(), value);
    }

    _syncSelection() {
        const options = this.boundOptionElements();
        const maxSelected = this.maxSelected;
        let selectedOptions = options.filter((option) => option.selected);
        if (maxSelected !== null && selectedOptions.length > maxSelected) {
            for (const option of selectedOptions.slice(maxSelected)) {
                option.selected = false;
            }
            selectedOptions = selectedOptions.slice(0, maxSelected);
            const select = this.boundSelectElement();
            if (select instanceof HTMLSelectElement) {
                dispatchValueEvents(select);
            }
        }
        const selectedValues = new Set(selectedOptions.map((option) => option.value));

        for (const checkbox of this.checkboxElements()) {
            checkbox.checked = selectedValues.has(checkbox.value);
        }
    }
}

export function registerCheckboxGroup() {
    if (window.customElements.get("fishy-checkbox-group")) {
        return;
    }

    window.customElements.define("fishy-checkbox-group", FishyCheckboxGroup);
}
