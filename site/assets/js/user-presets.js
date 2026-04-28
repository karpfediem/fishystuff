(function () {
  const STORAGE_KEY = "fishystuff.user-presets.v1";
  const CHANGED_EVENT = "fishystuff:user-presets-changed";
  const ADAPTERS_CHANGED_EVENT = "fishystuff:user-presets-adapters-changed";
  const EXPORT_FORMAT = "fishystuff-user-presets.v1";
  const CURRENT_EVENT_LIMIT = 100;

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
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
          id: trimString(entry.id) || `fixed_${index + 1}`,
          name: trimString(entry.name) || `Fixed ${index + 1}`,
          payload,
        };
      })
      .filter(Boolean);
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
      source: normalizeSource(value.source),
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

  function normalizeCurrentPreset(collectionKey, value) {
    if (!isPlainObject(value)) {
      return null;
    }
    let payload;
    try {
      payload = normalizePresetPayload(collectionKey, value.payload);
    } catch (_error) {
      return null;
    }
    return {
      origin: normalizeSource(value.origin),
      payload,
      updatedAt: trimString(value.updatedAt) || nowIso(),
      events: normalizePresetEvents(collectionKey, value.events),
      undoneEvents: normalizePresetEvents(collectionKey, value.undoneEvents),
    };
  }

  function normalizeCollectionSnapshot(collectionKey, value) {
    const key = normalizeCollectionKey(collectionKey);
    const source = isPlainObject(value) ? value : {};
    const seen = new Set();
    const presets = [];
    for (const [index, rawPreset] of (Array.isArray(source.presets) ? source.presets : []).entries()) {
      const preset = normalizePresetEntry(key, rawPreset, index);
      if (!preset || seen.has(preset.id)) {
        continue;
      }
      seen.add(preset.id);
      presets.push(preset);
    }
    const selectedPresetId = trimString(source.selectedPresetId);
    const selectedFixedId = trimString(source.selectedFixedId);
    const normalizedSelectedPresetId = presets.some((preset) => preset.id === selectedPresetId)
      ? selectedPresetId
      : "";
    return {
      selectedPresetId: normalizedSelectedPresetId,
      selectedFixedId: selectedFixedId && !normalizedSelectedPresetId ? selectedFixedId : "",
      current: normalizeCurrentPreset(key, source.current),
      presets,
    };
  }

  function collectionHasState(collection) {
    return Boolean(
      collection?.selectedPresetId
      || collection?.selectedFixedId
      || collection?.current
      || collection?.presets?.length,
    );
  }

  function assignCollection(draft, collectionKey, collection) {
    if (collectionHasState(collection)) {
      draft.collections[collectionKey] = collection;
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

  function datastarCollectionSummary(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key) {
      return null;
    }
    const collection = currentCollection(key);
    const actionState = collectionCurrentActionState(key, collection);
    const adapterFixedPresets = fixedPresets(key);
    const previewFixedPresets = globalThis.window?.__fishystuffPresetPreviews?.fixedPresets?.(key) || [];
    return {
      selectedPresetId: collection.selectedPresetId,
      selectedFixedId: collection.selectedFixedId,
      hasCurrent: actionState.hasCurrent,
      currentOrigin: actionState.currentOrigin,
      canSave: actionState.canSave,
      canDiscard: actionState.canDiscard,
      saveAction: actionState.saveAction,
      presetCount: collection.presets.length,
      fixedPresetCount: adapterFixedPresets.length || previewFixedPresets.length,
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

  function currentCollection(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    return cloneJson(
      currentSnapshot.collections[key]
        || {
          selectedPresetId: "",
          selectedFixedId: "",
          current: null,
          presets: [],
        },
    );
  }

  function currentPreset(collectionKey, presetId) {
    const collection = currentCollection(collectionKey);
    const normalizedId = trimString(presetId);
    return cloneJson(collection.presets.find((preset) => preset.id === normalizedId) || null);
  }

  function selectedPreset(collectionKey) {
    const collection = currentCollection(collectionKey);
    return cloneJson(
      collection.presets.find((preset) => preset.id === collection.selectedPresetId) || null,
    );
  }

  function currentPresetState(collectionKey) {
    return cloneJson(currentCollection(collectionKey).current || null);
  }

  function fixedPreset(collectionKey, fixedId) {
    const normalizedId = trimString(fixedId);
    return cloneJson(fixedPresets(collectionKey).find((preset) => preset.id === normalizedId) || null);
  }

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

  function selectedSource(collection) {
    if (collection?.selectedPresetId) {
      return presetSource(collection.selectedPresetId);
    }
    if (collection?.selectedFixedId) {
      return fixedSource(collection.selectedFixedId);
    }
    return normalizeSource(collection?.current?.origin);
  }

  function collectionCurrentActionState(collectionKey, value = null) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = value
      ? normalizeCollectionSnapshot(key, value)
      : currentCollection(key);
    const current = collection.current || null;
    const currentOrigin = normalizeSource(current?.origin);
    const selected = selectedSource(collection);
    const originPayload = current ? sourcePayload(key, collection, currentOrigin) : null;
    const originPreset = currentOrigin.kind === "preset"
      ? collection.presets.find((preset) => preset.id === currentOrigin.id) || null
      : null;
    const selectedPreset = collection.selectedPresetId
      ? collection.presets.find((preset) => preset.id === collection.selectedPresetId) || null
      : null;
    const selectedSnapshotCanSave = Boolean(
      !current
      && selectedPreset
      && collectionAdapter(key)?.captureOnSave === true,
    );
    const saveAction = current
      ? (originPreset ? "saved" : "created")
      : (selectedSnapshotCanSave ? "saved" : "none");
    return {
      hasCurrent: Boolean(current),
      current: cloneJson(current),
      currentOrigin,
      selectedSource: selected,
      canSave: Boolean(current || selectedSnapshotCanSave),
      canDiscard: Boolean(current && originPayload),
      saveAction,
      saveTargetPresetId: originPreset?.id || (!current ? selectedPreset?.id : "") || "",
      discardSource: originPayload ? currentOrigin : { kind: "none", id: "" },
    };
  }

  function refreshCurrentForAction(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key || options.refreshCurrent === false) {
      return currentCollection(key);
    }
    const adapter = collectionAdapter(key);
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    if (!hasPayload && (!adapter || typeof adapter.capture !== "function")) {
      return currentCollection(key);
    }
    const payload = hasPayload ? options.payload : capturePayload(key);
    if (!payload) {
      return currentCollection(key);
    }
    trackCurrentPayload(key, {
      payload,
      ...(Object.prototype.hasOwnProperty.call(options, "origin") ? { origin: options.origin } : {}),
    });
    return currentCollection(key);
  }

  function currentActionState(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key) {
      return collectionCurrentActionState(key);
    }
    const collection = options.refresh === true
      ? refreshCurrentForAction(key, options)
      : currentCollection(key);
    return collectionCurrentActionState(key, collection);
  }

  function canSaveCurrent(collectionKey, options = {}) {
    return currentActionState(collectionKey, options).canSave;
  }

  function canDiscardCurrent(collectionKey, options = {}) {
    return currentActionState(collectionKey, options).canDiscard;
  }

  function matchingSource(collectionKey, collection, payload) {
    const selected = selectedSource(collection);
    if (selected.kind !== "none" && payloadsEqual(collectionKey, sourcePayload(collectionKey, collection, selected), payload)) {
      return selected;
    }
    for (const preset of fixedPresets(collectionKey)) {
      if (payloadsEqual(collectionKey, preset.payload, payload)) {
        return fixedSource(preset.id);
      }
    }
    for (const preset of collection.presets) {
      if (payloadsEqual(collectionKey, preset.payload, payload)) {
        return presetSource(preset.id);
      }
    }
    return null;
  }

  function firstFixedSource(collectionKey) {
    const firstFixed = fixedPresets(collectionKey)[0] || null;
    return firstFixed ? fixedSource(firstFixed.id) : { kind: "none", id: "" };
  }

  function selectCollectionSource(collection, source, { clearCurrent = false } = {}) {
    const normalized = normalizeSource(source);
    collection.selectedPresetId = normalized.kind === "preset" ? normalized.id : "";
    collection.selectedFixedId = normalized.kind === "fixed" ? normalized.id : "";
    if (clearCurrent) {
      collection.current = null;
    }
    return collection;
  }

  function setSelectedPresetId(collectionKey, presetId, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedPresetId = trimString(presetId);
    const clearCurrent = options?.clearCurrent === true;
    return updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const source = collection.presets.some((preset) => preset.id === normalizedPresetId)
        ? presetSource(normalizedPresetId)
        : { kind: "none", id: "" };
      selectCollectionSource(collection, source, { clearCurrent });
      assignCollection(draft, key, collection);
    }, "select-preset", { collectionKey: key });
  }

  function setSelectedFixedId(collectionKey, fixedId, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedFixedId = trimString(fixedId);
    const allowUnknown = options?.allowUnknown === true;
    const clearCurrent = options?.clearCurrent === true;
    return updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const hasFixedSource = Boolean(allowUnknown && normalizedFixedId)
        || fixedPresets(key).some((preset) => preset.id === normalizedFixedId);
      const source = hasFixedSource
        ? fixedSource(normalizedFixedId)
        : { kind: "none", id: "" };
      selectCollectionSource(collection, source, { clearCurrent });
      assignCollection(draft, key, collection);
    }, "select-fixed-preset", { collectionKey: key, fixedId: normalizedFixedId });
  }

  function createPreset(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const select = options.select !== false;
    const payload = normalizePresetPayload(key, options.payload);
    const createdAt = nowIso();
    const preset = {
      id: randomId("preset_"),
      name: trimString(options.name) || defaultPresetName(key, currentCollection(key).presets.length + 1),
      payload,
      createdAt,
      updatedAt: createdAt,
    };
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      collection.presets.push(preset);
      if (select) {
        selectCollectionSource(collection, presetSource(preset.id), {
          clearCurrent: payloadsEqual(key, collection.current?.payload, payload),
        });
      }
      draft.collections[key] = collection;
    }, "create-preset", {
      collectionKey: key,
      presetId: preset.id,
    });
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
        selectCollectionSource(collection, presetSource(updatedPreset.id), {
          clearCurrent: payloadsEqual(key, collection.current?.payload, updatedPreset.payload),
        });
      }
      draft.collections[key] = collection;
    }, "update-preset", {
      collectionKey: key,
      presetId: normalizedPresetId,
    });
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
      collection.presets = collection.presets.filter((preset) => preset.id !== normalizedPresetId);
      if (collection.selectedPresetId === normalizedPresetId) {
        collection.selectedPresetId = "";
      }
      if (collection.current?.origin?.kind === "preset" && collection.current.origin.id === normalizedPresetId) {
        collection.current = {
          ...collection.current,
          origin: { kind: "none", id: "" },
        };
      }
      assignCollection(draft, key, collection);
    }, "delete-preset", {
      collectionKey: key,
      presetId: normalizedPresetId,
    });
  }

  function clearCollection(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    return updateSnapshot((draft) => {
      delete draft.collections[key];
    }, "clear-collection", { collectionKey: key });
  }

  function clearAll() {
    return replaceSnapshot({ collections: {} }, "clear-all");
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
    if (!adapter || typeof adapter.apply !== "function") {
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
    return null;
  }

  function selectSourcePayload(collectionKey, source, payload) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedSource = normalizeSource(source);
    let result = {
      action: normalizedSource.kind === "fixed" ? "matched-fixed" : "matched-preset",
      kind: normalizedSource.kind,
      source: normalizedSource,
      current: null,
      preset: null,
    };
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      selectCollectionSource(collection, normalizedSource, { clearCurrent: true });
      assignCollection(draft, key, collection);
      result = {
        ...result,
        preset: normalizedSource.kind === "preset"
          ? cloneJson(collection.presets.find((preset) => preset.id === normalizedSource.id) || null)
          : cloneJson(fixedPresets(key).find((preset) => preset.id === normalizedSource.id) || null),
      };
    }, "select-applied-source", { collectionKey: key });
    return cloneJson({
      ...result,
      payload: normalizePresetPayload(key, payload),
    });
  }

  function sourcePayloadAlreadyApplied(collectionKey, payload) {
    const key = normalizeCollectionKey(collectionKey);
    const captured = capturePayload(key);
    return Boolean(captured && payloadsEqual(key, captured, payload));
  }

  function applyAndTrackSourcePayload(collectionKey, source, payload) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedPayload = normalizePresetPayload(key, payload);
    if (sourcePayloadAlreadyApplied(key, normalizedPayload)) {
      return selectSourcePayload(key, source, normalizedPayload);
    }
    const adapter = collectionAdapter(key);
    if (!adapter || typeof adapter.apply !== "function") {
      return selectSourcePayload(key, source, normalizedPayload);
    }
    const appliedValue = applyPayload(key, normalizedPayload);
    const observedPayload = observedPayloadAfterApply(key, normalizedPayload, appliedValue);
    if (!observedPayload) {
      return {
        action: "apply-failed",
        kind: "current",
        source: normalizeSource(source),
        current: currentPresetState(key),
        preset: null,
      };
    }
    return trackCurrentPayload(key, {
      payload: observedPayload,
      origin: source,
    });
  }

  function activatePreset(collectionKey, presetId) {
    const key = normalizeCollectionKey(collectionKey);
    const preset = currentPreset(key, presetId);
    if (!preset) {
      setSelectedPresetId(key, "");
      return null;
    }
    applyAndTrackSourcePayload(key, presetSource(preset.id), preset.payload);
    return preset;
  }

  function activateFixedPreset(collectionKey, fixedId) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedFixedId = trimString(fixedId);
    const preset = fixedPreset(key, fixedId);
    if (!preset) {
      if (!collectionAdapter(key) && normalizedFixedId) {
        setSelectedFixedId(key, normalizedFixedId, {
          allowUnknown: true,
          clearCurrent: true,
        });
        return {
          id: normalizedFixedId,
          name: normalizedFixedId,
          payload: null,
        };
      }
      setSelectedFixedId(key, "");
      return null;
    }
    applyAndTrackSourcePayload(key, fixedSource(preset.id), preset.payload);
    return preset;
  }

  function discardCurrent(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (options.refreshCurrent !== false) {
      refreshCurrentForAction(key, options);
    }
    const collection = currentCollection(key);
    const current = collection.current;
    if (!current) {
      return {
        action: "none",
        kind: "none",
        source: { kind: "none", id: "" },
        current: null,
        preset: null,
      };
    }
    const origin = normalizeSource(current.origin);
    const originPayload = sourcePayload(key, collection, origin);
    if (!originPayload) {
      throw new Error("Current preset has no original preset to discard to.");
    }
    return applyAndTrackSourcePayload(key, origin, originPayload);
  }

  function trackCurrentPayload(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const payload = hasPayload ? normalizePresetPayload(key, options.payload) : capturePayload(key);
    let result = {
      action: "cleared",
      kind: "none",
      source: { kind: "none", id: "" },
      current: null,
      preset: null,
    };
    if (!payload) {
      setSelectedPresetId(key, "");
      return result;
    }
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const matchedSource = matchingSource(key, collection, payload);
      if (matchedSource) {
        if (!collection.current && sourcesEqual(selectedSource(collection), matchedSource)) {
          result = {
            action: "none",
            kind: matchedSource.kind,
            source: matchedSource,
            current: null,
            preset: matchedSource.kind === "preset"
              ? cloneJson(collection.presets.find((preset) => preset.id === matchedSource.id) || null)
              : cloneJson(fixedPresets(key).find((preset) => preset.id === matchedSource.id) || null),
          };
          return;
        }
        selectCollectionSource(collection, matchedSource, { clearCurrent: true });
        assignCollection(draft, key, collection);
        result = {
          action: matchedSource.kind === "fixed" ? "matched-fixed" : "matched-preset",
          kind: matchedSource.kind,
          source: matchedSource,
          current: null,
          preset: matchedSource.kind === "preset"
            ? cloneJson(collection.presets.find((preset) => preset.id === matchedSource.id) || null)
            : cloneJson(fixedPresets(key).find((preset) => preset.id === matchedSource.id) || null),
        };
        return;
      }

      const explicitOrigin = Object.prototype.hasOwnProperty.call(options, "origin")
        ? normalizeSource(options.origin)
        : null;
      const previousCurrent = collection.current;
      const origin = explicitOrigin && explicitOrigin.kind !== "none"
        ? explicitOrigin
        : (
            selectedSource(collection).kind !== "none"
              ? selectedSource(collection)
              : (previousCurrent && previousCurrent.origin?.kind !== "none" ? previousCurrent.origin : firstFixedSource(key))
          );
      const previousPayload = previousCurrent && sourcesEqual(previousCurrent.origin, origin)
        ? previousCurrent.payload
        : sourcePayload(key, collection, origin);
      const events = previousCurrent && sourcesEqual(previousCurrent.origin, origin)
        ? previousCurrent.events.slice()
        : [];
      if (previousCurrent && sourcesEqual(previousCurrent.origin, origin) && payloadsEqual(key, previousCurrent.payload, payload)) {
        result = {
          action: "none",
          kind: "current",
          source: origin,
          current: cloneJson(previousCurrent),
          preset: null,
        };
        return;
      }
      if (!payloadsEqual(key, previousPayload, payload)) {
        const event = createPayloadEvent(key, "payload-change", origin, previousPayload || payload, payload);
        if (event) {
          events.push(event);
        }
      }
      const current = {
        origin,
        payload,
        updatedAt: nowIso(),
        events: events.slice(-CURRENT_EVENT_LIMIT),
        undoneEvents: [],
      };
      selectCollectionSource(collection, origin, { clearCurrent: false });
      collection.current = current;
      assignCollection(draft, key, collection);
      result = {
        action: previousCurrent && sourcesEqual(previousCurrent.origin, origin) ? "updated-current" : "created-current",
        kind: "current",
        source: origin,
        current: cloneJson(current),
        preset: null,
      };
    }, "track-current", { collectionKey: key });
    return cloneJson(result);
  }

  function syncSelectedPresetToCurrent(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const payload = hasPayload ? options.payload : capturePayload(key);
    if (!payload) {
      setSelectedPresetId(key, "");
      return null;
    }
    const payloadJson = stablePresetPayloadJson(key, payload);
    const matchedPreset = currentCollection(key).presets.find((preset) => (
      stablePresetPayloadJson(key, preset.payload) === payloadJson
    )) || null;
    setSelectedPresetId(key, matchedPreset?.id || "");
    return cloneJson(matchedPreset);
  }

  function ensurePersistedSelection(collectionKey, options = {}) {
    return trackCurrentPayload(collectionKey, options);
  }

  function saveCurrentToSelectedPreset(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    let savedPreset = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const current = collection.current;
      const targetPresetId = current?.origin?.kind === "preset"
        ? current.origin.id
        : collection.selectedPresetId;
      if (!targetPresetId) {
        throw new Error("No saved preset is selected.");
      }
      const presetIndex = collection.presets.findIndex((preset) => preset.id === targetPresetId);
      if (presetIndex < 0) {
        throw new Error(`Unknown preset: ${targetPresetId}`);
      }
      const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
      const adapter = collectionAdapter(key);
      const requiresLiveSaveCapture = !hasPayload && adapter?.captureOnSave === true;
      const capturedPayload = hasPayload
        ? normalizePresetPayload(key, options.payload)
        : capturePayload(key, { intent: "save" });
      const nextPayload = capturedPayload || (
        !requiresLiveSaveCapture && current ? normalizePresetPayload(key, current.payload) : null
      );
      if (!nextPayload) {
        throw new Error("Preset save failed.");
      }
      const previousPreset = collection.presets[presetIndex];
      savedPreset = {
        ...previousPreset,
        payload: nextPayload,
        updatedAt: nowIso(),
      };
      collection.presets[presetIndex] = savedPreset;
      selectCollectionSource(collection, presetSource(savedPreset.id), { clearCurrent: true });
      assignCollection(draft, key, collection);
    }, "save-current", { collectionKey: key });
    return cloneJson(savedPreset);
  }

  function saveCurrent(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    if (options.refreshCurrent !== false) {
      refreshCurrentForAction(key, options);
    }
    const collection = currentCollection(key);
    const current = collection.current;
    const origin = normalizeSource(current?.origin);
    const originPresetExists = origin.kind === "preset"
      && collection.presets.some((preset) => preset.id === origin.id);
    const actionState = collectionCurrentActionState(key, collection);
    if (!actionState.canSave) {
      throw new Error("Preset save failed.");
    }
    if (!current || originPresetExists) {
      const preset = saveCurrentToSelectedPreset(key, options);
      return {
        action: "saved",
        kind: "preset",
        source: presetSource(preset.id),
        current: null,
        preset,
      };
    }

    const hasPayload = Object.prototype.hasOwnProperty.call(options, "payload");
    const adapter = collectionAdapter(key);
    const requiresLiveSaveCapture = !hasPayload && adapter?.captureOnSave === true;
    const capturedPayload = hasPayload
      ? normalizePresetPayload(key, options.payload)
      : capturePayload(key, { intent: "save" });
    const payload = capturedPayload || (
      !requiresLiveSaveCapture && current ? normalizePresetPayload(key, current.payload) : null
    );
    if (!payload) {
      throw new Error("Preset save failed.");
    }
    const preset = createPreset(key, {
      name: trimString(options.name) || defaultPresetName(key, collection.presets.length + 1),
      payload,
      select: true,
    });
    return {
      action: "created",
      kind: "preset",
      source: presetSource(preset.id),
      current: null,
      preset,
    };
  }

  function currentHistoryState(collectionKey) {
    const current = currentPresetState(collectionKey);
    return {
      canUndo: Boolean(current?.events?.length),
      canRedo: Boolean(current?.undoneEvents?.length),
      current,
    };
  }

  function undoCurrent(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    let nextCurrent = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const current = collection.current;
      const event = current?.events?.[current.events.length - 1] || null;
      if (!current || !event?.beforePayload) {
        nextCurrent = current || null;
        assignCollection(draft, key, collection);
        return;
      }
      current.events = current.events.slice(0, -1);
      current.undoneEvents = [...current.undoneEvents, event].slice(-CURRENT_EVENT_LIMIT);
      current.payload = normalizePresetPayload(key, event.beforePayload);
      current.updatedAt = nowIso();
      collection.current = current;
      selectCollectionSource(collection, current.origin, { clearCurrent: false });
      assignCollection(draft, key, collection);
      nextCurrent = cloneJson(current);
    }, "undo-current", { collectionKey: key });
    if (nextCurrent?.payload) {
      applyPayload(key, nextCurrent.payload);
    }
    return cloneJson(nextCurrent);
  }

  function redoCurrent(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    let nextCurrent = null;
    updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      const current = collection.current;
      const event = current?.undoneEvents?.[current.undoneEvents.length - 1] || null;
      if (!current || !event?.afterPayload) {
        nextCurrent = current || null;
        assignCollection(draft, key, collection);
        return;
      }
      current.undoneEvents = current.undoneEvents.slice(0, -1);
      current.events = [...current.events, event].slice(-CURRENT_EVENT_LIMIT);
      current.payload = normalizePresetPayload(key, event.afterPayload);
      current.updatedAt = nowIso();
      collection.current = current;
      selectCollectionSource(collection, current.origin, { clearCurrent: false });
      assignCollection(draft, key, collection);
      nextCurrent = cloneJson(current);
    }, "redo-current", { collectionKey: key });
    if (nextCurrent?.payload) {
      applyPayload(key, nextCurrent.payload);
    }
    return cloneJson(nextCurrent);
  }

  function exportCollectionPayload(collectionKey, options = {}) {
    const key = normalizeCollectionKey(collectionKey);
    const collection = currentCollection(key);
    const presetIds = Array.isArray(options.presetIds)
      ? options.presetIds.map(trimString).filter(Boolean)
      : [];
    let presets = presetIds.length
      ? collection.presets.filter((preset) => presetIds.includes(preset.id))
      : collection.presets;
    if (!presets.length && options.includeCurrent === true) {
      presets = [{
        id: randomId("preset_export_"),
        name: trimString(options.currentName) || defaultPresetName(key, 1),
        payload: normalizePresetPayload(key, options.currentPayload),
        createdAt: nowIso(),
        updatedAt: nowIso(),
      }];
    }
    return {
      format: EXPORT_FORMAT,
      collectionKey: key,
      exportedAt: nowIso(),
      selectedPresetId: presets.some((preset) => preset.id === collection.selectedPresetId)
        ? collection.selectedPresetId
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
      selectedPresetId: payload.selectedPresetId,
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
        if (preset.id === importedCollection.selectedPresetId) {
          selectedImportedId = nextPreset.id;
        }
      }
      if (selectImported) {
        const importedSelection = selectedImportedId || importedIds[0] || collection.selectedPresetId;
        if (importedSelection) {
          selectCollectionSource(collection, presetSource(importedSelection));
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
    snapshot() {
      return cloneJson(currentSnapshot);
    },
    collection: currentCollection,
    presets(collectionKey) {
      return currentCollection(collectionKey).presets;
    },
    preset: currentPreset,
    current: currentPresetState,
    fixedPreset,
    selectedPresetId(collectionKey) {
      return currentCollection(collectionKey).selectedPresetId;
    },
    selectedFixedId(collectionKey) {
      return currentCollection(collectionKey).selectedFixedId;
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
