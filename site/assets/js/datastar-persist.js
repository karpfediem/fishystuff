(function () {
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";

  function patchMatchesSignalFilter(patch, filter, prefix = "") {
    if (!patch || typeof patch !== "object") {
      return false;
    }
    const include = filter?.include && typeof filter.include.test === "function"
      ? filter.include
      : null;
    const exclude = filter?.exclude && typeof filter.exclude.test === "function"
      ? filter.exclude
      : null;
    return Object.entries(patch).some(([key, value]) => {
      const path = prefix ? `${prefix}.${key}` : key;
      if (include) {
        if (include.test(path)) {
          return true;
        }
      } else if (exclude) {
        if (!exclude.test(path)) {
          return true;
        }
      }
      return value && typeof value === "object" && patchMatchesSignalFilter(value, filter, path);
    });
  }

  function createDebouncedSignalPatchPersistor(options = {}) {
    const target = options.target && typeof options.target.addEventListener === "function"
      ? options.target
      : document;
    const delayMs = Number.isFinite(options.delayMs) ? Math.max(0, options.delayMs) : 150;
    const isReady = typeof options.isReady === "function" ? options.isReady : () => true;
    const persist = typeof options.persist === "function" ? options.persist : () => {};
    const filter = options.filter && typeof options.filter === "object" ? options.filter : null;
    const shouldPersistPatch = typeof options.shouldPersistPatch === "function"
      ? options.shouldPersistPatch
      : (patch) => patchMatchesSignalFilter(patch, filter);
    const state = {
      bound: false,
      timer: 0,
    };

    function clearTimer() {
      if (!state.timer) {
        return;
      }
      globalThis.clearTimeout?.(state.timer);
      state.timer = 0;
    }

    function schedulePersist() {
      clearTimer();
      state.timer = globalThis.setTimeout?.(() => {
        state.timer = 0;
        persist();
      }, delayMs);
    }

    function handleSignalPatch(event) {
      if (!isReady()) {
        return;
      }
      const patch = event?.detail;
      if (!shouldPersistPatch(patch)) {
        return;
      }
      schedulePersist();
    }

    function bind() {
      if (state.bound) {
        return;
      }
      target.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
      state.bound = true;
    }

    function dispose() {
      if (!state.bound) {
        return;
      }
      clearTimer();
      target.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
      state.bound = false;
    }

    return Object.freeze({
      bind,
      dispose,
      clearTimer,
      schedulePersist,
    });
  }

  window.__fishystuffDatastarPersist = Object.freeze({
    DATASTAR_SIGNAL_PATCH_EVENT,
    patchMatchesSignalFilter,
    createDebouncedSignalPatchPersistor,
  });
})();
