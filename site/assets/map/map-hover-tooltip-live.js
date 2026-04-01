import {
  buildHoverTooltipRows,
  patchTouchesHoverTooltipSignals,
} from "./map-hover-facts.js";
import { FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";

const ICON_SPRITE_URL = "/img/icons.svg";

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (char) =>
      (
        {
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&#39;",
        }[char] || char
      ),
  );
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  element[propertyName] = Boolean(value);
}

function setMarkup(element, renderKey, markup) {
  if (!element) {
    return;
  }
  const normalizedKey = String(renderKey ?? "");
  if (element.dataset.renderKey === normalizedKey) {
    return;
  }
  element.dataset.renderKey = normalizedKey;
  element.innerHTML = String(markup ?? "");
}

function spriteIcon(name, sizeClass = "size-4") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${escapeHtml(name)}"></use></svg>`;
}

function overviewRowMarkup(row) {
  const baseIcon = trimString(row?.icon || "information-circle");
  const swatchRgb = trimString(row?.swatchRgb);
  const statusIcon = trimString(row?.statusIcon);
  const statusIconTone = trimString(row?.statusIconTone);
  return `
    <div class="fishymap-overview-row">
      <span class="fishymap-overview-row-icon" aria-hidden="true">
        ${
          swatchRgb
            ? `<span class="fishymap-layer-fact-swatch" style="--fishymap-layer-fact-rgb:${escapeHtml(swatchRgb)};"></span>`
            : spriteIcon(baseIcon, "size-4")
        }
      </span>
      <span class="fishymap-overview-row-label">${escapeHtml(row?.label || "")}</span>
      <span class="fishymap-overview-row-value">
        ${escapeHtml(row?.value || "")}
        ${
          statusIcon
            ? `<span class="fishymap-overview-status ${
                statusIconTone === "subtle" ? "fishymap-overview-status--subtle" : ""
              }" aria-hidden="true">${spriteIcon(statusIcon, "size-4")}</span>`
            : ""
        }
      </span>
    </div>
  `;
}

function buildStateBundle(signals) {
  return {
    state: {
      catalog: {
        layers: Array.isArray(signals?._map_runtime?.catalog?.layers)
          ? cloneJson(signals._map_runtime.catalog.layers)
          : [],
      },
    },
    inputState: {
      filters: isPlainObject(signals?._map_bridged?.filters)
        ? cloneJson(signals._map_bridged.filters)
        : {},
    },
  };
}

function normalizeHoverEventDetail(detail) {
  if (isPlainObject(detail?.hover)) {
    return cloneJson(detail.hover);
  }
  return isPlainObject(detail) ? cloneJson(detail) : {};
}

export function createMapHoverTooltipController({
  shell,
  getSignals,
  canvas = shell?.querySelector?.("#bevy") || globalThis.document?.getElementById?.("bevy"),
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapHoverTooltipController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapHoverTooltipController requires getSignals()");
  }

  const elements = {
    hoverTooltip: shell.querySelector("#fishymap-hover-tooltip"),
    hoverLayers: shell.querySelector("#fishymap-hover-layers"),
  };
  if (!(elements.hoverTooltip instanceof HTMLElement) || !(elements.hoverLayers instanceof HTMLElement)) {
    throw new Error("createMapHoverTooltipController requires hover tooltip elements");
  }

  const state = {
    frameId: 0,
    pointerActive: false,
    hover: null,
  };
  let currentZoneCatalog = [];

  function signals() {
    return getSignals() || null;
  }

  function writePointerPosition(clientX, clientY) {
    elements.hoverTooltip.style.setProperty("--fishymap-hover-x", `${Math.round(clientX)}px`);
    elements.hoverTooltip.style.setProperty("--fishymap-hover-y", `${Math.round(clientY)}px`);
  }

  function render() {
    state.frameId = 0;
    const rows = buildHoverTooltipRows({
      hover: state.hover,
      stateBundle: buildStateBundle(signals()),
      visibilityByLayer: signals()?._map_ui?.layers?.hoverFactsVisibleByLayer || {},
      zoneCatalog: currentZoneCatalog,
    });
    if (!state.pointerActive || rows.length === 0) {
      setMarkup(elements.hoverLayers, "[]", "");
      setBooleanProperty(elements.hoverLayers, "hidden", true);
      setBooleanProperty(elements.hoverTooltip, "hidden", true);
      return;
    }
    setMarkup(
      elements.hoverLayers,
      JSON.stringify(rows.map((row) => [row.layerId, row.key, row.value])),
      rows.map((row) => overviewRowMarkup(row)).join(""),
    );
    setBooleanProperty(elements.hoverLayers, "hidden", false);
    setBooleanProperty(elements.hoverTooltip, "hidden", false);
  }

  function scheduleRender() {
    if (state.frameId) {
      return;
    }
    if (typeof requestAnimationFrameImpl === "function") {
      state.frameId = requestAnimationFrameImpl(() => {
        render();
      });
      return;
    }
    render();
  }

  canvas?.addEventListener?.("pointermove", (event) => {
    state.pointerActive = true;
    writePointerPosition(event.clientX, event.clientY);
    if (elements.hoverTooltip.hidden) {
      scheduleRender();
    }
  });

  canvas?.addEventListener?.("pointerleave", () => {
    state.pointerActive = false;
    setBooleanProperty(elements.hoverLayers, "hidden", true);
    setBooleanProperty(elements.hoverTooltip, "hidden", true);
  });

  shell.addEventListener("fishymap:hover-changed", (event) => {
    state.hover = normalizeHoverEventDetail(event?.detail);
    scheduleRender();
  });
  shell.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, (event) => {
    if (!patchTouchesHoverTooltipSignals(event?.detail)) {
      return;
    }
    scheduleRender();
  });
  shell.addEventListener(FISHYMAP_ZONE_CATALOG_READY_EVENT, (event) => {
    currentZoneCatalog = Array.isArray(event?.detail?.zoneCatalog)
      ? cloneJson(event.detail.zoneCatalog)
      : [];
    scheduleRender();
  });

  render();

  return Object.freeze({
    render,
    scheduleRender,
  });
}
