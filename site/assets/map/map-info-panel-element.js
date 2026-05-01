import { dispatchShellSignalPatch, FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";
import { FISHYMAP_LIVE_INIT_EVENT, readMapShellSignals } from "./map-shell-signals.js";
import { mapText, siteText } from "./map-i18n.js";
import { buildInfoViewModel, patchTouchesInfoSignals } from "./map-info-state.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";
import { loadZoneLootSummary, zoneRgbFromSelection } from "./map-zone-loot-summary.js";
import {
  loadTradeNpcMapCatalog,
  selectedTradeOriginFromLayerSamples,
} from "./map-trade-summary.js";
import {
  attachProvenanceTooltip,
  buildProvenanceSegments,
  provenanceAriaLabel,
} from "../js/components/provenance-indicator.js";

const INFO_PANEL_TAG_NAME = "fishymap-info-panel";
const HTMLElementBase = globalThis.HTMLElement ?? class {};
const POINT_SAMPLE_PAGE_SIZE = 50;

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
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-${name}"></use></svg>`;
}

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function pointSampleZoneIndicatorMarkup(zone) {
  if (zone?.zoneKind === "partial") {
    const style = trimString(zone?.swatchRgb)
      ? ` style="--fishymap-layer-fact-rgb:${escapeHtml(zone.swatchRgb)};"`
      : "";
    return `<svg class="fishy-icon size-4 fishymap-point-sample-zone-icon" viewBox="0 0 24 24" aria-hidden="true"${style}><use width="100%" height="100%" href="#fishy-ring-partial"></use></svg>`;
  }
  return trimString(zone?.swatchRgb)
    ? `<span class="fishymap-layer-fact-swatch" style="--fishymap-layer-fact-rgb:${escapeHtml(zone.swatchRgb)};"></span>`
    : "";
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
  const name = trimString(entry?.label) || mapText("info.fish.unknown");
  const gradeTone = itemGradeTone(entry?.iconGradeTone, entry?.isPrize === true);
  const toneClass = `fishy-item-grade-${escapeHtml(gradeTone)}`;
  const iconUrl = trimString(entry?.iconUrl);
  const iconMarkup = iconUrl
    ? `<span class="fishy-item-icon-frame is-sm ${toneClass}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishy-item-icon-frame is-sm ${toneClass}"><span class="fishy-item-icon-fallback ${toneClass}">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
  return `<span class="fishy-item-row fishy-item-row--surface fishymap-zone-loot-item-surface ${toneClass}">${iconMarkup}<span class="fishy-item-label fishymap-zone-loot-item-label truncate">${escapeHtml(name)}</span>${accessoryMarkup}</span>`;
}

function pointSampleZoneMarkup(row) {
  const zones = Array.isArray(row?.zones) ? row.zones : [];
  if (!zones.length) {
    return "";
  }
  return `
    <div class="fishymap-point-sample-zones">
      ${spriteIcon("hover-zone", "size-4")}
      <span class="fishymap-point-sample-zone-list">
        ${zones
          .map((zone) => `
            <span class="fishymap-point-sample-zone">
              ${pointSampleZoneIndicatorMarkup(zone)}
              <span class="truncate">${escapeHtml(zone?.name || "")}</span>
            </span>
          `)
          .join("")}
      </span>
    </div>
  `;
}

function pointSampleMarkup(row) {
  const name = trimString(row?.fishName) || mapText("info.fish.unknown");
  const gradeTone = itemGradeTone(row?.grade, row?.isPrize === true);
  const toneClass = `fishy-item-grade-${escapeHtml(gradeTone)}`;
  const iconUrl = trimString(row?.iconUrl);
  const itemId = Number.parseInt(row?.itemId, 10);
  const fishId = Number.parseInt(row?.fishId, 10);
  const count = Math.max(1, Number.parseInt(row?.sampleCount, 10) || 1);
  const sampleId = Number.parseInt(row?.sampleId, 10);
  const sampleBadge = count > 1
    ? `x${count}`
    : Number.isInteger(sampleId) && sampleId > 0
      ? `#${sampleId}`
      : "";
  const detailParts = [
    Number.isInteger(itemId) ? `Item ${itemId}` : "",
    Number.isInteger(fishId) ? `Fish ${fishId}` : "",
  ].filter(Boolean);
  const iconMarkup = iconUrl
    ? `<span class="fishy-item-icon-frame is-native ${toneClass}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(name)}" loading="lazy" decoding="async"></span>`
    : `<span class="fishy-item-icon-frame is-native ${toneClass}"><span class="fishy-item-icon-fallback ${toneClass}">${escapeHtml(name.charAt(0).toUpperCase() || "?")}</span></span>`;
  return `
    <div class="fishymap-point-sample-card" data-zone-kind="${escapeHtml(row?.zoneKind || "")}">
      <div class="fishymap-point-sample-main">
        <span class="fishy-item-row min-w-0">
          ${iconMarkup}
          <span class="fishymap-point-sample-fish min-w-0">
            <span class="fishymap-point-sample-name truncate">${escapeHtml(name)}</span>
            ${
              detailParts.length
                ? `<span class="fishymap-point-sample-ids truncate">${escapeHtml(detailParts.join(" / "))}</span>`
                : ""
            }
          </span>
        </span>
        ${sampleBadge ? `<span class="badge badge-soft badge-sm">${escapeHtml(sampleBadge)}</span>` : ""}
      </div>
      ${
        trimString(row?.dateText)
          ? `<div class="fishymap-point-sample-date">${spriteIcon("date-confirmed", "size-4")}<span>${escapeHtml(row.dateText)}</span></div>`
          : ""
      }
      ${pointSampleZoneMarkup(row)}
    </div>
  `;
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
    <div class="fishy-provenance-rail" aria-label="${escapeHtml(mapText("info.provenance"))}">
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
              data-fishy-provenance-source-kind="${escapeHtml(segment.sourceKind)}"
              data-fishy-provenance-source-tone="${escapeHtml(segment.sourceTone)}"
              data-fishy-provenance-source-icon="${escapeHtml(segment.sourceIcon)}"
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
  const conditionText = trimString(group?.conditionText);
  const conditionTooltip = trimString(group?.conditionTooltip);
  const conditionOptions = Array.isArray(group?.conditionOptions) ? group.conditionOptions : [];
  const conditionOptionCount = conditionOptions.length;
  const conditionOptionIndex = Number.parseInt(group?.conditionOptionIndex, 10);
  const canSwitchCondition =
    conditionText &&
    conditionOptionCount > 1 &&
    Number.isInteger(conditionOptionIndex) &&
    conditionOptionIndex >= 0 &&
    conditionOptionIndex < conditionOptionCount &&
    trimString(group?.conditionOptionKey);
  const conditionMarkup = conditionText
    ? `<div class="fishymap-zone-loot-group-condition" title="${escapeHtml(conditionTooltip || conditionText)}">${escapeHtml(conditionText)}</div>`
    : "";
  return `
    <div class="fishymap-zone-loot-group-header">
      <div class="fishymap-zone-loot-group-heading">
        <span class="badge badge-soft badge-sm">${escapeHtml(group.label)}</span>
        ${
          canSwitchCondition
            ? `<div class="fishymap-zone-loot-condition-control">
                <button
                  class="btn btn-ghost btn-xs fishymap-zone-loot-condition-button"
                  type="button"
                  data-zone-loot-condition-direction="-1"
                  data-zone-loot-condition-key="${escapeHtml(group.conditionOptionKey)}"
                  data-zone-loot-condition-current="${escapeHtml(conditionOptionIndex)}"
                  data-zone-loot-condition-count="${escapeHtml(conditionOptionCount)}"
                  aria-label="${escapeHtml(mapText("info.zone_loot.condition.previous"))}"
                  title="${escapeHtml(mapText("info.zone_loot.condition.previous"))}"
                >&lt;</button>
                ${conditionMarkup}
                <button
                  class="btn btn-ghost btn-xs fishymap-zone-loot-condition-button"
                  type="button"
                  data-zone-loot-condition-direction="1"
                  data-zone-loot-condition-key="${escapeHtml(group.conditionOptionKey)}"
                  data-zone-loot-condition-current="${escapeHtml(conditionOptionIndex)}"
                  data-zone-loot-condition-count="${escapeHtml(conditionOptionCount)}"
                  aria-label="${escapeHtml(mapText("info.zone_loot.condition.next"))}"
                  title="${escapeHtml(mapText("info.zone_loot.condition.next"))}"
                >&gt;</button>
              </div>`
            : conditionMarkup
        }
      </div>
      <div class="fishymap-zone-loot-group-rate">
        <div class="fishymap-zone-loot-metric fishymap-zone-loot-metric--group" style="--fishymap-zone-loot-fill:${escapeHtml(metric.fillColor)};--fishymap-zone-loot-stroke:${escapeHtml(metric.strokeColor)};--fishymap-zone-loot-text:${escapeHtml(metric.textColor)};">
          <div class="fishymap-zone-loot-metric-primary">${escapeHtml(group.dropRateText || "—")}</div>
        </div>
        ${provenanceRail}
      </div>
    </div>
  `;
}

