export const FISHYMAP_SIGNAL_PATCH_EVENT = "fishymap-signals-patch";

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
  if (
    !shell ||
    typeof shell.dispatchEvent !== "function" ||
    !isPlainObject(patch) ||
    typeof customEventConstructor !== "function"
  ) {
    return false;
  }
  shell.dispatchEvent(
    new customEventConstructor(FISHYMAP_SIGNAL_PATCH_EVENT, {
      bubbles: true,
      detail: cloneJson(patch),
    }),
  );
  return true;
}
