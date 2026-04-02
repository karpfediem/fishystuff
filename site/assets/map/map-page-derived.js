import { parseQuerySignalPatch } from "./map-query-state.js";
import { buildSearchProjectionSignalPatch } from "./map-search-projection.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function mergeProjectionPatch(target, patch) {
  if (!isPlainObject(target) || !isPlainObject(patch)) {
    return target;
  }
  for (const [key, value] of Object.entries(patch)) {
    if (Array.isArray(value)) {
      target[key] = cloneJson(value);
      continue;
    }
    if (isPlainObject(value)) {
      const nextTarget = isPlainObject(target[key]) ? target[key] : {};
      target[key] = nextTarget;
      mergeProjectionPatch(nextTarget, value);
      continue;
    }
    target[key] = value;
  }
  return target;
}

function currentLocationHref(globalRef = globalThis) {
  return globalRef.location?.href || globalRef.window?.location?.href || "";
}

export function buildSearchProjectionPatchForSignalPatch(signals, patch) {
  if (patch?._map_ui?.search?.selectedTerms == null) {
    return null;
  }
  const nextSignals = isPlainObject(signals) ? cloneJson(signals) : {};
  mergeProjectionPatch(nextSignals, patch);
  return buildSearchProjectionSignalPatch(nextSignals);
}

export function createMapPageDerivedController({
  globalRef = globalThis,
  shell = null,
  readSignals = () => null,
  dispatchPatch = () => {},
} = {}) {
  let boundShell = null;

  function handleSignalPatch(eventOrPatch) {
    const patch = eventOrPatch?.detail ?? eventOrPatch;
    const projectionPatch = buildSearchProjectionPatchForSignalPatch(readSignals(), patch);
    if (!projectionPatch) {
      return false;
    }
    dispatchPatch(projectionPatch);
    return true;
  }

  function applyInitialPatches(locationHref = currentLocationHref(globalRef)) {
    const queryPatch = parseQuerySignalPatch(locationHref);
    if (queryPatch) {
      dispatchPatch(queryPatch);
    }
    const projectionPatch = buildSearchProjectionSignalPatch(readSignals() || {});
    if (projectionPatch) {
      dispatchPatch(projectionPatch);
    }
    return Object.freeze({
      queryPatch,
      projectionPatch,
    });
  }

  function start(nextShell = shell) {
    const target = nextShell && typeof nextShell.addEventListener === "function" ? nextShell : null;
    if (!target) {
      return false;
    }
    if (boundShell && boundShell !== target && typeof boundShell.removeEventListener === "function") {
      boundShell.removeEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, handleSignalPatch);
    }
    if (boundShell === target) {
      return true;
    }
    target.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, handleSignalPatch);
    boundShell = target;
    return true;
  }

  return Object.freeze({
    applyInitialPatches,
    handleSignalPatch,
    start,
  });
}
