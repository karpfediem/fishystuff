import {
    DATASTAR_SIGNAL_PATCH_EVENT,
    readObjectPath,
} from "../datastar-signals.js";

export function readCalculatorSignal(path) {
    return cloneSignalValue(readObjectPath(
        window.__fishystuffCalculator?.signalObject?.() ?? null,
        path,
    ));
}

export function cloneSignalValue(value, seen = new WeakMap()) {
    if (!value || typeof value !== "object") {
        return value;
    }
    if (seen.has(value)) {
        return seen.get(value);
    }
    const clone = Array.isArray(value) ? [] : {};
    seen.set(value, clone);
    let keys = [];
    try {
        keys = Object.keys(value);
    } catch (_error) {
        return clone;
    }
    for (const key of keys) {
        try {
            clone[key] = cloneSignalValue(value[key], seen);
        } catch (_error) {
            // Keep snapshots detached from live Datastar proxies even if a field
            // becomes unavailable while the signal tree is being patched.
        }
    }
    return clone;
}

function isPlainObject(value) {
    return Boolean(value) && Object.prototype.toString.call(value) === "[object Object]";
}

export function patchTouchesSignalPath(patch, path) {
    const parts = String(path ?? "")
        .split(".")
        .filter(Boolean);
    if (!parts.length || !isPlainObject(patch)) {
        return true;
    }

    let current = patch;
    for (const part of parts) {
        if (!isPlainObject(current) || !(part in current)) {
            return false;
        }
        current = current[part];
    }
    return true;
}

export class FishyDatastarRenderElement extends HTMLElement {
    constructor() {
        super();
        this._rafId = 0;
        this._childObserver = null;
        this._resizeObserver = null;
        this._handleSignalPatchBound = (event) => {
            if (patchTouchesSignalPath(event?.detail, this.signalPath())) {
                this.scheduleRender();
            }
        };
    }

    connectedCallback() {
        this.scheduleRender();
        if (this.observeChildren()) {
            this._childObserver = new MutationObserver(() => this.scheduleRender());
            this._childObserver.observe(this, { childList: true });
        }
        if (this.observeResize()) {
            this._resizeObserver = new ResizeObserver(() => this.scheduleRender());
            this._resizeObserver.observe(this);
        }
        document.addEventListener(
            DATASTAR_SIGNAL_PATCH_EVENT,
            this._handleSignalPatchBound,
        );
    }

    disconnectedCallback() {
        if (this._childObserver) {
            this._childObserver.disconnect();
            this._childObserver = null;
        }
        if (this._resizeObserver) {
            this._resizeObserver.disconnect();
            this._resizeObserver = null;
        }
        if (this._rafId) {
            cancelAnimationFrame(this._rafId);
            this._rafId = 0;
        }
        document.removeEventListener(
            DATASTAR_SIGNAL_PATCH_EVENT,
            this._handleSignalPatchBound,
        );
    }

    attributeChangedCallback(name, oldValue, newValue) {
        if (oldValue === newValue) {
            return;
        }
        this.scheduleRender();
    }

    observeChildren() {
        return false;
    }

    observeResize() {
        return false;
    }

    signalPath() {
        return this.getAttribute("signal-path") || "";
    }

    scheduleRender() {
        if (this._rafId) {
            cancelAnimationFrame(this._rafId);
        }
        this._rafId = requestAnimationFrame(() => {
            this._rafId = 0;
            this.renderFromSignals();
        });
    }

    replaceRenderedChildren(...nodes) {
        if (this._childObserver) {
            this._childObserver.disconnect();
        }
        this.replaceChildren(...nodes);
        if (this._childObserver && this.isConnected) {
            this._childObserver.observe(this, { childList: true });
        }
    }

    renderFromSignals() {
        throw new Error("renderFromSignals must be implemented by subclasses");
    }
}
