export const FISHYMAP_LIVE_INIT_EVENT = "fishymap-live-init";

export function resolveMapPageShell(globalRef = globalThis) {
  const shell = globalRef.document?.getElementById?.("map-page-shell");
  return shell && typeof shell.dispatchEvent === "function" ? shell : null;
}

export function readMapShellSignals(shell) {
  if (!shell || typeof shell !== "object") {
    return null;
  }
  const liveSignals = shell.__fishymapLiveSignals;
  if (liveSignals && typeof liveSignals === "object") {
    return liveSignals;
  }
  const initialSignals = shell.__fishymapInitialSignals;
  return initialSignals && typeof initialSignals === "object" ? initialSignals : null;
}

export function clearInitialMapShellSignals(shell) {
  if (!shell || typeof shell !== "object" || !("__fishymapInitialSignals" in shell)) {
    return false;
  }
  delete shell.__fishymapInitialSignals;
  return true;
}

export function consumeInitialMapShellSignals(shell) {
  const signals =
    shell && typeof shell === "object" ? shell.__fishymapInitialSignals : null;
  if (!signals || typeof signals !== "object") {
    clearInitialMapShellSignals(shell);
    return null;
  }
  clearInitialMapShellSignals(shell);
  return signals;
}

export function writeMapShellLiveSignals(shell, signals) {
  if (!shell || typeof shell !== "object") {
    return null;
  }
  shell.__fishymapLiveSignals = signals && typeof signals === "object" ? signals : null;
  return shell.__fishymapLiveSignals;
}
