import test from "node:test";
import assert from "node:assert/strict";

class MemoryStorage {
  constructor(initial = {}) {
    this.map = new Map(Object.entries(initial));
  }

  getItem(key) {
    return this.map.has(key) ? this.map.get(key) : null;
  }

  setItem(key, value) {
    this.map.set(key, String(value));
  }

  removeItem(key) {
    this.map.delete(key);
  }
}

function createStore(snapshot = {}) {
  let settings = JSON.parse(JSON.stringify(snapshot));
  const listeners = new Set();

  function normalizePath(path) {
    return Array.isArray(path)
      ? path.map((part) => String(part || "").trim()).filter(Boolean)
      : String(path || "").split(".").map((part) => part.trim()).filter(Boolean);
  }

  function getAtPath(root, pathParts, fallback) {
    let current = root;
    for (const part of pathParts) {
      if (!current || typeof current !== "object" || !(part in current)) {
        return fallback;
      }
      current = current[part];
    }
    return current === undefined ? fallback : current;
  }

  function setAtPath(root, pathParts, value) {
    const nextRoot = root && typeof root === "object" ? { ...root } : {};
    let cursor = nextRoot;
    for (const part of pathParts.slice(0, -1)) {
      cursor[part] = cursor[part] && typeof cursor[part] === "object" ? { ...cursor[part] } : {};
      cursor = cursor[part];
    }
    cursor[pathParts[pathParts.length - 1]] = value;
    return nextRoot;
  }

  return {
    get(path, fallback) {
      return getAtPath(settings, normalizePath(path), fallback);
    },
    set(path, value) {
      settings = setAtPath(settings, normalizePath(path), value);
      for (const listener of listeners) {
        listener({ path: normalizePath(path).join("."), settings, source: "local" });
      }
      return settings;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    snapshot() {
      return settings;
    },
  };
}

async function loadModule() {
  const originalHTMLElement = globalThis.HTMLElement;
  globalThis.HTMLElement = globalThis.HTMLElement ?? class {};
  try {
    return await import(`./notice-disclosure.js?test=${Date.now()}-${Math.random()}`);
  } finally {
    globalThis.HTMLElement = originalHTMLElement;
  }
}

test("normalizeNoticeSettingsPath trims and filters empty segments", async () => {
  const { normalizeNoticeSettingsPath } = await loadModule();
  assert.deepEqual(normalizeNoticeSettingsPath(" map . zonePane . noticeOpen "), [
    "map",
    "zonePane",
    "noticeOpen",
  ]);
  assert.deepEqual(normalizeNoticeSettingsPath([" calculator ", "", "noticeOpen "]), [
    "calculator",
    "noticeOpen",
  ]);
});

test("notice disclosure persistence falls back to the author default when no value is stored", async () => {
  const { readPersistentNoticeDisclosureOpen } = await loadModule();
  const globalRef = {
    localStorage: new MemoryStorage(),
    window: {},
  };

  assert.equal(
    readPersistentNoticeDisclosureOpen("map.zonePane.noticeOpen", true, { globalRef }),
    true,
  );
  assert.equal(
    readPersistentNoticeDisclosureOpen("map.zonePane.noticeOpen", false, { globalRef }),
    false,
  );
});

test("notice disclosure persistence uses the shared ui settings store when available", async () => {
  const {
    readPersistentNoticeDisclosureOpen,
    writePersistentNoticeDisclosureOpen,
  } = await loadModule();
  const store = createStore({
    map: {
      zonePane: {
        noticeOpen: false,
      },
    },
  });
  const globalRef = {
    localStorage: new MemoryStorage(),
    window: {
      __fishystuffUiSettings: store,
    },
  };

  assert.equal(
    readPersistentNoticeDisclosureOpen("map.zonePane.noticeOpen", true, { globalRef }),
    false,
  );

  assert.equal(
    writePersistentNoticeDisclosureOpen("calculator.noticeOpen", false, { globalRef }),
    true,
  );
  assert.equal(store.get("calculator.noticeOpen", true), false);
});

test("notice disclosure persistence writes through to shared ui settings storage when no store is present", async () => {
  const {
    readPersistentNoticeDisclosureOpen,
    writePersistentNoticeDisclosureOpen,
  } = await loadModule();
  const localStorage = new MemoryStorage();
  const globalRef = {
    localStorage,
    window: {},
  };

  assert.equal(
    writePersistentNoticeDisclosureOpen("guides.noticeOpen", false, { globalRef }),
    true,
  );
  assert.deepEqual(
    JSON.parse(localStorage.getItem("fishystuff.ui.settings.v1")),
    {
      guides: {
        noticeOpen: false,
      },
    },
  );
  assert.equal(
    readPersistentNoticeDisclosureOpen("guides.noticeOpen", true, { globalRef }),
    false,
  );
});
