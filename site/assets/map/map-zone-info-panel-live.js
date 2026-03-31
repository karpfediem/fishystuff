import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";
import { dispatchShellSignalPatch } from "./map-signal-patch.js";
import {
  buildZoneInfoViewModel,
  patchTouchesZoneInfoSignals,
} from "./map-zone-info-state.js";

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

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  element[propertyName] = Boolean(value);
}

function setTextContent(element, text) {
  if (!element) {
    return;
  }
  element.textContent = String(text ?? "");
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

function spriteIcon(name, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${name}"></use></svg>`;
}

function tabButtonMarkup(tab, activeTabId) {
  const isActive = tab.id === activeTabId;
  return `
    <button
      class="tab gap-2 ${isActive ? "tab-active" : ""}"
      type="button"
      role="tab"
      aria-selected="${isActive ? "true" : "false"}"
      data-zone-info-tab="${escapeHtml(tab.id)}"
    >
      ${spriteIcon("information-circle", "size-4")}
      <span class="truncate">${escapeHtml(tab.label)}</span>
    </button>
  `;
}

function factMarkup(fact) {
  return `
    <div class="fishymap-overview-row">
      <span class="fishymap-overview-row-icon" aria-hidden="true">${spriteIcon(fact.icon || "information-circle", "size-4")}</span>
      <span class="fishymap-overview-row-label">${escapeHtml(fact.label)}</span>
      <span class="fishymap-overview-row-value">${escapeHtml(fact.value)}</span>
    </div>
  `;
}

function emptyPanelMarkup() {
  return '<div class="rounded-box border border-base-300/70 bg-base-200 px-3 py-3 text-sm text-base-content/60">Click the map, use a waypoint target, or select a bookmark to inspect layers at a world point.</div>';
}

export function createMapZoneInfoPanelController({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  documentRef = globalThis.document,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapZoneInfoPanelController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapZoneInfoPanelController requires getSignals()");
  }

  const elements = {
    zoneInfoTitle: shell.querySelector("#fishymap-zone-info-title"),
    zoneInfoTitleIcon: shell.querySelector("#fishymap-zone-info-title-icon"),
    zoneInfoStatusIcon: shell.querySelector("#fishymap-zone-info-status-icon"),
    zoneInfoStatusText: shell.querySelector("#fishymap-zone-info-status-text"),
    zoneInfoTabs: shell.querySelector("#fishymap-zone-info-tabs"),
    zoneInfoPanel: shell.querySelector("#fishymap-zone-info-panel"),
  };
  if (!(elements.zoneInfoPanel instanceof HTMLElement) || !(elements.zoneInfoTabs instanceof HTMLElement)) {
    throw new Error("createMapZoneInfoPanelController requires zone info elements");
  }

  const state = {
    frameId: 0,
  };

  function signals() {
    return getSignals() || null;
  }

  function render() {
    state.frameId = 0;
    const viewModel = buildZoneInfoViewModel(signals());
    setTextContent(elements.zoneInfoTitle, viewModel.descriptor.title);
    setTextContent(elements.zoneInfoStatusText, viewModel.descriptor.statusText);
    setMarkup(
      elements.zoneInfoTitleIcon,
      viewModel.descriptor.titleIcon,
      spriteIcon(viewModel.descriptor.titleIcon || "information-circle", "size-5"),
    );
    setMarkup(
      elements.zoneInfoStatusIcon,
      viewModel.descriptor.statusIcon,
      spriteIcon(viewModel.descriptor.statusIcon || "information-circle", "size-4"),
    );
    setBooleanProperty(elements.zoneInfoTabs, "hidden", viewModel.tabs.length === 0);
    setMarkup(
      elements.zoneInfoTabs,
      JSON.stringify(viewModel.tabs.map((tab) => [tab.id, tab.label, tab.id === viewModel.activeTabId ? 1 : 0])),
      viewModel.tabs.map((tab) => tabButtonMarkup(tab, viewModel.activeTabId)).join(""),
    );
    setMarkup(
      elements.zoneInfoPanel,
      JSON.stringify({
        empty: viewModel.empty,
        activeTabId: viewModel.activeTabId,
        facts: viewModel.facts,
      }),
      viewModel.empty
        ? emptyPanelMarkup()
        : `<section class="space-y-2"><div class="fishymap-overview-list">${viewModel.facts.map((fact) => factMarkup(fact)).join("")}</div></section>`,
    );
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

  function handleSignalPatch(event) {
    if (!patchTouchesZoneInfoSignals(event?.detail)) {
      return;
    }
    scheduleRender();
  }

  elements.zoneInfoTabs.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-zone-info-tab]");
    if (!button) {
      return;
    }
    const tabId = String(button.getAttribute("data-zone-info-tab") || "").trim();
    dispatchPatch(shell, {
      _map_ui: {
        windowUi: {
          zoneInfo: {
            tab: tabId,
          },
        },
      },
    });
    scheduleRender();
  });

  documentRef?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);

  return Object.freeze({
    render,
    scheduleRender,
  });
}
