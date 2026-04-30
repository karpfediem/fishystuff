import {
    getStringAttribute,
    setStringAttribute,
    upgradeProperty,
} from "./searchable-dropdown.js";

const UI_SETTINGS_KEY = "fishystuff.ui.settings.v1";
const DEFAULT_TITLE = "Notice";
const DEFAULT_ICON = "alert-triangle";
const DEFAULT_BODY_CLASS = "space-y-3 px-1 pb-1 pt-3";
const DETAILS_CLASS = "w-full rounded-box bg-base-200/45 p-2";
const SUMMARY_CLASS = "flex w-full items-center justify-between gap-3";
const SUMMARY_PREFIX_CLASS = "inline-flex items-center gap-2 leading-none";
const SUMMARY_TITLE_CLASS = "font-medium leading-none";
const ICON_CLASS = "fishy-icon size-[1.125rem] shrink-0 self-center opacity-70";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

function isPlainObject(value) {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
        return false;
    }
    const prototype = Object.getPrototypeOf(value);
    return prototype === Object.prototype || prototype === null;
}

export function normalizeNoticeSettingsPath(path) {
    return Array.isArray(path)
        ? path.map((part) => String(part || "").trim()).filter(Boolean)
        : String(path || "")
            .split(".")
            .map((part) => part.trim())
            .filter(Boolean);
}

function readJsonStorage(storage, key, fallback) {
    try {
        const raw = storage?.getItem?.(key);
        if (!raw) {
            return fallback;
        }
        const parsed = JSON.parse(raw);
        return parsed === undefined ? fallback : parsed;
    } catch (_error) {
        return fallback;
    }
}

function readAtPath(root, pathParts) {
    let current = isPlainObject(root) ? root : {};
    for (const part of pathParts) {
        if (!isPlainObject(current) || !(part in current)) {
            return undefined;
        }
        current = current[part];
    }
    return current;
}

function setAtPath(root, pathParts, value) {
    if (!pathParts.length) {
        return isPlainObject(value) ? value : {};
    }

    const nextRoot = isPlainObject(root) ? { ...root } : {};
    let cursor = nextRoot;
    for (const part of pathParts.slice(0, -1)) {
        cursor[part] = isPlainObject(cursor[part]) ? { ...cursor[part] } : {};
        cursor = cursor[part];
    }
    cursor[pathParts[pathParts.length - 1]] = value;
    return nextRoot;
}

function sharedUiSettingsStore(globalRef = globalThis) {
    const store = globalRef.window?.__fishystuffUiSettings ?? globalRef.__fishystuffUiSettings;
    return store
        && typeof store.get === "function"
        && typeof store.set === "function"
        && typeof store.subscribe === "function"
        ? store
        : null;
}

export function readPersistentNoticeDisclosureOpen(
    settingsPath,
    fallbackOpen,
    { globalRef = globalThis } = {},
) {
    const pathParts = normalizeNoticeSettingsPath(settingsPath);
    if (!pathParts.length) {
        return Boolean(fallbackOpen);
    }

    const store = sharedUiSettingsStore(globalRef);
    const current = store
        ? store.get(pathParts, undefined)
        : readAtPath(readJsonStorage(globalRef.localStorage, UI_SETTINGS_KEY, {}), pathParts);
    return typeof current === "boolean" ? current : Boolean(fallbackOpen);
}

export function writePersistentNoticeDisclosureOpen(
    settingsPath,
    open,
    { globalRef = globalThis } = {},
) {
    const pathParts = normalizeNoticeSettingsPath(settingsPath);
    if (!pathParts.length) {
        return false;
    }

    const normalizedOpen = Boolean(open);
    const store = sharedUiSettingsStore(globalRef);
    if (store) {
        store.set(pathParts, normalizedOpen);
        return true;
    }

    try {
        const nextSettings = setAtPath(
            readJsonStorage(globalRef.localStorage, UI_SETTINGS_KEY, {}),
            pathParts,
            normalizedOpen,
        );
        globalRef.localStorage?.setItem?.(UI_SETTINGS_KEY, JSON.stringify(nextSettings));
        return true;
    } catch (_error) {
        return false;
    }
}

