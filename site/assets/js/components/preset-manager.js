import { DATASTAR_SIGNAL_PATCH_EVENT } from "../datastar-signals.js";

const TAG_NAME = "fishy-preset-manager";
const LANGUAGE_CHANGE_EVENT = "fishystuff:languagechange";
const DEFAULT_TITLE = "Saved presets";
const ICON_SPRITE_FALLBACK_URL = "/img/icons.svg";
const FIXED_CARD_PREFIX = "fixed:";
const PRESET_CARD_PREFIX = "preset:";
const CURRENT_CARD_PREFIX = "current:";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function stableJson(value) {
  return JSON.stringify(value ?? null);
}

function formatText(text, vars = {}) {
  return String(text ?? "").replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) => {
    return Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : "";
  });
}

function languageHelper() {
  const helper = globalThis.window?.__fishystuffLanguage;
  return helper && typeof helper.t === "function" ? helper : null;
}

function toastHelper() {
  return globalThis.window?.__fishystuffToast ?? null;
}

function presetHelper() {
  return globalThis.window?.__fishystuffUserPresets ?? null;
}

function presetPreviewHelper() {
  return globalThis.window?.__fishystuffPresetPreviews ?? null;
}

function iconSpriteUrl() {
  return trimString(globalThis.window?.__fishystuffCalculator?.iconSpriteUrl) || ICON_SPRITE_FALLBACK_URL;
}

function iconMarkup(alias, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${iconSpriteUrl()}#fishy-${alias}"></use></svg>`;
}

function createIconElement(alias, className = "") {
  const normalizedAlias = trimString(alias);
  if (!normalizedAlias) {
    return null;
  }
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("class", trimString(`fishy-icon ${className}`) || "fishy-icon");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("width", "100%");
  use.setAttribute("height", "100%");
  use.setAttribute("href", `${iconSpriteUrl()}#fishy-${normalizedAlias}`);
  svg.append(use);
  return svg;
}

