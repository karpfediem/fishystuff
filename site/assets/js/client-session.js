(function () {
  const STORAGE_KEY = "fishystuff.client.v1";
  const SESSION_STORAGE_KEY = "fishystuff.client.session.v1";
  const UI_SETTINGS_KEY = "fishystuff.ui.settings.v1";
  const UI_SETTINGS_THEME_PATH = Object.freeze(["app", "theme"]);
  const LEGACY_THEME_STORAGE_KEY = "theme";
  const UI_SETTINGS_EVENT = "fishystuff:uisettingschange";
  const MAP_UI_STORAGE_KEY = "fishystuff.map.window_ui.v1";
  const MAP_BOOKMARKS_STORAGE_KEY = "fishystuff.map.bookmarks.v1";
  const MAP_LEGACY_PREFS_STORAGE_KEY = "fishystuff.map.prefs.v1";
  const MAP_SESSION_STORAGE_KEY = "fishystuff.map.session.v1";
  const DEX_UI_STORAGE_KEY = "fishystuff.fishydex.ui.v1";
  const DEX_CAUGHT_STORAGE_KEY = "fishystuff.fishydex.caught.v1";
  const DEX_FAVOURITES_STORAGE_KEY = "fishystuff.fishydex.favourites.v1";
  const CALCULATOR_DATA_STORAGE_KEY = "fishystuff.calculator.data.v1";
  const CALCULATOR_UI_STORAGE_KEY = "fishystuff.calculator.ui.v1";
  const USER_OVERLAYS_STORAGE_KEY = "fishystuff.user-overlays.v2";
  const USER_PRESETS_STORAGE_KEY = "fishystuff.user-presets.v1";
  const CHANGE_EVENT = "fishystuff:client-session-change";
  const VERSION = 1;
  const LOCAL_DATA_SCOPES = Object.freeze([
    { id: "profile-data", label: "Profile data" },
    { id: "profile-ui", label: "Profile UI" },
    { id: "browser-data", label: "Browser data" },
    { id: "browser-ui", label: "Browser UI" },
    { id: "map-data", label: "Map data" },
    { id: "map-ui", label: "Map UI" },
    { id: "dex-data", label: "Dex data" },
    { id: "dex-ui", label: "Dex UI" },
    { id: "calculator-data", label: "Calculator data" },
    { id: "calculator-ui", label: "Calculator UI" },
    { id: "presets-data", label: "Saved presets" },
    { id: "overrides-data", label: "Overrides data" },
    { id: "all", label: "All local data" },
  ]);

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

  function normalizePath(path) {
    if (Array.isArray(path)) {
      return path
        .map((part) => trimString(part))
        .filter(Boolean);
    }
    return trimString(path)
      .split(".")
      .map((part) => trimString(part))
      .filter(Boolean);
  }

  function pathStartsWith(pathParts, prefixParts) {
    if (!Array.isArray(pathParts) || !Array.isArray(prefixParts) || prefixParts.length > pathParts.length) {
      return false;
    }
    return prefixParts.every((part, index) => pathParts[index] === part);
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

  function compactIdToken(value, fallback = "local", length = 6) {
    const normalized = trimString(value);
    if (!normalized) {
      return fallback;
    }
    const parts = normalized.split(/[-_]+/).filter(Boolean);
    const candidate = parts[parts.length - 1] || normalized;
    return candidate.slice(0, Math.max(4, Math.trunc(length || 6))) || fallback;
  }

  function slugToken(value) {
    return trimString(value)
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function initials(value) {
    const parts = trimString(value)
      .split(/[\s_-]+/)
      .filter(Boolean);
    if (!parts.length) {
      return "FS";
    }
    return parts
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() || "")
      .join("") || "FS";
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

  function removeStorageKey(storage, key) {
    try {
      storage?.removeItem?.(key);
    } catch (_error) {
    }
  }

  function removeAtPath(root, pathParts) {
    const current = isPlainObject(root) ? root : {};
    if (!pathParts.length) {
      return {};
    }
    if (!(pathParts[0] in current)) {
      return current;
    }
    const next = { ...current };
    if (pathParts.length === 1) {
      delete next[pathParts[0]];
      return next;
    }
    const child = removeAtPath(next[pathParts[0]], pathParts.slice(1));
    if (isPlainObject(child) && Object.keys(child).length) {
      next[pathParts[0]] = child;
    } else {
      delete next[pathParts[0]];
    }
    return next;
  }

  function getAtPath(root, pathParts, fallback) {
    let current = isPlainObject(root) ? root : {};
    for (const part of pathParts) {
      if (!isPlainObject(current) || !(part in current)) {
        return fallback;
      }
      current = current[part];
    }
    return current === undefined ? fallback : current;
  }

  function setAtPath(root, pathParts, value) {
    if (!pathParts.length) {
      return isPlainObject(value) ? value : {};
    }

    const nextRoot = isPlainObject(root) ? { ...root } : {};
    let cursor = nextRoot;
    for (let index = 0; index < pathParts.length - 1; index += 1) {
      const part = pathParts[index];
      const existing = isPlainObject(cursor[part]) ? cursor[part] : {};
      cursor[part] = { ...existing };
      cursor = cursor[part];
    }
    cursor[pathParts[pathParts.length - 1]] = value;
    return nextRoot;
  }

  function cloneJsonOrValue(value) {
    return isPlainObject(value) || Array.isArray(value)
      ? cloneJson(value)
      : value;
  }

  function normalizeThemeSettingsBranch(value) {
    if (isPlainObject(value)) {
      return cloneJson(value);
    }
    const selected = trimString(value);
    return selected ? { selected } : null;
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
        detail: "Telemetry is unavailable.",
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
        detail: "Telemetry is enabled.",
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
        detail: "Telemetry is disabled.",
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
        detail: "Telemetry is enabled.",
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
      detail: "Telemetry is off.",
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
      detail: "Reports are shared manually.",
      lastPreparedAt: diagnosticReports.lastPreparedAt,
      lastSubmittedAt: diagnosticReports.lastSubmittedAt,
    };
  }

  function buildSnapshot(snapshot, session) {
    const actor = snapshot.actor;
    const continuous = deriveContinuousTelemetryState(snapshot);
    const localProfileShortId = compactIdToken(snapshot.localProfile.id, "local");
    const sessionShortId = compactIdToken(session.id, "session");
    const fallbackProfileName =
      actor.kind === "guest" && actor.displayName === "Guest"
        ? `Angler ${localProfileShortId.toUpperCase()}`
        : actor.displayName;
    const displayLabel = fallbackProfileName || actor.displayName;
    const handleSeed =
      actor.accountId
      || actor.displayName
      || `local-${localProfileShortId.toLowerCase()}`;
    const profileHandle =
      `@${slugToken(handleSeed) || `local-${localProfileShortId.toLowerCase()}`}`;
    const roleLabel = actor.kind === "guest" ? "Profile" : "Member";
    const summary =
      actor.kind === "guest"
        ? ""
        : `${actor.provider || "Account"} profile`;
    return {
      actor: {
        ...actor,
        isGuest: actor.kind === "guest",
        displayLabel,
        handle: profileHandle,
        roleLabel,
        avatarLabel: initials(displayLabel),
        summary,
        detail:
          actor.kind === "guest"
            ? ""
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

  function dispatchUiSettingsChange(settings, source, path) {
    globalThis.window?.dispatchEvent?.(
      new CustomEvent(UI_SETTINGS_EVENT, {
        detail: {
          key: UI_SETTINGS_KEY,
          path: trimString(path) || null,
          settings,
          source: trimString(source) || "local",
        },
      }),
    );
  }

  function persistUiSettingsSnapshot(nextSettings, source, path) {
    const normalized = isPlainObject(nextSettings) ? nextSettings : {};
    if (Object.keys(normalized).length) {
      try {
        globalThis.localStorage?.setItem?.(UI_SETTINGS_KEY, JSON.stringify(normalized));
      } catch (_error) {
      }
    } else {
      removeStorageKey(globalThis.localStorage, UI_SETTINGS_KEY);
    }
    dispatchUiSettingsChange(normalized, source, path);
    return normalized;
  }

  function syncThemeAfterUiSettingsChange() {
    const theme = globalThis.window?.__theme;
    if (theme && typeof theme.get === "function" && typeof theme.apply === "function") {
      theme.apply(theme.get());
    }
  }

  function clearSharedUiSettings(source) {
    const normalizedSource = trimString(source) || "local";
    const uiSettingsStore = globalThis.window?.__fishystuffUiSettings;
    if (uiSettingsStore && typeof uiSettingsStore.clear === "function") {
      uiSettingsStore.clear(normalizedSource);
    } else {
      persistUiSettingsSnapshot({}, normalizedSource, null);
    }

    removeStorageKey(globalThis.localStorage, LEGACY_THEME_STORAGE_KEY);
    syncThemeAfterUiSettingsChange();
  }

  function clearSharedUiSettingsExceptTheme(source) {
    const normalizedSource = trimString(source) || "local";
    const uiSettingsStore = globalThis.window?.__fishystuffUiSettings;

    if (
      uiSettingsStore
      && typeof uiSettingsStore.get === "function"
      && typeof uiSettingsStore.clear === "function"
      && typeof uiSettingsStore.set === "function"
    ) {
      const themeValue = normalizeThemeSettingsBranch(
        cloneJsonOrValue(uiSettingsStore.get(UI_SETTINGS_THEME_PATH, null)),
      );
      uiSettingsStore.clear(normalizedSource);
      if (themeValue) {
        uiSettingsStore.set(UI_SETTINGS_THEME_PATH, themeValue);
      }
    } else {
      const currentSettings = readJson(globalThis.localStorage, UI_SETTINGS_KEY, {});
      const themeValue = normalizeThemeSettingsBranch(
        getAtPath(currentSettings, UI_SETTINGS_THEME_PATH, null),
      );
      const nextSettings = themeValue
        ? setAtPath({}, UI_SETTINGS_THEME_PATH, themeValue)
        : {};
      persistUiSettingsSnapshot(nextSettings, normalizedSource, null);
    }

    removeStorageKey(globalThis.localStorage, LEGACY_THEME_STORAGE_KEY);
    syncThemeAfterUiSettingsChange();
  }

  function clearSharedUiSettingsPath(path, source) {
    const normalizedSource = trimString(source) || "local";
    const pathParts = normalizePath(path);
    if (!pathParts.length) {
      clearSharedUiSettings(normalizedSource);
      return {};
    }

    const uiSettingsStore = globalThis.window?.__fishystuffUiSettings;
    let nextSettings;
    if (uiSettingsStore && typeof uiSettingsStore.remove === "function") {
      nextSettings = uiSettingsStore.remove(pathParts, normalizedSource);
    } else {
      const currentSettings = readJson(globalThis.localStorage, UI_SETTINGS_KEY, {});
      nextSettings = persistUiSettingsSnapshot(
        removeAtPath(currentSettings, pathParts),
        normalizedSource,
        pathParts.join("."),
      );
    }

    if (pathStartsWith(pathParts, UI_SETTINGS_THEME_PATH)) {
      removeStorageKey(globalThis.localStorage, LEGACY_THEME_STORAGE_KEY);
      syncThemeAfterUiSettingsChange();
    }

    return nextSettings;
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

  function resetLocalProfileState(options = {}) {
    const snapshot = updateLocalSnapshot((draft) => {
      draft.actor = normalizeActor({});
      draft.preferences.telemetry.continuous = normalizeContinuousTelemetryPreference({});
      draft.preferences.telemetry.diagnosticReports = normalizeDiagnosticReportsPreference({});
    }, "profile-reset");
    maybeReload(options);
    return snapshot;
  }

  function resetLocalSessionState(options = {}) {
    removeStorageKey(globalThis.sessionStorage, SESSION_STORAGE_KEY);
    sessionSnapshot = normalizeSession({});
    writeJson(globalThis.sessionStorage, SESSION_STORAGE_KEY, sessionSnapshot);
    const snapshot = patchBoundSignals();
    emitChange(snapshot, "session-reset");
    maybeReload(options);
    return snapshot;
  }

  function clearMapDataLocalState() {
    removeStorageKey(globalThis.localStorage, MAP_BOOKMARKS_STORAGE_KEY);
  }

  function clearMapUiLocalState() {
    removeStorageKey(globalThis.localStorage, MAP_UI_STORAGE_KEY);
    removeStorageKey(globalThis.localStorage, MAP_LEGACY_PREFS_STORAGE_KEY);
    removeStorageKey(globalThis.sessionStorage, MAP_SESSION_STORAGE_KEY);
  }

  function clearDexDataLocalState() {
    removeStorageKey(globalThis.localStorage, DEX_CAUGHT_STORAGE_KEY);
    removeStorageKey(globalThis.localStorage, DEX_FAVOURITES_STORAGE_KEY);
  }

  function clearDexUiLocalState() {
    removeStorageKey(globalThis.localStorage, DEX_UI_STORAGE_KEY);
  }

  function clearCalculatorDataLocalState() {
    removeStorageKey(globalThis.localStorage, CALCULATOR_DATA_STORAGE_KEY);
  }

  function clearCalculatorUiLocalState() {
    removeStorageKey(globalThis.localStorage, CALCULATOR_UI_STORAGE_KEY);
  }

  function clearUserOverridesLocalState() {
    removeStorageKey(globalThis.localStorage, USER_OVERLAYS_STORAGE_KEY);
  }

  function clearUserPresetsLocalState() {
    removeStorageKey(globalThis.localStorage, USER_PRESETS_STORAGE_KEY);
  }

  function clearAllLocalState(options = {}) {
    clearMapDataLocalState();
    clearMapUiLocalState();
    clearDexDataLocalState();
    clearDexUiLocalState();
    clearCalculatorDataLocalState();
    clearCalculatorUiLocalState();
    clearUserPresetsLocalState();
    clearUserOverridesLocalState();
    clearSharedUiSettings("local-user-reset");

    removeStorageKey(globalThis.localStorage, STORAGE_KEY);
    removeStorageKey(globalThis.sessionStorage, SESSION_STORAGE_KEY);

    localSnapshot = normalizeSnapshot({});
    sessionSnapshot = normalizeSession({});
    writeJson(globalThis.localStorage, STORAGE_KEY, localSnapshot);
    writeJson(globalThis.sessionStorage, SESSION_STORAGE_KEY, sessionSnapshot);

    const snapshot = patchBoundSignals();
    emitChange(snapshot, "local-user-reset");
    maybeReload(options);
    return snapshot;
  }

  function localDataScopes() {
    return cloneJson(LOCAL_DATA_SCOPES);
  }

  function clearLocalDataScope(scopeId, options = {}) {
    const normalizedScopeId = trimString(scopeId).toLowerCase();
    switch (normalizedScopeId) {
      case "profile-data":
        return resetLocalProfileState(options);
      case "profile-ui":
        clearSharedUiSettingsPath(UI_SETTINGS_THEME_PATH, "local-profile-ui-reset");
        maybeReload(options);
        return current();
      case "browser-data":
        return resetLocalSessionState(options);
      case "browser-ui":
        clearSharedUiSettingsExceptTheme("local-browser-ui-reset");
        maybeReload(options);
        return current();
      case "map-data":
        clearMapDataLocalState();
        maybeReload(options);
        return current();
      case "map-ui":
        clearMapUiLocalState();
        maybeReload(options);
        return current();
      case "dex-data":
        clearDexDataLocalState();
        maybeReload(options);
        return current();
      case "dex-ui":
        clearDexUiLocalState();
        maybeReload(options);
        return current();
      case "calculator-data":
        clearCalculatorDataLocalState();
        maybeReload(options);
        return current();
      case "calculator-ui":
        clearCalculatorUiLocalState();
        maybeReload(options);
        return current();
      case "presets-data":
        clearUserPresetsLocalState();
        maybeReload(options);
        return current();
      case "overrides-data":
        clearUserOverridesLocalState();
        maybeReload(options);
        return current();
      case "all":
        return clearAllLocalState(options);
      default:
        throw new Error(`Unknown local data scope: ${scopeId}`);
    }
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
    clearAllLocalState,
    clearLocalDataScope,
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
    localDataScopes,
    resetLocalProfileState,
    resetLocalSessionState,
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
