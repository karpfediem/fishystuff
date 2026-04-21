import {
    getStringAttribute,
    setStringAttribute,
    upgradeProperty,
} from "./searchable-dropdown.js";

const DEFAULT_TITLE = "Notice";
const DEFAULT_ICON = "alert-triangle";
const DEFAULT_BODY_CLASS = "space-y-3 px-1 pb-1 pt-3";
const DETAILS_CLASS = "w-full rounded-box bg-base-200/45 p-2";
const SUMMARY_CLASS = "flex w-full items-center justify-between gap-3";
const SUMMARY_PREFIX_CLASS = "inline-flex items-center gap-2 leading-none";
const SUMMARY_TITLE_CLASS = "font-medium leading-none";
const ICON_CLASS = "fishy-icon size-[1.125rem] shrink-0 self-center opacity-70";

function createSvgUse(icon) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", ICON_CLASS);
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");

    const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("width", "100%");
    use.setAttribute("height", "100%");
    use.setAttribute("href", `/img/icons.svg?v=20260419-2#fishy-${icon}`);
    svg.append(use);

    return svg;
}

function appendChildNodes(target, nodes) {
    for (const node of nodes) {
        target.appendChild(node);
    }
}

export class FishyNoticeDisclosure extends HTMLElement {
    static get observedAttributes() {
        return ["body-class", "icon", "open", "title"];
    }

    constructor() {
        super();
        this._contentElement = null;
        this._detailsElement = null;
        this._iconUseElement = null;
        this._titleElement = null;

        this._handleToggle = this._handleToggle.bind(this);
    }

    get bodyClass() {
        return getStringAttribute(this, "body-class");
    }

    set bodyClass(value) {
        setStringAttribute(this, "body-class", value);
    }

    get icon() {
        return getStringAttribute(this, "icon") || DEFAULT_ICON;
    }

    set icon(value) {
        setStringAttribute(this, "icon", value || DEFAULT_ICON);
    }

    get open() {
        return this.hasAttribute("open");
    }

    set open(value) {
        if (value) {
            this.setAttribute("open", "");
            return;
        }
        this.removeAttribute("open");
    }

    get title() {
        return getStringAttribute(this, "title") || DEFAULT_TITLE;
    }

    set title(value) {
        setStringAttribute(this, "title", value || DEFAULT_TITLE);
    }

    connectedCallback() {
        upgradeProperty(this, "bodyClass");
        upgradeProperty(this, "icon");
        upgradeProperty(this, "open");
        upgradeProperty(this, "title");
        this._ensureShell();
        if (this._detailsElement) {
            this._detailsElement.removeEventListener("toggle", this._handleToggle);
            this._detailsElement.addEventListener("toggle", this._handleToggle);
        }
        this._syncUi();
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (oldValue === newValue) {
            return;
        }
        if (name === "open" && this._detailsElement) {
            const shouldOpen = this.open;
            if (this._detailsElement.open !== shouldOpen) {
                this._detailsElement.open = shouldOpen;
            }
        }
        this._syncUi();
    }

    _ensureShell() {
        if (this._detailsElement && this._contentElement && this._titleElement && this._iconUseElement) {
            return;
        }

        const preservedChildren = Array.from(this.childNodes);

        const menu = document.createElement("ul");
        menu.className = "menu menu-sm w-full bg-transparent p-0";
        menu.dataset.fishyNoticeRoot = "";

        const item = document.createElement("li");
        item.className = "w-full";

        const details = document.createElement("details");
        details.className = DETAILS_CLASS;
        details.addEventListener("toggle", this._handleToggle);

        const summary = document.createElement("summary");
        summary.className = SUMMARY_CLASS;

        const prefix = document.createElement("span");
        prefix.className = SUMMARY_PREFIX_CLASS;

        const iconSvg = createSvgUse(this.icon);
        const iconUse = iconSvg.querySelector("use");
        const title = document.createElement("span");
        title.className = SUMMARY_TITLE_CLASS;

        prefix.append(iconSvg);
        prefix.append(title);
        summary.append(prefix);

        const content = document.createElement("div");

        details.append(summary);
        details.append(content);
        item.append(details);
        menu.append(item);

        this.replaceChildren(menu);
        appendChildNodes(content, preservedChildren);

        this._contentElement = content;
        this._detailsElement = details;
        this._iconUseElement = iconUse;
        this._titleElement = title;
    }

    _handleToggle() {
        const details = this._detailsElement;
        if (!(details instanceof HTMLDetailsElement)) {
            return;
        }
        if (details.open) {
            if (!this.hasAttribute("open")) {
                this.setAttribute("open", "");
            }
            return;
        }
        if (this.hasAttribute("open")) {
            this.removeAttribute("open");
        }
    }

    _syncUi() {
        if (!this._contentElement || !this._detailsElement || !this._titleElement || !this._iconUseElement) {
            return;
        }
        this._contentElement.className = this.bodyClass || DEFAULT_BODY_CLASS;
        this._detailsElement.open = this.open;
        this._iconUseElement.setAttribute("href", `/img/icons.svg?v=20260419-2#fishy-${this.icon}`);
        this._titleElement.textContent = this.title;
    }
}

export function registerNoticeDisclosure() {
    if (window.customElements.get("fishy-notice-disclosure")) {
        return;
    }

    window.customElements.define("fishy-notice-disclosure", FishyNoticeDisclosure);
}
