(function () {
  const TAG_NAME = "fishy-calculator-overlay-panel";
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const GROUP_OPTIONS = Object.freeze([
    { slotIdx: 1, label: "Prize" },
    { slotIdx: 2, label: "Rare" },
    { slotIdx: 3, label: "High-Quality" },
    { slotIdx: 4, label: "General" },
    { slotIdx: 5, label: "Trash" },
  ]);
  const HTMLElementBase = globalThis.HTMLElement ?? class {};

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

  function normalizeNumber(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : null;
  }

  function silverText(value) {
    const rounded = Math.max(0, Math.round(Number(value) || 0));
    return rounded.toLocaleString();
  }

  function percentText(value) {
    const numeric = Number(value);
    if (!Number.isFinite(numeric)) {
      return "";
    }
    const absolute = Math.abs(numeric);
    const decimals = absolute < 0.0001
      ? 12
      : absolute < 0.01
        ? 10
        : absolute < 1
          ? 8
          : absolute < 100
            ? 4
            : 2;
    const compact = numeric.toFixed(decimals).replace(/\.?0+$/, "");
    if (compact === "0" && numeric !== 0) {
      return `${numeric.toFixed(14).replace(/\.?0+$/, "")}%`;
    }
    return `${compact}%`;
  }

  function itemGradeTone(grade) {
    const resolver = globalThis.window?.__fishystuffItemPresentation?.resolveGradeTone;
    if (typeof resolver === "function") {
      return resolver(grade, false);
    }
    return "unknown";
  }

  function readCalculatorSignals() {
    return globalThis.window?.__fishystuffCalculator?.signalObject?.() ?? null;
  }

  function patchCalculatorSignals(patch) {
    if (typeof globalThis.window?.__fishystuffCalculator?.patchSignals === "function") {
      globalThis.window.__fishystuffCalculator.patchSignals(patch);
    }
  }

  function replaceCalculatorSignalRoot(key, value) {
    const calculator = globalThis.window?.__fishystuffCalculator;
    const signals = calculator?.signalObject?.();
    if (!signals || !key) {
      return;
    }
    signals[key] = cloneJson(value);
    globalThis.document?.dispatchEvent?.(new CustomEvent(DATASTAR_SIGNAL_PATCH_EVENT, {
      detail: { [key]: cloneJson(value) },
    }));
  }

  function sharedUserOverlays() {
    return globalThis.window?.__fishystuffUserOverlays ?? null;
  }

  function groupLabel(slotIdx) {
    return GROUP_OPTIONS.find((option) => option.slotIdx === Number(slotIdx))?.label || "Unassigned";
  }

  function groupOptionsMarkup(selectedSlotIdx) {
    return GROUP_OPTIONS.map((option) => `
      <option value="${option.slotIdx}"${Number(selectedSlotIdx) === option.slotIdx ? " selected" : ""}>${escapeHtml(option.label)}</option>
    `).join("");
  }

  function downloadText(filename, text) {
    if (!globalThis.window?.document?.createElement || !globalThis.URL?.createObjectURL) {
      return false;
    }
    const blob = new Blob([text], { type: "application/json" });
    const url = globalThis.URL.createObjectURL(blob);
    const anchor = globalThis.document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    globalThis.document.body?.appendChild?.(anchor);
    anchor.click();
    anchor.remove();
    globalThis.URL.revokeObjectURL(url);
    return true;
  }

  function readTextFile(file) {
    if (typeof file?.text === "function") {
      return file.text();
    }
    const readerCtor = globalThis.FileReader;
    if (typeof readerCtor !== "function") {
      throw new Error("Overlay import is unavailable in this browser.");
    }
    return new Promise((resolve, reject) => {
      const reader = new readerCtor();
      reader.onerror = () => reject(reader.error || new Error("Failed to read overlay JSON."));
      reader.onload = () => resolve(String(reader.result ?? ""));
      reader.readAsText(file);
    });
  }

  function overlaySnapshot() {
    return sharedUserOverlays()?.overlaySignals?.() ?? { zones: {} };
  }

  function priceOverrideSnapshot() {
    return sharedUserOverlays()?.priceOverrides?.() ?? {};
  }

  function currentZoneOverlay(zoneKey) {
    return overlaySnapshot()?.zones?.[zoneKey] ?? { groups: {}, items: {} };
  }

  function currentOverlayEditor() {
    const signals = readCalculatorSignals();
    return isPlainObject(signals?._calc?.overlay_editor)
      ? cloneJson(signals._calc.overlay_editor)
      : {
          zone_rgb_key: trimString(signals?.zone),
          zone_name: trimString(signals?._calc?.zone_name || signals?.zone),
          groups: [],
          items: [],
        };
  }

  function zoneLabelForKey(zoneKey, editor) {
    if (trimString(editor?.zone_rgb_key) === trimString(zoneKey)) {
      return trimString(editor?.zone_name) || trimString(zoneKey);
    }
    return trimString(zoneKey);
  }

  function buildEditorItemMap(editor) {
    const itemMap = new Map();
    for (const row of Array.isArray(editor?.items) ? editor.items : []) {
      itemMap.set(String(row.item_id), row);
    }
    return itemMap;
  }

  function syntheticItemRow(itemId, itemOverlay, priceOverride) {
    return {
      item_id: Number.parseInt(itemId, 10) || 0,
      default_present: false,
      overlay_added: true,
      slot_idx: Number.parseInt(itemOverlay?.slotIdx, 10) || 4,
      group_label: groupLabel(itemOverlay?.slotIdx),
      label: trimString(itemOverlay?.name) || itemId,
      icon_url: globalThis.window?.__fishystuffResolveFishItemIconUrl?.(itemId) || "",
      icon_grade_tone: itemGradeTone(itemOverlay?.grade),
      default_raw_rate_pct: 0,
      default_raw_rate_text: "0%",
      normalized_rate_pct: 0,
      normalized_rate_text: "0%",
      base_price_raw: Number(priceOverride?.basePrice) || 0,
      base_price_text: silverText(priceOverride?.basePrice),
      is_fish: itemOverlay?.isFish !== false,
    };
  }

  function effectiveGroupState(row, groupOverlay) {
    return {
      present: groupOverlay?.present === false ? false : row.default_present !== false,
      rawRatePercent:
        normalizeNumber(groupOverlay?.rawRatePercent) ?? normalizeNumber(row.default_raw_rate_pct) ?? 0,
      changed: Boolean(groupOverlay),
    };
  }

  function effectiveItemState(row, itemOverlay, priceOverride) {
    return {
      present:
        row.default_present === false
          ? itemOverlay != null
          : itemOverlay?.present === false
            ? false
            : true,
      slotIdx:
        Number.parseInt(itemOverlay?.slotIdx, 10)
        || Number.parseInt(row.slot_idx, 10)
        || 4,
      rawRatePercent:
        normalizeNumber(itemOverlay?.rawRatePercent)
        ?? normalizeNumber(row.default_raw_rate_pct)
        ?? 0,
      basePrice:
        normalizeNumber(priceOverride?.basePrice)
        ?? normalizeNumber(row.base_price_raw)
        ?? 0,
      factChanged: Boolean(itemOverlay),
      priceChanged: Boolean(priceOverride),
    };
  }

  function buildDisplayRows(editor) {
    const itemMap = buildEditorItemMap(editor);
    const zoneOverlay = currentZoneOverlay(editor.zone_rgb_key);
    const priceOverrides = priceOverrideSnapshot();
    for (const [itemId, itemOverlay] of Object.entries(zoneOverlay.items || {})) {
      if (itemMap.has(itemId)) {
        continue;
      }
      itemMap.set(itemId, syntheticItemRow(itemId, itemOverlay, priceOverrides[itemId]));
    }
    return Array.from(itemMap.values()).sort((left, right) => {
      const leftSlot = Number.parseInt(left.slot_idx, 10) || 0;
      const rightSlot = Number.parseInt(right.slot_idx, 10) || 0;
      return leftSlot - rightSlot
        || String(left.label || "").localeCompare(String(right.label || ""))
        || ((left.item_id || 0) - (right.item_id || 0));
    });
  }

  function buildChangeEntries(editor) {
    const entries = [];
    const overlay = overlaySnapshot();
    const priceOverrides = priceOverrideSnapshot();
    const currentItemMap = buildEditorItemMap(editor);
    for (const [zoneKey, zoneOverlay] of Object.entries(overlay.zones || {})) {
      for (const [slotKey, groupOverlay] of Object.entries(zoneOverlay.groups || {})) {
        const label = trimString(
          trimString(zoneKey) === trimString(editor.zone_rgb_key)
            ? editor.groups.find((row) => String(row.slot_idx) === slotKey)?.label
            : groupLabel(slotKey),
        ) || groupLabel(slotKey);
        const detailParts = [];
        if (groupOverlay.present === false) {
          detailParts.push("removed from zone mix");
        } else if (groupOverlay.present === true) {
          detailParts.push("forced into zone mix");
        }
        if (normalizeNumber(groupOverlay.rawRatePercent) != null) {
          detailParts.push(`raw ${percentText(groupOverlay.rawRatePercent)}`);
        }
        entries.push({
          key: `group:${zoneKey}:${slotKey}`,
          scope: zoneLabelForKey(zoneKey, editor),
          label: `${label} group`,
          detail: detailParts.join(" · ") || "customized",
          resetKind: "group",
          zoneKey,
          slotKey,
        });
      }
      for (const [itemId, itemOverlay] of Object.entries(zoneOverlay.items || {})) {
        const editorRow = currentItemMap.get(itemId);
        const label = trimString(itemOverlay.name) || trimString(editorRow?.label) || itemId;
        const detailParts = [];
        if (editorRow?.default_present === false && itemOverlay) {
          detailParts.push("added to zone");
        } else if (itemOverlay.present === false) {
          detailParts.push("removed from zone");
        }
        if (Number.parseInt(itemOverlay.slotIdx, 10) >= 1) {
          detailParts.push(`group ${groupLabel(itemOverlay.slotIdx)}`);
        }
        if (normalizeNumber(itemOverlay.rawRatePercent) != null) {
          detailParts.push(`raw ${percentText(itemOverlay.rawRatePercent)}`);
        }
        entries.push({
          key: `item:${zoneKey}:${itemId}`,
          scope: zoneLabelForKey(zoneKey, editor),
          label,
          detail: detailParts.join(" · ") || "customized",
          resetKind: "item",
          zoneKey,
          itemId,
        });
      }
    }
    for (const [itemId, priceOverride] of Object.entries(priceOverrides)) {
      const editorRow = currentItemMap.get(itemId);
      const label = trimString(editorRow?.label) || `Item ${itemId}`;
      entries.push({
        key: `price:${itemId}`,
        scope: "Global price",
        label,
        detail: normalizeNumber(priceOverride.basePrice) != null
          ? `base price ${silverText(priceOverride.basePrice)}`
          : "customized",
        resetKind: "price",
        itemId,
      });
    }
    return entries.sort((left, right) => `${left.scope} ${left.label}`.localeCompare(`${right.scope} ${right.label}`));
  }

  function changedBadge(active, label) {
    if (!active) {
      return "";
    }
    return `<span class="badge badge-soft badge-warning badge-xs">${escapeHtml(label)}</span>`;
  }

  function explainableStatMarkup({
    valueText,
    detailText,
    breakdown,
    color = "var(--color-info)",
  }) {
    const payload = trimString(breakdown);
    const attrText = payload
      ? ` tabindex="0" data-fishy-stat-breakdown="${escapeHtml(payload)}" data-fishy-stat-color="${escapeHtml(color)}"`
      : "";
    const explainableClass = payload ? " fishy-explainable-stat" : "";
    return `
      <div class="rounded-box border border-base-300 bg-base-200 px-3 py-2${explainableClass}"${attrText}>
        <div class="text-sm font-semibold leading-tight">${escapeHtml(valueText || "0%")}</div>
        <div class="mt-1 text-[11px] leading-snug text-base-content/60">${escapeHtml(detailText || "")}</div>
      </div>
    `;
  }

  function itemRowMarkup(row, state) {
    const label = trimString(row.label) || `Item ${row.item_id}`;
    const tone = trimString(row.icon_grade_tone) || "unknown";
    const iconUrl = trimString(row.icon_url);
    const iconMarkup = iconUrl
      ? `<span class="fishy-item-icon-frame is-sm fishy-item-grade-${escapeHtml(tone)}"><img class="fishy-item-icon" src="${escapeHtml(iconUrl)}" alt="${escapeHtml(label)}" loading="lazy" decoding="async"></span>`
      : `<span class="fishy-item-icon-frame is-sm fishy-item-grade-${escapeHtml(tone)}"><span class="fishy-item-icon-fallback fishy-item-grade-${escapeHtml(tone)}">${escapeHtml(label.charAt(0).toUpperCase() || "?")}</span></span>`;
    return `
      <tr>
        <td class="align-top">
          <div class="flex items-center gap-2">
            ${iconMarkup}
            <div class="min-w-0">
              <div class="font-medium">${escapeHtml(label)}</div>
              <div class="text-[11px] text-base-content/55">ID ${escapeHtml(row.item_id)}</div>
            </div>
          </div>
        </td>
        <td class="align-top text-xs text-base-content/70">
          <div>${escapeHtml(row.group_label || "Unassigned")}</div>
          <div>Raw ${escapeHtml(row.default_raw_rate_text || "0%")}</div>
          <div>${escapeHtml(row.base_price_text || "0")}</div>
        </td>
        <td class="align-top">
          <div class="flex flex-wrap items-center gap-2">
            ${changedBadge(state.factChanged, "Facts")}
            ${changedBadge(state.priceChanged, "Price")}
          </div>
          <label class="label cursor-pointer justify-start gap-2 py-1">
            <input
              type="checkbox"
              class="checkbox checkbox-sm checkbox-primary"
              data-item-present="${escapeHtml(row.item_id)}"
              ${state.present ? "checked" : ""}
            >
            <span class="text-xs">Included</span>
          </label>
          <select class="select select-sm w-full max-w-36" data-item-slot="${escapeHtml(row.item_id)}">
            ${groupOptionsMarkup(state.slotIdx)}
          </select>
        </td>
        <td class="align-top">
          <input
            type="number"
            min="0"
            step="any"
            class="input input-sm w-24"
            data-item-rate="${escapeHtml(row.item_id)}"
            value="${escapeHtml(state.rawRatePercent)}"
          >
          <div class="mt-1 text-[11px] text-base-content/55">raw %</div>
        </td>
        <td class="align-top text-xs text-base-content/70">
          <div>${escapeHtml(row.normalized_rate_text || "0%")}</div>
          <div>normalized</div>
        </td>
        <td class="align-top">
          <input
            type="number"
            min="0"
            step="1"
            class="input input-sm w-28"
            data-item-price="${escapeHtml(row.item_id)}"
            value="${escapeHtml(state.basePrice)}"
          >
          <div class="mt-1 text-[11px] text-base-content/55">base silver</div>
        </td>
      </tr>
    `;
  }

  function groupRowMarkup(row, state) {
    const currentRawText = percentText(state.rawRatePercent);
    const bonusText = Number(row.bonus_rate_pct) > 0
      ? `+${trimString(row.bonus_rate_text || "")}`
      : trimString(row.bonus_rate_text || "0%");
    return `
      <tr>
        <td class="font-medium">${escapeHtml(row.label)}</td>
        <td class="align-top min-w-40">
          ${explainableStatMarkup({
            valueText: trimString(row.default_raw_rate_text || "0%"),
            detailText: row.default_present
              ? "included in source defaults"
              : "absent from source defaults",
          })}
        </td>
        <td>
          <label class="label cursor-pointer justify-start gap-2 py-0">
            <input
              type="checkbox"
              class="checkbox checkbox-sm checkbox-primary"
              data-group-present="${escapeHtml(row.slot_idx)}"
              ${state.present ? "checked" : ""}
            >
            <span class="text-xs">Included</span>
          </label>
        </td>
        <td>
          <input
            type="number"
            min="0"
            step="any"
            class="input input-sm w-24"
            data-group-rate="${escapeHtml(row.slot_idx)}"
            value="${escapeHtml(state.rawRatePercent)}"
          >
          <div class="mt-1 text-[11px] text-base-content/55">raw base %</div>
        </td>
        <td class="align-top min-w-40">
          ${explainableStatMarkup({
            valueText: bonusText,
            detailText: `effective raw ${trimString(row.effective_raw_weight_text || "0%")} before normalization`,
            breakdown: row.bonus_rate_breakdown,
            color: "var(--color-warning)",
          })}
        </td>
        <td class="align-top min-w-44">
          ${explainableStatMarkup({
            valueText: trimString(row.normalized_share_text || "0%"),
            detailText: `raw ${currentRawText} + bonus ${trimString(row.bonus_rate_text || "0%")} before normalization`,
            breakdown: row.normalized_share_breakdown,
            color: "var(--color-info)",
          })}
        </td>
      </tr>
    `;
  }

  class FishyCalculatorOverlayPanel extends HTMLElementBase {
    constructor() {
      super();
      this._rafId = 0;
      this._handleSignalPatch = () => this.scheduleRender();
      this._handleOverlayChange = () => this.scheduleRender();
      this._handleClick = (event) => this.handleClick(event);
      this._handleChange = (event) => this.handleChange(event);
    }

    connectedCallback() {
      this.addEventListener("click", this._handleClick);
      this.addEventListener("change", this._handleChange);
      globalThis.document?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, this._handleSignalPatch);
      globalThis.window?.addEventListener?.(
        sharedUserOverlays()?.CHANGED_EVENT || "fishystuff:user-overlays-changed",
        this._handleOverlayChange,
      );
      this.scheduleRender();
    }

    disconnectedCallback() {
      this.removeEventListener("click", this._handleClick);
      this.removeEventListener("change", this._handleChange);
      globalThis.document?.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, this._handleSignalPatch);
      globalThis.window?.removeEventListener?.(
        sharedUserOverlays()?.CHANGED_EVENT || "fishystuff:user-overlays-changed",
        this._handleOverlayChange,
      );
      if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
        globalThis.cancelAnimationFrame(this._rafId);
      }
      this._rafId = 0;
    }

    scheduleRender() {
      if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
        globalThis.cancelAnimationFrame(this._rafId);
      }
      if (typeof globalThis.requestAnimationFrame === "function") {
        this._rafId = globalThis.requestAnimationFrame(() => {
          this._rafId = 0;
          this.render();
        }) || 0;
        if (this._rafId) {
          return;
        }
      }
      this.render();
    }

    render() {
      const editor = currentOverlayEditor();
      const zoneOverlay = currentZoneOverlay(editor.zone_rgb_key);
      const rows = buildDisplayRows(editor);
      const changes = buildChangeEntries(editor);
      const groupMarkup = (Array.isArray(editor.groups) ? editor.groups : [])
        .map((row) => groupRowMarkup(row, effectiveGroupState(row, zoneOverlay.groups?.[row.slot_idx])))
        .join("");
      const itemMarkup = rows
        .map((row) => itemRowMarkup(row, effectiveItemState(
          row,
          zoneOverlay.items?.[String(row.item_id)],
          priceOverrideSnapshot()[String(row.item_id)],
        )))
        .join("");
      const changeMarkup = changes.length
        ? changes.map((entry) => `
            <div class="fishy-overlay-change rounded-box border border-base-300 bg-base-200/70 px-3 py-2">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">${escapeHtml(entry.scope)}</div>
                  <div class="font-medium">${escapeHtml(entry.label)}</div>
                  <div class="text-xs text-base-content/70">${escapeHtml(entry.detail)}</div>
                </div>
                <button
                  type="button"
                  class="btn btn-xs btn-dash btn-error"
                  data-reset-kind="${escapeHtml(entry.resetKind)}"
                  data-reset-zone="${escapeHtml(entry.zoneKey || "")}"
                  data-reset-slot="${escapeHtml(entry.slotKey || "")}"
                  data-reset-item="${escapeHtml(entry.itemId || "")}"
                >Restore</button>
              </div>
            </div>
          `).join("")
        : '<div class="rounded-box border border-dashed border-base-300 bg-base-200/50 px-3 py-3 text-sm text-base-content/60">No personal overlay changes yet. Changes stay local until you export the JSON and send it to maintainers.</div>';

      this.innerHTML = `
        <div class="fishy-overlay-panel space-y-4">
          <div class="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-between">
            <div class="space-y-1">
              <p class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/45">Personal Overlay</p>
              <h3 class="text-lg font-semibold">${escapeHtml(editor.zone_name || "Current zone proposal")}</h3>
              <p class="max-w-3xl text-sm text-base-content/72">These changes stay in browser storage only. Export the overlay JSON when you want to submit a proposal that can later be turned into a Dolt merge request.</p>
            </div>
            <div class="flex flex-wrap gap-2">
              <input type="file" class="hidden" accept=".json,application/json" data-import-file>
              <button type="button" class="btn btn-soft btn-secondary" data-action="import-json">Import JSON</button>
              <button type="button" class="btn btn-soft btn-secondary" data-action="export-json">Export JSON</button>
              <button type="button" class="btn btn-dash btn-warning" data-action="reset-zone">Reset Zone</button>
              <button type="button" class="btn btn-dash btn-error" data-action="reset-all">Reset All</button>
            </div>
          </div>

          <div class="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(18rem,0.8fr)]">
            <section class="rounded-box border border-base-300 bg-base-100 p-3">
              <div class="mb-3">
                <div class="text-sm font-semibold">Zone Groups</div>
                <div class="text-xs text-base-content/65">Edit raw base group rates only. Bonus and normalized values are read-only calculator outputs.</div>
              </div>
              <div class="mb-3 rounded-box border border-warning/20 bg-warning/8 px-3 py-2 text-xs leading-relaxed text-base-content/72">
                Normalized share uses effective raw weight, not just the raw input. The calculator adds any accrued group bonus to the raw % first, then normalizes all active groups to 100%. Hover <span class="font-semibold">Bonus</span> or <span class="font-semibold">Normalized</span> to inspect the computed-stat breakdown.
              </div>
              <div class="overflow-x-auto">
                <table class="table table-sm fishy-overlay-table">
                  <thead>
                    <tr>
                      <th>Group</th>
                      <th>Default</th>
                      <th>Present</th>
                      <th>Raw %</th>
                      <th>Bonus</th>
                      <th>Normalized</th>
                    </tr>
                  </thead>
                  <tbody>${groupMarkup}</tbody>
                </table>
              </div>
            </section>

            <section class="rounded-box border border-base-300 bg-base-100 p-3">
              <div class="mb-3">
                <div class="text-sm font-semibold">Current Changes</div>
                <div class="text-xs text-base-content/65">${changes.length} active overlay entr${changes.length === 1 ? "y" : "ies"} across zones and prices.</div>
              </div>
              <div class="space-y-2">${changeMarkup}</div>
            </section>
          </div>

          <section class="rounded-box border border-base-300 bg-base-100 p-3">
            <div class="mb-3">
              <div class="text-sm font-semibold">Zone Items</div>
              <div class="text-xs text-base-content/65">Change zone membership, raw item rates, or local item prices for the current calculator zone. Normalized results stay read-only.</div>
            </div>
            <div class="overflow-x-auto">
              <table class="table table-sm fishy-overlay-table fishy-overlay-item-table">
                <thead>
                  <tr>
                    <th>Item</th>
                    <th>Default</th>
                    <th>State</th>
                    <th>Raw %</th>
                    <th>Normalized</th>
                    <th>Base Price</th>
                  </tr>
                </thead>
                <tbody>${itemMarkup}</tbody>
              </table>
            </div>
          </section>

          <section class="rounded-box border border-base-300 bg-base-100 p-3">
            <div class="mb-3">
              <div class="text-sm font-semibold">Add Item</div>
              <div class="text-xs text-base-content/65">Use this for items that are missing from the current zone defaults. Added rows are overlay-only until submitted and merged into the source dataset.</div>
            </div>
            <div class="grid gap-3 md:grid-cols-6">
              <label class="fieldset">
                <span class="fieldset-legend">Item ID</span>
                <input type="number" min="1" step="1" class="input input-sm w-full" data-add-item-id>
              </label>
              <label class="fieldset md:col-span-2">
                <span class="fieldset-legend">Label</span>
                <input type="text" class="input input-sm w-full" data-add-item-name placeholder="Fish or item name">
              </label>
              <label class="fieldset">
                <span class="fieldset-legend">Group</span>
                <select class="select select-sm w-full" data-add-item-slot>${groupOptionsMarkup(4)}</select>
              </label>
              <label class="fieldset">
                <span class="fieldset-legend">Raw %</span>
                <input type="number" min="0" step="any" class="input input-sm w-full" data-add-item-rate value="0">
              </label>
              <label class="fieldset">
                <span class="fieldset-legend">Base Price</span>
                <input type="number" min="0" step="1" class="input input-sm w-full" data-add-item-price value="0">
              </label>
              <label class="fieldset">
                <span class="fieldset-legend">Grade</span>
                <select class="select select-sm w-full" data-add-item-grade>
                  <option value="">Auto</option>
                  <option value="Prize">Prize</option>
                  <option value="Rare">Rare</option>
                  <option value="HighQuality">High-Quality</option>
                  <option value="General">General</option>
                  <option value="Trash">Trash</option>
                </select>
              </label>
              <label class="fieldset">
                <span class="fieldset-legend">Fish</span>
                <label class="label cursor-pointer justify-start gap-2 rounded-box border border-base-300 bg-base-200 px-3 py-2">
                  <input type="checkbox" class="checkbox checkbox-sm checkbox-primary" data-add-item-is-fish checked>
                  <span class="text-sm">Is fish</span>
                </label>
              </label>
              <div class="fieldset md:col-span-2 self-end">
                <button type="button" class="btn btn-primary btn-sm" data-action="add-item">Add Overlay Item</button>
              </div>
            </div>
          </section>
        </div>
      `;
    }

    writeZoneOverlay(zoneKey, zoneOverlay) {
      const shared = sharedUserOverlays();
      if (!shared) {
        return;
      }
      const nextOverlay = overlaySnapshot();
      nextOverlay.zones = isPlainObject(nextOverlay.zones) ? nextOverlay.zones : {};
      if (isPlainObject(zoneOverlay) && (
        Object.keys(zoneOverlay.groups || {}).length
        || Object.keys(zoneOverlay.items || {}).length
      )) {
        nextOverlay.zones[zoneKey] = zoneOverlay;
      } else {
        delete nextOverlay.zones[zoneKey];
      }
      shared.setOverlaySignals(nextOverlay);
      replaceCalculatorSignalRoot("overlay", nextOverlay);
    }

    writePriceOverrides(priceOverrides) {
      const shared = sharedUserOverlays();
      if (!shared) {
        return;
      }
      shared.setPriceOverrides(priceOverrides);
      replaceCalculatorSignalRoot("priceOverrides", priceOverrides);
    }

    openImportPicker() {
      const input = this.querySelector("[data-import-file]");
      if (!input) {
        return;
      }
      input.value = "";
      if (typeof input.showPicker === "function") {
        try {
          input.showPicker();
          return;
        } catch (_error) {}
      }
      input.click?.();
    }

    async importOverlayFile(file, inputElement = null) {
      try {
        if (!file) {
          return null;
        }
        const shared = sharedUserOverlays();
        if (!shared?.importText) {
          throw new Error("Overlay import is unavailable.");
        }
        const importedSnapshot = shared.importText(await readTextFile(file));
        replaceCalculatorSignalRoot("overlay", importedSnapshot.overlay || { zones: {} });
        replaceCalculatorSignalRoot("priceOverrides", importedSnapshot.priceOverrides || {});
        globalThis.window?.__fishystuffToast?.success?.("Overlay JSON imported.");
        return importedSnapshot;
      } catch (error) {
        globalThis.window?.__fishystuffToast?.error?.(
          trimString(error?.message) || "Overlay JSON import failed.",
        );
        return null;
      } finally {
        if (inputElement && "value" in inputElement) {
          inputElement.value = "";
        }
      }
    }

    updateGroup(slotIdx, inputValue) {
      const editor = currentOverlayEditor();
      const zoneKey = trimString(editor.zone_rgb_key);
      if (!zoneKey) {
        return;
      }
      const baseRow = (editor.groups || []).find((row) => Number(row.slot_idx) === Number(slotIdx));
      if (!baseRow) {
        return;
      }
      const zoneOverlay = cloneJson(currentZoneOverlay(zoneKey));
      zoneOverlay.groups = isPlainObject(zoneOverlay.groups) ? zoneOverlay.groups : {};
      const entry = isPlainObject(zoneOverlay.groups[String(slotIdx)])
        ? cloneJson(zoneOverlay.groups[String(slotIdx)])
        : {};
      const presentInput = this.querySelector(`[data-group-present="${slotIdx}"]`);
      const present = presentInput?.checked === true;
      const rawRatePercent = normalizeNumber(inputValue);
      if (present !== (baseRow.default_present === true)) {
        entry.present = present;
      } else {
        delete entry.present;
      }
      if (
        rawRatePercent != null
        && rawRatePercent !== (Number(baseRow.default_raw_rate_pct) || 0)
      ) {
        entry.rawRatePercent = Math.max(0, rawRatePercent);
      } else {
        delete entry.rawRatePercent;
      }
      if (Object.keys(entry).length) {
        zoneOverlay.groups[String(slotIdx)] = entry;
      } else {
        delete zoneOverlay.groups[String(slotIdx)];
      }
      this.writeZoneOverlay(zoneKey, zoneOverlay);
    }

    updateItem(itemId) {
      const editor = currentOverlayEditor();
      const zoneKey = trimString(editor.zone_rgb_key);
      if (!zoneKey) {
        return;
      }
      const row = buildDisplayRows(editor).find((candidate) => String(candidate.item_id) === String(itemId));
      if (!row) {
        return;
      }
      const presentInput = this.querySelector(`[data-item-present="${itemId}"]`);
      const slotInput = this.querySelector(`[data-item-slot="${itemId}"]`);
      const rateInput = this.querySelector(`[data-item-rate="${itemId}"]`);
      const priceInput = this.querySelector(`[data-item-price="${itemId}"]`);
      const present = presentInput?.checked === true;
      const slotIdx = Number.parseInt(slotInput?.value, 10) || Number(row.slot_idx) || 4;
      const rawRatePercent = normalizeNumber(rateInput?.value);
      const basePrice = normalizeNumber(priceInput?.value);
      const zoneOverlay = cloneJson(currentZoneOverlay(zoneKey));
      zoneOverlay.groups = isPlainObject(zoneOverlay.groups) ? zoneOverlay.groups : {};
      zoneOverlay.items = isPlainObject(zoneOverlay.items) ? zoneOverlay.items : {};
      const entry = {};
      const isAddedRow = row.default_present === false;
      if (isAddedRow) {
        if (!present) {
          delete zoneOverlay.items[String(itemId)];
        } else {
          entry.present = true;
          entry.slotIdx = slotIdx;
          entry.rawRatePercent = Math.max(0, rawRatePercent ?? 0);
          entry.name = trimString(row.label) || String(itemId);
          if (trimString(row.icon_grade_tone) && trimString(row.icon_grade_tone) !== "unknown") {
            const gradeMap = {
              red: "Prize",
              yellow: "Rare",
              blue: "HighQuality",
              green: "General",
              white: "Trash",
            };
            const mappedGrade = gradeMap[trimString(row.icon_grade_tone)];
            if (mappedGrade) {
              entry.grade = mappedGrade;
            }
          }
          entry.isFish = row.is_fish !== false;
          zoneOverlay.items[String(itemId)] = entry;
        }
      } else {
        if (!present) {
          entry.present = false;
        }
        if (slotIdx !== Number(row.slot_idx)) {
          entry.slotIdx = slotIdx;
        }
        if (
          rawRatePercent != null
          && rawRatePercent !== (Number(row.default_raw_rate_pct) || 0)
        ) {
          entry.rawRatePercent = Math.max(0, rawRatePercent);
        }
        if (Object.keys(entry).length) {
          zoneOverlay.items[String(itemId)] = entry;
        } else {
          delete zoneOverlay.items[String(itemId)];
        }
      }
      const priceOverrides = cloneJson(priceOverrideSnapshot());
      if (
        basePrice != null
        && Math.abs(basePrice - (Number(row.base_price_raw) || 0)) > 0.5
      ) {
        priceOverrides[String(itemId)] = {
          ...(isPlainObject(priceOverrides[String(itemId)]) ? priceOverrides[String(itemId)] : {}),
          basePrice: Math.max(0, Math.round(basePrice)),
        };
      } else if (isPlainObject(priceOverrides[String(itemId)])) {
        delete priceOverrides[String(itemId)].basePrice;
        if (!Object.keys(priceOverrides[String(itemId)]).length) {
          delete priceOverrides[String(itemId)];
        }
      }
      this.writeZoneOverlay(zoneKey, zoneOverlay);
      this.writePriceOverrides(priceOverrides);
    }

    addItemFromForm() {
      const editor = currentOverlayEditor();
      const zoneKey = trimString(editor.zone_rgb_key);
      if (!zoneKey) {
        return;
      }
      const itemId = Number.parseInt(this.querySelector("[data-add-item-id]")?.value, 10);
      const name = trimString(this.querySelector("[data-add-item-name]")?.value);
      const slotIdx = Number.parseInt(this.querySelector("[data-add-item-slot]")?.value, 10) || 4;
      const rawRatePercent = Math.max(
        0,
        normalizeNumber(this.querySelector("[data-add-item-rate]")?.value) ?? 0,
      );
      const basePrice = Math.max(
        0,
        Math.round(normalizeNumber(this.querySelector("[data-add-item-price]")?.value) ?? 0),
      );
      const grade = trimString(this.querySelector("[data-add-item-grade]")?.value);
      const isFish = this.querySelector("[data-add-item-is-fish]")?.checked !== false;
      if (!Number.isInteger(itemId) || itemId <= 0 || !name) {
        globalThis.window?.__fishystuffToast?.info?.("Add-item form needs an item ID and label.");
        return;
      }
      const zoneOverlay = cloneJson(currentZoneOverlay(zoneKey));
      zoneOverlay.groups = isPlainObject(zoneOverlay.groups) ? zoneOverlay.groups : {};
      zoneOverlay.items = isPlainObject(zoneOverlay.items) ? zoneOverlay.items : {};
      zoneOverlay.items[String(itemId)] = {
        present: true,
        slotIdx,
        rawRatePercent,
        name,
        grade: grade || undefined,
        isFish,
      };
      const priceOverrides = cloneJson(priceOverrideSnapshot());
      if (basePrice > 0) {
        priceOverrides[String(itemId)] = {
          ...(isPlainObject(priceOverrides[String(itemId)]) ? priceOverrides[String(itemId)] : {}),
          basePrice,
        };
      }
      this.writeZoneOverlay(zoneKey, zoneOverlay);
      this.writePriceOverrides(priceOverrides);
      this.querySelector("[data-add-item-id]").value = "";
      this.querySelector("[data-add-item-name]").value = "";
      this.querySelector("[data-add-item-rate]").value = "0";
      this.querySelector("[data-add-item-price]").value = "0";
      this.querySelector("[data-add-item-grade]").value = "";
      this.querySelector("[data-add-item-is-fish]").checked = true;
    }

    resetEntry(kind, zoneKey, slotKey, itemId) {
      if (kind === "group") {
        const zoneOverlay = cloneJson(currentZoneOverlay(zoneKey));
        zoneOverlay.groups = isPlainObject(zoneOverlay.groups) ? zoneOverlay.groups : {};
        delete zoneOverlay.groups[String(slotKey)];
        this.writeZoneOverlay(zoneKey, zoneOverlay);
        return;
      }
      if (kind === "item") {
        const zoneOverlay = cloneJson(currentZoneOverlay(zoneKey));
        zoneOverlay.items = isPlainObject(zoneOverlay.items) ? zoneOverlay.items : {};
        delete zoneOverlay.items[String(itemId)];
        this.writeZoneOverlay(zoneKey, zoneOverlay);
        return;
      }
      if (kind === "price") {
        const priceOverrides = cloneJson(priceOverrideSnapshot());
        delete priceOverrides[String(itemId)];
        this.writePriceOverrides(priceOverrides);
      }
    }

    handleClick(event) {
      const button = event.target.closest("button");
      if (!button) {
        return;
      }
      const action = trimString(button.getAttribute("data-action"));
      if (action === "import-json") {
        this.openImportPicker();
        return;
      }
      if (action === "export-json") {
        const exported = sharedUserOverlays()?.exportText?.() || "{}";
        const filename = `fishystuff-overlay-${new Date().toISOString().slice(0, 10)}.json`;
        if (downloadText(filename, exported)) {
          globalThis.window?.__fishystuffToast?.info?.("Overlay JSON downloaded.");
        }
        return;
      }
      if (action === "reset-zone") {
        const editor = currentOverlayEditor();
        this.writeZoneOverlay(trimString(editor.zone_rgb_key), {});
        return;
      }
      if (action === "reset-all") {
        sharedUserOverlays()?.clearAll?.();
        replaceCalculatorSignalRoot("overlay", { zones: {} });
        replaceCalculatorSignalRoot("priceOverrides", {});
        return;
      }
      if (action === "add-item") {
        this.addItemFromForm();
        return;
      }
      const resetKind = trimString(button.getAttribute("data-reset-kind"));
      if (resetKind) {
        this.resetEntry(
          resetKind,
          trimString(button.getAttribute("data-reset-zone")),
          trimString(button.getAttribute("data-reset-slot")),
          trimString(button.getAttribute("data-reset-item")),
        );
      }
    }

    handleChange(event) {
      const target = event.target;
      if (!target || typeof target.getAttribute !== "function" || typeof target.hasAttribute !== "function") {
        return;
      }
      if (target.hasAttribute("data-import-file")) {
        void this.importOverlayFile(target.files?.[0], target);
        return;
      }
      if (target.hasAttribute("data-group-present") || target.hasAttribute("data-group-rate")) {
        const slotIdx = trimString(
          target.getAttribute("data-group-present") || target.getAttribute("data-group-rate"),
        );
        const rateInput = this.querySelector(`[data-group-rate="${slotIdx}"]`);
        this.updateGroup(slotIdx, rateInput?.value);
        return;
      }
      if (
        target.hasAttribute("data-item-present")
        || target.hasAttribute("data-item-slot")
        || target.hasAttribute("data-item-rate")
        || target.hasAttribute("data-item-price")
      ) {
        const itemId = trimString(
          target.getAttribute("data-item-present")
          || target.getAttribute("data-item-slot")
          || target.getAttribute("data-item-rate")
          || target.getAttribute("data-item-price"),
        );
        this.updateItem(itemId);
      }
    }
  }

  if (
    globalThis.customElements
    && typeof globalThis.customElements.get === "function"
    && typeof globalThis.customElements.define === "function"
    && !globalThis.customElements.get(TAG_NAME)
  ) {
    globalThis.customElements.define(TAG_NAME, FishyCalculatorOverlayPanel);
  }
})();