function zoneLootGroupCollectionMarkup(groups) {
  const groupMarkup = groups.length
    ? groups
        .map((group) => `
          <div class="fishymap-zone-loot-group rounded-box border border-base-300 bg-base-200/75 p-2">
            ${zoneLootGroupHeaderMarkup(group)}
            <div class="fishymap-zone-loot-group-rows">
              ${
                group.rows.length
                  ? group.rows.map((row) => zoneLootRowMarkup(row)).join("")
                  : `<div class="px-2 py-2 text-[11px] text-base-content/55">${escapeHtml(mapText("info.empty_group"))}</div>`
              }
            </div>
          </div>
        `)
        .join("")
    : `<div class="px-2 py-3 text-xs text-base-content/60">${escapeHtml(mapText("info.empty_zone"))}</div>`;
  return `<div class="fishymap-zone-loot-groups">${groupMarkup}</div>`;
}

function zoneLootProfileMarkup(profile) {
  const groups = Array.isArray(profile?.groups) ? profile.groups : [];
  return `
    <div class="fishymap-zone-loot-profile rounded-box border border-base-300/85 bg-base-100/75 p-3">
      <div class="fishymap-zone-loot-profile-header">
        <span class="badge badge-outline badge-sm">${escapeHtml(profile?.label || "")}</span>
        ${
          trimString(profile?.note)
            ? `<span class="fishymap-zone-loot-profile-note">${escapeHtml(profile.note)}</span>`
            : ""
        }
      </div>
      ${zoneLootGroupCollectionMarkup(groups)}
    </div>
  `;
}

