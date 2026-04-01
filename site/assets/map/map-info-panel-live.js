import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";
import { dispatchShellSignalPatch } from "./map-signal-patch.js";
import { buildInfoViewModel, patchTouchesInfoSignals } from "./map-info-state.js";
import { loadZoneLootSummary, zoneRgbFromSelection } from "./map-zone-loot-summary.js";

const ICON_SPRITE_URL = "/img/icons.svg";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

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

function factIconMarkup(fact) {
  if (trimString(fact?.swatchRgb)) {
    return `<span class="fishymap-layer-fact-swatch" style="--fishymap-layer-fact-rgb:${escapeHtml(fact.swatchRgb)};" aria-hidden="true"></span>`;
  }
  return spriteIcon(fact?.icon || "information-circle", "size-4");
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function tabButtonMarkup(tab, activePaneId) {
  const isActive = tab.id === activePaneId;
  return `
    <button
      class="tab gap-2 ${isActive ? "tab-active" : ""}"
      type="button"
      role="tab"
      aria-selected="${isActive ? "true" : "false"}"
      data-zone-info-tab="${escapeHtml(tab.id)}"
      title="${escapeHtml(tab.summary || tab.label)}"
    >
      ${spriteIcon(tab.icon || "information-circle", "size-4")}
      <span class="truncate">${escapeHtml(tab.label)}</span>
    </button>
  `;
}

function factMarkup(fact) {
  return `
    <div class="fishymap-overview-row">
      <span class="fishymap-overview-row-icon" aria-hidden="true">${factIconMarkup(fact)}</span>
      <span class="fishymap-overview-row-label">${escapeHtml(fact.label)}</span>
      <span class="fishymap-overview-row-value">${escapeHtml(fact.value)}</span>
    </div>
  `;
}

function normalizedGradeTone(value) {
  const tone = trimString(value).toLowerCase();
  return tone || "unknown";
}

function fishIdentityMarkup(entry) {
  const name = trimString(entry?.label) || "Unknown fish";
  const gradeTone = normalizedGradeTone(entry?.iconGradeTone);
  const iconUrl = trimString(entry?.iconUrl);
  const iconMarkup = iconUrl
    ? `<span class="fishymap-item-icon-frame grade-${escapeHtml(gradeTone)} size-7"><img class="fishymap-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishymap-item-icon-frame grade-${escapeHtml(gradeTone)} size-7"><span class="fishymap-item-icon-fallback">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
  return `<span class="inline-flex min-w-0 items-center gap-2">${iconMarkup}<span class="truncate font-semibold text-base-content">${escapeHtml(name)}</span></span>`;
}

function zoneLootMetricTone(entry) {
  return {
    fillColor: trimString(entry?.fillColor) || "var(--color-base-200)",
    strokeColor: trimString(entry?.strokeColor) || "var(--color-base-300)",
    textColor: trimString(entry?.textColor) || "var(--color-base-content)",
  };
}

function zoneLootRowMarkup(entry) {
  const metric = zoneLootMetricTone(entry);
  const tooltip = trimString(entry?.dropRateTooltip);
  const dropDotColor =
    trimString(entry?.dropRateSourceKind) === "database"
      ? "var(--color-success)"
      : trimString(entry?.dropRateSourceKind) === "community"
        ? "var(--color-warning)"
        : "var(--color-info)";
  return `
    <div class="fishymap-zone-loot-row">
      <div class="fishymap-zone-loot-metric" style="--fishymap-zone-loot-fill:${escapeHtml(metric.fillColor)};--fishymap-zone-loot-stroke:${escapeHtml(metric.strokeColor)};--fishymap-zone-loot-text:${escapeHtml(metric.textColor)};">
        <div class="fishymap-zone-loot-metric-primary">${escapeHtml(entry.dropRateText || "—")}</div>
        <div class="fishymap-zone-loot-metric-secondary">${escapeHtml(entry.expectedCountText || "—")}</div>
        ${
          tooltip
            ? `<span class="fishymap-zone-loot-dot" style="--fishymap-zone-loot-dot:${escapeHtml(dropDotColor)};" aria-hidden="true" title="${escapeHtml(tooltip)}"></span>`
            : ""
        }
      </div>
      <div class="min-w-0">${fishIdentityMarkup(entry)}</div>
    </div>
  `;
}

function zoneLootSectionMarkup(section) {
  const groups = Array.isArray(section?.groups) ? section.groups : [];
  const groupMarkup = groups.length
    ? groups
        .map((group) => `
          <div class="fishymap-zone-loot-group rounded-box border border-base-300 bg-base-200/75 p-2">
            <div class="fishymap-zone-loot-group-header">
              <span class="badge badge-soft badge-sm">${escapeHtml(group.label)}</span>
              <span class="text-xs font-semibold text-base-content/65">${escapeHtml(group.countShareText)} · ${escapeHtml(group.expectedCountText)}</span>
            </div>
            <div class="fishymap-zone-loot-group-rows">
              ${group.rows.map((row) => zoneLootRowMarkup(row)).join("")}
            </div>
          </div>
        `)
        .join("")
    : '<div class="px-2 py-3 text-xs text-base-content/60">No fish rows are available for this zone yet.</div>';
  return `
    <section class="space-y-2">
      <div class="flex items-center justify-between gap-3">
        <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(section.title || "Fish")}</p>
        <span class="text-[11px] text-base-content/55">${escapeHtml(section.statusText || "zone loot: idle")}</span>
      </div>
      <p class="text-xs text-base-content/70">${escapeHtml(section.summary || "")}</p>
      ${
        trimString(section.note)
          ? `<div class="rounded-box border border-warning/35 bg-warning/10 px-3 py-2 text-xs text-base-content/80">${escapeHtml(section.note)}</div>`
          : ""
      }
      <div class="fishymap-zone-loot-groups">${groupMarkup}</div>
    </section>
  `;
}

function sectionMarkup(section) {
  switch (trimString(section?.kind)) {
    case "facts":
      return `
        <section class="space-y-2">
          ${
            trimString(section?.title)
              ? `<p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(section.title)}</p>`
              : ""
          }
          <div class="fishymap-overview-list">${(Array.isArray(section?.facts) ? section.facts : [])
            .map((fact) => factMarkup(fact))
            .join("")}</div>
        </section>
      `;
    case "zone-loot":
      return zoneLootSectionMarkup(section);
    default:
      return "";
  }
}

function emptyPanelMarkup() {
  return '<div class="rounded-box border border-base-300/70 bg-base-200 px-3 py-3 text-sm text-base-content/60">Click the map, use a waypoint target, or select a bookmark to inspect facts at a world point.</div>';
}

export function createMapInfoPanelController({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  documentRef = globalThis.document,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  listenToSignalPatches = true,
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapInfoPanelController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapInfoPanelController requires getSignals()");
  }

  const elements = {
    title: shell.querySelector("#fishymap-zone-info-title"),
    titleIcon: shell.querySelector("#fishymap-zone-info-title-icon"),
    statusIcon: shell.querySelector("#fishymap-zone-info-status-icon"),
    statusText: shell.querySelector("#fishymap-zone-info-status-text"),
    tabs: shell.querySelector("#fishymap-zone-info-tabs"),
    panel: shell.querySelector("#fishymap-zone-info-panel"),
  };
  if (!(elements.tabs instanceof HTMLElement) || !(elements.panel instanceof HTMLElement)) {
    throw new Error("createMapInfoPanelController requires info panel elements");
  }

  const state = {
    frameId: 0,
    zoneCatalog: [],
    zoneLootStatus: "idle",
    zoneLootSummary: null,
    zoneLootRgb: null,
    zoneLootRequestToken: 0,
  };

  function signals() {
    return getSignals() || null;
  }

  function render() {
    state.frameId = 0;
    const viewModel = buildInfoViewModel(signals(), {
      zoneCatalog: state.zoneCatalog,
      zoneLootSummary: state.zoneLootSummary,
      zoneLootStatus: state.zoneLootStatus,
    });
    setTextContent(elements.title, viewModel.descriptor.title);
    setTextContent(elements.statusText, viewModel.descriptor.statusText);
    setMarkup(
      elements.titleIcon,
      viewModel.descriptor.titleIcon,
      spriteIcon(viewModel.descriptor.titleIcon || "information-circle", "size-5"),
    );
    setMarkup(
      elements.statusIcon,
      viewModel.descriptor.statusIcon,
      spriteIcon(viewModel.descriptor.statusIcon || "information-circle", "size-4"),
    );
    setBooleanProperty(elements.tabs, "hidden", viewModel.panes.length === 0);
    setMarkup(
      elements.tabs,
      JSON.stringify(viewModel.panes.map((pane) => [pane.id, pane.label, pane.id === viewModel.activePaneId ? 1 : 0])),
      viewModel.panes.map((pane) => tabButtonMarkup(pane, viewModel.activePaneId)).join(""),
    );
    setMarkup(
      elements.panel,
      JSON.stringify({
        empty: viewModel.empty,
        activePaneId: viewModel.activePaneId,
        sections: viewModel.activePane?.sections || [],
      }),
      viewModel.empty
        ? emptyPanelMarkup()
        : `<section class="space-y-3">${(viewModel.activePane?.sections || []).map((section) => sectionMarkup(section)).join("")}</section>`,
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
    if (!patchTouchesInfoSignals(event?.detail)) {
      return;
    }
    if (event?.detail?._map_runtime?.selection != null) {
      void refreshZoneLootSummary();
    }
    scheduleRender();
  }

  async function refreshZoneLootSummary() {
    const selection = signals()?._map_runtime?.selection || null;
    const zoneRgb = zoneRgbFromSelection(selection);
    if (!Number.isInteger(zoneRgb) || zoneRgb < 0) {
      state.zoneLootRequestToken += 1;
      state.zoneLootRgb = null;
      state.zoneLootStatus = "idle";
      state.zoneLootSummary = null;
      scheduleRender();
      return;
    }
    if (
      state.zoneLootRgb === zoneRgb &&
      (state.zoneLootStatus === "loading" ||
        state.zoneLootStatus === "loaded" ||
        state.zoneLootStatus === "error")
    ) {
      return;
    }
    state.zoneLootRgb = zoneRgb;
    state.zoneLootStatus = "loading";
    state.zoneLootSummary = null;
    scheduleRender();

    const requestToken = state.zoneLootRequestToken + 1;
    state.zoneLootRequestToken = requestToken;
    try {
      const summary = await loadZoneLootSummary(zoneRgb);
      if (state.zoneLootRequestToken !== requestToken || state.zoneLootRgb !== zoneRgb) {
        return;
      }
      state.zoneLootSummary = summary;
      state.zoneLootStatus = "loaded";
    } catch (error) {
      if (state.zoneLootRequestToken !== requestToken || state.zoneLootRgb !== zoneRgb) {
        return;
      }
      state.zoneLootSummary = {
        available: false,
        zoneName: "",
        profileLabel: "",
        note: trimString(error?.message) || "Zone loot summary is unavailable.",
        groups: [],
        speciesRows: [],
      };
      state.zoneLootStatus = "error";
    }
    scheduleRender();
  }

  elements.tabs.addEventListener("click", (event) => {
    const button = event.target.closest("button[data-zone-info-tab]");
    if (!button) {
      return;
    }
    const paneId = trimString(button.getAttribute("data-zone-info-tab"));
    dispatchPatch(shell, {
      _map_ui: {
        windowUi: {
          zoneInfo: {
            tab: paneId,
          },
        },
      },
    });
    scheduleRender();
  });

  if (listenToSignalPatches) {
    documentRef?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
  }

  return Object.freeze({
    render,
    scheduleRender,
    setZoneCatalog(nextZoneCatalog) {
      state.zoneCatalog = Array.isArray(nextZoneCatalog) ? cloneJson(nextZoneCatalog) : [];
      scheduleRender();
    },
    refreshZoneLootSummary,
  });
}
