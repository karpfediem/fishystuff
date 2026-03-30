(function () {
  const DEFAULT_STORAGE_KEYS = Object.freeze({
    caught: "fishystuff.fishydex.caught.v1",
    favourites: "fishystuff.fishydex.favourites.v1",
  });

  function normalizeFishIds(values) {
    let ids = [];
    if (Array.isArray(values)) {
      ids = values;
    } else if (values && typeof values === "object") {
      ids = Object.entries(values)
        .filter(function (entry) {
          return entry[1];
        })
        .map(function (entry) {
          return entry[0];
        });
    }

    const unique = new Set();
    for (const raw of ids) {
      const fishId = Number.parseInt(String(raw), 10);
      if (Number.isInteger(fishId) && fishId > 0) {
        unique.add(fishId);
      }
    }
    return Array.from(unique).sort(function (left, right) {
      return left - right;
    });
  }

  function parseStoredFishIds(raw) {
    return normalizeFishIds(JSON.parse(raw));
  }

  function loadRecord(storageKey, storage) {
    let ids = [];
    let status = "ok";

    try {
      const raw = storage?.getItem?.(storageKey);
      if (raw) {
        try {
          ids = parseStoredFishIds(raw);
        } catch (_error) {
          storage?.removeItem?.(storageKey);
          status = "corrupted";
        }
      }
    } catch (_error) {
      status = "unavailable";
    }

    return {
      ids: ids,
      json: JSON.stringify(ids),
      status: status,
    };
  }

  function persistRecord(storageKey, values, storage) {
    const ids = normalizeFishIds(values);
    const json = JSON.stringify(ids);
    try {
      storage?.setItem?.(storageKey, json);
      return {
        ids: ids,
        json: json,
        ok: true,
      };
    } catch (_error) {
      return {
        ids: ids,
        json: json,
        ok: false,
      };
    }
  }

  function loadState(storageKeys, storage) {
    const keys = {
      caught: (storageKeys && storageKeys.caught) || DEFAULT_STORAGE_KEYS.caught,
      favourites: (storageKeys && storageKeys.favourites) || DEFAULT_STORAGE_KEYS.favourites,
    };
    const caught = loadRecord(keys.caught, storage);
    const favourites = loadRecord(keys.favourites, storage);
    return {
      caught: caught,
      favourites: favourites,
      caughtIds: caught.ids,
      favouriteIds: favourites.ids,
      caughtSet: new Set(caught.ids),
      favouriteSet: new Set(favourites.ids),
    };
  }

  window.__fishystuffSharedFishState = Object.freeze({
    DEFAULT_STORAGE_KEYS: DEFAULT_STORAGE_KEYS,
    normalizeIds: normalizeFishIds,
    parse: parseStoredFishIds,
    loadRecord: loadRecord,
    persistRecord: persistRecord,
    loadState: loadState,
  });
})();