function zoneLootNoticeToneStyles(tone = "info") {
  if (tone === "warning") {
    return {
      card:
        "border-color: color-mix(in oklab, var(--color-warning, #c77d19) 56%, var(--color-base-300, #d4d4d8) 44%); background: color-mix(in oklab, var(--color-warning, #c77d19) 14%, var(--color-base-100, #ffffff) 86%);",
      icon: "color: var(--color-warning, #f59e0b);",
      title:
        "color: color-mix(in oklab, var(--color-warning, #c77d19) 78%, var(--color-base-content, #1f2937) 22%);",
    };
  }
  return {
    card:
      "border-color: color-mix(in oklab, var(--color-info, #0ea5e9) 42%, var(--color-base-300, #d4d4d8) 58%); background: color-mix(in oklab, var(--color-info, #0ea5e9) 10%, var(--color-base-100, #ffffff) 90%);",
    icon: "color: var(--color-info, #0ea5e9);",
    title:
      "color: color-mix(in oklab, var(--color-info, #0ea5e9) 72%, var(--color-base-content, #1f2937) 28%);",
  };
}

function zoneLootNoticeCardMarkup({
  tone = "info",
  iconName = "information-circle",
  title = "",
  paragraphs = [],
} = {}) {
  const content = Array.from(
    new Set(
      (Array.isArray(paragraphs) ? paragraphs : [])
        .map((paragraph) => trimString(paragraph))
        .filter(Boolean),
    ),
  );
  const heading = trimString(title);
  if (!heading && !content.length) {
    return "";
  }
  const styles = zoneLootNoticeToneStyles(tone);
  return `
    <div class="rounded-box border px-4 py-4" style="${styles.card}">
      <div class="flex items-start gap-3">
        <div class="shrink-0 pt-0.5" style="${styles.icon}">
          ${spriteIcon(iconName, "size-6")}
        </div>
        <div class="min-w-0">
          ${
            heading
              ? `<div class="text-sm font-semibold uppercase tracking-widest" style="${styles.title}">${escapeHtml(heading)}</div>`
              : ""
          }
          ${
            content.length
              ? `<div class="mt-2 space-y-2 text-sm leading-relaxed text-base-content/85">${content.map((paragraph) => `<p>${escapeHtml(paragraph)}</p>`).join("")}</div>`
              : ""
          }
        </div>
      </div>
    </div>
  `;
}

