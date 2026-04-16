import { MAP_SEARCH_LAYER_SUPPORT } from "./constants.js";

export function layerSupportsSearchTerm(layerId, termKind) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedTermKind = String(termKind ?? "").trim();
  const layerSupport = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId];
  return !!layerSupport?.termKinds?.includes(normalizedTermKind);
}

export function layerSupportsAttachmentClipMode(layerId, clipMode) {
  const normalizedLayerId = String(layerId ?? "").trim();
  const normalizedClipMode = String(clipMode ?? "").trim();
  const layerSupport = MAP_SEARCH_LAYER_SUPPORT[normalizedLayerId];
  return !!layerSupport?.attachmentClipModes?.includes(normalizedClipMode);
}
