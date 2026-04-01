import {
  loadRestoreState,
} from "./map-page-state.js";
import {
  applyMapPageSignalsPatch,
} from "./map-page-signals.js";
import { createMapPagePersistController } from "./map-page-persist.js";

export const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
export const FISHYMAP_LIVE_INIT_EVENT = "fishymap-live-init";

export function createMapPageLive({ globalRef = globalThis } = {}) {
  const state = {
    shell: null,
    liveSignals: null,
    uiStateRestored: false,
    signalPatchListenerBound: false,
    initListenerBound: false,
    restoreResolved: false,
    restorePromise: null,
    resolveRestore: null,
  };
  state.restorePromise = new Promise((resolve) => {
    state.resolveRestore = resolve;
  });

  function signalObject() {
    return state.liveSignals && typeof state.liveSignals === "object" ? state.liveSignals : null;
  }

  function resolveShell() {
    const shell = globalRef.document?.getElementById?.("map-page-shell");
    return shell && typeof shell.dispatchEvent === "function" ? shell : null;
  }

  function consumeInitialSignals(shell) {
    if (!shell || state.uiStateRestored !== false || !("__fishymapInitialSignals" in shell)) {
      return null;
    }
    const signals = shell.__fishymapInitialSignals;
    delete shell.__fishymapInitialSignals;
    return signals && typeof signals === "object" ? signals : null;
  }

  function connect(signals) {
    state.liveSignals = signals && typeof signals === "object" ? signals : null;
    state.shell = resolveShell() || state.shell;
    return state.liveSignals;
  }

  function currentLocationHref() {
    return globalRef.location?.href || globalRef.window?.location?.href || "";
  }

  const persistor = createMapPagePersistController({
    globalRef,
    isReady: () => state.uiStateRestored,
    readSnapshot: signalObject,
  });

  function bindSignalPatchListener() {
    if (state.signalPatchListenerBound) {
      return;
    }
    globalRef.document?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, persistor.handleSignalPatch);
    state.signalPatchListenerBound = true;
  }

  function handleLiveInit(event) {
    if (event?.currentTarget && "__fishymapInitialSignals" in event.currentTarget) {
      delete event.currentTarget.__fishymapInitialSignals;
    }
    const signals = event?.detail;
    if (!signals || typeof signals !== "object") {
      return;
    }
    restore(signals);
  }

  function bindInitListener() {
    const shell = state.shell || resolveShell();
    if (!shell || state.initListenerBound) {
      return;
    }
    shell.addEventListener(FISHYMAP_LIVE_INIT_EVENT, handleLiveInit);
    state.shell = shell;
    state.initListenerBound = true;
  }

  function applyPatch(signals, patch) {
    const liveSignals = signals && typeof signals === "object" ? signals : state.liveSignals;
    if (!liveSignals || !patch || typeof patch !== "object") {
      return;
    }
    applyMapPageSignalsPatch(liveSignals, patch);
    connect(liveSignals);
    persistor.handleSignalPatch(patch);
  }

  function patchSignals(patch) {
    applyPatch(state.liveSignals, patch);
  }

  function restore(signals) {
    connect(signals);
    bindSignalPatchListener();
    const restoreState = loadRestoreState({
      localStorage: globalRef.localStorage,
      sessionStorage: globalRef.sessionStorage,
      locationHref: currentLocationHref(),
    });
    patchSignals(restoreState.sharedFishPatch);
    if (restoreState.uiPatch) {
      patchSignals(restoreState.uiPatch);
    }
    if (restoreState.bookmarkPatch) {
      patchSignals(restoreState.bookmarkPatch);
    }
    if (restoreState.sessionPatch) {
      patchSignals(restoreState.sessionPatch);
    }
    persistor.seed(signals);
    state.uiStateRestored = true;
    if (!state.restoreResolved) {
      state.restoreResolved = true;
      state.resolveRestore?.();
    }
  }

  function start() {
    state.shell = resolveShell();
    bindInitListener();
    const initialSignals = consumeInitialSignals(state.shell);
    if (initialSignals) {
      restore(initialSignals);
    }
  }

  return Object.freeze({
    patchSignals,
    signalObject,
    start,
    whenRestored() {
      return state.restorePromise;
    },
  });
}
