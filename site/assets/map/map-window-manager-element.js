import { FISHYMAP_LIVE_INIT_EVENT, readMapShellSignals, resolveMapPageShell } from "./map-shell-signals.js";
import { dispatchShellSignalPatch } from "./map-signal-patch.js";
import { createMapWindowManager } from "./map-window-manager.js";

const WINDOW_MANAGER_TAG_NAME = "fishymap-window-manager";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

export class FishyMapWindowManagerElement extends HTMLElementBase {
  constructor() {
    super();
    this._shell = null;
    this._manager = null;
    this._handleLiveInit = () => {
      this._manager?.applyFromSignals?.();
    };
  }

  connectedCallback() {
    this.hidden = true;
    if (this._manager) {
      this._handleLiveInit();
      return;
    }
    const shell = this.closest?.("#map-page-shell") || resolveMapPageShell(globalThis);
    if (!shell) {
      return;
    }
    this._shell = shell;
    this._manager = createMapWindowManager({
      shell,
      dispatchPatch: dispatchShellSignalPatch,
      getSignals: () => readMapShellSignals(shell),
    });
    shell.addEventListener(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this._handleLiveInit();
  }

  disconnectedCallback() {
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
  }
}

export function registerFishyMapWindowManagerElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (!registry.get(WINDOW_MANAGER_TAG_NAME)) {
    registry.define(WINDOW_MANAGER_TAG_NAME, FishyMapWindowManagerElement);
  }
  return true;
}

registerFishyMapWindowManagerElement();
