import { dispatchShellSignalPatch, FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { buildInfoViewModel, patchTouchesInfoSignals } from "./map-info-state.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";
import { loadZoneLootSummary, zoneRgbFromSelection } from "./map-zone-loot-summary.js";
import {
  attachProvenanceTooltip,
  buildProvenanceSegments,
  provenanceAriaLabel,
} from "../js/components/provenance-indicator.js";

const INFO_PANEL_TAG_NAME = "fishymap-info-panel";
const FISHYMAP_LIVE_INIT_EVENT = "fishymap-live-init";
const ICON_SPRITE_URL = "/img/icons.svg";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>\"']/g,
    (char) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[char] || char,
  );
}

function readMapShellSignals(shell) {
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

export function readMapInfoPanelShellSignals(shell) {
  return readMapShellSignals(shell);
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

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function factIconMarkup(fact) {
  if (trimString(fact?.swatchRgb)) {
    return `<span class="fishymap-layer-fact-swatch" style="--fishymap-layer-fact-rgb:${escapeHtml(fact.swatchRgb)};" aria-hidden="true"></span>`;
  }
  return spriteIcon(fact?.icon || "information-circle", "size-4");
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

function itemGradeTone(grade, isPrize) {
  const resolver = globalThis.window?.__fishystuffItemPresentation?.resolveGradeTone;
  if (typeof resolver === "function") {
    return resolver(grade, isPrize);
  }
  const normalized = trimString(grade).toLowerCase();
  if (isPrize === true || normalized === "prize" || normalized === "red") {
    return "red";
  }
  switch (normalized) {
    case "rare":
    case "yellow":
      return "yellow";
    case "highquality":
    case "high_quality":
    case "high-quality":
    case "blue":
      return "blue";
    case "general":
    case "green":
      return "green";
    case "trash":
    case "white":
      return "white";
    default:
      return "unknown";
  }
}

function fishIdentityMarkup(entry, accessoryMarkup = "") {
  const name = trimString(entry?.label) || "Unknown fish";
  const gradeTone = itemGradeTone(entry?.iconGradeTone, entry?.isPrize === true);
  const toneClass = `fishy-item-grade-${escapeHtml(gradeTone)}`;
  const iconUrl = trimString(entry?.iconUrl);
  const iconMarkup = iconUrl
    ? `<span class="fishy-item-icon-frame is-sm ${toneClass}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishy-item-icon-frame is-sm ${toneClass}"><span class="fishy-item-icon-fallback ${toneClass}">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
  return `<span class="fishy-item-row fishy-item-row--surface fishymap-zone-loot-item-surface ${toneClass}">${iconMarkup}<span class="fishy-item-label fishymap-zone-loot-item-label truncate">${escapeHtml(name)}</span>${accessoryMarkup}</span>`;
}

function zoneLootMetricTone(entry) {
  return {
    fillColor: trimString(entry?.fillColor) || "var(--color-base-200)",
    strokeColor: trimString(entry?.strokeColor) || "var(--color-base-300)",
    textColor: trimString(entry?.textColor) || "var(--color-base-content)",
  };
}

function provenanceRailMarkup(entry) {
  const segments = buildProvenanceSegments({
    rateSourceKind: trimString(entry?.dropRateSourceKind),
    rateDetail: trimString(entry?.dropRateTooltip),
    rateValueText: trimString(entry?.dropRateText),
    presenceSourceKind: trimString(entry?.presenceSourceKind),
    presenceDetail: trimString(entry?.presenceTooltip),
    presenceValueText: trimString(entry?.presenceText),
  });
  return `
    <div class="fishy-provenance-rail" aria-label="Fact provenance">
      ${segments
        .map(
          (segment) => `
            <span
              class="fishy-provenance-rail__segment${segment.active ? "" : " is-inactive"}"
              style="--fishy-provenance-color:${escapeHtml(segment.color)};"
              tabindex="0"
              aria-label="${escapeHtml(provenanceAriaLabel(segment))}"
              data-fishy-provenance-label="${escapeHtml(segment.label)}"
              data-fishy-provenance-source="${escapeHtml(segment.sourceLabel)}"
              data-fishy-provenance-detail="${escapeHtml(segment.detail)}"
              data-fishy-provenance-color="${escapeHtml(segment.color)}"
            ></span>
          `,
        )
        .join("")}
    </div>
  `;
}

function zoneLootRowMarkup(entry) {
  const metric = zoneLootMetricTone(entry);
  const provenanceRail = provenanceRailMarkup(entry);
  return `
    <div class="fishymap-zone-loot-row">
      <div class="fishymap-zone-loot-metric" style="--fishymap-zone-loot-fill:${escapeHtml(metric.fillColor)};--fishymap-zone-loot-stroke:${escapeHtml(metric.strokeColor)};--fishymap-zone-loot-text:${escapeHtml(metric.textColor)};">
        <div class="fishymap-zone-loot-metric-primary">${escapeHtml(entry.dropRateText || "—")}</div>
      </div>
      <div class="fishymap-zone-loot-item">
        ${fishIdentityMarkup(entry, provenanceRail)}
      </div>
    </div>
  `;
}

function zoneLootGroupHeaderMarkup(group) {
  const metric = zoneLootMetricTone(group);
  const provenanceRail = provenanceRailMarkup(group);
  return `
    <div class="fishymap-zone-loot-group-header">
      <span class="badge badge-soft badge-sm">${escapeHtml(group.label)}</span>
      <div class="fishymap-zone-loot-group-rate">
        <div class="fishymap-zone-loot-metric fishymap-zone-loot-metric--group" style="--fishymap-zone-loot-fill:${escapeHtml(metric.fillColor)};--fishymap-zone-loot-stroke:${escapeHtml(metric.strokeColor)};--fishymap-zone-loot-text:${escapeHtml(metric.textColor)};">
          <div class="fishymap-zone-loot-metric-primary">${escapeHtml(group.dropRateText || "—")}</div>
        </div>
        ${provenanceRail}
      </div>
    </div>
  `;
}

function zoneLootSectionMarkup(section) {
  const groups = Array.isArray(section?.groups) ? section.groups : [];
  const groupMarkup = groups.length
    ? groups
        .map((group) => `
          <div class="fishymap-zone-loot-group rounded-box border border-base-300 bg-base-200/75 p-2">
            ${zoneLootGroupHeaderMarkup(group)}
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

function ensureInfoPanelMarkup(host) {
  if (host.querySelector("#fishymap-zone-info-tabs")) {
    return;
  }
  host.innerHTML = `
    <div
      id="fishymap-zone-info-tabs"
      role="tablist"
      class="tabs tabs-box bg-base-200/80 p-1"
      aria-label="Point layer tabs"
      hidden
    ></div>
    <div id="fishymap-zone-info-panel" class="space-y-3">
      ${emptyPanelMarkup()}
    </div>
  `;
}

export class FishyMapInfoPanelElement extends HTMLElementBase {
  constructor() {
    super();
    this._shell = null;
    this._rafId = 0;
    this._elements = null;
    this._state = {
      zoneCatalog: [],
      zoneLootStatus: "idle",
      zoneLootSummary: null,
      zoneLootRgb: null,
      zoneLootRequestToken: 0,
    };
    this._handleSignalPatched = (event) => {
      this.handleSignalPatch(event?.detail || null);
    };
    this._handleZoneCatalogReady = (event) => {
      this._state.zoneCatalog = Array.isArray(event?.detail?.zoneCatalog)
        ? cloneJson(event.detail.zoneCatalog)
        : [];
      this.scheduleRender();
      void this.refreshZoneLootSummary();
    };
    this._handleLiveInit = () => {
      this.scheduleRender();
      void this.refreshZoneLootSummary();
    };
    this._handleClick = (event) => {
      const button = event.target.closest("button[data-zone-info-tab]");
      if (!button) {
        return;
      }
      const paneId = trimString(button.getAttribute("data-zone-info-tab"));
      dispatchShellSignalPatch(this._shell, {
        _map_ui: {
          windowUi: {
            zoneInfo: {
              tab: paneId,
            },
          },
        },
      });
    };
  }

  connectedCallback() {
    this._shell = this.closest?.("#map-page-shell") || null;
    ensureInfoPanelMarkup(this);
    this._elements = {
      title: this._shell?.querySelector?.("#fishymap-zone-info-title") || null,
      titleIcon: this._shell?.querySelector?.("#fishymap-zone-info-title-icon") || null,
      statusIcon: this._shell?.querySelector?.("#fishymap-zone-info-status-icon") || null,
      statusText: this._shell?.querySelector?.("#fishymap-zone-info-status-text") || null,
      tabs: this.querySelector("#fishymap-zone-info-tabs"),
      panel: this.querySelector("#fishymap-zone-info-panel"),
    };
    this.addEventListener("click", this._handleClick);
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.addEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    attachProvenanceTooltip(this._shell);
    this.render();
  }

  disconnectedCallback() {
    this.removeEventListener("click", this._handleClick);
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    this._rafId = 0;
    this._shell = null;
    this._elements = null;
  }

  signals() {
    return readMapShellSignals(this._shell);
  }

  render() {
    this._rafId = 0;
    const viewModel = buildInfoViewModel(this.signals(), {
      zoneCatalog: this._state.zoneCatalog,
      zoneLootSummary: this._state.zoneLootSummary,
      zoneLootStatus: this._state.zoneLootStatus,
    });
    setTextContent(this._elements?.title, viewModel.descriptor.title);
    setTextContent(this._elements?.statusText, viewModel.descriptor.statusText);
    setMarkup(
      this._elements?.titleIcon,
      viewModel.descriptor.titleIcon,
      spriteIcon(viewModel.descriptor.titleIcon || "information-circle", "size-5"),
    );
    setMarkup(
      this._elements?.statusIcon,
      viewModel.descriptor.statusIcon,
      spriteIcon(viewModel.descriptor.statusIcon || "information-circle", "size-4"),
    );
    setBooleanProperty(this._elements?.tabs, "hidden", viewModel.panes.length === 0);
    setMarkup(
      this._elements?.tabs,
      JSON.stringify(viewModel.panes.map((pane) => [pane.id, pane.label, pane.id === viewModel.activePaneId ? 1 : 0])),
      viewModel.panes.map((pane) => tabButtonMarkup(pane, viewModel.activePaneId)).join(""),
    );
    setMarkup(
      this._elements?.panel,
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

  scheduleRender() {
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    if (typeof globalThis.requestAnimationFrame === "function") {
      this._rafId = globalThis.requestAnimationFrame(() => {
        this.render();
      }) || 0;
      if (this._rafId) {
        return;
      }
    }
    this.render();
  }

  handleSignalPatch(patch) {
    if (!patchTouchesInfoSignals(patch)) {
      return;
    }
    if (patch?._map_runtime?.selection != null) {
      void this.refreshZoneLootSummary();
    }
    this.scheduleRender();
  }

  async refreshZoneLootSummary() {
    const selection = this.signals()?._map_runtime?.selection || null;
    const zoneRgb = zoneRgbFromSelection(selection);
    if (!Number.isInteger(zoneRgb) || zoneRgb < 0) {
      this._state.zoneLootRequestToken += 1;
      this._state.zoneLootRgb = null;
      this._state.zoneLootStatus = "idle";
      this._state.zoneLootSummary = null;
      this.scheduleRender();
      return;
    }
    if (
      this._state.zoneLootRgb === zoneRgb &&
      (this._state.zoneLootStatus === "loading" ||
        this._state.zoneLootStatus === "loaded" ||
        this._state.zoneLootStatus === "error")
    ) {
      return;
    }
    this._state.zoneLootRgb = zoneRgb;
    this._state.zoneLootStatus = "loading";
    this._state.zoneLootSummary = null;
    this.scheduleRender();

    const requestToken = this._state.zoneLootRequestToken + 1;
    this._state.zoneLootRequestToken = requestToken;
    try {
      const summary = await loadZoneLootSummary(zoneRgb);
      if (this._state.zoneLootRequestToken !== requestToken || this._state.zoneLootRgb !== zoneRgb) {
        return;
      }
      this._state.zoneLootSummary = summary;
      this._state.zoneLootStatus = "loaded";
    } catch (error) {
      if (this._state.zoneLootRequestToken !== requestToken || this._state.zoneLootRgb !== zoneRgb) {
        return;
      }
      this._state.zoneLootSummary = {
        available: false,
        zoneName: "",
        profileLabel: "",
        note: trimString(error?.message) || "Zone loot summary is unavailable.",
        groups: [],
        speciesRows: [],
      };
      this._state.zoneLootStatus = "error";
    }
    this.scheduleRender();
  }
}

export function registerFishyMapInfoPanelElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (registry.get(INFO_PANEL_TAG_NAME)) {
    return true;
  }
  registry.define(INFO_PANEL_TAG_NAME, FishyMapInfoPanelElement);
  return true;
}

registerFishyMapInfoPanelElement();