function zoneLootDataQualityWarningMarkup(section) {
  if (section?.available !== true) {
    return "";
  }
  return zoneLootNoticeCardMarkup({
    tone: "warning",
    iconName: "alert-fill",
    title: siteText("calculator.server.disclaimer.title"),
    paragraphs: [
      siteText("calculator.server.disclaimer.p1"),
      siteText("calculator.server.disclaimer.p2"),
      siteText("calculator.server.disclaimer.p3"),
      siteText("calculator.server.disclaimer.p4"),
      siteText("calculator.server.disclaimer.p5"),
    ],
  });
}

function zoneLootCalculatorNoticeMarkup(section) {
  const available = section?.available === true;
  return zoneLootNoticeCardMarkup({
    tone: "info",
    iconName: "information-circle",
    title: available
      ? mapText("info.zone_loot.notice.calculator_title")
      : mapText("info.zone_loot.notice.status_title"),
    paragraphs: available
      ? [trimString(section?.dataQualityNote), trimString(section?.note)]
      : [trimString(section?.note)],
  });
}

function zoneLootNoticeDisclosureMarkup(section) {
  const notices = [
    zoneLootDataQualityWarningMarkup(section),
    zoneLootCalculatorNoticeMarkup(section),
  ].filter(Boolean);
  if (!notices.length) {
    return "";
  }
  return `
    <fishy-notice-disclosure
      title="${escapeHtml(mapText("info.notice.title"))}"
      icon="alert-triangle"
      settings-path="map.zonePane.noticeOpen"
      body-class="space-y-3 px-1 pb-1 pt-3"
      open
    >
      ${notices.join("")}
    </fishy-notice-disclosure>
  `;
}

function zoneLootSectionMarkup(section) {
  const profiles = Array.isArray(section?.profiles) ? section.profiles : [];
  const groups = Array.isArray(section?.groups) ? section.groups : [];
  const profileMarkup = profiles.length
    ? profiles.map((profile) => zoneLootProfileMarkup(profile)).join("")
    : zoneLootGroupCollectionMarkup(groups);
  return `
    <section class="space-y-2">
      <div class="flex items-center justify-between gap-3">
        <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(section.title || mapText("info.section.fish"))}</p>
        <span class="text-[11px] text-base-content/55">${escapeHtml(section.statusText || mapText("info.zone_loot.status.idle"))}</span>
      </div>
      ${zoneLootNoticeDisclosureMarkup(section)}
      <div class="fishymap-zone-loot-profiles">${profileMarkup}</div>
    </section>
  `;
}

function pointSamplePageCount(rows, pageSize = POINT_SAMPLE_PAGE_SIZE) {
  return Math.max(1, Math.ceil(rows.length / pageSize));
}

function normalizePointSamplePage(value, rows, pageSize = POINT_SAMPLE_PAGE_SIZE) {
  const page = Number.parseInt(value, 10);
  if (!Number.isInteger(page) || page < 0) {
    return 0;
  }
  return Math.min(page, pointSamplePageCount(rows, pageSize) - 1);
}

