import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const SOURCE = fs.readFileSync(
  new URL("./client-session.js", import.meta.url),
  "utf8",
);

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

function createEventTarget() {
  const listeners = new Map();
  return {
    addEventListener(type, listener) {
      if (!listeners.has(type)) {
        listeners.set(type, []);
      }
      listeners.get(type).push(listener);
    },
    removeEventListener(type, listener) {
      const current = listeners.get(type) || [];
      listeners.set(
        type,
        current.filter((candidate) => candidate !== listener),
      );
    },
    dispatchEvent(event) {
      for (const listener of listeners.get(event.type) || []) {
        listener(event);
      }
      return true;
    },
  };
}

function createEnv({
  runtimeConfig = {},
  localStorageValues = {},
  sessionStorageValues = {},
  locationHref = "https://fishystuff.fish/map/",
} = {}) {
  const localStorage = new MemoryStorage(localStorageValues);
  const sessionStorage = new MemoryStorage(sessionStorageValues);
  const window = createEventTarget();
  const location = new URL(locationHref);
  let reloadCount = 0;
  location.reload = () => {
    reloadCount += 1;
  };
  const context = {
    JSON,
    Object,
    Array,
    String,
    Number,
    Boolean,
    RegExp,
    Error,
    Date,
    Map,
    Set,
    URL,
    Math,
    console,
    localStorage,
    sessionStorage,
    location,
    window,
    __fishystuffRuntimeConfig: runtimeConfig,
    crypto: {
      randomUUID() {
        return "00000000-0000-4000-8000-000000000000";
      },
    },
    CustomEvent: class CustomEvent {
      constructor(type, options = {}) {
        this.type = type;
        this.detail = options.detail;
      }
    },
    globalThis: null,
  };
  context.globalThis = context;
  context.window.location = location;
  vm.runInNewContext(SOURCE, context, { filename: "client-session.js" });
  return {
    context,
    helper: context.window.__fishystuffClientSession,
    localStorage,
    sessionStorage,
    reloadCount() {
      return reloadCount;
    },
  };
}

test("client session defaults automatic telemetry to opt-in for production-like builds", () => {
  const env = createEnv({
    runtimeConfig: {
      client: {
        telemetry: {
          defaultMode: "opt-in",
        },
      },
      tracing: {
        enabled: true,
      },
    },
  });

  const snapshot = env.helper.current();

  assert.equal(snapshot.actor.displayLabel, "Angler 000000");
  assert.equal(snapshot.actor.handle, "@guest");
  assert.equal(snapshot.actor.roleLabel, "Profile");
  assert.equal(snapshot.telemetry.continuous.defaultMode, "opt-in");
  assert.equal(snapshot.telemetry.continuous.effectiveEnabled, false);
  assert.equal(snapshot.telemetry.continuous.reason, "opt-in-required");
  assert.equal(snapshot.telemetry.diagnosticReports.statusLabel, "Manual");
});

test("client session follows runtime-enabled local defaults until the user opts out", () => {
  const env = createEnv({
    runtimeConfig: {
      client: {
        telemetry: {
          defaultMode: "enabled",
        },
      },
      tracing: {
        enabled: true,
      },
    },
  });

  assert.equal(
    env.helper.current().telemetry.continuous.reason,
    "enabled-by-runtime-default",
  );

  env.helper.disableTelemetry({ reload: true });

  const snapshot = env.helper.current();
  assert.equal(snapshot.telemetry.continuous.effectiveEnabled, false);
  assert.equal(snapshot.telemetry.continuous.reason, "disabled-by-user");
  assert.equal(env.reloadCount(), 1);
  assert.match(
    env.localStorage.getItem(env.helper.STORAGE_KEY),
    /"choice":"disabled"/,
  );
});

test("client session lets the user opt in and later return to runtime defaults", () => {
  const env = createEnv({
    runtimeConfig: {
      client: {
        telemetry: {
          defaultMode: "opt-in",
        },
      },
      tracing: {
        enabled: true,
      },
    },
  });

  env.helper.enableTelemetry();
  assert.equal(env.helper.current().telemetry.continuous.reason, "enabled-by-user");

  env.helper.clearTelemetryPreference();
  const snapshot = env.helper.current();
  assert.equal(snapshot.telemetry.continuous.choice, "unset");
  assert.equal(snapshot.telemetry.continuous.reason, "opt-in-required");
});

test("client session persists actor state so future auth can reuse the same model", () => {
  const env = createEnv();

  env.helper.setActor({
    kind: "user",
    provider: "oauth",
    accountId: "user-123",
    displayName: "Carp",
  });

  assert.equal(env.helper.current().actor.kind, "user");
  assert.equal(env.helper.current().actor.displayLabel, "Carp");
  assert.equal(env.helper.current().actor.handle, "@user-123");

  env.helper.clearActor();
  assert.equal(env.helper.current().actor.kind, "guest");
  assert.equal(env.helper.current().actor.displayLabel, "Angler 000000");
});

