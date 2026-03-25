import { writeFile } from "node:fs/promises";

import { icons as mingcuteIcons } from "@iconify-json/mingcute";
import { getIconData, iconToSVG, replaceIDs } from "@iconify/utils";

const heroiconsOverrides = {
  "check-badge-outline": {
    body: `<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12.75 11.25 15 15 9.75M21 12c0 1.268-.63 2.39-1.593 3.068a3.745 3.745 0 0 1-1.043 3.296 3.745 3.745 0 0 1-3.296 1.043A3.745 3.745 0 0 1 12 21c-1.268 0-2.39-.63-3.068-1.593a3.746 3.746 0 0 1-3.296-1.043 3.745 3.745 0 0 1-1.043-3.296A3.745 3.745 0 0 1 3 12c0-1.268.63-2.39 1.593-3.068a3.745 3.745 0 0 1 1.043-3.296 3.746 3.746 0 0 1 3.296-1.043A3.746 3.746 0 0 1 12 3c1.268 0 2.39.63 3.068 1.593a3.746 3.746 0 0 1 3.296 1.043 3.746 3.746 0 0 1 1.043 3.296A3.745 3.745 0 0 1 21 12Z"/>`,
    width: 24,
    height: 24,
  },
  "check-badge-solid": {
    body: `<path fill="currentColor" fill-rule="evenodd" clip-rule="evenodd" d="M8.603 3.799A4.49 4.49 0 0 1 12 2.25c1.357 0 2.573.6 3.397 1.549a4.49 4.49 0 0 1 3.498 1.307 4.491 4.491 0 0 1 1.307 3.497A4.49 4.49 0 0 1 21.75 12a4.49 4.49 0 0 1-1.549 3.397 4.491 4.491 0 0 1-1.307 3.497 4.491 4.491 0 0 1-3.497 1.307A4.49 4.49 0 0 1 12 21.75a4.49 4.49 0 0 1-3.397-1.549 4.49 4.49 0 0 1-3.498-1.306 4.491 4.491 0 0 1-1.307-3.498A4.49 4.49 0 0 1 2.25 12c0-1.357.6-2.573 1.549-3.397a4.49 4.49 0 0 1 1.307-3.497 4.49 4.49 0 0 1 3.497-1.307Zm7.007 6.387a.75.75 0 1 0-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.14-.094l3.75-5.25Z"/>`,
    width: 24,
    height: 24,
  },
};

const iconSources = {
  menu: { type: "mingcute", name: "menu-fill" },
  "caret-down": { type: "mingcute", name: "down-fill" },
  "academic-cap": { type: "mingcute", name: "hat-2-fill" },
  "map-pin": { type: "mingcute", name: "map-pin-fill" },
  "magnifying-glass": { type: "mingcute", name: "search-fill" },
  link: { type: "mingcute", name: "link-fill" },
  "share-nodes": { type: "mingcute", name: "share-2-fill" },
  "x-circle": { type: "mingcute", name: "close-circle-fill" },
  "book-open": { type: "mingcute", name: "book-2-fill" },
  "nav-guides": { type: "mingcute", name: "book-2-fill" },
  "nav-map": { type: "mingcute", name: "map-pin-fill" },
  "nav-dex": { type: "mingcute", name: "fish-fill" },
  "nav-calculator": { type: "mingcute", name: "chart-line-fill" },
  "nav-community": { type: "mingcute", name: "group-3-fill" },
  "nav-log": { type: "mingcute", name: "paper-fill" },
  "theme-palette": { type: "mingcute", name: "palette-2-fill" },
  "adjustments-horizontal": { type: "mingcute", name: "settings-3-fill" },
  "information-circle": { type: "mingcute", name: "live-location-fill" },
  "squares-2x2": { type: "mingcute", name: "layers-fill" },
  "search-field": { type: "mingcute", name: "search-fill" },
  "map-view": { type: "mingcute", name: "map-fill" },
  "cube-view": { type: "mingcute", name: "cube-3d-fill" },
  "hover-zone": { type: "mingcute", name: "fullscreen-line" },
  crosshair: { type: "mingcute", name: "aiming-2-line" },
  "hover-origin": { type: "mingcute", name: "map-pin-fill" },
  "trade-origin": { type: "mingcute", name: "wheel-fill" },
  "hover-resources": { type: "mingcute", name: "chart-bar-fill" },
  "question-mark": { type: "mingcute", name: "question-2-fill" },
  bookmark: { type: "mingcute", name: "bookmark-fill" },
  bookmarks: { type: "mingcute", name: "bookmarks-fill" },
  "bookmark-add": { type: "mingcute", name: "bookmark-add-fill" },
  "bookmark-edit": { type: "mingcute", name: "bookmark-edit-fill" },
  copy: { type: "mingcute", name: "copy-2-fill" },
  export: { type: "mingcute", name: "download-3-fill" },
  import: { type: "mingcute", name: "upload-3-fill" },
  "select-all": { type: "mingcute", name: "check-2-fill" },
  clear: { type: "mingcute", name: "close-circle-fill" },
  trash: { type: "mingcute", name: "delete-2-fill" },
  eye: { type: "mingcute", name: "eye-fill" },
  "eye-slash": { type: "mingcute", name: "eye-close-fill" },
  "drag-handle": { type: "mingcute", name: "move-fill" },
  "settings-1": { type: "mingcute", name: "settings-1-fill" },
  "check-badge-outline": { type: "custom", name: "check-badge-outline" },
  "check-badge-solid": { type: "custom", name: "check-badge-solid" },
  "coin-stack": { type: "mingcute", name: "coin-2-fill" },
};

function resolveIconData(alias) {
  const source = iconSources[alias];
  if (!source) {
    throw new Error(`Missing icon source for "${alias}"`);
  }

  if (source.type === "custom") {
    const data = heroiconsOverrides[source.name];
    if (!data) {
      throw new Error(`Missing custom icon data for "${source.name}"`);
    }
    return data;
  }

  const data = getIconData(mingcuteIcons, source.name);
  if (!data) {
    throw new Error(`Missing MingCute icon "${source.name}" for alias "${alias}"`);
  }
  return data;
}

function buildSymbol(alias) {
  const data = resolveIconData(alias);
  const rendered = iconToSVG(data, {
    width: "unset",
    height: "unset",
  });

  return [
    `  <symbol id="fishy-${alias}" viewBox="${rendered.attributes.viewBox}">`,
    `    ${replaceIDs(rendered.body, `fishy-${alias}-`)}`,
    "  </symbol>",
  ].join("\n");
}

const sprite = [
  '<svg xmlns="http://www.w3.org/2000/svg">',
  ...Object.keys(iconSources).map(buildSymbol),
  "</svg>",
  "",
].join("\n");

await writeFile(new URL("../assets/img/icons.svg", import.meta.url), sprite, "utf8");
