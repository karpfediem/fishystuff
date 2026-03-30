(function () {
  function isPlainObject(value) {
    return value && typeof value === "object" && !Array.isArray(value);
  }

  function readObjectPath(root, path) {
    return String(path ?? "")
      .split(".")
      .filter(Boolean)
      .reduce((current, key) => {
        if (current && typeof current === "object" && key in current) {
          return current[key];
        }
        return undefined;
      }, root);
  }

  function setObjectPath(root, path, value) {
    if (!root || typeof root !== "object") {
      return root;
    }
    const parts = String(path ?? "").split(".").filter(Boolean);
    if (!parts.length) {
      return root;
    }
    let current = root;
    for (const key of parts.slice(0, -1)) {
      if (!current[key] || typeof current[key] !== "object" || Array.isArray(current[key])) {
        current[key] = {};
      }
      current = current[key];
    }
    current[parts[parts.length - 1]] = value;
    return root;
  }

  function toggleBooleanPath(root, path) {
    return setObjectPath(root, path, !Boolean(readObjectPath(root, path)));
  }

  function mergeObjectPatch(root, patch) {
    if (!isPlainObject(root) || !isPlainObject(patch)) {
      return patch;
    }
    for (const [key, value] of Object.entries(patch)) {
      if (isPlainObject(value) && isPlainObject(root[key])) {
        mergeObjectPatch(root[key], value);
        continue;
      }
      root[key] = value;
    }
    return root;
  }

  function createSignalStore() {
    const state = {
      signals: null,
    };

    return Object.freeze({
      connect(signals) {
        state.signals = signals && typeof signals === "object" ? signals : null;
        return state.signals;
      },
      signalObject() {
        return state.signals && typeof state.signals === "object" ? state.signals : null;
      },
      patchSignals(patch) {
        const signals = this.signalObject();
        if (!signals || !patch || typeof patch !== "object") {
          return;
        }
        mergeObjectPatch(signals, patch);
      },
      readSignal(path) {
        return readObjectPath(this.signalObject(), path);
      },
    });
  }

  window.__fishystuffDatastarState = Object.freeze({
    createSignalStore,
    mergeObjectPatch,
    readObjectPath,
    setObjectPath,
    toggleBooleanPath,
  });
})();
