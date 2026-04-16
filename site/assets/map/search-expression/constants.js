export const FISH_FILTER_TERM_ORDER = Object.freeze([
  "favourite",
  "missing",
  "red",
  "yellow",
  "blue",
  "green",
  "white",
]);

export const DEFAULT_SEARCH_EXPRESSION_OPERATOR = "or";
export const EMPTY_SEARCH_EXPRESSION = Object.freeze({
  type: "group",
  operator: DEFAULT_SEARCH_EXPRESSION_OPERATOR,
  children: Object.freeze([]),
});

export const MAP_SEARCH_LAYER_SUPPORT = Object.freeze({
  bookmarks: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze([]),
  }),
  fish_evidence: Object.freeze({
    termKinds: Object.freeze(["fish", "fish-filter", "zone"]),
    attachmentClipModes: Object.freeze(["zone-membership"]),
  }),
  minimap: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  node_waypoints: Object.freeze({
    termKinds: Object.freeze([]),
    attachmentClipModes: Object.freeze([]),
  }),
  region_groups: Object.freeze({
    termKinds: Object.freeze(["semantic"]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  regions: Object.freeze({
    termKinds: Object.freeze(["semantic"]),
    attachmentClipModes: Object.freeze(["mask-sample"]),
  }),
  zone_mask: Object.freeze({
    termKinds: Object.freeze(["fish", "fish-filter", "zone"]),
    attachmentClipModes: Object.freeze([]),
  }),
});
