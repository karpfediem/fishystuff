(function () {
  const STORAGE_KEY = "fishystuff.user-presets.v1";
  const CHANGED_EVENT = "fishystuff:user-presets-changed";
  const ADAPTERS_CHANGED_EVENT = "fishystuff:user-presets-adapters-changed";
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const EXPORT_FORMAT = "fishystuff-user-presets.v1";
  const CURRENT_EVENT_LIMIT = 100;
  const DEFAULT_FIXED_PRESET_ID = "default";

  function cloneJson(value) {
    return value == null ? value : JSON.parse(JSON.stringify(value));
  }

  function isPlainObject(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value);
  }

  function trimString(value) {
    const normalized = String(value ?? "").trim();
    return normalized || "";
  }

  function nowIso() {
    return new Date().toISOString();
  }

  function randomId(prefix) {
    const normalizedPrefix = trimString(prefix);
    if (globalThis.crypto && typeof globalThis.crypto.randomUUID === "function") {
      return `${normalizedPrefix}${globalThis.crypto.randomUUID()}`;
    }
    return `${normalizedPrefix}${Date.now().toString(16)}${Math.random().toString(16).slice(2)}`;
  }

  function normalizeCollectionKey(value) {
    return trimString(value)
      .toLowerCase()
      .replace(/[^a-z0-9_-]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function normalizeSource(value) {
    const source = isPlainObject(value) ? value : {};
    const kind = trimString(source.kind).toLowerCase();
    const id = trimString(source.id);
    if ((kind === "preset" || kind === "fixed") && id) {
      return { kind, id };
    }
    return { kind: "none", id: "" };
  }

  function presetSource(presetId) {
    const id = trimString(presetId);
    return id ? { kind: "preset", id } : { kind: "none", id: "" };
  }

  function fixedSource(fixedId) {
    const id = trimString(fixedId);
    return id ? { kind: "fixed", id } : { kind: "none", id: "" };
  }

  function sourceKey(source) {
    const normalized = normalizeSource(source);
    return normalized.kind === "none" ? "none" : `${normalized.kind}:${normalized.id}`;
  }

  function sourcesEqual(left, right) {
    return sourceKey(left) === sourceKey(right);
  }

  const collectionAdapters = new Map();

  function collectionAdapter(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    return key ? collectionAdapters.get(key) || null : null;
  }

  function defaultPresetName(collectionKey, index = 1) {
    const adapter = collectionAdapter(collectionKey);
    if (adapter && typeof adapter.defaultPresetName === "function") {
      const label = trimString(adapter.defaultPresetName(index));
      if (label) {
        return label;
      }
    }
    return `Preset ${Math.max(1, Number.parseInt(index, 10) || 1)}`;
  }

  function fixedPresets(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    const adapter = collectionAdapter(key);
    const entries = adapter && typeof adapter.fixedPresets === "function" ? adapter.fixedPresets() : [];
    if (!Array.isArray(entries)) {
      return [];
    }
    return entries
      .map((entry, index) => {
        if (!isPlainObject(entry)) {
          return null;
        }
        let payload;
        try {
          payload = normalizePresetPayload(key, entry.payload);
        } catch (_error) {
          return null;
        }
        return {
          id: trimString(entry.id) || (index === 0 ? DEFAULT_FIXED_PRESET_ID : `fixed_${index + 1}`),
          name: trimString(entry.name) || (index === 0 ? "Default" : `Fixed ${index + 1}`),
          payload,
        };
      })
      .filter(Boolean);
  }

  function firstFixedSource(collectionKey) {
    const firstFixed = fixedPresets(collectionKey)[0] || null;
    return firstFixed ? fixedSource(firstFixed.id) : fixedSource(DEFAULT_FIXED_PRESET_ID);
  }

  function normalizePresetPayload(collectionKey, payload) {
    const adapter = collectionAdapter(collectionKey);
    let normalized = payload;
    if (adapter && typeof adapter.normalizePayload === "function") {
      normalized = adapter.normalizePayload(payload);
    }
    if (!isPlainObject(normalized)) {
      throw new Error("Preset payload must be an object.");
    }
    return cloneJson(normalized);
  }

  function stablePresetPayloadJson(collectionKey, payload) {
    return JSON.stringify(normalizePresetPayload(collectionKey, payload));
  }

  function payloadsEqual(collectionKey, left, right) {
    if (!left || !right) {
      return false;
    }
    const adapter = collectionAdapter(collectionKey);
    if (adapter && typeof adapter.payloadsEqual === "function") {
      try {
        return adapter.payloadsEqual(
          normalizePresetPayload(collectionKey, left),
          normalizePresetPayload(collectionKey, right),
        ) === true;
      } catch (_error) {
        return false;
      }
    }
    try {
      return stablePresetPayloadJson(collectionKey, left) === stablePresetPayloadJson(collectionKey, right);
    } catch (_error) {
      return false;
    }
  }

  function normalizePresetEvent(collectionKey, value) {
    if (!isPlainObject(value)) {
      return null;
    }
    let beforePayload = null;
    let afterPayload = null;
    if (Object.prototype.hasOwnProperty.call(value, "beforePayload")) {
      try {
        beforePayload = normalizePresetPayload(collectionKey, value.beforePayload);
      } catch (_error) {
        beforePayload = null;
      }
    }
    if (Object.prototype.hasOwnProperty.call(value, "afterPayload")) {
      try {
        afterPayload = normalizePresetPayload(collectionKey, value.afterPayload);
      } catch (_error) {
        afterPayload = null;
      }
    }
    if (!beforePayload && !afterPayload) {
      return null;
    }
    return {
      id: trimString(value.id) || randomId("preset_event_"),
      action: trimString(value.action) || "payload-change",
      at: trimString(value.at) || nowIso(),
      source: normalizeSource(value.source || value.origin),
      beforePayload,
      afterPayload,
    };
  }

  function normalizePresetEvents(collectionKey, value) {
    return (Array.isArray(value) ? value : [])
      .map((entry) => normalizePresetEvent(collectionKey, entry))
      .filter(Boolean)
      .slice(-CURRENT_EVENT_LIMIT);
  }

  function createPayloadEvent(collectionKey, action, source, beforePayload, afterPayload) {
    return normalizePresetEvent(collectionKey, {
      id: randomId("preset_event_"),
      action,
      at: nowIso(),
      source,
      beforePayload,
      afterPayload,
    });
  }

  function normalizePresetEntry(collectionKey, value, index = 0) {
    if (!isPlainObject(value)) {
      return null;
    }
    let payload;
    try {
      payload = normalizePresetPayload(collectionKey, value.payload);
    } catch (_error) {
      return null;
    }
    const createdAt = trimString(value.createdAt) || nowIso();
    return {
      id: trimString(value.id) || randomId("preset_"),
      name: trimString(value.name) || defaultPresetName(collectionKey, index + 1),
      payload,
      createdAt,
      updatedAt: trimString(value.updatedAt) || createdAt,
    };
  }

  function normalizeWorkingCopyEntry(collectionKey, value, index = 0) {
    if (!isPlainObject(value)) {
      return null;
    }
    const source = normalizeSource(value.source || value.origin);
    let payload = null;
    if (Object.prototype.hasOwnProperty.call(value, "payload") && value.payload != null) {
      try {
        payload = normalizePresetPayload(collectionKey, value.payload);
      } catch (_error) {
        payload = null;
      }
    }
    if (source.kind === "none" && !payload) {
      return null;
    }
    const createdAt = trimString(value.createdAt) || nowIso();
    return {
      id: trimString(value.id) || randomId(`work_${index + 1}_`),
      source,
      payload,
      createdAt,
      updatedAt: trimString(value.updatedAt) || createdAt,
      events: normalizePresetEvents(collectionKey, value.events),
      undoneEvents: normalizePresetEvents(collectionKey, value.undoneEvents),
    };
  }

  function normalizeCollectionSnapshot(collectionKey, value) {
    const key = normalizeCollectionKey(collectionKey);
    const source = isPlainObject(value) ? value : {};
    const seenPresets = new Set();
    const presets = [];
    for (const [index, rawPreset] of (Array.isArray(source.presets) ? source.presets : []).entries()) {
      const preset = normalizePresetEntry(key, rawPreset, index);
      if (!preset || seenPresets.has(preset.id)) {
        continue;
      }
      seenPresets.add(preset.id);
      presets.push(preset);
    }

    const seenWorkingCopies = new Set();
    const workingCopies = [];
    for (const [index, rawWorkingCopy] of (Array.isArray(source.workingCopies) ? source.workingCopies : []).entries()) {
      const workingCopy = normalizeWorkingCopyEntry(key, rawWorkingCopy, index);
      if (!workingCopy || seenWorkingCopies.has(workingCopy.id)) {
        continue;
      }
      seenWorkingCopies.add(workingCopy.id);
      workingCopies.push(workingCopy);
    }

    let activeWorkingCopyId = trimString(source.activeWorkingCopyId);
    if (!workingCopies.some((workingCopy) => workingCopy.id === activeWorkingCopyId)) {
      activeWorkingCopyId = workingCopies[0]?.id || "";
    }

    return {
      presets,
      workingCopies,
      activeWorkingCopyId,
    };
  }

  function collectionHasState(collection) {
    return Boolean(
      collection?.presets?.length
      || collection?.workingCopies?.length
      || collection?.activeWorkingCopyId,
    );
  }

  function assignCollection(draft, collectionKey, collection) {
    if (collectionHasState(collection)) {
      draft.collections[collectionKey] = {
        presets: cloneJson(collection.presets || []),
        workingCopies: cloneJson(collection.workingCopies || []),
        activeWorkingCopyId: trimString(collection.activeWorkingCopyId),
      };
    } else {
      delete draft.collections[collectionKey];
    }
  }

  function normalizeSnapshot(value) {
    const source = isPlainObject(value) ? value : {};
    const collections = {};
    const rawCollections = isPlainObject(source.collections) ? source.collections : {};
    for (const [rawKey, rawCollection] of Object.entries(rawCollections)) {
      const key = normalizeCollectionKey(rawKey);
      if (!key) {
        continue;
      }
      const normalizedCollection = normalizeCollectionSnapshot(key, rawCollection);
      if (collectionHasState(normalizedCollection)) {
        collections[key] = normalizedCollection;
      }
    }
    return { collections };
  }

  function stableSnapshotJson(snapshot) {
    return JSON.stringify(normalizeSnapshot(snapshot));
  }

  function loadSnapshot(storage) {
    try {
      const raw = storage?.getItem?.(STORAGE_KEY);
      if (!raw) {
        return normalizeSnapshot({});
      }
      return normalizeSnapshot(JSON.parse(raw));
    } catch (_error) {
      return normalizeSnapshot({});
    }
  }

  function persistSnapshot(snapshot, storage) {
    const normalized = normalizeSnapshot(snapshot);
    try {
      storage?.setItem?.(STORAGE_KEY, JSON.stringify(normalized));
    } catch (_error) {
      return normalized;
    }
    return normalized;
  }

  let currentSnapshot = loadSnapshot(globalThis.localStorage);
  let datastarVersion = 0;
  const boundDatastarSignals = new Set();

  function sourcePayload(collectionKey, collection, source) {
    const normalized = normalizeSource(source);
    if (normalized.kind === "preset") {
      return collection.presets.find((preset) => preset.id === normalized.id)?.payload || null;
    }
    if (normalized.kind === "fixed") {
      return fixedPresets(collectionKey).find((preset) => preset.id === normalized.id)?.payload || null;
    }
    return null;
  }

  function sourcePreset(collectionKey, collection, source) {
    const normalized = normalizeSource(source);
    if (normalized.kind === "preset") {
      return cloneJson(collection.presets.find((preset) => preset.id === normalized.id) || null);
    }
    if (normalized.kind === "fixed") {
      return cloneJson(fixedPresets(collectionKey).find((preset) => preset.id === normalized.id) || null);
    }
    return null;
  }

  function createWorkingCopy(collectionKey, source, payload = null, options = {}) {
    const normalizedSource = normalizeSource(source);
    let normalizedPayload = null;
    if (payload) {
      try {
        normalizedPayload = normalizePresetPayload(collectionKey, payload);
      } catch (_error) {
        normalizedPayload = null;
      }
    }
    const createdAt = nowIso();
    return {
      id: trimString(options.id) || randomId("work_"),
      source: normalizedSource,
      payload: normalizedPayload,
      createdAt,
      updatedAt: createdAt,
      events: [],
      undoneEvents: [],
    };
  }

  function virtualWorkingCopyId(collectionKey, source) {
    const normalized = normalizeSource(source);
    const key = normalizeCollectionKey(collectionKey);
    const sourceId = normalized.kind === "none" ? "none" : `${normalized.kind}_${normalized.id}`;
    return `virtual_work_${key}_${sourceId}`.replace(/[^a-z0-9_-]+/g, "_");
  }

  function activeWorkingCopyFromCollection(collection) {
    return collection.workingCopies.find((workingCopy) => workingCopy.id === collection.activeWorkingCopyId) || null;
  }

  function workingCopyPayload(collectionKey, collection, workingCopy) {
    if (!workingCopy) {
      return null;
    }
    if (workingCopy.payload) {
      return workingCopy.payload;
    }
    return sourcePayload(collectionKey, collection, workingCopy.source);
  }

  function hydrateWorkingCopyPayload(collectionKey, collection, workingCopy) {
    if (!workingCopy || workingCopy.payload) {
      return workingCopy;
    }
    const payload = sourcePayload(collectionKey, collection, workingCopy.source);
    if (payload) {
      workingCopy.payload = normalizePresetPayload(collectionKey, payload);
      workingCopy.updatedAt = nowIso();
    }
    return workingCopy;
  }

  function workingCopyModified(collectionKey, collection, workingCopy) {
    const payload = workingCopyPayload(collectionKey, collection, workingCopy);
    const originPayload = sourcePayload(collectionKey, collection, workingCopy?.source);
    return Boolean(payload && !payloadsEqual(collectionKey, payload, originPayload));
  }

  function workingCopyCompat(collectionKey, collection, workingCopy) {
    if (!workingCopy) {
      return null;
    }
    const payload = workingCopyPayload(collectionKey, collection, workingCopy);
    return {
      id: workingCopy.id,
      source: cloneJson(workingCopy.source),
      origin: cloneJson(workingCopy.source),
      payload: cloneJson(payload),
      updatedAt: workingCopy.updatedAt,
      events: cloneJson(workingCopy.events || []),
      undoneEvents: cloneJson(workingCopy.undoneEvents || []),
      modified: workingCopyModified(collectionKey, collection, workingCopy),
    };
  }

  function selectedSource(collection) {
    const active = activeWorkingCopyFromCollection(collection);
    return normalizeSource(active?.source);
  }

  function selectedPresetIdFromCollection(collection) {
    const source = selectedSource(collection);
    return source.kind === "preset" ? source.id : "";
  }

  function selectedFixedIdFromCollection(collection) {
    const source = selectedSource(collection);
    return source.kind === "fixed" ? source.id : "";
  }

  function derivedCollection(collectionKey, collection) {
    const active = activeWorkingCopyFromCollection(collection);
    const current = active && workingCopyModified(collectionKey, collection, active)
      ? workingCopyCompat(collectionKey, collection, active)
      : null;
    return {
      presets: cloneJson(collection.presets),
      workingCopies: cloneJson(collection.workingCopies.map((workingCopy) => ({
        ...workingCopy,
        payload: workingCopyPayload(collectionKey, collection, workingCopy),
        origin: workingCopy.source,
        modified: workingCopyModified(collectionKey, collection, workingCopy),
      }))),
      activeWorkingCopyId: collection.activeWorkingCopyId,
      activeWorkingCopy: active ? workingCopyCompat(collectionKey, collection, active) : null,
      selectedPresetId: selectedPresetIdFromCollection(collection),
      selectedFixedId: selectedFixedIdFromCollection(collection),
      current,
    };
  }

  function ensureActiveWorkingCopy(collectionKey, collection, preferredSource = null, options = {}) {
    let active = activeWorkingCopyFromCollection(collection);
    if (active) {
      hydrateWorkingCopyPayload(collectionKey, collection, active);
      return active;
    }
    const source = normalizeSource(preferredSource).kind !== "none"
      ? normalizeSource(preferredSource)
      : firstFixedSource(collectionKey);
    const payload = sourcePayload(collectionKey, collection, source);
    active = createWorkingCopy(collectionKey, source, payload, options.virtual === true
      ? { id: virtualWorkingCopyId(collectionKey, source) }
      : {});
    collection.workingCopies.push(active);
    collection.activeWorkingCopyId = active.id;
    return active;
  }

  function pruneCleanInactiveWorkingCopies(collectionKey, collection) {
    const activeId = trimString(collection.activeWorkingCopyId);
    collection.workingCopies = collection.workingCopies.filter((workingCopy) => (
      workingCopy.id === activeId || workingCopyModified(collectionKey, collection, workingCopy)
    ));
  }

  function collectionCurrentActionState(collectionKey, value = null) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = value
      ? normalizeCollectionSnapshot(key, value)
      : currentCollectionCanonical(key);
    const active = activeWorkingCopyFromCollection(collection);
    const source = normalizeSource(active?.source);
    const sourcePayloadValue = active ? sourcePayload(key, collection, source) : null;
    const modified = Boolean(active && workingCopyModified(key, collection, active));
    const sourceIsSavedPreset = source.kind === "preset"
      && collection.presets.some((preset) => preset.id === source.id);
    const saveAction = modified
      ? (sourceIsSavedPreset ? "saved" : "created")
      : "none";
    return {
      hasCurrent: modified,
      current: modified ? workingCopyCompat(key, collection, active) : null,
      currentOrigin: source,
      selectedSource: source,
      activeWorkingCopy: active ? workingCopyCompat(key, collection, active) : null,
      activeWorkingCopyId: active?.id || "",
      isModified: modified,
      canSave: modified,
      canDiscard: Boolean(modified && sourcePayloadValue),
      saveAction,
      saveTargetPresetId: sourceIsSavedPreset ? source.id : "",
      discardSource: sourcePayloadValue ? source : { kind: "none", id: "" },
    };
  }

  function datastarCollectionSummary(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key) {
      return null;
    }
    const collection = currentCollectionCanonical(key);
    const actionState = collectionCurrentActionState(key, collection);
    const adapterFixedPresets = fixedPresets(key);
    const previewFixedPresets = globalThis.window?.__fishystuffPresetPreviews?.fixedPresets?.(key) || [];
    return {
      selectedPresetId: selectedPresetIdFromCollection(collection),
      selectedFixedId: selectedFixedIdFromCollection(collection),
      activeWorkingCopyId: actionState.activeWorkingCopyId,
      hasCurrent: actionState.hasCurrent,
      currentOrigin: actionState.currentOrigin,
      canSave: actionState.canSave,
      canDiscard: actionState.canDiscard,
      saveAction: actionState.saveAction,
      presetCount: collection.presets.length,
      fixedPresetCount: adapterFixedPresets.length || previewFixedPresets.length,
      workingCopyCount: collection.workingCopies.length,
    };
  }

  function datastarSnapshot() {
    const collectionKeys = new Set([
      ...Object.keys(currentSnapshot.collections || {}),
      ...Array.from(collectionAdapters.keys()),
    ]);
    const collections = {};
    for (const key of Array.from(collectionKeys).sort()) {
      const summary = datastarCollectionSummary(key);
      if (summary) {
        collections[key] = summary;
      }
    }
    return {
      version: datastarVersion,
      collections,
    };
  }

  function patchBoundDatastarSignals() {
    const snapshot = datastarSnapshot();
    const serializedSnapshot = JSON.stringify(snapshot);
    for (const signals of boundDatastarSignals) {
      if (!signals || typeof signals !== "object") {
        continue;
      }
      if (JSON.stringify(signals._user_presets ?? null) === serializedSnapshot) {
        continue;
      }
      signals._user_presets = cloneJson(snapshot);
    }
    return snapshot;
  }

  function boundDatastarSignalsNeedPatch(serializedSnapshot) {
    for (const signals of boundDatastarSignals) {
      if (!signals || typeof signals !== "object") {
        continue;
      }
      if (JSON.stringify(signals._user_presets ?? null) !== serializedSnapshot) {
        return true;
      }
    }
    return false;
  }

  function dispatchDatastarSignalPatch(snapshot) {
    globalThis.document?.dispatchEvent?.(
      new CustomEvent(DATASTAR_SIGNAL_PATCH_EVENT, {
        detail: {
          _user_presets: cloneJson(snapshot),
        },
      }),
    );
  }

  function refreshDatastar() {
    const snapshot = datastarSnapshot();
    const stale = boundDatastarSignalsNeedPatch(JSON.stringify(snapshot));
    if (stale) {
      datastarVersion += 1;
    }
    const nextSnapshot = patchBoundDatastarSignals();
    if (stale) {
      dispatchDatastarSignalPatch(nextSnapshot);
    }
    return cloneJson(nextSnapshot);
  }

  function bindDatastar(signals) {
    if (!signals || typeof signals !== "object") {
      return null;
    }
    boundDatastarSignals.add(signals);
    signals._user_presets = datastarSnapshot();
    return signals;
  }

  function unbindDatastar(signals) {
    boundDatastarSignals.delete(signals);
  }

  function emitChange(reason, detail = {}) {
    datastarVersion += 1;
    const datastar = patchBoundDatastarSignals();
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(CHANGED_EVENT, {
        detail: {
          reason: trimString(reason) || "update",
          snapshot: cloneJson(currentSnapshot),
          datastar,
          ...detail,
        },
      }),
    );
  }

  function emitAdaptersChange(collectionKey) {
    datastarVersion += 1;
    const datastar = patchBoundDatastarSignals();
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(ADAPTERS_CHANGED_EVENT, {
        detail: {
          collectionKey: normalizeCollectionKey(collectionKey),
          datastar,
        },
      }),
    );
  }

  function replaceSnapshot(snapshot, reason, detail = {}) {
    const nextSnapshot = normalizeSnapshot(snapshot);
    if (stableSnapshotJson(nextSnapshot) === stableSnapshotJson(currentSnapshot)) {
      return cloneJson(currentSnapshot);
    }
    currentSnapshot = persistSnapshot(nextSnapshot, globalThis.localStorage);
    emitChange(reason, detail);
    return cloneJson(currentSnapshot);
  }

  function updateSnapshot(mutator, reason, detail = {}) {
    const draft = cloneJson(currentSnapshot);
    mutator(draft);
    return replaceSnapshot(draft, reason, detail);
  }

  function currentCollectionCanonical(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = normalizeCollectionSnapshot(key, currentSnapshot.collections[key]);
    ensureActiveWorkingCopy(key, collection, null, { virtual: true });
    return collection;
  }

  function currentCollection(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    return derivedCollection(key, currentCollectionCanonical(key));
  }

  function currentPreset(collectionKey, presetId) {
    const collection = currentCollectionCanonical(collectionKey);
    const normalizedId = trimString(presetId);
    return cloneJson(collection.presets.find((preset) => preset.id === normalizedId) || null);
  }

  function selectedPreset(collectionKey) {
    const collection = currentCollectionCanonical(collectionKey);
    const presetId = selectedPresetIdFromCollection(collection);
    return cloneJson(collection.presets.find((preset) => preset.id === presetId) || null);
  }

  function currentPresetState(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollectionCanonical(key);
    const active = activeWorkingCopyFromCollection(collection);
    return active && workingCopyModified(key, collection, active)
      ? workingCopyCompat(key, collection, active)
      : null;
  }

  function activeWorkingCopy(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollectionCanonical(key);
    return workingCopyCompat(key, collection, activeWorkingCopyFromCollection(collection));
  }

  function workingCopies(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollectionCanonical(key);
    const includeClean = options.includeClean === true;
    return collection.workingCopies
      .filter((workingCopy) => includeClean || workingCopyModified(key, collection, workingCopy))
      .map((workingCopy) => workingCopyCompat(key, collection, workingCopy));
  }

  function fixedPreset(collectionKey, fixedId) {
    const normalizedId = trimString(fixedId);
    return cloneJson(fixedPresets(collectionKey).find((preset) => preset.id === normalizedId) || null);
  }

  function capturePayload(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const adapter = collectionAdapter(key);
    if (!adapter || typeof adapter.capture !== "function") {
      return null;
    }
    const captured = adapter.capture(options);
    if (!captured || typeof captured !== "object") {
      return null;
    }
    return normalizePresetPayload(key, captured);
  }

  function applyPayload(collectionKey, payload) {
    const key = normalizeCollectionKey(collectionKey);
    const adapter = collectionAdapter(key);
    if (!adapter || typeof adapter.apply !== "function" || !payload) {
      return null;
    }
    return adapter.apply(normalizePresetPayload(key, payload));
  }

  function observedPayloadAfterApply(collectionKey, fallbackPayload, appliedValue) {
    const key = normalizeCollectionKey(collectionKey);
    const captured = capturePayload(key);
    if (captured) {
      return captured;
    }
    if (appliedValue && typeof appliedValue === "object") {
      try {
        return normalizePresetPayload(key, appliedValue);
      } catch (_error) {
        return normalizePresetPayload(key, fallbackPayload);
      }
    }
    return fallbackPayload ? normalizePresetPayload(key, fallbackPayload) : null;
  }

  function activeSourcePayload(collectionKey, collection, active) {
    return sourcePayload(collectionKey, collection, active?.source);
  }

  function selectWorkingCopy(collectionKey, workingCopyId, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedId = trimString(workingCopyId);
    let selected = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const workingCopy = collection.workingCopies.find((candidate) => candidate.id === normalizedId) || null;
      if (!workingCopy) {
        assignCollection(draft, key, collection);
        return;
      }
      hydrateWorkingCopyPayload(key, collection, workingCopy);
      collection.activeWorkingCopyId = workingCopy.id;
      selected = workingCopyCompat(key, collection, workingCopy);
      assignCollection(draft, key, collection);
    }, "select-working-copy", { collectionKey: key, workingCopyId: normalizedId });

    if (selected?.payload && options.apply !== false) {
      const appliedValue = applyPayload(key, selected.payload);
      const observed = observedPayloadAfterApply(key, selected.payload, appliedValue);
      if (observed) {
        trackCurrentPayload(key, {
          payload: observed,
          refreshCurrent: false,
        });
      }
    }
    return cloneJson(selected);
  }

  function createAndSelectWorkingCopy(collectionKey, source, payload, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedSource = normalizeSource(source);
    const workingCopy = createWorkingCopy(key, normalizedSource, payload);
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      collection.workingCopies.push(workingCopy);
      collection.activeWorkingCopyId = workingCopy.id;
      pruneCleanInactiveWorkingCopies(key, collection);
      assignCollection(draft, key, collection);
    }, "create-working-copy", { collectionKey: key, workingCopyId: workingCopy.id });

    if (
      workingCopy.payload
      && options.apply !== false
      && !payloadsEqual(key, capturePayload(key), workingCopy.payload)
    ) {
      const appliedValue = applyPayload(key, workingCopy.payload);
      const observed = observedPayloadAfterApply(key, workingCopy.payload, appliedValue);
      if (observed) {
        trackCurrentPayload(key, {
          payload: observed,
          refreshCurrent: false,
        });
      }
    }
    return activeWorkingCopy(key);
  }

  function activateSource(collectionKey, source, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollectionCanonical(key);
    const normalizedSource = normalizeSource(source);
    const payload = sourcePayload(key, collection, normalizedSource);
    if (!payload && normalizedSource.kind === "preset") {
      return null;
    }
    return createAndSelectWorkingCopy(key, normalizedSource, payload, options);
  }

  function activatePreset(collectionKey, presetId) {
    const key = normalizeCollectionKey(collectionKey);
    const preset = currentPreset(key, presetId);
    if (!preset) {
      return null;
    }
    activateSource(key, presetSource(preset.id));
    return preset;
  }

  function activateFixedPreset(collectionKey, fixedId) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedFixedId = trimString(fixedId) || DEFAULT_FIXED_PRESET_ID;
    const preset = fixedPreset(key, normalizedFixedId);
    activateSource(key, fixedSource(normalizedFixedId));
    return preset || {
      id: normalizedFixedId,
      name: normalizedFixedId,
      payload: null,
    };
  }

  function refreshCurrentForAction(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key || options.refreshCurrent === false) {
      return currentCollectionCanonical(key);
    }
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const payload = hasPayload ? options.payload : capturePayload(key, options);
    if (!payload) {
      return currentCollectionCanonical(key);
    }
    trackCurrentPayload(key, {
      payload,
      refreshCurrent: false,
    });
    return currentCollectionCanonical(key);
  }

  function currentActionState(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = options.refresh === true
      ? refreshCurrentForAction(key, options)
      : currentCollectionCanonical(key);
    const state = collectionCurrentActionState(key, collection);
    if (options.refresh === true && options.patchDatastar !== false) {
      refreshDatastar();
    }
    return state;
  }

  function canSaveCurrent(collectionKey, options = {}) {
    return currentActionState(collectionKey, options).canSave;
  }

  function canDiscardCurrent(collectionKey, options = {}) {
    return currentActionState(collectionKey, options).canDiscard;
  }

  function trackCurrentPayload(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const payload = hasPayload ? normalizePresetPayload(key, options.payload) : capturePayload(key, options);
    let result = {
      action: "cleared",
      kind: "none",
      source: { kind: "none", id: "" },
      current: null,
      preset: null,
      workingCopy: null,
    };
    if (!payload) {
      return result;
    }
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const hadActive = Boolean(activeWorkingCopyFromCollection(collection));
      const active = ensureActiveWorkingCopy(
        key,
        collection,
        Object.prototype.hasOwnProperty.call(options, "origin") ? normalizeSource(options.origin) : null,
      );
      hydrateWorkingCopyPayload(key, collection, active);
      const previousPayload = workingCopyPayload(key, collection, active) || activeSourcePayload(key, collection, active) || payload;
      const previousModified = workingCopyModified(key, collection, active);
      if (payloadsEqual(key, previousPayload, payload)) {
        const modified = workingCopyModified(key, collection, active);
        if (!modified && (active.events.length || active.undoneEvents.length)) {
          active.events = [];
          active.undoneEvents = [];
        }
        result = {
          action: modified || hadActive ? "none" : (active.source.kind === "fixed" ? "matched-fixed" : "matched-preset"),
          kind: modified ? "current" : active.source.kind,
          source: cloneJson(active.source),
          current: modified ? workingCopyCompat(key, collection, active) : null,
          preset: sourcePreset(key, collection, active.source),
          workingCopy: workingCopyCompat(key, collection, active),
        };
        assignCollection(draft, key, collection);
        return;
      }
      if (!payloadsEqual(key, previousPayload, payload)) {
        const event = createPayloadEvent(key, "payload-change", active.source, previousPayload || payload, payload);
        if (event) {
          active.events.push(event);
        }
      }
      active.payload = payload;
      active.updatedAt = nowIso();
      active.events = active.events.slice(-CURRENT_EVENT_LIMIT);
      active.undoneEvents = [];
      const modified = workingCopyModified(key, collection, active);
      if (!modified) {
        active.events = [];
        active.undoneEvents = [];
      }
      result = {
        action: modified
          ? (previousModified ? "updated-current" : "created-current")
          : (active.source.kind === "fixed" ? "matched-fixed" : "matched-preset"),
        kind: modified ? "current" : active.source.kind,
        source: cloneJson(active.source),
        current: modified ? workingCopyCompat(key, collection, active) : null,
        preset: sourcePreset(key, collection, active.source),
        workingCopy: workingCopyCompat(key, collection, active),
      };
      assignCollection(draft, key, collection);
    }, "track-working-copy", { collectionKey: key });
    return cloneJson(result);
  }

  function ensurePersistedSelection(collectionKey, options = {}) {
    return trackCurrentPayload(collectionKey, options);
  }

  function discardCurrent(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (options.refreshCurrent !== false) {
      refreshCurrentForAction(key, options);
    }
    const collection = currentCollectionCanonical(key);
    const active = activeWorkingCopyFromCollection(collection);
    const origin = normalizeSource(active?.source);
    const originPayload = sourcePayload(key, collection, origin);
    if (!active || !originPayload) {
      throw new Error("Current preset has no original preset to discard to.");
    }
    let replacement = null;
    updateSnapshot((draft) => {
      const nextCollection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const nextActive = activeWorkingCopyFromCollection(nextCollection);
      if (!nextActive) {
        assignCollection(draft, key, nextCollection);
        return;
      }
      const nextOrigin = normalizeSource(nextActive.source);
      const nextOriginPayload = sourcePayload(key, nextCollection, nextOrigin) || originPayload;
      const nextWorkingCopy = createWorkingCopy(key, nextOrigin, nextOriginPayload);
      nextCollection.workingCopies = nextCollection.workingCopies.filter((workingCopy) => workingCopy.id !== nextActive.id);
      nextCollection.workingCopies.push(nextWorkingCopy);
      nextCollection.activeWorkingCopyId = nextWorkingCopy.id;
      pruneCleanInactiveWorkingCopies(key, nextCollection);
      replacement = workingCopyCompat(key, nextCollection, nextWorkingCopy);
      assignCollection(draft, key, nextCollection);
    }, "discard-working-copy", { collectionKey: key });
    applyPayload(key, originPayload);
    return {
      action: "discarded",
      kind: origin.kind,
      source: origin,
      current: null,
      preset: sourcePreset(key, collection, origin),
      workingCopy: replacement || activeWorkingCopy(key),
      payload: normalizePresetPayload(key, originPayload),
    };
  }

  function createPreset(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const select = options.select !== false;
    const payload = normalizePresetPayload(key, options.payload);
    const createdAt = nowIso();
    const preset = {
      id: randomId("preset_"),
      name: trimString(options.name) || defaultPresetName(key, currentCollectionCanonical(key).presets.length + 1),
      payload,
      createdAt,
      updatedAt: createdAt,
    };
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      collection.presets.push(preset);
      if (select) {
        const workingCopy = createWorkingCopy(key, presetSource(preset.id), payload);
        collection.workingCopies.push(workingCopy);
        collection.activeWorkingCopyId = workingCopy.id;
        pruneCleanInactiveWorkingCopies(key, collection);
      }
      assignCollection(draft, key, collection);
    }, "create-preset", {
      collectionKey: key,
      presetId: preset.id,
    });
    if (select) {
      applyPayload(key, payload);
    }
    return cloneJson(preset);
  }

  function updatePreset(collectionKey, presetId, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedPresetId = trimString(presetId);
    if (!normalizedPresetId) {
      throw new Error("Preset id is required.");
    }
    let updatedPreset = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const presetIndex = collection.presets.findIndex((preset) => preset.id === normalizedPresetId);
      if (presetIndex < 0) {
        throw new Error(`Unknown preset: ${normalizedPresetId}`);
      }
      const current = collection.presets[presetIndex];
      const nextPayload = Object.prototype.hasOwnProperty.call(options, "payload")
        ? normalizePresetPayload(key, options.payload)
        : current.payload;
      const nextName = Object.prototype.hasOwnProperty.call(options, "name")
        ? (trimString(options.name) || current.name)
        : current.name;
      updatedPreset = {
        ...current,
        name: nextName,
        payload: nextPayload,
        updatedAt: nowIso(),
      };
      collection.presets[presetIndex] = updatedPreset;
      if (options.select === true) {
        const workingCopy = createWorkingCopy(key, presetSource(updatedPreset.id), updatedPreset.payload);
        collection.workingCopies.push(workingCopy);
        collection.activeWorkingCopyId = workingCopy.id;
        pruneCleanInactiveWorkingCopies(key, collection);
      }
      assignCollection(draft, key, collection);
    }, "update-preset", {
      collectionKey: key,
      presetId: normalizedPresetId,
    });
    if (options.select === true && updatedPreset?.payload) {
      applyPayload(key, updatedPreset.payload);
    }
    return cloneJson(updatedPreset);
  }

  function renamePreset(collectionKey, presetId, name) {
    return updatePreset(collectionKey, presetId, { name });
  }

  function deletePreset(collectionKey, presetId) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedPresetId = trimString(presetId);
    if (!normalizedPresetId) {
      return currentCollection(key);
    }
    return updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const deletedPreset = collection.presets.find((preset) => preset.id === normalizedPresetId) || null;
      collection.presets = collection.presets.filter((preset) => preset.id !== normalizedPresetId);
      const defaultSource = firstFixedSource(key);
      collection.workingCopies = collection.workingCopies
        .map((workingCopy) => {
          if (workingCopy.source.kind !== "preset" || workingCopy.source.id !== normalizedPresetId) {
            return workingCopy;
          }
          const payload = workingCopyPayload(key, collection, workingCopy);
          const wasModified = Boolean(
            payload
              && deletedPreset?.payload
              && !payloadsEqual(key, payload, deletedPreset.payload),
          );
          if (!wasModified) {
            return null;
          }
          return {
            ...workingCopy,
            source: defaultSource,
            events: [],
            undoneEvents: [],
          };
        })
        .filter(Boolean);
      if (!collection.workingCopies.some((workingCopy) => workingCopy.id === collection.activeWorkingCopyId)) {
        collection.activeWorkingCopyId = collection.workingCopies[0]?.id || "";
      }
      ensureActiveWorkingCopy(key, collection, defaultSource);
      pruneCleanInactiveWorkingCopies(key, collection);
      assignCollection(draft, key, collection);
    }, "delete-preset", {
      collectionKey: key,
      presetId: normalizedPresetId,
    });
  }

  function saveCurrentToSelectedPreset(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (options.refreshCurrent !== false) {
      refreshCurrentForAction(key, options);
    }
    const collection = currentCollectionCanonical(key);
    const active = activeWorkingCopyFromCollection(collection);
    const source = normalizeSource(active?.source);
    if (source.kind !== "preset") {
      throw new Error("Preset save failed.");
    }
    const preset = collection.presets.find((entry) => entry.id === source.id) || null;
    if (!preset) {
      throw new Error(`Unknown preset: ${source.id}`);
    }
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const adapter = collectionAdapter(key);
    const requiresLiveSaveCapture = !hasPayload && adapter?.captureOnSave === true;
    const capturedPayload = hasPayload
      ? normalizePresetPayload(key, options.payload)
      : capturePayload(key, { intent: "save" });
    const payload = capturedPayload || (
      !requiresLiveSaveCapture && active?.payload ? normalizePresetPayload(key, active.payload) : null
    );
    if (!payload) {
      throw new Error("Preset save failed.");
    }
    const savedPreset = updatePreset(key, preset.id, {
      payload,
      select: false,
    });
    updateSnapshot((draft) => {
      const nextCollection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const nextActive = activeWorkingCopyFromCollection(nextCollection);
      if (nextActive && sourcesEqual(nextActive.source, presetSource(savedPreset.id))) {
        nextActive.payload = normalizePresetPayload(key, payload);
        nextActive.updatedAt = nowIso();
        nextActive.events = [];
        nextActive.undoneEvents = [];
      }
      assignCollection(draft, key, nextCollection);
    }, "save-selected-working-copy", { collectionKey: key, presetId: savedPreset.id });
    return savedPreset;
  }

  function saveCurrent(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (options.refreshCurrent !== false) {
      refreshCurrentForAction(key, options);
    }
    const collection = currentCollectionCanonical(key);
    const active = activeWorkingCopyFromCollection(collection);
    if (!active || !workingCopyModified(key, collection, active)) {
      throw new Error("Preset save failed.");
    }
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const adapter = collectionAdapter(key);
    const requiresLiveSaveCapture = !hasPayload && adapter?.captureOnSave === true;
    const capturedPayload = hasPayload
      ? normalizePresetPayload(key, options.payload)
      : capturePayload(key, { intent: "save" });
    const payload = capturedPayload || (
      !requiresLiveSaveCapture && active.payload ? normalizePresetPayload(key, active.payload) : null
    );
    if (!payload) {
      throw new Error("Preset save failed.");
    }
    const source = normalizeSource(active.source);
    const sourcePresetExists = source.kind === "preset"
      && collection.presets.some((preset) => preset.id === source.id);
    if (sourcePresetExists) {
      const preset = updatePreset(key, source.id, {
        payload,
        select: false,
      });
      updateSnapshot((draft) => {
        const nextCollection = normalizeCollectionSnapshot(key, draft.collections[key]);
        const nextActive = activeWorkingCopyFromCollection(nextCollection);
        if (nextActive) {
          nextActive.payload = normalizePresetPayload(key, payload);
          nextActive.updatedAt = nowIso();
          nextActive.events = [];
          nextActive.undoneEvents = [];
        }
        assignCollection(draft, key, nextCollection);
      }, "save-working-copy", { collectionKey: key, presetId: preset.id });
      return {
        action: "saved",
        kind: "preset",
        source: presetSource(preset.id),
        current: null,
        preset,
        workingCopy: activeWorkingCopy(key),
      };
    }

    const preset = createPreset(key, {
      name: trimString(options.name) || defaultPresetName(key, collection.presets.length + 1),
      payload,
      select: false,
    });
    updateSnapshot((draft) => {
      const nextCollection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const nextActive = activeWorkingCopyFromCollection(nextCollection);
      if (nextActive) {
        nextActive.source = presetSource(preset.id);
        nextActive.payload = normalizePresetPayload(key, payload);
        nextActive.updatedAt = nowIso();
        nextActive.events = [];
        nextActive.undoneEvents = [];
      }
      assignCollection(draft, key, nextCollection);
    }, "save-working-copy-created", { collectionKey: key, presetId: preset.id });
    return {
      action: "created",
      kind: "preset",
      source: presetSource(preset.id),
      current: null,
      preset,
      workingCopy: activeWorkingCopy(key),
    };
  }

  function currentHistoryState(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollectionCanonical(key);
    const active = activeWorkingCopyFromCollection(collection);
    return {
      canUndo: Boolean(active?.events?.length),
      canRedo: Boolean(active?.undoneEvents?.length),
      current: active ? workingCopyCompat(key, collection, active) : null,
    };
  }

  function undoCurrent(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    let nextActive = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const active = activeWorkingCopyFromCollection(collection);
      const event = active?.events?.[active.events.length - 1] || null;
      if (!active || !event?.beforePayload) {
        nextActive = active || null;
        assignCollection(draft, key, collection);
        return;
      }
      active.events = active.events.slice(0, -1);
      active.undoneEvents = [...active.undoneEvents, event].slice(-CURRENT_EVENT_LIMIT);
      active.payload = normalizePresetPayload(key, event.beforePayload);
      active.updatedAt = nowIso();
      nextActive = cloneJson(active);
      assignCollection(draft, key, collection);
    }, "undo-working-copy", { collectionKey: key });
    if (nextActive?.payload) {
      applyPayload(key, nextActive.payload);
    }
    return cloneJson(nextActive);
  }

  function redoCurrent(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    let nextActive = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const active = activeWorkingCopyFromCollection(collection);
      const event = active?.undoneEvents?.[active.undoneEvents.length - 1] || null;
      if (!active || !event?.afterPayload) {
        nextActive = active || null;
        assignCollection(draft, key, collection);
        return;
      }
      active.undoneEvents = active.undoneEvents.slice(0, -1);
      active.events = [...active.events, event].slice(-CURRENT_EVENT_LIMIT);
      active.payload = normalizePresetPayload(key, event.afterPayload);
      active.updatedAt = nowIso();
      nextActive = cloneJson(active);
      assignCollection(draft, key, collection);
    }, "redo-working-copy", { collectionKey: key });
    if (nextActive?.payload) {
      applyPayload(key, nextActive.payload);
    }
    return cloneJson(nextActive);
  }

  function setSelectedPresetId(collectionKey, presetId) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedPresetId = trimString(presetId);
    return normalizedPresetId ? activatePreset(key, normalizedPresetId) : activateFixedPreset(key, DEFAULT_FIXED_PRESET_ID);
  }

  function setSelectedFixedId(collectionKey, fixedId) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedFixedId = trimString(fixedId) || DEFAULT_FIXED_PRESET_ID;
    return activateFixedPreset(key, normalizedFixedId);
  }

  function syncSelectedPresetToCurrent(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const payload = hasPayload ? options.payload : capturePayload(key);
    if (!payload) {
      return null;
    }
    const payloadJson = stablePresetPayloadJson(key, payload);
    const matchedPreset = currentCollectionCanonical(key).presets.find((preset) => (
      stablePresetPayloadJson(key, preset.payload) === payloadJson
    )) || null;
    if (matchedPreset) {
      activatePreset(key, matchedPreset.id);
    }
    return cloneJson(matchedPreset);
  }

  function clearCollection(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    return updateSnapshot((draft) => {
      delete draft.collections[key];
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      ensureActiveWorkingCopy(key, collection);
      assignCollection(draft, key, collection);
    }, "clear-collection", { collectionKey: key });
  }

  function clearAll() {
    return replaceSnapshot({ collections: {} }, "clear-all");
  }

  function exportCollectionPayload(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollectionCanonical(key);
    const presetIds = Array.isArray(options.presetIds)
      ? options.presetIds.map(trimString).filter(Boolean)
      : [];
    let presets = presetIds.length
      ? collection.presets.filter((preset) => presetIds.includes(preset.id))
      : collection.presets;
    if (!presets.length && options.includeCurrent === true) {
      const currentPayload = options.currentPayload || activeWorkingCopyFromCollection(collection)?.payload;
      presets = [{
        id: randomId("preset_export_"),
        name: trimString(options.currentName) || defaultPresetName(key, 1),
        payload: normalizePresetPayload(key, currentPayload),
        createdAt: nowIso(),
        updatedAt: nowIso(),
      }];
    }
    return {
      format: EXPORT_FORMAT,
      collectionKey: key,
      exportedAt: nowIso(),
      selectedPresetId: presets.some((preset) => preset.id === selectedPresetIdFromCollection(collection))
        ? selectedPresetIdFromCollection(collection)
        : "",
      presets: cloneJson(presets),
    };
  }

  function exportCollectionText(collectionKey, options = {}) {
    return JSON.stringify(exportCollectionPayload(collectionKey, options), null, 2);
  }

  function importCollectionPayload(collectionKey, payload, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (!isPlainObject(payload)) {
      throw new Error("Preset JSON must contain an object.");
    }
    if (trimString(payload.format) !== EXPORT_FORMAT) {
      throw new Error(`Preset JSON must use ${EXPORT_FORMAT} format.`);
    }
    const payloadCollectionKey = normalizeCollectionKey(payload.collectionKey);
    if (payloadCollectionKey && payloadCollectionKey !== key) {
      throw new Error(`Preset JSON contains ${payloadCollectionKey}, expected ${key}.`);
    }
    const importedCollection = normalizeCollectionSnapshot(key, {
      presets: payload.presets,
    });
    const importedIds = [];
    const selectImported = options.selectImported !== false;
    let selectedImportedId = "";
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      for (const preset of importedCollection.presets) {
        let nextPreset = preset;
        if (collection.presets.some((current) => current.id === nextPreset.id)) {
          nextPreset = {
            ...nextPreset,
            id: randomId("preset_"),
            createdAt: nowIso(),
            updatedAt: nowIso(),
          };
        }
        collection.presets.push(nextPreset);
        importedIds.push(nextPreset.id);
        if (preset.id === trimString(payload.selectedPresetId)) {
          selectedImportedId = nextPreset.id;
        }
      }
      if (selectImported) {
        const importedSelection = selectedImportedId || importedIds[0] || "";
        if (importedSelection) {
          const importedPreset = collection.presets.find((preset) => preset.id === importedSelection);
          const workingCopy = createWorkingCopy(key, presetSource(importedSelection), importedPreset?.payload || null);
          collection.workingCopies.push(workingCopy);
          collection.activeWorkingCopyId = workingCopy.id;
          pruneCleanInactiveWorkingCopies(key, collection);
        }
      }
      assignCollection(draft, key, collection);
    }, "import-collection", {
      collectionKey: key,
      presetIds: importedIds,
    });
    return {
      collectionKey: key,
      presetIds: importedIds,
      selectedPresetId: selectImported ? (selectedImportedId || importedIds[0] || "") : "",
    };
  }

  function importCollectionText(collectionKey, text, options = {}) {
    const serialized = trimString(text);
    if (!serialized) {
      throw new Error("Preset JSON file is empty.");
    }
    let parsed;
    try {
      parsed = JSON.parse(serialized);
    } catch (_error) {
      throw new Error("Preset JSON file is not valid JSON.");
    }
    return importCollectionPayload(collectionKey, parsed, options);
  }

  function registerCollectionAdapter(collectionKey, adapter) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key) {
      throw new Error("Collection key is required.");
    }
    if (!adapter || typeof adapter !== "object") {
      throw new Error(`Preset adapter for ${key} must be an object.`);
    }
    collectionAdapters.set(key, { ...adapter });
    const nextSnapshot = normalizeSnapshot(currentSnapshot);
    currentSnapshot = persistSnapshot(nextSnapshot, globalThis.localStorage);
    emitAdaptersChange(key);
    return collectionAdapter(key);
  }

  globalThis.window?.addEventListener?.("storage", (event) => {
    if (event?.key !== STORAGE_KEY) {
      return;
    }
    currentSnapshot = loadSnapshot(globalThis.localStorage);
    emitChange("storage");
  });

  globalThis.window.__fishystuffUserPresets = Object.freeze({
    STORAGE_KEY,
    CHANGED_EVENT,
    ADAPTERS_CHANGED_EVENT,
    EXPORT_FORMAT,
    bindDatastar,
    unbindDatastar,
    datastarSnapshot,
    refreshDatastar,
    snapshot() {
      return cloneJson(currentSnapshot);
    },
    collection: currentCollection,
    presets(collectionKey) {
      return currentCollectionCanonical(collectionKey).presets.map(cloneJson);
    },
    preset: currentPreset,
    current: currentPresetState,
    activeWorkingCopy,
    activeWorkingCopyId(collectionKey) {
      return activeWorkingCopy(collectionKey)?.id || "";
    },
    workingCopies,
    fixedPreset,
    selectedPresetId(collectionKey) {
      return selectedPresetIdFromCollection(currentCollectionCanonical(collectionKey));
    },
    selectedFixedId(collectionKey) {
      return selectedFixedIdFromCollection(currentCollectionCanonical(collectionKey));
    },
    selectedPreset,
    setSelectedPresetId,
    setSelectedFixedId,
    createPreset,
    updatePreset,
    renamePreset,
    deletePreset,
    clearCollection,
    clearAll,
    capturePayload,
    applyPayload,
    activatePreset,
    activateFixedPreset,
    activateWorkingCopy: selectWorkingCopy,
    discardCurrent,
    trackCurrentPayload,
    saveCurrent,
    saveCurrentToSelectedPreset,
    currentActionState,
    canSaveCurrent,
    canDiscardCurrent,
    undoCurrent,
    redoCurrent,
    currentHistoryState,
    ensurePersistedSelection,
    fixedPresets,
    syncSelectedPresetToCurrent,
    exportCollectionPayload,
    exportCollectionText,
    importCollectionPayload,
    importCollectionText,
    registerCollectionAdapter,
    collectionAdapter,
    normalizeSnapshot,
    normalizeCollectionKey,
    normalizePresetPayload,
  });
})();
