import {
  buildBridgeCommandPatchFromSignals,
  buildBridgeInputPatchFromSignals,
  normalizeMapActionState,
  projectRuntimeSnapshotToSignals,
  projectSessionSnapshotToSignals,
} from "./map-runtime-adapter.js";

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

export function mergeBridgePatches(inputPatch, commandPatch) {
  const left = isPlainObject(inputPatch) ? inputPatch : {};
  const right = isPlainObject(commandPatch) ? commandPatch : {};
  return {
    ...cloneJson(left),
    ...cloneJson(right),
    ...(left.filters || right.filters
      ? {
          filters: {
            ...(isPlainObject(left.filters) ? cloneJson(left.filters) : {}),
            ...(isPlainObject(right.filters) ? cloneJson(right.filters) : {}),
          },
        }
      : {}),
    ...(left.ui || right.ui
      ? {
          ui: {
            ...(isPlainObject(left.ui) ? cloneJson(left.ui) : {}),
            ...(isPlainObject(right.ui) ? cloneJson(right.ui) : {}),
          },
        }
      : {}),
    ...(left.commands || right.commands
      ? {
          commands: {
            ...(isPlainObject(left.commands) ? cloneJson(left.commands) : {}),
            ...(isPlainObject(right.commands) ? cloneJson(right.commands) : {}),
          },
        }
      : {}),
  };
}

export function createMapApp(options = {}) {
  let lastActionState = normalizeMapActionState(options.initialActionState);

  return {
    nextBridgePatch(signals) {
      const inputPatch = buildBridgeInputPatchFromSignals(signals);
      const commandPatch = buildBridgeCommandPatchFromSignals(signals, lastActionState);
      return commandPatch ? mergeBridgePatches(inputPatch, commandPatch) : inputPatch;
    },

    consumeSignals(signals) {
      lastActionState = normalizeMapActionState(signals?._map_actions);
      return cloneJson(lastActionState);
    },

    readLastActionState() {
      return cloneJson(lastActionState);
    },

    projectRuntimeSnapshot(snapshot) {
      return projectRuntimeSnapshotToSignals(snapshot);
    },
    projectSessionSnapshot(snapshot) {
      return projectSessionSnapshotToSignals(snapshot);
    },
  };
}
