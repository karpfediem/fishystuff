export function readObjectPath(root, path) {
    return String(path ?? "")
        .split(".")
        .filter(Boolean)
        .reduce((current, key) => {
            if (current && typeof current === "object" && key in current) {
                return current[key];
            }
            return null;
        }, root);
}

export function readCalculatorSignal(path) {
    return readObjectPath(window.__fishystuffCalculator?.signalObject?.() ?? null, path);
}

export class FishyDatastarRenderElement extends HTMLElement {
    constructor() {
        super();
        this._rafId = 0;
        this._childObserver = null;
        this._resizeObserver = null;
        this._handleSignalPatchBound = () => this.scheduleRender();
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
            "datastar-patch-signals",
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
            "datastar-patch-signals",
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