function pointSampleSectionMarkup(section, { pointSamplePage = 0 } = {}) {
  const rows = Array.isArray(section?.rows) ? section.rows : [];
  if (!rows.length) {
    return "";
  }
  const page = normalizePointSamplePage(pointSamplePage, rows);
  const pageCount = pointSamplePageCount(rows);
  const start = page * POINT_SAMPLE_PAGE_SIZE;
  const end = Math.min(start + POINT_SAMPLE_PAGE_SIZE, rows.length);
  const visibleRows = rows.slice(start, end);
  const hasPages = rows.length > POINT_SAMPLE_PAGE_SIZE;
  const sampleRangeText = hasPages
    ? `Showing ${start + 1}-${end} of ${rows.length} samples`
    : `${rows.length} sample${rows.length === 1 ? "" : "s"}`;
  const pagingMarkup = hasPages
    ? `
      <div class="fishymap-point-sample-pager">
        <button
          class="btn btn-ghost btn-xs"
          type="button"
          data-point-sample-page="${Math.max(0, page - 1)}"
          ${page <= 0 ? "disabled" : ""}
        >Prev</button>
        <span class="text-[11px] text-base-content/55">Page ${page + 1}/${pageCount}</span>
        <button
          class="btn btn-ghost btn-xs"
          type="button"
          data-point-sample-page="${Math.min(pageCount - 1, page + 1)}"
          ${page >= pageCount - 1 ? "disabled" : ""}
        >Next</button>
      </div>
    `
    : "";
  return `
    <section class="space-y-2">
      ${
        trimString(section?.title)
          ? `<p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(section.title)}</p>`
          : ""
      }
      <div class="fishymap-point-sample-summary">
        <span>${escapeHtml(sampleRangeText)}</span>
        ${hasPages ? `<span>Sorted by occurrence</span>` : ""}
      </div>
      ${pagingMarkup}
      <div class="fishymap-point-sample-list">${visibleRows.map((row) => pointSampleMarkup(row)).join("")}</div>
      ${pagingMarkup}
    </section>
  `;
}

