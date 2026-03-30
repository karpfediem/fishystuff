import {
    dispatchValueEvents,
    getStringAttribute,
    setStringAttribute,
    upgradeProperty,
} from "./searchable-dropdown.js";

export class FishyCheckboxGroup extends HTMLElement {
    static get observedAttributes() {
        return ["bound-select-id"];
    }

    constructor() {
        super();
        this._releaseInputs = [];

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

    connectedCallback() {
        upgradeProperty(this, "boundSelectId");
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
        }
    }

    boundSelectElement() {
        const id = this.boundSelectId;
        if (id) {
            const root = this.ownerDocument?.getElementById(id) ?? null;
            return root?.querySelector('select[data-role="bound-select"]') ?? null;
        }
        return this.querySelector('select[data-role="bound-select"]');
    }

    checkboxElements() {
        return Array.from(this.querySelectorAll("input[data-checkbox-group-option]")).filter(
            (element) => element instanceof HTMLInputElement,
        );
    }

    boundOptionElements() {
        const select = this.boundSelectElement();
        if (!(select instanceof HTMLSelectElement)) {
            return [];
        }
        return Array.from(select.options).filter((element) => element instanceof HTMLOptionElement);
    }

    _bindInputs() {
        this._unbindInputs();

        const select = this.boundSelectElement();
        if (select instanceof HTMLSelectElement) {
            select.addEventListener("input", this._handleBoundInputEvent);
            select.addEventListener("change", this._handleBoundInputEvent);
            this._releaseInputs.push(() => {
                select.removeEventListener("input", this._handleBoundInputEvent);
                select.removeEventListener("change", this._handleBoundInputEvent);
            });
        }
    }

    _unbindInputs() {
        while (this._releaseInputs.length) {
            const release = this._releaseInputs.pop();
            if (typeof release === "function") {
                release();
            }
        }
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

        option.selected = target.checked;
        dispatchValueEvents(select);
    }

    _findBoundOptionByValue(value) {
        const normalized = String(value ?? "");
        return this.boundOptionElements().find((option) => option.value === normalized) ?? null;
    }

    _syncSelection() {
        const selectedValues = new Set(
            this.boundOptionElements()
                .filter((option) => option.selected)
                .map((option) => option.value),
        );

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