function downloadTextFile(filename, text) {
  const blob = new Blob([String(text ?? "")], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = trimString(filename) || "presets.json";
  link.style.display = "none";
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

function setDialogOpen(dialog, open) {
  if (!(dialog instanceof HTMLElement)) {
    return;
  }
  if (open) {
    if ("open" in dialog && dialog.open) {
      return;
    }
    if (typeof dialog.showModal === "function") {
      dialog.showModal();
      return;
    }
    dialog.setAttribute("open", "");
    return;
  }
  if ("open" in dialog && !dialog.open) {
    return;
  }
  if (typeof dialog.close === "function") {
    dialog.close();
    return;
  }
  dialog.removeAttribute("open");
}

function presetCardKey(presetId) {
  return `${PRESET_CARD_PREFIX}${trimString(presetId)}`;
}

function fixedCardKey(fixedId) {
  return `${FIXED_CARD_PREFIX}${trimString(fixedId)}`;
}

function currentCardKey(source) {
  const normalized = normalizeSource(source);
  return `${CURRENT_CARD_PREFIX}${sourceKey(normalized)}`;
}

function presetIdFromCardKey(cardKey) {
  const normalized = trimString(cardKey);
  return normalized.startsWith(PRESET_CARD_PREFIX) ? trimString(normalized.slice(PRESET_CARD_PREFIX.length)) : "";
}

function normalizeSource(value) {
  const source = isPlainObject(value) ? value : {};
  const kind = trimString(source.kind).toLowerCase();
  const id = trimString(source.id);
  if ((kind === "preset" || kind === "fixed") && id) {
    return { kind, id };
  }
  return { kind: "none", id: "" };
}

function sourceKey(source) {
  const normalized = normalizeSource(source);
  return normalized.kind === "none" ? "none" : `${normalized.kind}:${normalized.id}`;
}

function sourceMatchesCard(source, cardKey) {
  const normalized = normalizeSource(source);
  if (normalized.kind === "preset") {
    return presetCardKey(normalized.id) === cardKey;
  }
  if (normalized.kind === "fixed") {
    return fixedCardKey(normalized.id) === cardKey;
  }
  return false;
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

export function patchTouchesPresetManager(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  return patch._user_presets != null || patch._preset_manager_ui != null;
}

function isFocused(element) {
  return Boolean(element) && globalThis.document?.activeElement === element;
}

function setElementText(element, text) {
  if (element) {
    element.textContent = String(text ?? "");
  }
}

function normalizePayload(adapter, payload) {
  if (adapter && typeof adapter.normalizePayload === "function") {
    return cloneJson(adapter.normalizePayload(payload));
  }
  if (isPlainObject(payload)) {
    return cloneJson(payload);
  }
  return null;
}

export class FishyPresetManager extends HTMLElementBase {
  constructor() {
    super();
    this.handleLanguageChange = this.handleLanguageChange.bind(this);
    this.handleSignalPatch = this.handleSignalPatch.bind(this);
    this.selectedCardKey = "";
    this.lastSelectedTitleSource = "";
  }

  connectedCallback() {
    if (this.dataset.presetManagerReady === "true") {
      return;
    }
    this.dataset.presetManagerReady = "true";
    this.render();
    globalThis.window?.addEventListener?.(LANGUAGE_CHANGE_EVENT, this.handleLanguageChange);
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, this.handleSignalPatch);
    this.sync({ refreshNames: true });
  }

  disconnectedCallback() {
    globalThis.window?.removeEventListener?.(LANGUAGE_CHANGE_EVENT, this.handleLanguageChange);
    document.removeEventListener(DATASTAR_SIGNAL_PATCH_EVENT, this.handleSignalPatch);
  }

  get collectionKey() {
    return trimString(this.dataset.presetCollection || this.getAttribute("data-preset-collection"));
  }

  get fileBaseName() {
    const adapter = this.adapter();
    return trimString(adapter?.fileBaseName) || `${this.collectionKey || "presets"}`;
  }

  adapter() {
    return presetHelper()?.collectionAdapter?.(this.collectionKey) ?? null;
  }

  translate(key, fallback, vars = {}) {
    const normalizedFallback = formatText(trimString(fallback), vars);
    const normalizedKey = trimString(key);
    if (!normalizedKey) {
      return normalizedFallback;
    }
    const helper = languageHelper();
    if (!helper) {
      return normalizedFallback || normalizedKey;
    }
    const translated = helper.t(normalizedKey, vars);
    if (!translated || translated === normalizedKey) {
      return normalizedFallback || normalizedKey;
    }
    return translated;
  }

  titleText() {
    const adapter = this.adapter();
    return this.translate(adapter?.titleKey, adapter?.titleFallback || DEFAULT_TITLE);
  }

  openLabelText() {
    const adapter = this.adapter();
    if (adapter?.openLabelKey) {
      return this.translate(adapter.openLabelKey, adapter?.openLabelFallback || "Preset Manager");
    }
    if (adapter?.openLabelFallback) {
      return this.translate("", adapter.openLabelFallback);
    }
    return this.translate(
      "presets.button.open_manager",
      "Preset Manager",
    );
  }

  defaultName(index) {
    const adapter = this.adapter();
    if (adapter && typeof adapter.defaultPresetName === "function") {
      const label = trimString(adapter.defaultPresetName(index));
      if (label) {
        return label;
      }
    }
    return this.translate("", `Preset ${Math.max(1, Number.parseInt(index, 10) || 1)}`);
  }

  selectedTitleInputId() {
    return `fishy-preset-title-${this.uiStateKey().replace(/[^a-zA-Z0-9_-]+/g, "-") || "default"}`;
  }

  uiStateKey() {
    return this.collectionKey || "default";
  }

  uiSignalsExpression() {
    return JSON.stringify({
      _preset_manager_ui: {
        [this.uiStateKey()]: {
          open: false,
        },
      },
    });
  }

  uiOpenExpression() {
    return `$_preset_manager_ui[${JSON.stringify(this.uiStateKey())}].open`;
  }

  render() {
    const openExpression = this.uiOpenExpression();
    const selectedTitleInputId = this.selectedTitleInputId();
    this.classList.add("fishy-preset-manager");
    this.innerHTML = `
      <div data-signals='${this.uiSignalsExpression()}'>
        <div hidden data-effect="window.__fishystuffPresetManager.refresh(el, $_user_presets.version)"></div>
        <button
          type="button"
          class="btn btn-soft btn-secondary"
          data-role="open"
          data-on:click='window.__fishystuffPresetManager.refresh(el); ${openExpression} = true'
        >
          ${iconMarkup("layout-fill", "size-5")}
          <span data-role="open-label"></span>
        </button>
        <input
          class="hidden"
          type="file"
          accept="application/json,.json"
          data-role="file-input"
          data-on:change='window.__fishystuffPresetManager.importFile(el)'
        >
        <dialog
          class="modal"
          data-role="manager-dialog"
          data-effect='window.__fishystuffPresetManager.syncDialog(el, ${openExpression})'
          data-on:close='${openExpression} = false'
        >
          <div class="modal-box w-11/12 max-w-6xl p-0">
            <div class="flex items-center justify-between gap-4 border-b border-base-300/70 px-6 py-4">
              <div class="inline-flex min-w-0 items-center gap-3">
                ${iconMarkup("layout-fill", "size-5")}
                <h3 class="text-lg font-semibold text-base-content" data-role="manager-title"></h3>
              </div>
              <form method="dialog" class="shrink-0">
                <button type="submit" class="btn btn-sm btn-circle btn-ghost" data-role="close" aria-label="Close">
                  ${iconMarkup("clear", "size-4")}
                </button>
              </form>
            </div>
            <div class="grid gap-4 px-6 py-5">
              <section class="card card-border bg-base-100">
                <div class="card-body gap-4">
                  <div class="flex flex-wrap items-center justify-between gap-2">
                    <div class="inline-flex min-w-0 items-center gap-2">
                      <div class="text-sm font-semibold text-base-content" data-role="selected-section-title"></div>
                      <span class="badge badge-sm fishy-preset-manager__status" data-role="status"></span>
                    </div>
                    <span class="badge badge-sm badge-outline" data-role="grid-count"></span>
                  </div>
                  <fieldset class="fieldset">
                    <label class="fieldset-legend" for="${selectedTitleInputId}" data-role="selected-title-label"></label>
                    <input
                      id="${selectedTitleInputId}"
                      name="${selectedTitleInputId}"
                      type="text"
                      class="input input-bordered w-full"
                      data-role="selected-title-input"
                      data-on:input__debounce.180ms='window.__fishystuffPresetManager.commitSelectedTitle(el)'
                      data-on:blur='window.__fishystuffPresetManager.commitSelectedTitle(el)'
                      data-on:keydown='window.__fishystuffPresetManager.handleSelectedTitleKeydown(evt, el)'
                    >
                  </fieldset>
                  <div class="flex flex-wrap gap-2">
                    <button type="button" class="btn btn-primary" data-role="save" data-on:click='window.__fishystuffPresetManager.saveCurrent(el)'></button>
                    <button type="button" class="btn btn-warning btn-outline" data-role="discard" data-on:click='window.__fishystuffPresetManager.discardCurrent(el)'></button>
                    <button type="button" class="btn btn-outline" data-role="copy" data-on:click='window.__fishystuffPresetManager.copySelected(el)'></button>
                    <button type="button" class="btn btn-outline" data-role="export" data-on:click='window.__fishystuffPresetManager.exportSelected(el)'></button>
                    <button type="button" class="btn btn-outline" data-role="import" data-on:click='window.__fishystuffPresetManager.openImport(el)'></button>
                    <button type="button" class="btn btn-error btn-outline" data-role="delete" data-on:click='window.__fishystuffPresetManager.deleteSelected(el)'></button>
                  </div>
                </div>
              </section>
              <section class="card card-border bg-base-100">
                <div class="card-body gap-3">
                  <div class="flex flex-wrap items-center justify-between gap-2">
                    <div class="card-title text-base" data-role="grid-title"></div>
                  </div>
                  <div class="fishy-preset-manager__preset-grid" data-role="preset-cards"></div>
                  <p class="text-sm text-base-content/55" data-role="grid-empty"></p>
                </div>
              </section>
            </div>
          </div>
          <form method="dialog" class="modal-backdrop">
            <button type="submit" data-role="backdrop-close">close</button>
          </form>
        </dialog>
      </div>
    `;
  }

  element(role) {
    return this.querySelector(`[data-role="${role}"]`);
  }

  cardsContainer() {
    return this.element("preset-cards");
  }

  dialogElement() {
    return this.element("manager-dialog");
  }

  fileInput() {
    return this.element("file-input");
  }

  selectedTitleInput() {
    return this.element("selected-title-input");
  }

  button(role) {
    return this.element(role);
  }

  currentPayload() {
    return presetHelper()?.capturePayload?.(this.collectionKey) ?? null;
  }

  capturePayload(options = {}) {
    return presetHelper()?.capturePayload?.(this.collectionKey, options) ?? null;
  }

  activePresetId() {
    return presetHelper()?.selectedPresetId?.(this.collectionKey) ?? "";
  }

  activeFixedId() {
    return presetHelper()?.selectedFixedId?.(this.collectionKey) ?? "";
  }

  titleIconAlias(item) {
    return presetPreviewHelper()?.titleIconAlias?.(this.collectionKey, {
      adapter: this.adapter(),
      item: cloneJson(item),
      payload: cloneJson(item?.payload),
    }) || "";
  }

  fixedItems() {
    const adapter = this.adapter();
    const entries = adapter && typeof adapter.fixedPresets === "function" ? adapter.fixedPresets() : [];
    if (!Array.isArray(entries)) {
      return [];
    }
    return entries
      .map((entry, index) => {
        const normalizedEntry = isPlainObject(entry) ? entry : {};
        const id = trimString(normalizedEntry.id) || `fixed_${index + 1}`;
        const name = trimString(normalizedEntry.name) || `Fixed ${index + 1}`;
        const payload = normalizePayload(adapter, normalizedEntry.payload);
        if (!payload) {
          return null;
        }
        return {
          key: fixedCardKey(id),
          kind: "fixed",
          id,
          name,
          payload,
          source: { kind: "fixed", id },
          editableName: false,
          removable: false,
        };
      })
      .filter(Boolean);
  }

  presetItems() {
    return (presetHelper()?.presets?.(this.collectionKey) ?? []).map((preset) => ({
      key: presetCardKey(preset.id),
      kind: "preset",
      id: preset.id,
      name: preset.name,
      payload: cloneJson(preset.payload),
      source: { kind: "preset", id: preset.id },
      editableName: true,
      removable: true,
    }));
  }

  currentItem(baseItems) {
    const helper = presetHelper();
    const current = helper?.current?.(this.collectionKey) ?? null;
    const payload = normalizePayload(this.adapter(), current?.payload);
    if (!current || !payload) {
      return null;
    }
    const origin = normalizeSource(current.origin);
    const sourceItem = baseItems.find((item) => sourceMatchesCard(origin, item.key)) || null;
    if (sourceItem && this.payloadsEqual(sourceItem.payload, payload)) {
      return null;
    }
    const sourceName = trimString(sourceItem?.name);
    return {
      key: currentCardKey(origin),
      kind: "current",
      id: sourceKey(origin),
      name: sourceName
        ? this.translate("presets.current.modified_from", "Modified: {$name}", { name: sourceName })
        : this.translate("presets.current.modified", "Modified current preset"),
      payload,
      source: origin,
      sourceKey: sourceItem?.key || "",
      editableName: false,
      removable: false,
    };
  }

  cardItems() {
    const fixedItems = this.fixedItems();
    const presetItems = this.presetItems();
    const baseItems = [
      ...fixedItems,
      ...presetItems,
    ];
    const currentItem = this.currentItem(baseItems);
    const items = [];
    let insertedCurrent = false;
    for (const item of baseItems) {
      items.push(item);
      if (currentItem && sourceMatchesCard(currentItem.source, item.key)) {
        items.push(currentItem);
        insertedCurrent = true;
      }
    }
    if (currentItem && !insertedCurrent) {
      items.unshift(currentItem);
    }
    const currentPayload = this.currentPayload();
    return {
      currentPayload,
      fixedItems,
      presetItems,
      currentItem,
      items,
    };
  }

  findItem(items, cardKey) {
    const normalizedKey = trimString(cardKey);
    return items.find((item) => item.key === normalizedKey) || null;
  }

  selectedItem(items) {
    return this.findItem(items, this.selectedCardKey);
  }

  selectedSavedPreset() {
    const presetId = presetIdFromCardKey(this.selectedCardKey);
    return presetId ? (presetHelper()?.preset?.(this.collectionKey, presetId) ?? null) : null;
  }

  sourceItemForCurrent(currentItem, items = this.cardItems().items) {
    if (!currentItem) {
      return null;
    }
    return items.find((item) => sourceMatchesCard(currentItem.source, item.key)) || null;
  }

  linkedSavedPresetForCurrent(currentItem = this.cardItems().currentItem) {
    if (!currentItem || currentItem.source?.kind !== "preset") {
      return null;
    }
    return presetHelper()?.preset?.(this.collectionKey, currentItem.source.id) ?? null;
  }

  ensureSelectedCard(items, activePresetId, activeFixedId = "") {
    const existing = this.selectedItem(items);
    if (existing) {
      return false;
    }
    const currentPayload = this.currentPayload();
    const currentItem = items.find((item) => item.kind === "current" && this.isCardApplied(item, activePresetId, currentPayload));
    if (currentItem) {
      this.selectedCardKey = currentItem.key;
      return true;
    }
    const activeCardKey = activePresetId ? presetCardKey(activePresetId) : "";
    if (activeCardKey && this.findItem(items, activeCardKey)) {
      this.selectedCardKey = activeCardKey;
      return true;
    }
    const activeFixedCardKey = activeFixedId ? fixedCardKey(activeFixedId) : "";
    if (activeFixedCardKey && this.findItem(items, activeFixedCardKey)) {
      this.selectedCardKey = activeFixedCardKey;
      return true;
    }
    const appliedItem = items.find((item) => this.isCardApplied(item, activePresetId, currentPayload));
    if (appliedItem) {
      this.selectedCardKey = appliedItem.key;
      return true;
    }
    this.selectedCardKey = items[0]?.key || "";
    return true;
  }

  isPresetActive(item, activePresetId) {
    return item?.kind === "preset" && item.id === trimString(activePresetId);
  }

  isFixedActive(item, activePresetId, currentPayload) {
    return item?.kind === "fixed"
      && !trimString(activePresetId)
      && currentPayload
      && this.payloadsEqual(currentPayload, item.payload);
  }

  isCurrentActive(item, _activePresetId, currentPayload) {
    return item?.kind === "current"
      && currentPayload
      && this.payloadsEqual(currentPayload, item.payload);
  }

  isCardApplied(item, activePresetId, currentPayload) {
    if (!item || !currentPayload) {
      return false;
    }
    if (item.kind === "current") {
      return this.isCurrentActive(item, activePresetId, currentPayload);
    }
    if (item.kind === "fixed") {
      return this.isFixedActive(item, activePresetId, currentPayload);
    }
    return this.isPresetActive(item, activePresetId)
      && this.payloadsEqual(item.payload, currentPayload);
  }

  cardBadge(item, activePresetId, currentPayload) {
    if (!item) {
      return null;
    }
    if (item.kind === "current") {
      return {
        className: "badge badge-sm badge-warning",
        text: this.translate("presets.status.modified", "Modified"),
      };
    }
    const current = presetHelper()?.current?.(this.collectionKey) ?? null;
    if (current && sourceMatchesCard(current.origin, item.key)) {
      return {
        className: "badge badge-sm badge-outline",
        text: this.translate("presets.status.original", "Original"),
      };
    }
    return null;
  }

  selectedStatus(item, _activePresetId, _currentPayload) {
    if (!item) {
      return {
        className: "badge badge-sm badge-outline",
        text: "",
      };
    }
    if (item.kind === "current") {
      return {
        className: "badge badge-sm badge-warning",
        text: this.translate("presets.status.modified", "Modified"),
      };
    }
    if (item.kind === "fixed") {
      return {
        className: "badge badge-sm badge-outline",
        text: this.translate("presets.status.default", "Default"),
      };
    }
    return {
      className: "badge badge-sm badge-ghost",
      text: this.translate("presets.status.saved", "Saved"),
    };
  }

  selectedTitleValue(item) {
    return item?.name || "";
  }

  copyPayload(item) {
    return item ? cloneJson(item.payload) : null;
  }

  clonePayload(item) {
    const adapter = this.adapter();
    if (adapter?.captureOnClone === true) {
      return this.capturePayload({
        intent: "clone",
        source: cloneJson(item?.source || {}),
        payload: item?.payload ? cloneJson(item.payload) : null,
      });
    }
    return this.copyPayload(item);
  }

  payloadsEqual(left, right) {
    if (!left || !right) {
      return false;
    }
    const adapter = this.adapter();
    const normalizedLeft = normalizePayload(adapter, left);
    const normalizedRight = normalizePayload(adapter, right);
    if (!normalizedLeft || !normalizedRight) {
      return false;
    }
    if (adapter && typeof adapter.payloadsEqual === "function") {
      try {
        return adapter.payloadsEqual(normalizedLeft, normalizedRight) === true;
      } catch (_error) {
        return false;
      }
    }
    return stableJson(normalizedLeft) === stableJson(normalizedRight);
  }

  sync({ refreshNames = false } = {}) {
    const helper = presetHelper();
    const adapter = this.adapter();
    const canInteract = Boolean(helper && adapter && this.collectionKey);
    const { items, presetItems, currentPayload, currentItem } = this.cardItems();
    const activePresetId = this.activePresetId();
    const activeFixedId = this.activeFixedId();
    const selectionChanged = this.ensureSelectedCard(items, activePresetId, activeFixedId);
    const selectedItem = this.selectedItem(items);
    const selectedSavedPreset = this.selectedSavedPreset();
    const linkedSavedPreset = this.linkedSavedPresetForCurrent(currentItem);
    const linkedSourceItem = this.sourceItemForCurrent(currentItem, items);
    const copyItem = currentItem || selectedItem;
    const shouldHighlightCopy = Boolean(currentItem && !linkedSavedPreset);
    const canSaveCurrent = Boolean(currentItem && linkedSavedPreset);
    const canSaveSelectedSnapshot = Boolean(!currentItem && selectedSavedPreset && adapter?.captureOnSave === true);

    setElementText(this.element("open-label"), this.openLabelText());
    setElementText(this.element("manager-title"), this.titleText());
    setElementText(
      this.element("grid-title"),
      this.translate("presets.grid.title", "Presets"),
    );
    setElementText(
      this.element("grid-count"),
      this.translate("presets.grid.count", "{$count} saved", { count: String(presetItems.length) }),
    );
    const gridEmpty = this.element("grid-empty");
    if (gridEmpty) {
      gridEmpty.textContent = presetItems.length
        ? ""
        : this.translate("presets.grid.empty", "No saved presets yet.");
      gridEmpty.hidden = presetItems.length > 0;
    }

    this.renderCards(items, activePresetId, currentPayload);

    const selectedStatus = this.selectedStatus(selectedItem, activePresetId, currentPayload);
    const status = this.element("status");
    if (status) {
      status.className = `fishy-preset-manager__status ${selectedStatus.className}`;
      status.textContent = selectedStatus.text;
    }

    setElementText(
      this.element("selected-section-title"),
      this.translate("presets.section.selected.title", "Selected preset"),
    );
    const selectedTitleInput = this.selectedTitleInput();
    if (selectedTitleInput instanceof HTMLInputElement) {
      const selectedTitleSource = selectedItem?.key || "";
      if (refreshNames || selectionChanged || this.lastSelectedTitleSource !== selectedTitleSource || !isFocused(selectedTitleInput)) {
        selectedTitleInput.value = this.selectedTitleValue(selectedItem);
      }
      this.lastSelectedTitleSource = selectedTitleSource;
      selectedTitleInput.disabled = !selectedSavedPreset;
      selectedTitleInput.readOnly = !selectedSavedPreset;
      selectedTitleInput.placeholder = this.selectedTitleValue(selectedItem);
      selectedTitleInput.setAttribute(
        "aria-label",
        this.translate("presets.field.selected_title.label", "Selected title"),
      );
    }
    setElementText(
      this.element("selected-title-label"),
      this.translate("presets.field.selected_title.label", "Title"),
    );

    const saveButton = this.button("save");
    if (saveButton) {
      saveButton.innerHTML = `${iconMarkup("check-badge-solid", "size-4")}<span>${this.translate("presets.button.save", "Save")}</span>`;
      saveButton.disabled = !canInteract || (!canSaveCurrent && !canSaveSelectedSnapshot);
    }

    const discardButton = this.button("discard");
    if (discardButton) {
      discardButton.innerHTML = `${iconMarkup("clear", "size-4")}<span>${this.translate("presets.button.discard", "Discard")}</span>`;
      discardButton.disabled = !canInteract || !currentItem || !linkedSourceItem;
    }

    const copyButton = this.button("copy");
    if (copyButton) {
      copyButton.className = shouldHighlightCopy ? "btn btn-primary" : "btn btn-outline";
      copyButton.innerHTML = `${iconMarkup("copy", "size-4")}<span>${this.translate("presets.button.copy", "Clone")}</span>`;
      copyButton.disabled = !canInteract || !copyItem || !(adapter?.captureOnClone === true || this.copyPayload(copyItem));
    }

    const exportButton = this.button("export");
    if (exportButton) {
      exportButton.innerHTML = `${iconMarkup("export", "size-4")}<span>${this.translate("presets.button.export", "Export")}</span>`;
      exportButton.disabled = !canInteract || !selectedItem || !selectedItem.payload;
    }
    const importButton = this.button("import");
    if (importButton) {
      importButton.innerHTML = `${iconMarkup("import", "size-4")}<span>${this.translate("presets.button.import", "Import")}</span>`;
      importButton.disabled = !canInteract;
    }

    const deleteButton = this.button("delete");
    if (deleteButton) {
      deleteButton.innerHTML = `${iconMarkup("trash", "size-4")}<span>${this.translate("presets.button.delete", "Delete")}</span>`;
      deleteButton.disabled = !canInteract || !selectedSavedPreset;
    }
  }

  renderCards(items, activePresetId, currentPayload) {
    const container = this.cardsContainer();
    if (!(container instanceof HTMLElement)) {
      return;
    }
    container.replaceChildren();
    for (const item of items) {
      const card = document.createElement("article");
      card.className = "fishy-preset-manager__preset-card";
      if (item.kind === "current") {
        card.classList.add("fishy-preset-manager__preset-card--current");
      }
      if (items.some((candidate) => candidate.kind === "current" && sourceMatchesCard(candidate.source, item.key))) {
        card.classList.add("fishy-preset-manager__preset-card--source");
      }
      if (item.key === this.selectedCardKey) {
        card.classList.add("fishy-preset-manager__preset-card--selected");
      }
      card.dataset.role = "preset-card";
      card.dataset.cardKey = item.key;
      card.setAttribute("role", "button");
      card.setAttribute("tabindex", "0");
      card.setAttribute("aria-pressed", item.key === this.selectedCardKey ? "true" : "false");
      card.setAttribute("data-on:click", "window.__fishystuffPresetManager.selectCard(el)");
      card.setAttribute("data-on:keydown", "window.__fishystuffPresetManager.handleCardKeydown(evt, el)");

      const header = document.createElement("div");
      header.className = "fishy-preset-manager__preset-card-header";

      const heading = document.createElement("div");
      heading.className = "fishy-preset-manager__preset-card-heading";
      const titleIcon = createIconElement(this.titleIconAlias(item), "fishy-preset-manager__preset-card-title-icon size-4");
      if (titleIcon) {
        heading.append(titleIcon);
      }
      const title = document.createElement("div");
      title.className = "fishy-preset-manager__preset-card-title";
      title.textContent = item.name;
      heading.append(title);
      header.append(heading);

      const badgeDefinition = this.cardBadge(item, activePresetId, currentPayload);
      if (badgeDefinition) {
        const badge = document.createElement("span");
        badge.className = badgeDefinition.className;
        badge.textContent = badgeDefinition.text;
        header.append(badge);
      }

      const previewShell = presetPreviewHelper()?.createShell?.({
        cardKey: item.key,
      });

      card.append(header);
      if (previewShell?.shell) {
        card.append(previewShell.shell);
      }
      container.append(card);
      if (previewShell?.preview) {
        this.renderPreview(previewShell.preview, item);
      }
    }
  }

  renderPreview(container, item) {
    if (!(container instanceof HTMLElement)) {
      return;
    }
    presetPreviewHelper()?.render?.(container, {
      collectionKey: this.collectionKey,
      adapter: this.adapter(),
      item: cloneJson(item),
      payload: cloneJson(item?.payload),
      previewSize: 200,
      variant: "manager",
      errorMessage: "fishy preset preview render failed",
    });
  }

  commitSelectedTitleChange(nextValue = null) {
    const helper = presetHelper();
    const selectedPreset = this.selectedSavedPreset();
    const input = this.selectedTitleInput();
    if (!helper || !selectedPreset || !(input instanceof HTMLInputElement)) {
      return;
    }
    const nextName = trimString(nextValue ?? input.value) || selectedPreset.name;
    if (nextName === selectedPreset.name) {
      input.value = selectedPreset.name;
      return;
    }
    try {
      helper.renamePreset(this.collectionKey, selectedPreset.id, nextName);
      input.value = nextName;
    } catch (error) {
      input.value = selectedPreset.name;
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.save", "Preset save failed."),
      );
    }
  }

  handleLanguageChange() {
    this.sync({ refreshNames: true });
  }

  handleSignalPatch(event) {
    if (!patchTouchesPresetManager(event?.detail || null)) {
      return;
    }
    this.sync();
  }

  closeDialogBeforeApply() {
    setDialogOpen(this.dialogElement(), false);
  }

  applyCardSelection(cardKey) {
    const helper = presetHelper();
    const { items } = this.cardItems();
    const selectedItem = this.findItem(items, cardKey);
    if (!selectedItem) {
      return;
    }
    this.selectedCardKey = selectedItem.key;
    if (helper) {
      const current = helper.current?.(this.collectionKey) ?? null;
      if (current && selectedItem.kind !== "current") {
        this.sync({ refreshNames: true });
        return;
      }
      this.closeDialogBeforeApply();
      if (selectedItem.kind === "preset") {
        helper.activatePreset(this.collectionKey, selectedItem.id);
      } else if (selectedItem.kind === "fixed") {
        helper.activateFixedPreset?.(this.collectionKey, selectedItem.id);
      } else {
        helper.trackCurrentPayload?.(this.collectionKey, {
          payload: selectedItem.payload,
          origin: selectedItem.source,
        });
        helper.applyPayload(this.collectionKey, selectedItem.payload);
      }
    }
    this.sync({ refreshNames: true });
  }

  handleCardClick(card) {
    if (!(card instanceof HTMLElement)) {
      return;
    }
    this.commitSelectedTitleChange();
    this.applyCardSelection(trimString(card.dataset.cardKey));
  }

  handleCardKeyDown(event, card) {
    if (event?.key !== "Enter" && event?.key !== " ") {
      return;
    }
    if (!(card instanceof HTMLElement)) {
      return;
    }
    event.preventDefault();
    this.commitSelectedTitleChange();
    this.applyCardSelection(trimString(card.dataset.cardKey));
  }

  handleSelectedTitleKeyDown(event, input) {
    if (event?.key === "Enter") {
      event.preventDefault();
      this.commitSelectedTitleChange(input instanceof HTMLInputElement ? input.value : null);
      input?.blur?.();
      return;
    }
    if (event?.key === "Escape") {
      const selectedItem = this.selectedItem(this.cardItems().items);
      if (input instanceof HTMLInputElement) {
        input.value = this.selectedTitleValue(selectedItem);
      }
      input?.blur?.();
    }
  }

  handleSaveClick() {
    const helper = presetHelper();
    if (!helper || typeof helper.saveCurrentToSelectedPreset !== "function") {
      return;
    }
    try {
      const saved = helper.saveCurrentToSelectedPreset(this.collectionKey);
      if (saved?.id) {
        this.selectedCardKey = presetCardKey(saved.id);
      }
      toastHelper()?.success?.(
        this.translate("presets.toast.saved", 'Saved "{$name}".', { name: saved?.name || "" }),
      );
      this.sync({ refreshNames: true });
    } catch (error) {
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.save", "Preset save failed."),
      );
    }
  }

  handleDiscardClick() {
    const helper = presetHelper();
    if (!helper || typeof helper.discardCurrent !== "function") {
      return;
    }
    try {
      this.closeDialogBeforeApply();
      const result = helper.discardCurrent(this.collectionKey);
      if (result?.source?.kind === "preset") {
        this.selectedCardKey = presetCardKey(result.source.id);
      } else if (result?.source?.kind === "fixed") {
        this.selectedCardKey = fixedCardKey(result.source.id);
      }
      if (result?.current) {
        toastHelper()?.error?.(
          this.translate("presets.error.discard", "Preset discard failed."),
        );
        this.sync({ refreshNames: true });
        return;
      }
      toastHelper()?.info?.(
        this.translate("presets.toast.discarded", "Discarded current changes."),
      );
      this.sync({ refreshNames: true });
    } catch (error) {
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.discard", "Preset discard failed."),
      );
    }
  }

  handleCopyClick() {
    const helper = presetHelper();
    const { items, currentItem } = this.cardItems();
    const selectedItem = currentItem || this.selectedItem(items);
    const payload = this.clonePayload(selectedItem);
    if (!helper || !payload) {
      return;
    }
    try {
      const created = helper.createPreset(this.collectionKey, {
        name: this.defaultName(this.presetItems().length + 1),
        payload,
        select: true,
      });
      this.selectedCardKey = presetCardKey(created.id);
      toastHelper()?.success?.(
        this.translate("presets.toast.copied", 'Cloned "{$name}".', { name: created.name }),
      );
      this.sync({ refreshNames: true });
    } catch (error) {
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.save", "Preset save failed."),
      );
    }
  }

  handleDeleteClick() {
    const helper = presetHelper();
    const selectedPreset = this.selectedSavedPreset();
    if (!helper || !selectedPreset) {
      return;
    }
    const confirmed = globalThis.window?.confirm?.(
      this.translate("presets.confirm.delete", 'Delete "{$name}"?', { name: selectedPreset.name }),
    );
    if (confirmed === false) {
      return;
    }
    helper.deletePreset(this.collectionKey, selectedPreset.id);
    toastHelper()?.info?.(
      this.translate("presets.toast.deleted", 'Deleted "{$name}".', { name: selectedPreset.name }),
    );
    this.sync({ refreshNames: true });
  }

  handleExportClick() {
    const helper = presetHelper();
    const { items } = this.cardItems();
    const selectedItem = this.selectedItem(items);
    if (!helper || !selectedItem?.payload) {
      return;
    }
    const payload = selectedItem.kind === "preset"
      ? helper.exportCollectionPayload(this.collectionKey, { presetIds: [selectedItem.id] })
      : helper.exportCollectionPayload(this.collectionKey, {
          includeCurrent: true,
          currentName: selectedItem.name,
          currentPayload: cloneJson(selectedItem.payload),
        });
    downloadTextFile(
      `${this.fileBaseName}.json`,
      JSON.stringify(payload, null, 2),
    );
    toastHelper()?.success?.(this.translate("presets.toast.exported", "Preset exported."));
  }

  handleImportClick() {
    const input = this.fileInput();
    if (!(input instanceof HTMLInputElement)) {
      return;
    }
    input.value = "";
    input.click();
  }

  async handleFileChange(input) {
    const helper = presetHelper();
    const file = input instanceof HTMLInputElement ? input.files?.[0] : null;
    if (!helper || !file) {
      return;
    }
    try {
      const text = await file.text();
      helper.importCollectionText(this.collectionKey, text, {
        selectImported: false,
      });
      toastHelper()?.success?.(this.translate("presets.toast.imported", "Preset imported."));
      this.sync({ refreshNames: true });
    } catch (error) {
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.import", "Preset import failed."),
      );
    }
  }
}

