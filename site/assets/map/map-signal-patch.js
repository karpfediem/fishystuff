export const FISHYMAP_SIGNAL_PATCH_EVENT = "fishymap-signals-patch";
export const FISHYMAP_SIGNAL_PATCHED_EVENT = "fishymap:signal-patched";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

export function combineSignalPatches(...patches) {
  const next = {};
  for (const patch of patches) {
    if (!isPlainObject(patch)) {
      continue;
    }
    Object.assign(next, cloneJson(patch));
  }
  return next;
}

export function dispatchShellSignalPatch(
  shell,
  patch,
  customEventConstructor = globalThis.CustomEvent,
) {
  return dispatchShellCustomPatchEvent(
    shell,
    FISHYMAP_SIGNAL_PATCH_EVENT,
    patch,
    customEventConstructor,
  );
}

export function dispatchShellPatchedSignalEvent(
  shell,
  patch,
  customEventConstructor = globalThis.CustomEvent,
) {
  return dispatchShellCustomPatchEvent(
    shell,
    FISHYMAP_SIGNAL_PATCHED_EVENT,
    patch,
    customEventConstructor,
  );
}

function dispatchShellCustomPatchEvent(
  shell,
  eventType,
  patch,
  customEventConstructor = globalThis.CustomEvent,
) {
  if (
    !shell ||
    typeof shell.dispatchEvent !== "function" ||
    !isPlainObject(patch) ||
    typeof customEventConstructor !== "function"
  ) {
    return false;
  }
  shell.dispatchEvent(
    new customEventConstructor(eventType, {
      bubbles: true,
      detail: cloneJson(patch),
    }),
  );
  return true;
}
