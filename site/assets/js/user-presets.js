(function () {
  const STORAGE_KEY = "fishystuff.user-presets.v1";
  const CHANGED_EVENT = "fishystuff:user-presets-changed";
  const ADAPTERS_CHANGED_EVENT = "fishystuff:user-presets-adapters-changed";
  const EXPORT_FORMAT = "fishystuff-user-presets.v1";

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
    return {
      selectedPresetId: presets.some((preset) => preset.id === selectedPresetId)
        ? selectedPresetId
        : "",
      presets,
    };
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
      if (normalizedCollection.presets.length || normalizedCollection.selectedPresetId) {
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

  function emitChange(reason, detail = {}) {
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(CHANGED_EVENT, {
        detail: {
          reason: trimString(reason) || "update",
          snapshot: cloneJson(currentSnapshot),
          ...detail,
        },
      }),
    );
  }

  function emitAdaptersChange(collectionKey) {
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(ADAPTERS_CHANGED_EVENT, {
        detail: {
          collectionKey: normalizeCollectionKey(collectionKey),
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

  function setSelectedPresetId(collectionKey, presetId) {
    const key = normalizeCollectionKey(collectionKey);
    const normalizedPresetId = trimString(presetId);
    return updateSnapshot((draft) => {
      const collection = normalizeCollectionSnapshot(key, draft.collections[key]);
      collection.selectedPresetId = collection.presets.some((preset) => preset.id === normalizedPresetId)
        ? normalizedPresetId
        : "";
      if (collection.presets.length || collection.selectedPresetId) {
        draft.collections[key] = collection;
      } else {
        delete draft.collections[key];
      }
    }, "select-preset", { collectionKey: key });
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
        collection.selectedPresetId = preset.id;
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
        collection.selectedPresetId = updatedPreset.id;
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
      if (collection.presets.length || collection.selectedPresetId) {
        draft.collections[key] = collection;
      } else {
        delete draft.collections[key];
      }
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

  function capturePayload(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    const adapter = collectionAdapter(key);
    if (!adapter || typeof adapter.capture !== "function") {
      return null;
    }
    return normalizePresetPayload(key, adapter.capture());
  }

  function applyPayload(collectionKey, payload) {
    const key = normalizeCollectionKey(collectionKey);
    const adapter = collectionAdapter(key);
    if (!adapter || typeof adapter.apply !== "function") {
      return null;
    }
    return adapter.apply(normalizePresetPayload(key, payload));
  }

  function activatePreset(collectionKey, presetId) {
    const key = normalizeCollectionKey(collectionKey);
    const preset = currentPreset(key, presetId);
    if (!preset) {
      setSelectedPresetId(key, "");
      return null;
    }
    applyPayload(key, preset.payload);
    setSelectedPresetId(key, preset.id);
    return preset;
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
        collection.selectedPresetId = selectedImportedId || importedIds[0] || collection.selectedPresetId;
      }
      if (collection.presets.length || collection.selectedPresetId) {
        draft.collections[key] = collection;
      } else {
        delete draft.collections[key];
      }
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
    snapshot() {
      return cloneJson(currentSnapshot);
    },
    collection: currentCollection,
    presets(collectionKey) {
      return currentCollection(collectionKey).presets;
    },
    preset: currentPreset,
    selectedPresetId(collectionKey) {
      return currentCollection(collectionKey).selectedPresetId;
    },
    selectedPreset,
    setSelectedPresetId,
    createPreset,
    updatePreset,
    renamePreset,
    deletePreset,
    clearCollection,
    clearAll,
    capturePayload,
    applyPayload,
    activatePreset,
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