function managerFromNode(node) {
  return typeof node?.closest === "function" ? node.closest(TAG_NAME) : null;
}

function bindPresetManagerHelpers() {
  if (!globalThis.window) {
    return;
  }
  globalThis.window.__fishystuffPresetManager = Object.freeze({
    syncDialog(dialog, open) {
      setDialogOpen(dialog, Boolean(open));
    },
    refresh(node) {
      managerFromNode(node)?.sync({ refreshNames: true });
    },
    selectCard(node) {
      managerFromNode(node)?.handleCardClick(node);
    },
    handleCardKeydown(event, node) {
      managerFromNode(node)?.handleCardKeyDown(event, node);
    },
    commitSelectedTitle(node) {
      managerFromNode(node)?.commitSelectedTitleChange(node instanceof HTMLInputElement ? node.value : null);
    },
    handleSelectedTitleKeydown(event, node) {
      managerFromNode(node)?.handleSelectedTitleKeyDown(event, node);
    },
    saveCurrent(node) {
      managerFromNode(node)?.handleSaveClick();
    },
    discardCurrent(node) {
      managerFromNode(node)?.handleDiscardClick();
    },
    copySelected(node) {
      managerFromNode(node)?.handleCopyClick();
    },
    exportSelected(node) {
      managerFromNode(node)?.handleExportClick();
    },
    openImport(node) {
      managerFromNode(node)?.handleImportClick();
    },
    importFile(node) {
      managerFromNode(node)?.handleFileChange(node);
    },
    deleteSelected(node) {
      managerFromNode(node)?.handleDeleteClick();
    },
  });
}

export function registerPresetManager(registry = globalThis.customElements) {
  if (!registry || typeof registry.define !== "function") {
    return;
  }
  if (!registry.get(TAG_NAME)) {
    registry.define(TAG_NAME, FishyPresetManager);
  }
}

bindPresetManagerHelpers();
registerPresetManager();
