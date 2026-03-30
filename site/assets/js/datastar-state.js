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

  function toggleOrderedValue(values, candidate, order = []) {
    const next = new Set(Array.isArray(values) ? values.map(String) : []);
    const normalizedCandidate = String(candidate ?? "");
    if (next.has(normalizedCandidate)) {
      next.delete(normalizedCandidate);
    } else {
      next.add(normalizedCandidate);
    }
    const normalizedOrder = Array.isArray(order) ? order.map(String) : [];
    if (!normalizedOrder.length) {
      return Array.from(next);
    }
    return normalizedOrder.filter((value) => next.has(value));
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

  function normalizeCounterTokenValue(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? Math.max(0, Math.trunc(numeric)) : 0;
  }

  function normalizeCounterTokenState(raw, defaults = {}) {
    const source = raw && typeof raw === "object" ? raw : {};
    return Object.fromEntries(
      Object.entries(defaults).map(([key, defaultValue]) => [
        key,
        normalizeCounterTokenValue(source[key] ?? defaultValue),
      ]),
    );
  }

  function consumeIncrementedCounterTokens(previousState, nextState, handlers = {}) {
    const previous = previousState && typeof previousState === "object" ? previousState : {};
    const next = nextState && typeof nextState === "object" ? nextState : {};
    const normalizedKeys = new Set([
      ...Object.keys(previous),
      ...Object.keys(next),
      ...Object.keys(handlers),
    ]);
    const handledState = {};
    let mutated = false;

    for (const key of normalizedKeys) {
      const previousValue = normalizeCounterTokenValue(previous[key]);
      const nextValue = normalizeCounterTokenValue(next[key]);
      handledState[key] = nextValue;
      if (nextValue > previousValue) {
        const handler = handlers[key];
        if (typeof handler === "function" && handler(nextValue, previousValue) === true) {
          mutated = true;
        }
      }
    }

    return {
      handledState,
      mutated,
    };
  }

  function createCounterTokenController(defaults = {}) {
    let handledState = normalizeCounterTokenState({}, defaults);

    return Object.freeze({
      current(raw) {
        return normalizeCounterTokenState(raw, defaults);
      },
      consume(raw, handlers = {}) {
        const consumption = consumeIncrementedCounterTokens(
          handledState,
          this.current(raw),
          handlers,
        );
        handledState = consumption.handledState;
        return consumption;
      },
      handledState() {
        return { ...handledState };
      },
      reset() {
        handledState = normalizeCounterTokenState({}, defaults);
      },
    });
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

  function createPageSignalStore() {
    return createSignalStore();
  }

  window.__fishystuffDatastarState = Object.freeze({
    consumeIncrementedCounterTokens,
    createCounterTokenController,
    createPageSignalStore,
    createSignalStore,
    mergeObjectPatch,
    normalizeCounterTokenState,
    readObjectPath,
    setObjectPath,
    toggleBooleanPath,
    toggleOrderedValue,
  });
})();