test("client session can reset profile state without rotating the browser profile", () => {
  const env = createEnv({
    runtimeConfig: {
      client: {
        telemetry: {
          defaultMode: "opt-in",
        },
      },
      tracing: {
        enabled: true,
      },
    },
  });

  env.helper.setActor({
    kind: "user",
    provider: "oauth",
    accountId: "user-123",
    displayName: "Carp",
  });
  env.helper.enableTelemetry();
  env.helper.markDiagnosticReportPrepared();

  const before = env.helper.current();
  env.helper.resetLocalProfileState();
  const after = env.helper.current();

  assert.equal(after.actor.kind, "guest");
  assert.equal(after.localProfile.id, before.localProfile.id);
  assert.equal(after.telemetry.continuous.choice, "unset");
  assert.equal(after.telemetry.continuous.reason, "opt-in-required");
  assert.equal(after.telemetry.diagnosticReports.lastPreparedAt, "");
});

test("client session can reset only the current browser session", () => {
  const env = createEnv({
    sessionStorageValues: {
      "fishystuff.client.session.v1": JSON.stringify({
        id: "session_existing",
        startedAt: "2026-04-18T10:00:00.000Z",
      }),
      "fishystuff.map.session.v1": JSON.stringify({
        view: {
          viewMode: "2d",
        },
      }),
    },
  });

  env.helper.resetLocalSessionState();
  const after = env.helper.current();

  assert.notEqual(after.session.id, "session_existing");
  assert.notEqual(env.sessionStorage.getItem("fishystuff.client.session.v1"), null);
  assert.notEqual(env.sessionStorage.getItem("fishystuff.map.session.v1"), null);
});

test("client session can clear scoped local app data without touching unrelated state", () => {
  const env = createEnv({
    runtimeConfig: {
      client: {
        telemetry: {
          defaultMode: "opt-in",
        },
      },
      tracing: {
        enabled: true,
      },
    },
    localStorageValues: {
      "fishystuff.client.v1": JSON.stringify({
        actor: {
          kind: "user",
          provider: "oauth",
          accountId: "user-123",
          displayName: "Carp",
        },
        localProfile: {
          id: "profile_existing",
          createdAt: "2026-04-18T10:00:00.000Z",
        },
        preferences: {
          telemetry: {
            continuous: {
              choice: "enabled",
              updatedAt: "2026-04-18T11:00:00.000Z",
            },
            diagnosticReports: {
              lastPreparedAt: "2026-04-18T12:00:00.000Z",
            },
          },
        },
      }),
      "fishystuff.ui.settings.v1": JSON.stringify({
        app: {
          theme: {
            selected: "dark",
          },
          density: {
            compact: true,
          },
        },
      }),
      theme: "dark",
      "fishystuff.map.window_ui.v1": JSON.stringify({ layers: { open: true } }),
      "fishystuff.map.bookmarks.v1": JSON.stringify([{ id: "bookmark:1" }]),
      "fishystuff.map.prefs.v1": JSON.stringify({ legacy: true }),
      "fishystuff.fishydex.ui.v1": JSON.stringify({ search_query: "eel" }),
      "fishystuff.fishydex.caught.v1": JSON.stringify([77]),
      "fishystuff.fishydex.favourites.v1": JSON.stringify([912]),
      "fishystuff.calculator.data.v1": JSON.stringify({ level: 25 }),
      "fishystuff.calculator.ui.v1": JSON.stringify({
        distribution_tab: "loot_flow",
      }),
      "fishystuff.user-presets.v1": JSON.stringify({
        collections: {
          "calculator-layouts": {
            selectedPresetId: "preset_a",
            presets: [
              {
                id: "preset_a",
                name: "Alpha",
                payload: {
                  pinned_layout: [],
                },
              },
            ],
          },
        },
      }),
      "fishystuff.user-overlays.v2": JSON.stringify({
        overlay: {
          zones: {
            velia: {
              groups: {
                1: {
                  rawRatePercent: 12.5,
                },
              },
            },
          },
        },
      }),
    },
    sessionStorageValues: {
      "fishystuff.client.session.v1": JSON.stringify({
        id: "session_existing",
        startedAt: "2026-04-18T10:00:00.000Z",
      }),
      "fishystuff.map.session.v1": JSON.stringify({
        view: {
          viewMode: "3d",
        },
      }),
    },
  });

  env.helper.clearLocalDataScope("browser-ui");
  assert.deepEqual(
    JSON.parse(env.localStorage.getItem("fishystuff.ui.settings.v1")),
    {
      app: {
        theme: {
          selected: "dark",
        },
      },
    },
  );
  assert.equal(env.localStorage.getItem("theme"), null);

  env.helper.clearLocalDataScope("profile-ui");
  assert.equal(env.localStorage.getItem("fishystuff.ui.settings.v1"), null);

  env.helper.clearLocalDataScope("map-data");
  assert.notEqual(env.localStorage.getItem("fishystuff.map.window_ui.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.map.bookmarks.v1"), null);
  assert.notEqual(env.localStorage.getItem("fishystuff.map.prefs.v1"), null);
  assert.notEqual(env.sessionStorage.getItem("fishystuff.map.session.v1"), null);

  env.helper.clearLocalDataScope("map-ui");
  assert.equal(env.localStorage.getItem("fishystuff.map.window_ui.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.map.prefs.v1"), null);
  assert.equal(env.sessionStorage.getItem("fishystuff.map.session.v1"), null);

  env.helper.clearLocalDataScope("dex-data");
  assert.notEqual(env.localStorage.getItem("fishystuff.fishydex.ui.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.fishydex.caught.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.fishydex.favourites.v1"), null);

  env.helper.clearLocalDataScope("dex-ui");
  assert.equal(env.localStorage.getItem("fishystuff.fishydex.ui.v1"), null);

  env.helper.clearLocalDataScope("calculator-data");
  assert.equal(env.localStorage.getItem("fishystuff.calculator.data.v1"), null);
  assert.notEqual(env.localStorage.getItem("fishystuff.calculator.ui.v1"), null);
  assert.notEqual(env.localStorage.getItem("fishystuff.user-overlays.v2"), null);
  assert.notEqual(env.localStorage.getItem("fishystuff.user-presets.v1"), null);

  env.helper.clearLocalDataScope("calculator-ui");
  assert.equal(env.localStorage.getItem("fishystuff.calculator.ui.v1"), null);
  assert.notEqual(env.localStorage.getItem("fishystuff.user-overlays.v2"), null);
  assert.notEqual(env.localStorage.getItem("fishystuff.user-presets.v1"), null);

  env.helper.clearLocalDataScope("presets-data");
  assert.equal(env.localStorage.getItem("fishystuff.user-presets.v1"), null);

  const profileBefore = env.helper.current().localProfile.id;
  env.helper.clearLocalDataScope("profile-data");
  assert.equal(env.helper.current().actor.kind, "guest");
  assert.equal(env.helper.current().localProfile.id, profileBefore);

  const sessionBefore = env.helper.current().session.id;
  env.helper.clearLocalDataScope("browser-data");
  assert.notEqual(env.helper.current().session.id, sessionBefore);

  env.helper.clearLocalDataScope("overrides-data");
  assert.equal(env.localStorage.getItem("fishystuff.user-overlays.v2"), null);
});

