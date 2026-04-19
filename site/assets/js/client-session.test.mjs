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
