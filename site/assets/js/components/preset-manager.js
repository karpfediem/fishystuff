import * as d3 from "../d3.js";
import { DATASTAR_SIGNAL_PATCH_EVENT } from "../datastar-signals.js";

const TAG_NAME = "fishy-preset-manager";
const LANGUAGE_CHANGE_EVENT = "fishystuff:languagechange";
const DEFAULT_TITLE = "Saved presets";
const DEFAULT_CURRENT_LABEL = "Current";
const ICON_SPRITE_FALLBACK_URL = "/img/icons.svg";
const CURRENT_CARD_KEY = "current";
const FIXED_CARD_PREFIX = "fixed:";
const PRESET_CARD_PREFIX = "preset:";
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
    if (typeof dialog.showModal === "function") {
      dialog.showModal();
      return;
    }
    dialog.setAttribute("open", "");
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

function isCurrentCardKey(cardKey) {
  return trimString(cardKey) === CURRENT_CARD_KEY;
}

function presetIdFromCardKey(cardKey) {
  const normalized = trimString(cardKey);
  return normalized.startsWith(PRESET_CARD_PREFIX) ? trimString(normalized.slice(PRESET_CARD_PREFIX.length)) : "";
}

function fixedIdFromCardKey(cardKey) {
  const normalized = trimString(cardKey);
  return normalized.startsWith(FIXED_CARD_PREFIX) ? trimString(normalized.slice(FIXED_CARD_PREFIX.length)) : "";
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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
    this.handlePresetChange = this.handlePresetChange.bind(this);
    this.handleAdapterChange = this.handleAdapterChange.bind(this);
    this.handleLanguageChange = this.handleLanguageChange.bind(this);
    this.handleSignalPatch = this.handleSignalPatch.bind(this);
    this.handleOpenClick = this.handleOpenClick.bind(this);
    this.handleCardClick = this.handleCardClick.bind(this);
    this.handleCardKeyDown = this.handleCardKeyDown.bind(this);
    this.handleApplyClick = this.handleApplyClick.bind(this);
    this.handleSaveClick = this.handleSaveClick.bind(this);
    this.handleSaveAsClick = this.handleSaveAsClick.bind(this);
    this.handleExportClick = this.handleExportClick.bind(this);
    this.handleImportClick = this.handleImportClick.bind(this);
    this.handleDeleteClick = this.handleDeleteClick.bind(this);
    this.handleFileChange = this.handleFileChange.bind(this);
    this.handleSelectedTitleInput = this.handleSelectedTitleInput.bind(this);
    this.handleSelectedTitleBlur = this.handleSelectedTitleBlur.bind(this);
    this.handleSelectedTitleKeyDown = this.handleSelectedTitleKeyDown.bind(this);
    this.handleSaveAsNameInput = this.handleSaveAsNameInput.bind(this);
    this.handleSaveAsNameKeyDown = this.handleSaveAsNameKeyDown.bind(this);
    this.selectedCardKey = "";
    this.lastSelectedTitleSource = "";
    this.saveAsNameDirty = false;
    this.renameTimer = null;
  }

  connectedCallback() {
    if (this.dataset.presetManagerReady === "true") {
      return;
    }
    this.dataset.presetManagerReady = "true";
    this.render();
    const helper = presetHelper();
    globalThis.window?.addEventListener?.(
      helper?.CHANGED_EVENT || "fishystuff:user-presets-changed",
      this.handlePresetChange,
    );
    globalThis.window?.addEventListener?.(
      helper?.ADAPTERS_CHANGED_EVENT || "fishystuff:user-presets-adapters-changed",
      this.handleAdapterChange,
    );
    globalThis.window?.addEventListener?.(LANGUAGE_CHANGE_EVENT, this.handleLanguageChange);
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, this.handleSignalPatch);
    this.bindUiEvents();
    this.sync({ refreshNames: true });
  }

  disconnectedCallback() {
    const helper = presetHelper();
    globalThis.window?.removeEventListener?.(
      helper?.CHANGED_EVENT || "fishystuff:user-presets-changed",
      this.handlePresetChange,
    );
    globalThis.window?.removeEventListener?.(
      helper?.ADAPTERS_CHANGED_EVENT || "fishystuff:user-presets-adapters-changed",
      this.handleAdapterChange,
    );
    globalThis.window?.removeEventListener?.(LANGUAGE_CHANGE_EVENT, this.handleLanguageChange);
    document.removeEventListener(DATASTAR_SIGNAL_PATCH_EVENT, this.handleSignalPatch);
    globalThis.clearTimeout?.(this.renameTimer);
    this.renameTimer = null;
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

  currentLabelText() {
    const adapter = this.adapter();
    return this.translate(adapter?.currentLabelKey, adapter?.currentLabelFallback || DEFAULT_CURRENT_LABEL);
  }

  openLabelText() {
    return this.translate("presets.button.open_manager", "Layout Manager");
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

  render() {
    this.classList.add("fishy-preset-manager");
    this.innerHTML = `
      <button type="button" class="btn btn-soft btn-secondary" data-role="open">
        ${iconMarkup("layout-fill", "size-5")}
        <span data-role="open-label"></span>
      </button>
      <input class="hidden" type="file" accept="application/json,.json" data-role="file-input">
      <dialog class="modal" data-role="manager-dialog">
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
                <div class="grid gap-3 lg:grid-cols-2">
                  <fieldset class="fieldset">
                    <legend class="fieldset-legend" data-role="selected-title-label"></legend>
                    <input type="text" class="input input-bordered w-full" data-role="selected-title-input">
                    <p class="label" data-role="selected-title-hint"></p>
                  </fieldset>
                  <fieldset class="fieldset">
                    <legend class="fieldset-legend" data-role="save-as-name-label"></legend>
                    <input type="text" class="input input-bordered w-full" data-role="save-as-name-input">
                    <p class="label" data-role="save-as-name-hint"></p>
                  </fieldset>
                </div>
                <div class="flex flex-wrap gap-2">
                  <button type="button" class="btn btn-primary" data-role="apply"></button>
                  <button type="button" class="btn btn-outline" data-role="save"></button>
                  <button type="button" class="btn btn-outline" data-role="save-as"></button>
                  <button type="button" class="btn btn-outline" data-role="export"></button>
                  <button type="button" class="btn btn-outline" data-role="import"></button>
                  <button type="button" class="btn btn-error btn-outline" data-role="delete"></button>
                </div>
              </div>
            </section>
            <section class="card card-border bg-base-100">
              <div class="card-body gap-3">
                <div class="flex flex-wrap items-center justify-between gap-2">
                  <div class="card-title text-base" data-role="grid-title"></div>
                </div>
                <div class="fishy-preset-manager__layout-grid" data-role="preset-cards"></div>
                <p class="text-sm text-base-content/55" data-role="grid-empty"></p>
              </div>
            </section>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button type="submit" data-role="backdrop-close">close</button>
        </form>
      </dialog>
    `;
  }

  bindUiEvents() {
    this.button("open")?.addEventListener("click", this.handleOpenClick);
    this.cardsContainer()?.addEventListener("click", this.handleCardClick);
    this.cardsContainer()?.addEventListener("keydown", this.handleCardKeyDown);
    this.button("apply")?.addEventListener("click", this.handleApplyClick);
    this.button("save")?.addEventListener("click", this.handleSaveClick);
    this.button("save-as")?.addEventListener("click", this.handleSaveAsClick);
    this.button("export")?.addEventListener("click", this.handleExportClick);
    this.button("import")?.addEventListener("click", this.handleImportClick);
    this.button("delete")?.addEventListener("click", this.handleDeleteClick);
    this.fileInput()?.addEventListener("change", this.handleFileChange);
    this.selectedTitleInput()?.addEventListener("input", this.handleSelectedTitleInput);
    this.selectedTitleInput()?.addEventListener("blur", this.handleSelectedTitleBlur);
    this.selectedTitleInput()?.addEventListener("keydown", this.handleSelectedTitleKeyDown);
    this.saveAsNameInput()?.addEventListener("input", this.handleSaveAsNameInput);
    this.saveAsNameInput()?.addEventListener("keydown", this.handleSaveAsNameKeyDown);
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

  saveAsNameInput() {
    return this.element("save-as-name-input");
  }

  button(role) {
    return this.element(role);
  }

  currentPayload() {
    return presetHelper()?.capturePayload?.(this.collectionKey) ?? null;
  }

  activePresetId() {
    return presetHelper()?.selectedPresetId?.(this.collectionKey) ?? "";
  }

  activePreset() {
    return presetHelper()?.selectedPreset?.(this.collectionKey) ?? null;
  }

  titleIconAlias(item) {
    const adapter = this.adapter();
    if (!adapter || typeof adapter.titleIconAlias !== "function") {
      return "";
    }
    try {
      return trimString(adapter.titleIconAlias({
        item: cloneJson(item),
        payload: cloneJson(item?.payload),
      }));
    } catch (error) {
      console.error("fishy preset title icon resolution failed", error);
      return "";
    }
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
      editableName: true,
      removable: true,
    }));
  }

  currentItem(otherItems, currentPayload) {
    if (!currentPayload) {
      return null;
    }
    const currentJson = stableJson(currentPayload);
    if (otherItems.some((item) => stableJson(item.payload) === currentJson)) {
      return null;
    }
    return {
      key: CURRENT_CARD_KEY,
      kind: "current",
      id: CURRENT_CARD_KEY,
      name: this.currentLabelText(),
      payload: cloneJson(currentPayload),
      editableName: false,
      removable: false,
    };
  }

  cardItems() {
    const fixedItems = this.fixedItems();
    const presetItems = this.presetItems();
    const currentPayload = this.currentPayload();
    const currentItem = this.currentItem([...fixedItems, ...presetItems], currentPayload);
    return {
      currentPayload,
      fixedItems,
      presetItems,
      currentItem,
      items: [
        ...fixedItems,
        ...(currentItem ? [currentItem] : []),
        ...presetItems,
      ],
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

  ensureSelectedCard(items, activePresetId, currentItem) {
    const existing = this.selectedItem(items);
    if (existing) {
      return false;
    }
    const activeCardKey = activePresetId ? presetCardKey(activePresetId) : "";
    if (currentItem) {
      this.selectedCardKey = CURRENT_CARD_KEY;
      return true;
    }
    if (activeCardKey && this.findItem(items, activeCardKey)) {
      this.selectedCardKey = activeCardKey;
      return true;
    }
    this.selectedCardKey = items[0]?.key || "";
    return true;
  }

  isPresetActive(item, activePresetId) {
    return item?.kind === "preset" && item.id === trimString(activePresetId);
  }

  isPresetModified(item, activePresetId, currentPayload) {
    return this.isPresetActive(item, activePresetId)
      && currentPayload
      && stableJson(currentPayload) !== stableJson(item.payload);
  }

  isFixedActive(item, activePresetId, currentPayload) {
    return item?.kind === "fixed"
      && !trimString(activePresetId)
      && currentPayload
      && stableJson(currentPayload) === stableJson(item.payload);
  }

  isCardApplied(item, activePresetId, currentPayload) {
    if (!item || !currentPayload) {
      return false;
    }
    const currentJson = stableJson(currentPayload);
    if (item.kind === "current") {
      return stableJson(item.payload) === currentJson;
    }
    if (item.kind === "fixed") {
      return this.isFixedActive(item, activePresetId, currentPayload);
    }
    return this.isPresetActive(item, activePresetId)
      && stableJson(item.payload) === currentJson;
  }

  cardBadge(item, activePresetId, currentPayload) {
    if (!item) {
      return null;
    }
    if (item.kind === "fixed") {
      return null;
    }
    if (item.kind === "current") {
      return {
        className: "badge badge-sm badge-outline",
        text: this.translate("presets.status.current", "Current"),
      };
    }
    if (this.isPresetModified(item, activePresetId, currentPayload)) {
      return {
        className: "badge badge-sm badge-warning",
        text: this.translate("presets.status.modified", "Modified"),
      };
    }
    return null;
  }

  selectedStatus(item, activePresetId, currentPayload) {
    if (!item) {
      return {
        className: "badge badge-sm badge-outline",
        text: "",
      };
    }
    if (item.kind === "fixed") {
      return {
        className: this.isFixedActive(item, activePresetId, currentPayload)
          ? "badge badge-sm badge-success"
          : "badge badge-sm badge-outline",
        text: this.isFixedActive(item, activePresetId, currentPayload)
          ? this.translate("presets.status.applied", "Applied")
          : this.translate("presets.status.default", "Default"),
      };
    }
    if (item.kind === "current") {
      return {
        className: "badge badge-sm badge-outline",
        text: this.translate("presets.status.current", "Current"),
      };
    }
    if (this.isPresetModified(item, activePresetId, currentPayload)) {
      return {
        className: "badge badge-sm badge-warning",
        text: this.translate("presets.status.modified", "Modified"),
      };
    }
    if (this.isCardApplied(item, activePresetId, currentPayload)) {
      return {
        className: "badge badge-sm badge-success",
        text: this.translate("presets.status.applied", "Applied"),
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

  suggestedSaveAsName(item, presetCount) {
    const fallback = this.defaultName(presetCount + 1);
    if (!item) {
      return fallback;
    }
    if (item.kind === "current") {
      return fallback;
    }
    return fallback;
  }

  saveAsPayload(item) {
    return item ? cloneJson(item.payload) : null;
  }

  sync({ refreshNames = false } = {}) {
    const helper = presetHelper();
    const adapter = this.adapter();
    const canInteract = Boolean(helper && adapter && this.collectionKey);
    const { items, presetItems, currentItem, currentPayload } = this.cardItems();
    const activePresetId = this.activePresetId();
    const selectionChanged = this.ensureSelectedCard(items, activePresetId, currentItem);
    const selectedItem = this.selectedItem(items);
    const selectedSavedPreset = this.selectedSavedPreset();

    setElementText(this.element("open-label"), this.openLabelText());
    setElementText(this.element("manager-title"), this.titleText());
    setElementText(
      this.element("grid-title"),
      this.translate("presets.grid.title", "Layouts"),
    );
    setElementText(
      this.element("grid-count"),
      this.translate("presets.grid.count", "{$count} saved", { count: String(presetItems.length) }),
    );
    const gridEmpty = this.element("grid-empty");
    if (gridEmpty) {
      gridEmpty.textContent = presetItems.length
        ? ""
        : this.translate("presets.grid.empty", "No saved layouts yet.");
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
      this.translate("presets.section.selected.title", "Selected layout"),
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
    setElementText(
      this.element("selected-title-hint"),
      "",
    );

    const applyButton = this.button("apply");
    if (applyButton) {
      applyButton.textContent = this.translate("presets.button.apply", "Apply");
      applyButton.disabled = !canInteract || !selectedItem || selectedItem.kind === "current";
    }

    const saveButton = this.button("save");
    if (saveButton) {
      saveButton.textContent = this.translate("presets.button.save", "Save");
      saveButton.disabled = !canInteract
        || !selectedSavedPreset
        || !currentPayload
        || stableJson(currentPayload) === stableJson(selectedSavedPreset.payload);
    }

    setElementText(
      this.element("save-as-name-label"),
      this.translate("presets.field.save_as_name.label", "New layout title"),
    );
    setElementText(
      this.element("save-as-name-hint"),
      "",
    );
    const saveAsNameInput = this.saveAsNameInput();
    if (saveAsNameInput instanceof HTMLInputElement) {
      if (refreshNames || selectionChanged || (!this.saveAsNameDirty && !isFocused(saveAsNameInput))) {
        saveAsNameInput.value = this.suggestedSaveAsName(selectedItem, presetItems.length);
        this.saveAsNameDirty = false;
      }
      saveAsNameInput.disabled = !canInteract || !selectedItem || !this.saveAsPayload(selectedItem);
      saveAsNameInput.setAttribute(
        "aria-label",
        this.translate("presets.field.save_as_name.label", "New layout title"),
      );
    }
    const saveAsButton = this.button("save-as");
    if (saveAsButton) {
      saveAsButton.textContent = this.translate("presets.button.save_as", "Save as new");
      saveAsButton.disabled = !canInteract || !selectedItem || !this.saveAsPayload(selectedItem);
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
      card.className = "fishy-preset-manager__layout-card";
      if (this.isCardApplied(item, activePresetId, currentPayload)) {
        card.classList.add("fishy-preset-manager__layout-card--applied");
      }
      if (item.key === this.selectedCardKey) {
        card.classList.add("fishy-preset-manager__layout-card--selected");
      }
      if (item.kind === "current") {
        card.classList.add("fishy-preset-manager__layout-card--current");
      }
      card.dataset.role = "preset-card";
      card.dataset.cardKey = item.key;
      card.setAttribute("role", "button");
      card.setAttribute("tabindex", "0");
      card.setAttribute("aria-pressed", item.key === this.selectedCardKey ? "true" : "false");

      const header = document.createElement("div");
      header.className = "fishy-preset-manager__layout-card-header";

      const heading = document.createElement("div");
      heading.className = "fishy-preset-manager__layout-card-heading";
      const titleIcon = createIconElement(this.titleIconAlias(item), "fishy-preset-manager__layout-card-title-icon size-4");
      if (titleIcon) {
        heading.append(titleIcon);
      }
      const title = document.createElement("div");
      title.className = "fishy-preset-manager__layout-card-title";
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

      const previewShell = document.createElement("div");
      previewShell.className = "fishy-preset-manager__layout-preview-shell";
      const previewViewport = document.createElement("div");
      previewViewport.className = "fishy-preset-manager__layout-preview-viewport";
      const preview = document.createElement("div");
      preview.className = "fishy-preset-manager__layout-preview";
      preview.dataset.cardKey = item.key;
      previewViewport.append(preview);
      previewShell.append(previewViewport);

      card.append(header, previewShell);
      container.append(card);
      this.renderPreview(preview, item);
    }
  }

  renderPreview(container, item) {
    if (!(container instanceof HTMLElement)) {
      return;
    }
    container.replaceChildren();
    const adapter = this.adapter();
    if (adapter && typeof adapter.renderPreview === "function") {
      try {
        adapter.renderPreview(container, {
          item: cloneJson(item),
          payload: cloneJson(item.payload),
          d3,
          previewSize: 200,
        });
        return;
      } catch (error) {
        console.error("fishy preset preview render failed", error);
      }
    }
    const fallback = document.createElement("div");
    fallback.className = "fishy-preset-manager__preview-fallback";
    for (let index = 0; index < 3; index += 1) {
      const bar = document.createElement("span");
      bar.className = "fishy-preset-manager__preview-fallback-bar";
      fallback.append(bar);
    }
    container.append(fallback);
  }

  openManager() {
    setDialogOpen(this.dialogElement(), true);
    this.sync({ refreshNames: true });
  }

  commitSelectedTitleChange() {
    const helper = presetHelper();
    const selectedPreset = this.selectedSavedPreset();
    const input = this.selectedTitleInput();
    globalThis.clearTimeout?.(this.renameTimer);
    this.renameTimer = null;
    if (!helper || !selectedPreset || !(input instanceof HTMLInputElement)) {
      return;
    }
    const nextName = trimString(input.value) || selectedPreset.name;
    if (nextName === selectedPreset.name) {
      return;
    }
    try {
      helper.renamePreset(this.collectionKey, selectedPreset.id, nextName);
    } catch (error) {
      input.value = selectedPreset.name;
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.save", "Preset save failed."),
      );
    }
  }

  scheduleSelectedTitleCommit() {
    globalThis.clearTimeout?.(this.renameTimer);
    this.renameTimer = globalThis.setTimeout?.(() => {
      this.commitSelectedTitleChange();
    }, 180);
  }

  selectedSaveAsName() {
    const input = this.saveAsNameInput();
    return trimString(input?.value);
  }

  handlePresetChange() {
    this.sync();
  }

  handleAdapterChange(event) {
    const changedCollectionKey = trimString(event?.detail?.collectionKey);
    if (!changedCollectionKey || changedCollectionKey === this.collectionKey) {
      this.sync({ refreshNames: true });
    }
  }

  handleLanguageChange() {
    this.sync({ refreshNames: true });
  }

  handleSignalPatch() {
    this.sync();
  }

  handleOpenClick() {
    this.openManager();
  }

  handleCardClick(event) {
    const cardFromPath = typeof event?.composedPath === "function"
      ? event.composedPath().find((node) => node instanceof HTMLElement && node.dataset.role === "preset-card")
      : null;
    const card = cardFromPath || (event?.target instanceof Element
      ? event.target.closest('[data-role="preset-card"]')
      : null);
    if (!(card instanceof HTMLElement)) {
      return;
    }
    this.commitSelectedTitleChange();
    this.selectedCardKey = trimString(card.dataset.cardKey) || CURRENT_CARD_KEY;
    this.sync({ refreshNames: true });
  }

  handleCardKeyDown(event) {
    if (event?.key !== "Enter" && event?.key !== " ") {
      return;
    }
    const cardFromPath = typeof event?.composedPath === "function"
      ? event.composedPath().find((node) => node instanceof HTMLElement && node.dataset.role === "preset-card")
      : null;
    const card = cardFromPath || (event?.target instanceof Element
      ? event.target.closest('[data-role="preset-card"]')
      : null);
    if (!(card instanceof HTMLElement)) {
      return;
    }
    event.preventDefault();
    this.commitSelectedTitleChange();
    this.selectedCardKey = trimString(card.dataset.cardKey) || CURRENT_CARD_KEY;
    this.sync({ refreshNames: true });
  }

  handleSelectedTitleInput() {
    if (!this.selectedSavedPreset()) {
      return;
    }
    this.scheduleSelectedTitleCommit();
  }

  handleSelectedTitleBlur() {
    this.commitSelectedTitleChange();
  }

  handleSelectedTitleKeyDown(event) {
    if (event?.key === "Enter") {
      event.preventDefault();
      this.commitSelectedTitleChange();
      event.target?.blur?.();
      return;
    }
    if (event?.key === "Escape") {
      const selectedItem = this.selectedItem(this.cardItems().items);
      if (event.target instanceof HTMLInputElement) {
        event.target.value = this.selectedTitleValue(selectedItem);
      }
      globalThis.clearTimeout?.(this.renameTimer);
      this.renameTimer = null;
      event.target?.blur?.();
    }
  }

  handleSaveAsNameInput(event) {
    this.saveAsNameDirty = trimString(event?.target?.value) !== "";
  }

  handleSaveAsNameKeyDown(event) {
    if (event?.key === "Enter") {
      event.preventDefault();
      this.handleSaveAsClick();
    }
  }

  handleApplyClick() {
    const helper = presetHelper();
    const { items } = this.cardItems();
    const selectedItem = this.selectedItem(items);
    if (!helper || !selectedItem) {
      return;
    }
    if (selectedItem.kind === "preset") {
      helper.activatePreset(this.collectionKey, selectedItem.id);
      toastHelper()?.success?.(
        this.translate("presets.toast.applied", 'Applied "{$name}".', { name: selectedItem.name }),
      );
      this.sync({ refreshNames: true });
      return;
    }
    if (selectedItem.kind === "fixed") {
      helper.applyPayload(this.collectionKey, selectedItem.payload);
      helper.setSelectedPresetId(this.collectionKey, "");
      toastHelper()?.success?.(
        this.translate("presets.toast.applied", 'Applied "{$name}".', { name: selectedItem.name }),
      );
      this.sync({ refreshNames: true });
    }
  }

  handleSaveClick() {
    const helper = presetHelper();
    const selectedPreset = this.selectedSavedPreset();
    const currentPayload = this.currentPayload();
    if (!helper || !selectedPreset || !currentPayload) {
      return;
    }
    try {
      const updated = helper.updatePreset(this.collectionKey, selectedPreset.id, {
        payload: cloneJson(currentPayload),
        select: false,
      });
      toastHelper()?.success?.(
        this.translate("presets.toast.saved", 'Saved "{$name}".', { name: updated.name }),
      );
      this.sync({ refreshNames: true });
    } catch (error) {
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.save", "Preset save failed."),
      );
    }
  }

  handleSaveAsClick() {
    const helper = presetHelper();
    const { items } = this.cardItems();
    const selectedItem = this.selectedItem(items);
    const payload = this.saveAsPayload(selectedItem);
    if (!helper || !payload) {
      return;
    }
    try {
      const created = helper.createPreset(this.collectionKey, {
        name: this.selectedSaveAsName() || this.defaultName(this.presetItems().length + 1),
        payload,
        select: false,
      });
      this.selectedCardKey = presetCardKey(created.id);
      this.saveAsNameDirty = false;
      toastHelper()?.success?.(
        this.translate("presets.toast.created", 'Saved new preset "{$name}".', { name: created.name }),
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
    this.selectedCardKey = "";
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

  async handleFileChange(event) {
    const helper = presetHelper();
    const input = event?.target;
    const file = input instanceof HTMLInputElement ? input.files?.[0] : null;
    if (!helper || !file) {
      return;
    }
    try {
      const text = await file.text();
      const result = helper.importCollectionText(this.collectionKey, text, {
        selectImported: false,
      });
      const importedPresetId = trimString(result.presetIds?.[0]);
      if (importedPresetId) {
        this.selectedCardKey = presetCardKey(importedPresetId);
      }
      toastHelper()?.success?.(this.translate("presets.toast.imported", "Preset imported."));
      this.saveAsNameDirty = false;
      this.sync({ refreshNames: true });
    } catch (error) {
      toastHelper()?.error?.(
        error instanceof Error ? error.message : this.translate("presets.error.import", "Preset import failed."),
      );
    }
  }
}

export function registerPresetManager(registry = globalThis.customElements) {
  if (!registry || typeof registry.define !== "function") {
    return;
  }
  if (!registry.get(TAG_NAME)) {
    registry.define(TAG_NAME, FishyPresetManager);
  }
}

registerPresetManager();