test("client session can clear all local user state and start a fresh browser profile", () => {
  const env = createEnv({
    runtimeConfig: {
      client: {
        telemetry: {
          defaultMode: "opt-in",
        },
      },
      tracing: {
        enabled: true,
      },
    },
    localStorageValues: {
      "fishystuff.client.v1": JSON.stringify({
        actor: {
          kind: "user",
          provider: "oauth",
          accountId: "user-123",
          displayName: "Carp",
        },
        localProfile: {
          id: "profile_existing",
          createdAt: "2026-04-18T10:00:00.000Z",
        },
        preferences: {
          telemetry: {
            continuous: {
              choice: "enabled",
              updatedAt: "2026-04-18T11:00:00.000Z",
            },
            diagnosticReports: {
              lastPreparedAt: "2026-04-18T12:00:00.000Z",
            },
          },
        },
      }),
      "fishystuff.ui.settings.v1": JSON.stringify({
        app: {
          theme: {
            selected: "dark",
          },
        },
      }),
      theme: "dark",
      "fishystuff.map.window_ui.v1": JSON.stringify({ layers: { open: true } }),
      "fishystuff.fishydex.caught.v1": JSON.stringify([77]),
      "fishystuff.calculator.data.v1": JSON.stringify({ level: 25 }),
      "fishystuff.calculator.ui.v1": JSON.stringify({
        distribution_tab: "groups",
      }),
      "fishystuff.user-presets.v1": JSON.stringify({
        collections: {
          "calculator-layouts": {
            selectedPresetId: "preset_a",
            presets: [
              {
                id: "preset_a",
                name: "Alpha",
                payload: {
                  pinned_layout: [],
                },
              },
            ],
          },
        },
      }),
      "fishystuff.user-overlays.v2": JSON.stringify({ priceOverrides: { 77: { basePrice: 10 } } }),
    },
    sessionStorageValues: {
      "fishystuff.client.session.v1": JSON.stringify({
        id: "session_existing",
        startedAt: "2026-04-18T10:00:00.000Z",
      }),
      "fishystuff.map.session.v1": JSON.stringify({
        view: {
          viewMode: "3d",
        },
      }),
    },
  });

  env.helper.clearAllLocalState();
  const after = env.helper.current();

  assert.equal(after.actor.kind, "guest");
  assert.notEqual(after.localProfile.id, "profile_existing");
  assert.notEqual(after.session.id, "session_existing");
  assert.equal(after.telemetry.continuous.choice, "unset");
  assert.equal(env.localStorage.getItem("fishystuff.ui.settings.v1"), null);
  assert.equal(env.localStorage.getItem("theme"), null);
  assert.equal(env.localStorage.getItem("fishystuff.map.window_ui.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.fishydex.caught.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.calculator.data.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.calculator.ui.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.user-presets.v1"), null);
  assert.equal(env.localStorage.getItem("fishystuff.user-overlays.v2"), null);
  assert.equal(env.sessionStorage.getItem("fishystuff.map.session.v1"), null);
});