function createSvgUse(icon) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", ICON_CLASS);
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");

    const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("width", "100%");
    use.setAttribute("height", "100%");
    use.setAttribute("href", `#fishy-${icon}`);
    svg.append(use);

    return svg;
}

function appendChildNodes(target, nodes) {
    for (const node of nodes) {
        target.appendChild(node);
    }
}

export class FishyNoticeDisclosure extends HTMLElementBase {
    static get observedAttributes() {
        return ["body-class", "icon", "open", "settings-path", "title"];
    }

    constructor() {
        super();
        this._contentElement = null;
        this._detailsElement = null;
        this._iconUseElement = null;
        this._titleElement = null;
        this._defaultOpen = null;
        this._suppressOpenReflection = false;
        this._unsubscribeSettings = null;

        this._handleToggle = this._handleToggle.bind(this);
        this._handleSettingsChange = this._handleSettingsChange.bind(this);
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

    get settingsPath() {
        return getStringAttribute(this, "settings-path");
    }

    set settingsPath(value) {
        setStringAttribute(this, "settings-path", value);
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
        upgradeProperty(this, "settingsPath");
        upgradeProperty(this, "title");
        if (this._defaultOpen === null) {
            this._defaultOpen = this.hasAttribute("open");
        }
        this._ensureShell();
        if (this._detailsElement) {
            this._detailsElement.removeEventListener("toggle", this._handleToggle);
            this._detailsElement.addEventListener("toggle", this._handleToggle);
        }
        this._subscribeToSettings();
        this._syncUi();
    }

    disconnectedCallback() {
        this._detailsElement?.removeEventListener("toggle", this._handleToggle);
        if (typeof this._unsubscribeSettings === "function") {
            this._unsubscribeSettings();
        }
        this._unsubscribeSettings = null;
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (oldValue === newValue) {
            return;
        }
        if (name === "settings-path") {
            this._subscribeToSettings();
        }
        if (name === "open" && this._suppressOpenReflection) {
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

    _resolvedOpenState() {
        return readPersistentNoticeDisclosureOpen(this.settingsPath, this._defaultOpen, {
            globalRef: globalThis,
        });
    }

    _reflectOpenAttribute(open) {
        this._suppressOpenReflection = true;
        if (open) {
            this.setAttribute("open", "");
        } else {
            this.removeAttribute("open");
        }
        this._suppressOpenReflection = false;
    }

    _subscribeToSettings() {
        if (typeof this._unsubscribeSettings === "function") {
            this._unsubscribeSettings();
        }
        this._unsubscribeSettings = null;

        const store = sharedUiSettingsStore(globalThis);
        if (store && typeof store.subscribe === "function") {
            this._unsubscribeSettings = store.subscribe(this._handleSettingsChange);
        }
    }

    _handleSettingsChange() {
        if (!normalizeNoticeSettingsPath(this.settingsPath).length) {
            return;
        }
        this._syncUi();
    }

    _handleToggle() {
        const details = this._detailsElement;
        if (!details || typeof details.open !== "boolean") {
            return;
        }
        this._reflectOpenAttribute(details.open);
        writePersistentNoticeDisclosureOpen(this.settingsPath, details.open, {
            globalRef: globalThis,
        });
    }

    _syncUi() {
        if (!this._contentElement || !this._detailsElement || !this._titleElement || !this._iconUseElement) {
            return;
        }
        this._contentElement.className = this.bodyClass || DEFAULT_BODY_CLASS;
        const resolvedOpen = this._resolvedOpenState();
        this._detailsElement.open = resolvedOpen;
        this._reflectOpenAttribute(resolvedOpen);
        this._iconUseElement.setAttribute("href", `#fishy-${this.icon}`);
        this._titleElement.textContent = this.title;
    }
}

export function registerNoticeDisclosure() {
    if (window.customElements.get("fishy-notice-disclosure")) {
        return;
    }

    window.customElements.define("fishy-notice-disclosure", FishyNoticeDisclosure);
}
