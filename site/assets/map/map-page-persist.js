import {
  MAP_BOOKMARKS_STORAGE_KEY,
  MAP_SESSION_STORAGE_KEY,
  MAP_UI_STORAGE_KEY,
  createPersistedState,
} from "./map-page-state.js";
import { patchMatchesMapPagePersistFilter } from "./map-page-signals.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";

export function createMapPagePersistController({
  globalRef = globalThis,
  shell = null,
  readSnapshot = () => null,
  isReady = () => true,
  createPersistedStateImpl = createPersistedState,
  shouldPersistPatch = patchMatchesMapPagePersistFilter,
  delayMs = 120,
  uiStorageKey = MAP_UI_STORAGE_KEY,
  bookmarksStorageKey = MAP_BOOKMARKS_STORAGE_KEY,
  sessionStorageKey = MAP_SESSION_STORAGE_KEY,
  listenToSignalPatches = true,
} = {}) {
  const state = {
    timer: 0,
    persistedUiJson: "",
    persistedBookmarksJson: "",
    persistedSessionJson: "",
  };

  function clearTimer() {
    if (!state.timer) {
      return;
    }
    globalRef.clearTimeout?.(state.timer);
    state.timer = 0;
  }

  function computePersistedState(snapshot) {
    if (!snapshot) {
      return null;
    }
    try {
      return createPersistedStateImpl(snapshot);
    } catch (_error) {
      return null;
    }
  }

  function seed(snapshot) {
    const persistedState = computePersistedState(snapshot);
    if (!persistedState) {
      return false;
    }
    state.persistedUiJson = persistedState.uiJson;
    state.persistedBookmarksJson = persistedState.bookmarksJson;
    state.persistedSessionJson = persistedState.sessionJson;
    return true;
  }

  function persistNow() {
    if (!isReady()) {
      return false;
    }
    const snapshot = readSnapshot();
    const persistedState = computePersistedState(snapshot);
    if (!persistedState) {
      return false;
    }
    try {
      if (persistedState.uiJson !== state.persistedUiJson) {
        globalRef.localStorage?.setItem?.(uiStorageKey, persistedState.uiJson);
        state.persistedUiJson = persistedState.uiJson;
      }
      if (persistedState.bookmarksJson !== state.persistedBookmarksJson) {
        globalRef.localStorage?.setItem?.(bookmarksStorageKey, persistedState.bookmarksJson);
        state.persistedBookmarksJson = persistedState.bookmarksJson;
      }
      if (persistedState.sessionJson !== state.persistedSessionJson) {
        globalRef.sessionStorage?.setItem?.(sessionStorageKey, persistedState.sessionJson);
        state.persistedSessionJson = persistedState.sessionJson;
      }
      return true;
    } catch (_error) {
      return false;
    }
  }

  function schedulePersist() {
    clearTimer();
    state.timer = globalRef.setTimeout?.(() => {
      state.timer = 0;
      persistNow();
    }, delayMs);
    return Boolean(state.timer);
  }

  function handleSignalPatch(eventOrPatch) {
    if (!isReady()) {
      return false;
    }
    const patch = eventOrPatch?.detail ?? eventOrPatch;
    if (!shouldPersistPatch(patch)) {
      return false;
    }
    schedulePersist();
    return true;
  }

  if (listenToSignalPatches && shell && typeof shell.addEventListener === "function") {
    shell.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, handleSignalPatch);
  }

  return Object.freeze({
    clearTimer,
    handleSignalPatch,
    persistNow,
    schedulePersist,
    seed,
  });
}