function sectionMarkup(section, options = {}) {
  switch (trimString(section?.kind)) {
    case "point-samples":
      return pointSampleSectionMarkup(section, options);
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
  return `<div class="rounded-box border border-base-300/70 bg-base-200 px-3 py-3 text-sm text-base-content/60">${escapeHtml(mapText("info.empty_selection"))}</div>`;
}

function pointSampleSectionKey(section) {
  const rows = Array.isArray(section?.rows) ? section.rows : [];
  if (!rows.length) {
    return "";
  }
  return [
    trimString(section?.id),
    rows.length,
    trimString(rows[0]?.key),
    trimString(rows[rows.length - 1]?.key),
  ].join(":");
}

function panelRenderKey(viewModel, pointSamplePage) {
  const sections = viewModel.activePane?.sections || [];
  return JSON.stringify({
    empty: viewModel.empty,
    activePaneId: viewModel.activePaneId,
    pointSamplePage,
    sections: sections.map((section) => {
      if (trimString(section?.kind) !== "point-samples") {
        return section;
      }
      const rows = Array.isArray(section?.rows) ? section.rows : [];
      const page = normalizePointSamplePage(pointSamplePage, rows);
      const start = page * POINT_SAMPLE_PAGE_SIZE;
      const end = Math.min(start + POINT_SAMPLE_PAGE_SIZE, rows.length);
      return {
        id: section.id,
        kind: section.kind,
        title: section.title,
        rowCount: rows.length,
        page,
        visibleRowKeys: rows.slice(start, end).map((row) => row?.key || ""),
      };
    }),
  });
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
      aria-label="${escapeHtml(mapText("info.tabs"))}"
      hidden
    ></div>
    <div id="fishymap-zone-info-panel" class="space-y-3">
      ${emptyPanelMarkup()}
    </div>
  `;
}

export class FishyMapInfoPanelElement extends HTMLElementBase {
  static get observedAttributes() {
    return ["data-normalize-rates"];
  }

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
      zoneLootConditionSelection: {},
      pointSamplePage: 0,
      pointSampleSectionKey: "",
      tradeNpcMapCatalog: null,
      tradeNpcMapStatus: "idle",
      tradeNpcMapOriginKey: "",
      tradeNpcMapRequestToken: 0,
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
      void this.refreshTradeNpcMapCatalog();
    };
    this._handleUserOverlaysChanged = () => {
      void this.refreshZoneLootSummary({ force: true });
    };
    this._handleLanguageChanged = () => {
      this.scheduleRender();
      void this.refreshZoneLootSummary({ force: true });
    };
    this._handleClick = (event) => {
      const conditionButton = event.target.closest(
        "button[data-zone-loot-condition-direction]",
      );
      if (conditionButton) {
        event.preventDefault?.();
        const key = trimString(conditionButton.getAttribute("data-zone-loot-condition-key"));
        const direction = Number.parseInt(
          conditionButton.getAttribute("data-zone-loot-condition-direction"),
          10,
        );
        const current = Number.parseInt(
          conditionButton.getAttribute("data-zone-loot-condition-current"),
          10,
        );
        const count = Number.parseInt(
          conditionButton.getAttribute("data-zone-loot-condition-count"),
          10,
        );
        if (key && Number.isInteger(direction) && Number.isInteger(current) && count > 1) {
          this._state.zoneLootConditionSelection = {
            ...(this._state.zoneLootConditionSelection || {}),
            [key]: (current + direction + count) % count,
          };
          this.scheduleRender();
        }
        return;
      }

      const pointSamplePageButton = event.target.closest("button[data-point-sample-page]");
      if (pointSamplePageButton) {
        event.preventDefault?.();
        this._state.pointSamplePage = Math.max(
          0,
          Number.parseInt(pointSamplePageButton.getAttribute("data-point-sample-page"), 10) || 0,
        );
        this.scheduleRender();
        return;
      }

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

  attributeChangedCallback(name, previousValue, nextValue) {
    if (name !== "data-normalize-rates" || previousValue === nextValue) {
      return;
    }
    this.scheduleRender();
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
    globalThis.window?.addEventListener?.(
      globalThis.window?.__fishystuffUserOverlays?.CHANGED_EVENT || "fishystuff:user-overlays-changed",
      this._handleUserOverlaysChanged,
    );
    globalThis.window?.addEventListener?.(
      globalThis.window?.__fishystuffLanguage?.event || "fishystuff:languagechange",
      this._handleLanguageChanged,
    );
    attachProvenanceTooltip(this._shell);
    this.render();
    void this.refreshTradeNpcMapCatalog();
  }

  disconnectedCallback() {
    this.removeEventListener("click", this._handleClick);
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    globalThis.window?.removeEventListener?.(
      globalThis.window?.__fishystuffUserOverlays?.CHANGED_EVENT || "fishystuff:user-overlays-changed",
      this._handleUserOverlaysChanged,
    );
    globalThis.window?.removeEventListener?.(
      globalThis.window?.__fishystuffLanguage?.event || "fishystuff:languagechange",
      this._handleLanguageChanged,
    );
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

  normalizeRatesEnabled(signals = this.signals()) {
    const signalValue = signals?._map_ui?.windowUi?.settings?.normalizeRates;
    if (typeof signalValue === "boolean") {
      return signalValue;
    }
    const attrValue = trimString(this.getAttribute?.("data-normalize-rates")).toLowerCase();
    return attrValue !== "false" && attrValue !== "0";
  }

  render() {
    this._rafId = 0;
    const signals = this.signals();
    const viewModel = buildInfoViewModel(signals, {
      zoneCatalog: this._state.zoneCatalog,
      zoneLootSummary: this._state.zoneLootSummary,
      zoneLootStatus: this._state.zoneLootStatus,
      zoneLootConditionSelection: this._state.zoneLootConditionSelection,
      normalizeRates: this.normalizeRatesEnabled(signals),
      tradeNpcMapCatalog: this._state.tradeNpcMapCatalog,
      tradeNpcMapStatus: this._state.tradeNpcMapStatus,
    });
    const activePointSampleSection = (viewModel.activePane?.sections || []).find(
      (section) => trimString(section?.kind) === "point-samples",
    );
    const activePointSampleSectionKey = activePointSampleSection
      ? pointSampleSectionKey(activePointSampleSection)
      : "";
    if (this._state.pointSampleSectionKey !== activePointSampleSectionKey) {
      this._state.pointSampleSectionKey = activePointSampleSectionKey;
      this._state.pointSamplePage = 0;
    }
    this._state.pointSamplePage = normalizePointSamplePage(
      this._state.pointSamplePage,
      Array.isArray(activePointSampleSection?.rows) ? activePointSampleSection.rows : [],
    );
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
      panelRenderKey(viewModel, this._state.pointSamplePage),
      viewModel.empty
        ? emptyPanelMarkup()
        : `<section class="space-y-3">${(viewModel.activePane?.sections || []).map((section) => sectionMarkup(section, { pointSamplePage: this._state.pointSamplePage })).join("")}</section>`,
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
      void this.refreshTradeNpcMapCatalog();
    }
    this.scheduleRender();
  }

  async refreshTradeNpcMapCatalog({ force = false } = {}) {
    const signals = this.signals();
    const layerSamples = Array.isArray(signals?._map_runtime?.selection?.layerSamples)
      ? signals._map_runtime.selection.layerSamples
      : [];
    const origin = selectedTradeOriginFromLayerSamples(layerSamples);
    const originKey = origin
      ? [origin.regionId ?? "", origin.worldX ?? "", origin.worldZ ?? "", origin.label || ""].join(":")
      : "";
    if (!originKey) {
      this._state.tradeNpcMapRequestToken += 1;
      this._state.tradeNpcMapOriginKey = "";
      this._state.tradeNpcMapStatus = "idle";
      this.scheduleRender();
      return;
    }
    if (
      !force &&
      this._state.tradeNpcMapOriginKey === originKey &&
      (this._state.tradeNpcMapStatus === "loading" ||
        this._state.tradeNpcMapStatus === "loaded" ||
        this._state.tradeNpcMapStatus === "error")
    ) {
      return;
    }
    this._state.tradeNpcMapOriginKey = originKey;
    if (!this._state.tradeNpcMapCatalog || force) {
      this._state.tradeNpcMapStatus = "loading";
      this.scheduleRender();
    } else {
      this._state.tradeNpcMapStatus = "loaded";
      this.scheduleRender();
      return;
    }

    const requestToken = this._state.tradeNpcMapRequestToken + 1;
    this._state.tradeNpcMapRequestToken = requestToken;
    try {
      const catalog = await loadTradeNpcMapCatalog({ force });
      if (
        this._state.tradeNpcMapRequestToken !== requestToken ||
        this._state.tradeNpcMapOriginKey !== originKey
      ) {
        return;
      }
      this._state.tradeNpcMapCatalog = catalog;
      this._state.tradeNpcMapStatus = "loaded";
    } catch (_error) {
      if (
        this._state.tradeNpcMapRequestToken !== requestToken ||
        this._state.tradeNpcMapOriginKey !== originKey
      ) {
        return;
      }
      this._state.tradeNpcMapStatus = "error";
    }
    this.scheduleRender();
  }

  async refreshZoneLootSummary({ force = false } = {}) {
    const signals = this.signals();
    const selection = signals?._map_runtime?.selection || null;
    const normalizeRates = this.normalizeRatesEnabled(signals);
    const zoneRgb = zoneRgbFromSelection(selection);
    if (!Number.isInteger(zoneRgb) || zoneRgb < 0) {
      this._state.zoneLootRequestToken += 1;
      this._state.zoneLootRgb = null;
      this._state.zoneLootStatus = "idle";
      this._state.zoneLootSummary = null;
      this._state.zoneLootConditionSelection = {};
      this.scheduleRender();
      return;
    }
    if (
      !force &&
      this._state.zoneLootRgb === zoneRgb &&
      (this._state.zoneLootStatus === "loading" ||
        this._state.zoneLootStatus === "loaded" ||
        this._state.zoneLootStatus === "error")
    ) {
      return;
    }
    if (force || this._state.zoneLootRgb !== zoneRgb) {
      this._state.zoneLootConditionSelection = {};
    }
    this._state.zoneLootRgb = zoneRgb;
    this._state.zoneLootStatus = "loading";
    this._state.zoneLootSummary = null;
    this.scheduleRender();

    const requestToken = this._state.zoneLootRequestToken + 1;
    this._state.zoneLootRequestToken = requestToken;
    try {
      const summary = await loadZoneLootSummary(zoneRgb, { normalizeRates });
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
        note: trimString(error?.message) || mapText("info.zone_loot.unavailable"),
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
