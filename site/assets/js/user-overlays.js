(function () {
  const STORAGE_KEY = "fishystuff.user-overlays.v2";
  const CHANGED_EVENT = "fishystuff:user-overlays-changed";
  const EXPORT_FORMAT = "fishystuff-user-overlay-v2";

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

  function normalizeNumber(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : null;
  }

  function normalizeItemIdKey(value) {
    const normalized = trimString(value).replace(/^item:/, "");
    return /^\d+$/.test(normalized) ? normalized : "";
  }

  function normalizeGroupKey(value) {
    const normalized = trimString(value);
    const slotIdx = Number.parseInt(normalized, 10);
    return Number.isInteger(slotIdx) && slotIdx >= 1 && slotIdx <= 5 ? String(slotIdx) : "";
  }

  function normalizeZoneKey(value) {
    return trimString(value);
  }

  function normalizePriceOverride(value) {
    if (!isPlainObject(value)) {
      return null;
    }
    const normalized = {};
    const basePrice = normalizeNumber(value.basePrice);
    const tradePriceCurvePercent = normalizeNumber(value.tradePriceCurvePercent);
    if (basePrice != null) {
      normalized.basePrice = Math.max(0, basePrice);
    }
    if (tradePriceCurvePercent != null) {
      normalized.tradePriceCurvePercent = Math.max(0, tradePriceCurvePercent);
    }
    return Object.keys(normalized).length ? normalized : null;
  }

  function normalizeGroupOverlay(value) {
    if (!isPlainObject(value)) {
      return null;
    }
    const normalized = {};
    if (value.present === true || value.present === false) {
      normalized.present = value.present;
    }
    const rawRatePercent = normalizeNumber(value.rawRatePercent);
    if (rawRatePercent != null) {
      normalized.rawRatePercent = Math.max(0, rawRatePercent);
    }
    return Object.keys(normalized).length ? normalized : null;
  }

  function normalizeItemOverlay(value) {
    if (!isPlainObject(value)) {
      return null;
    }
    const normalized = {};
    if (value.present === true || value.present === false) {
      normalized.present = value.present;
    }
    const slotIdx = Number.parseInt(value.slotIdx, 10);
    if (Number.isInteger(slotIdx) && slotIdx >= 1 && slotIdx <= 5) {
      normalized.slotIdx = slotIdx;
    }
    const rawRatePercent = normalizeNumber(value.rawRatePercent);
    if (rawRatePercent != null) {
      normalized.rawRatePercent = Math.max(0, rawRatePercent);
    }
    const name = trimString(value.name);
    if (name) {
      normalized.name = name;
    }
    const grade = trimString(value.grade);
    if (grade) {
      normalized.grade = grade;
    }
    if (value.isFish === true || value.isFish === false) {
      normalized.isFish = value.isFish;
    }
    return Object.keys(normalized).length ? normalized : null;
  }

  function normalizeZoneOverlay(value) {
    if (!isPlainObject(value)) {
      return null;
    }
    const groups = {};
    const items = {};
    if (isPlainObject(value.groups)) {
      for (const [rawKey, rawValue] of Object.entries(value.groups)) {
        const key = normalizeGroupKey(rawKey);
        const entry = normalizeGroupOverlay(rawValue);
        if (key && entry) {
          groups[key] = entry;
        }
      }
    }
    if (isPlainObject(value.items)) {
      for (const [rawKey, rawValue] of Object.entries(value.items)) {
        const key = normalizeItemIdKey(rawKey);
        const entry = normalizeItemOverlay(rawValue);
        if (key && entry) {
          items[key] = entry;
        }
      }
    }
    if (!Object.keys(groups).length && !Object.keys(items).length) {
      return null;
    }
    return { groups, items };
  }

  function normalizeOverlaySignals(value) {
    const zones = {};
    if (isPlainObject(value?.zones)) {
      for (const [rawZoneKey, rawZoneOverlay] of Object.entries(value.zones)) {
        const zoneKey = normalizeZoneKey(rawZoneKey);
        const zoneOverlay = normalizeZoneOverlay(rawZoneOverlay);
        if (zoneKey && zoneOverlay) {
          zones[zoneKey] = zoneOverlay;
        }
      }
    }
    return { zones };
  }

  function normalizeSnapshot(value) {
    const source = isPlainObject(value) ? value : {};
    const overlay = normalizeOverlaySignals(source.overlay);
    const priceOverrides = {};
    if (isPlainObject(source.priceOverrides)) {
      for (const [rawKey, rawValue] of Object.entries(source.priceOverrides)) {
        const key = normalizeItemIdKey(rawKey);
        const entry = normalizePriceOverride(rawValue);
        if (key && entry) {
          priceOverrides[key] = entry;
        }
      }
    }
    return {
      overlay,
      priceOverrides,
    };
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

  function emitChange(reason) {
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(CHANGED_EVENT, {
        detail: {
          reason: trimString(reason) || "update",
          snapshot: cloneJson(currentSnapshot),
        },
      }),
    );
  }

  function updateSnapshot(mutator, reason) {
    const draft = cloneJson(currentSnapshot);
    mutator(draft);
    return replaceSnapshot(draft, reason);
  }

  function replaceSnapshot(snapshot, reason) {
    const nextSnapshot = normalizeSnapshot(snapshot);
    if (stableSnapshotJson(nextSnapshot) === stableSnapshotJson(currentSnapshot)) {
      return cloneJson(currentSnapshot);
    }
    currentSnapshot = persistSnapshot(nextSnapshot, globalThis.localStorage);
    emitChange(reason);
    return cloneJson(currentSnapshot);
  }

  function setOverlaySignals(overlaySignals) {
    return updateSnapshot((draft) => {
      draft.overlay = normalizeOverlaySignals(overlaySignals);
    }, "set-overlay");
  }

  function setPriceOverrides(priceOverrides) {
    return updateSnapshot((draft) => {
      draft.priceOverrides = normalizeSnapshot({ priceOverrides }).priceOverrides;
    }, "set-prices");
  }

  function mergeLegacyPriceOverrides(priceOverrides) {
    const normalizedIncoming = normalizeSnapshot({ priceOverrides }).priceOverrides;
    return updateSnapshot((draft) => {
      if (!isPlainObject(draft.priceOverrides)) {
        draft.priceOverrides = {};
      }
      for (const [itemId, override] of Object.entries(normalizedIncoming)) {
        if (!draft.priceOverrides[itemId]) {
          draft.priceOverrides[itemId] = override;
        }
      }
    }, "merge-legacy-prices");
  }

  function clearAll() {
    return updateSnapshot((draft) => {
      draft.overlay = { zones: {} };
      draft.priceOverrides = {};
    }, "clear-all");
  }

  function importPayload(payload) {
    if (!isPlainObject(payload)) {
      throw new Error("Overlay JSON must contain an object.");
    }
    if (trimString(payload.format) !== EXPORT_FORMAT) {
      throw new Error(`Overlay JSON must use ${EXPORT_FORMAT} format.`);
    }
    return replaceSnapshot({
      overlay: payload.overlay,
      priceOverrides: payload.priceOverrides,
    }, "import");
  }

  function importText(text) {
    const serialized = trimString(text);
    if (!serialized) {
      throw new Error("Overlay JSON file is empty.");
    }
    let parsed;
    try {
      parsed = JSON.parse(serialized);
    } catch (_error) {
      throw new Error("Overlay JSON file is not valid JSON.");
    }
    return importPayload(parsed);
  }

  function exportPayload() {
    return {
      format: EXPORT_FORMAT,
      exportedAt: new Date().toISOString(),
      overlay: cloneJson(currentSnapshot.overlay),
      priceOverrides: cloneJson(currentSnapshot.priceOverrides),
    };
  }

  function exportText() {
    return JSON.stringify(exportPayload(), null, 2);
  }

  globalThis.window?.addEventListener?.("storage", (event) => {
    if (event?.key !== STORAGE_KEY) {
      return;
    }
    currentSnapshot = loadSnapshot(globalThis.localStorage);
    emitChange("storage");
  });

  globalThis.window.__fishystuffUserOverlays = Object.freeze({
    STORAGE_KEY,
    CHANGED_EVENT,
    snapshot() {
      return cloneJson(currentSnapshot);
    },
    overlaySignals() {
      return cloneJson(currentSnapshot.overlay);
    },
    priceOverrides() {
      return cloneJson(currentSnapshot.priceOverrides);
    },
    setOverlaySignals,
    setPriceOverrides,
    mergeLegacyPriceOverrides,
    clearAll,
    importPayload,
    importText,
    exportPayload,
    exportText,
    normalizeSnapshot,
  });
})();
