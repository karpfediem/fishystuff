(function () {
  const STORAGE_KEY = "fishystuff.client.v1";
  const SESSION_STORAGE_KEY = "fishystuff.client.session.v1";
  const CHANGE_EVENT = "fishystuff:client-session-change";
  const VERSION = 1;

  function isPlainObject(value) {
    return Boolean(value) && Object.prototype.toString.call(value) === "[object Object]";
  }

  function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function trimString(value) {
    const normalized = String(value ?? "").trim();
    return normalized || "";
  }

  function nowIso() {
    return new Date().toISOString();
  }

  function randomHex(byteLength) {
    const length = Number.isFinite(byteLength) ? Math.max(1, Math.trunc(byteLength)) : 8;
    const cryptoRef = globalThis.crypto;
    if (cryptoRef && typeof cryptoRef.getRandomValues === "function") {
      const bytes = new Uint8Array(length);
      cryptoRef.getRandomValues(bytes);
      return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
    }
    return `${Date.now().toString(16)}${Math.random().toString(16).slice(2)}`.slice(0, length * 2);
  }

  function randomId(prefix) {
    const normalizedPrefix = trimString(prefix);
    if (globalThis.crypto && typeof globalThis.crypto.randomUUID === "function") {
      return `${normalizedPrefix}${globalThis.crypto.randomUUID()}`;
    }
    return `${normalizedPrefix}${randomHex(16)}`;
  }

  function shortId(value, length = 8) {
    return trimString(value).slice(0, Math.max(4, Math.trunc(length || 8)));
  }

  function normalizeTelemetryChoice(value) {
    const normalized = trimString(value).toLowerCase();
    if (normalized === "enabled" || normalized === "disabled" || normalized === "unset") {
      return normalized;
    }
    return "unset";
  }

  function normalizeTelemetryDefaultMode(value, fallback) {
    const normalized = trimString(value).toLowerCase();
    if (normalized === "enabled" || normalized === "opt-in" || normalized === "disabled") {
      return normalized;
    }
    return fallback;
  }

  function normalizeActor(value) {
    const source = isPlainObject(value) ? value : {};
    const kind = trimString(source.kind).toLowerCase() === "user" ? "user" : "guest";
    return {
      kind,
      provider: trimString(source.provider),
      accountId: trimString(source.accountId),
      displayName: trimString(source.displayName) || (kind === "user" ? "User" : "Guest"),
    };
  }

  function normalizeLocalProfile(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      id: trimString(source.id) || randomId("profile_"),
      createdAt: trimString(source.createdAt) || nowIso(),
    };
  }

  function normalizeContinuousTelemetryPreference(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      choice: normalizeTelemetryChoice(source.choice),
      updatedAt: trimString(source.updatedAt),
    };
  }

  function normalizeDiagnosticReportsPreference(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      lastPreparedAt: trimString(source.lastPreparedAt),
      lastSubmittedAt: trimString(source.lastSubmittedAt),
    };
  }

  function normalizeSnapshot(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      version: VERSION,
      actor: normalizeActor(source.actor),
      localProfile: normalizeLocalProfile(source.localProfile),
      preferences: {
        telemetry: {
          continuous: normalizeContinuousTelemetryPreference(
            source.preferences?.telemetry?.continuous,
          ),
          diagnosticReports: normalizeDiagnosticReportsPreference(
            source.preferences?.telemetry?.diagnosticReports,
          ),
        },
      },
    };
  }

  function normalizeSession(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      version: VERSION,
      id: trimString(source.id) || randomId("session_"),
      startedAt: trimString(source.startedAt) || nowIso(),
    };
  }

  function readJson(storage, key, fallback) {
    try {
      const raw = storage?.getItem?.(key);
      if (!raw) {
        return fallback;
      }
      return JSON.parse(raw);
    } catch (_error) {
      return fallback;
    }
  }

  function writeJson(storage, key, value) {
    try {
      storage?.setItem?.(key, JSON.stringify(value));
    } catch (_error) {
    }
  }

  function runtimeTelemetryDefaultMode() {
    const runtimeConfig = globalThis.__fishystuffRuntimeConfig || {};
    const tracingEnabled = runtimeConfig?.tracing?.enabled === true;
    return normalizeTelemetryDefaultMode(
      runtimeConfig?.client?.telemetry?.defaultMode,
      tracingEnabled ? "enabled" : "opt-in",
    );
  }

  function deriveContinuousTelemetryState(snapshot) {
    const defaultMode = runtimeTelemetryDefaultMode();
    const choice = snapshot.preferences.telemetry.continuous.choice;
    if (defaultMode === "disabled") {
      return {
        defaultMode,
        choice,
        effectiveEnabled: false,
        source: "runtime-policy",
        reason: "disabled-by-runtime-policy",
        statusLabel: "Off",
        preferenceLabel:
          choice === "enabled"
            ? "Requested on"
            : choice === "disabled"
              ? "Stored off"
              : "Unavailable",
        detail: "Automatic telemetry is disabled by runtime policy for this site.",
        canEnable: false,
        canDisable: false,
        resetAvailable: choice !== "unset",
      };
    }
    if (choice === "enabled") {
      return {
        defaultMode,
        choice,
        effectiveEnabled: true,
        source: "user",
        reason: "enabled-by-user",
        statusLabel: "On",
        preferenceLabel: "Always on",
        detail: "Automatic telemetry is enabled because you explicitly opted in on this browser.",
        canEnable: false,
        canDisable: true,
        resetAvailable: true,
      };
    }
    if (choice === "disabled") {
      return {
        defaultMode,
        choice,
        effectiveEnabled: false,
        source: "user",
        reason: "disabled-by-user",
        statusLabel: "Off",
        preferenceLabel: "Always off",
        detail: "Automatic telemetry is disabled because you explicitly turned it off on this browser.",
        canEnable: true,
        canDisable: false,
        resetAvailable: true,
      };
    }
    if (defaultMode === "enabled") {
      return {
        defaultMode,
        choice,
        effectiveEnabled: true,
        source: "runtime-default",
        reason: "enabled-by-runtime-default",
        statusLabel: "On",
        preferenceLabel: "Using default",
        detail: "Automatic telemetry is enabled by the local development default.",
        canEnable: false,
        canDisable: true,
        resetAvailable: false,
      };
    }
    return {
      defaultMode,
      choice,
      effectiveEnabled: false,
      source: "runtime-default",
      reason: "opt-in-required",
      statusLabel: "Off",
      preferenceLabel: "Using default",
      detail: "Automatic telemetry is off until you enable it on this browser.",
      canEnable: true,
      canDisable: false,
      resetAvailable: false,
    };
  }

  function deriveDiagnosticReportState(snapshot) {
    const diagnosticReports = snapshot.preferences.telemetry.diagnosticReports;
    return {
      mode: "manual",
      statusLabel: "Manual",
      detail:
        "Future incident reports can attach a one-off trace bundle and short report without enabling background telemetry.",
      lastPreparedAt: diagnosticReports.lastPreparedAt,
      lastSubmittedAt: diagnosticReports.lastSubmittedAt,
    };
  }

  function buildSnapshot(snapshot, session) {
    const actor = snapshot.actor;
    const continuous = deriveContinuousTelemetryState(snapshot);
    const localProfileShortId = shortId(snapshot.localProfile.id);
    const sessionShortId = shortId(session.id);
    return {
      actor: {
        ...actor,
        isGuest: actor.kind === "guest",
        displayLabel: actor.displayName,
        detail:
          actor.kind === "guest"
            ? "Local guest profile in this browser"
            : `${actor.provider || "Account"} user`,
      },
      localProfile: {
        ...snapshot.localProfile,
        shortId: localProfileShortId,
        label: `Browser ${localProfileShortId}`,
      },
      session: {
        ...session,
        shortId: sessionShortId,
        label: `Session ${sessionShortId}`,
      },
      telemetry: {
        continuous,
        diagnosticReports: deriveDiagnosticReportState(snapshot),
      },
    };
  }

  function emitChange(snapshot, reason) {
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(CHANGE_EVENT, {
        detail: {
          reason: trimString(reason) || "update",
          snapshot,
        },
      }),
    );
  }

  let localSnapshot = normalizeSnapshot(readJson(globalThis.localStorage, STORAGE_KEY, {}));
  writeJson(globalThis.localStorage, STORAGE_KEY, localSnapshot);

  let sessionSnapshot = normalizeSession(readJson(globalThis.sessionStorage, SESSION_STORAGE_KEY, {}));
  writeJson(globalThis.sessionStorage, SESSION_STORAGE_KEY, sessionSnapshot);

  const boundSignalStores = new Set();

  function current() {
    return cloneJson(buildSnapshot(localSnapshot, sessionSnapshot));
  }

  function telemetryState() {
    return current().telemetry;
  }

  function patchBoundSignals() {
    const snapshot = current();
    for (const signals of boundSignalStores) {
      if (!signals || typeof signals !== "object") {
        continue;
      }
      signals._client_session = cloneJson(snapshot);
    }
    return snapshot;
  }

  function replaceLocalSnapshot(nextSnapshot, reason) {
    localSnapshot = normalizeSnapshot(nextSnapshot);
    writeJson(globalThis.localStorage, STORAGE_KEY, localSnapshot);
    const snapshot = patchBoundSignals();
    emitChange(snapshot, reason);
    return snapshot;
  }

  function updateLocalSnapshot(mutator, reason) {
    const draft = cloneJson(localSnapshot);
    mutator(draft);
    return replaceLocalSnapshot(draft, reason);
  }

  function maybeReload(options) {
    if (!options || options.reload !== true) {
      return;
    }
    globalThis.location?.reload?.();
  }

  function setContinuousTelemetryChoice(choice, options = {}) {
    const normalizedChoice = normalizeTelemetryChoice(choice);
    const nextChoice = normalizedChoice === "unset" ? "unset" : normalizedChoice;
    const snapshot = updateLocalSnapshot((draft) => {
      draft.preferences.telemetry.continuous.choice = nextChoice;
      draft.preferences.telemetry.continuous.updatedAt =
        nextChoice === "unset" ? "" : nowIso();
    }, "telemetry-preference");
    maybeReload(options);
    return snapshot;
  }

  function setActor(actor) {
    return updateLocalSnapshot((draft) => {
      draft.actor = normalizeActor(actor);
    }, "actor");
  }

  function markDiagnosticReport(eventKey) {
    const normalizedEventKey = trimString(eventKey);
    if (normalizedEventKey !== "lastPreparedAt" && normalizedEventKey !== "lastSubmittedAt") {
      return current();
    }
    return updateLocalSnapshot((draft) => {
      draft.preferences.telemetry.diagnosticReports[normalizedEventKey] = nowIso();
    }, "diagnostic-report");
  }

  function createDiagnosticReportDraft(context) {
    const snapshot = current();
    const normalizedContext = isPlainObject(context) ? context : {};
    return {
      createdAt: nowIso(),
      actor: snapshot.actor,
      localProfile: snapshot.localProfile,
      session: snapshot.session,
      telemetry: snapshot.telemetry,
      page: {
        href: trimString(globalThis.location?.href),
        path: trimString(globalThis.location?.pathname),
      },
      report: {
        summary: trimString(normalizedContext.summary),
        category: trimString(normalizedContext.category),
      },
    };
  }

  function subscribe(listener) {
    if (typeof listener !== "function") {
      return function () {};
    }
    function handle(event) {
      listener(event.detail || {});
    }
    globalThis.window?.addEventListener?.(CHANGE_EVENT, handle);
    return function () {
      globalThis.window?.removeEventListener?.(CHANGE_EVENT, handle);
    };
  }

  function bindDatastar(signals) {
    if (!signals || typeof signals !== "object") {
      return null;
    }
    boundSignalStores.add(signals);
    signals._client_session = current();
    return signals;
  }

  function unbindDatastar(signals) {
    boundSignalStores.delete(signals);
  }

  globalThis.window?.addEventListener?.("storage", (event) => {
    if (event.key !== STORAGE_KEY) {
      return;
    }
    localSnapshot = normalizeSnapshot(readJson(globalThis.localStorage, STORAGE_KEY, {}));
    const snapshot = patchBoundSignals();
    emitChange(snapshot, "storage");
  });

  globalThis.window.__fishystuffClientSession = Object.freeze({
    STORAGE_KEY,
    SESSION_STORAGE_KEY,
    CHANGE_EVENT,
    bindDatastar,
    createDiagnosticReportDraft,
    current,
    clearActor() {
      return setActor({
        kind: "guest",
        provider: "",
        accountId: "",
        displayName: "Guest",
      });
    },
    disableTelemetry(options) {
      return setContinuousTelemetryChoice("disabled", options);
    },
    enableTelemetry(options) {
      return setContinuousTelemetryChoice("enabled", options);
    },
    clearTelemetryPreference(options) {
      return setContinuousTelemetryChoice("unset", options);
    },
    markDiagnosticReportPrepared() {
      return markDiagnosticReport("lastPreparedAt");
    },
    markDiagnosticReportSubmitted() {
      return markDiagnosticReport("lastSubmittedAt");
    },
    setActor,
    subscribe,
    telemetryState,
    unbindDatastar,
  });
})();
