const DEFAULT_FOCUS_VIEW_MODE = "2d";

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizeHistoryBehavior(value) {
  const normalized = trimString(value).toLowerCase();
  return normalized === "navigate" ? "navigate" : "append";
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

export function buildFocusWorldPointSignalPatch(focusWorldPoint, signals = {}) {
  const worldX = Number(focusWorldPoint?.worldX);
  const worldZ = Number(focusWorldPoint?.worldZ);
  if (!Number.isFinite(worldX) || !Number.isFinite(worldZ)) {
    return null;
  }
  const currentToken = Number(signals?._map_actions?.focusWorldPointToken || 0);
  const currentView = isPlainObject(signals?._map_session?.view)
    ? signals._map_session.view
    : {};
  const currentCamera = isPlainObject(currentView.camera) ? currentView.camera : {};
  const viewMode = trimString(currentView.viewMode) || DEFAULT_FOCUS_VIEW_MODE;
  return {
    _map_actions: {
      focusWorldPoint: {
        elementKind: trimString(focusWorldPoint?.elementKind),
        worldX,
        worldZ,
        pointKind: trimString(focusWorldPoint?.pointKind),
        pointLabel: trimString(focusWorldPoint?.pointLabel),
        historyBehavior: normalizeHistoryBehavior(focusWorldPoint?.historyBehavior),
      },
      focusWorldPointToken: currentToken + 1,
    },
    _map_session: {
      view: {
        viewMode,
        camera: {
          ...cloneJson(currentCamera),
          centerWorldX: worldX,
          centerWorldZ: worldZ,
        },
      },
    },
  };
}
