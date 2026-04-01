export const FISHYMAP_ZONE_CATALOG_READY_EVENT = "fishymap:zone-catalog-ready";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

export function dispatchShellZoneCatalogReadyEvent(
  shell,
  zoneCatalog,
  customEventConstructor = globalThis.CustomEvent,
) {
  if (
    !shell ||
    typeof shell.dispatchEvent !== "function" ||
    typeof customEventConstructor !== "function"
  ) {
    return false;
  }
  shell.dispatchEvent(
    new customEventConstructor(FISHYMAP_ZONE_CATALOG_READY_EVENT, {
      bubbles: true,
      detail: {
        zoneCatalog: Array.isArray(zoneCatalog) ? cloneJson(zoneCatalog) : [],
      },
    }),
  );
  return true;
}
