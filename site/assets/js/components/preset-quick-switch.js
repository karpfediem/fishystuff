import { registerSearchableDropdown } from "./searchable-dropdown.js";

const TAG_NAME = "fishy-preset-quick-switch";
const SEARCHABLE_DROPDOWN_TAG_NAME = "fishy-searchable-dropdown";
const LANGUAGE_CHANGE_EVENT = "fishystuff:languagechange";
const ENTRIES_CHANGE_EVENT = "fishystuff:preset-quick-switch-entries-changed";
const SEARCHABLE_DROPDOWN_OPEN_EVENT = "fishystuff:searchable-dropdown-open";
const SEARCHABLE_DROPDOWN_CLOSE_EVENT = "fishystuff:searchable-dropdown-close";
const ICON_SPRITE_FALLBACK_URL = "";
const HTMLElementBase = globalThis.HTMLElement ?? class {};
let nextQuickSwitchInstanceId = 1;

const DEFAULT_PRESET_QUICK_SWITCH_ENTRIES = Object.freeze([
  {
    collectionKey: "calculator-layouts",
    labelKey: "presets.quick_switch.layout",
    labelFallback: "Workspace",
    order: 10,
    fixedFallbacks: [{ id: "default", labelFallback: "Default" }],
  },
  {
    collectionKey: "calculator-presets",
    labelKey: "presets.quick_switch.calculator",
    labelFallback: "Calculator",
    order: 20,
    fixedFallbacks: [{ id: "default", labelFallback: "Default calculator" }],
  },
  {
    collectionKey: "map-presets",
    labelKey: "presets.quick_switch.map",
    labelFallback: "Map",
    order: 30,
    fixedFallbacks: [{ id: "default", labelFallback: "Default map" }],
  },
  {
    collectionKey: "fishydex-presets",
    labelKey: "presets.quick_switch.dex",
    labelFallback: "Dex",
    order: 40,
    fixedFallbacks: [{
      id: "default",
      labelKey: "fishydex.presets.default",
      labelFallback: "Default dex",
    }],
  },
]);

const registeredEntries = new Map();

function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizeCollectionKey(value) {
  return trimString(value)
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
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

function optionKey(kind, id) {
  const normalizedKind = trimString(kind).toLowerCase();
  const normalizedId = trimString(id);
  return normalizedKind && normalizedId ? `${normalizedKind}:${normalizedId}` : "";
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

function presetHelper() {
  return globalThis.window?.__fishystuffUserPresets ?? null;
}

function toastHelper() {
  return globalThis.window?.__fishystuffToast ?? null;
}

function presetPreviewHelper() {
  return globalThis.window?.__fishystuffPresetPreviews ?? null;
}

function iconSpriteUrl() {
  return trimString(globalThis.window?.__fishystuffCalculator?.iconSpriteUrl) || ICON_SPRITE_FALLBACK_URL;
}

function cloneJson(value) {
  if (value == null) {
    return value;
  }
  try {
    return JSON.parse(JSON.stringify(value));
  } catch (_error) {
    return value;
  }
}

function defaultTranslate(key, fallback, vars = {}) {
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

function normalizeFixedFallbacks(value) {
  return (Array.isArray(value) ? value : [])
    .map((entry) => {
      const source = isPlainObject(entry) ? entry : {};
      const id = trimString(source.id);
      if (!id) {
        return null;
      }
      return {
        id,
        labelKey: trimString(source.labelKey),
        labelFallback: trimString(source.labelFallback || source.name || source.label) || id,
      };
    })
    .filter(Boolean);
}

export function normalizePresetQuickSwitchEntry(entry, index = 0) {
  const source = isPlainObject(entry) ? entry : {};
  const collectionKey = normalizeCollectionKey(source.collectionKey || source.key);
  if (!collectionKey) {
    return null;
  }
  const order = Number(source.order);
  return {
    collectionKey,
    labelKey: trimString(source.labelKey),
    labelFallback: trimString(source.labelFallback || source.label || source.name) || collectionKey,
    order: Number.isFinite(order) ? order : 1000 + index,
    fixedFallbacks: normalizeFixedFallbacks(source.fixedFallbacks),
  };
}

export function registerPresetQuickSwitchEntry(entry) {
  const normalized = normalizePresetQuickSwitchEntry(entry, registeredEntries.size);
  if (!normalized) {
    throw new Error("Preset quick-switch entry requires a collectionKey.");
  }
  registeredEntries.set(normalized.collectionKey, normalized);
  globalThis.window?.dispatchEvent?.(
    new CustomEvent(ENTRIES_CHANGE_EVENT, {
      detail: { collectionKey: normalized.collectionKey },
    }),
  );
  return { ...normalized, fixedFallbacks: normalized.fixedFallbacks.map((fallback) => ({ ...fallback })) };
}

export function presetQuickSwitchEntries(extraEntries = []) {
  const merged = new Map();
  for (const [index, entry] of DEFAULT_PRESET_QUICK_SWITCH_ENTRIES.entries()) {
    const normalized = normalizePresetQuickSwitchEntry(entry, index);
    if (normalized) {
      merged.set(normalized.collectionKey, normalized);
    }
  }
  for (const entry of registeredEntries.values()) {
    merged.set(entry.collectionKey, entry);
  }
  for (const [index, entry] of (Array.isArray(extraEntries) ? extraEntries : []).entries()) {
    const normalized = normalizePresetQuickSwitchEntry(entry, index);
    if (normalized) {
      merged.set(normalized.collectionKey, normalized);
    }
  }
  return Array.from(merged.values())
    .sort((left, right) => left.order - right.order || left.labelFallback.localeCompare(right.labelFallback))
    .map((entry) => ({
      ...entry,
      fixedFallbacks: entry.fixedFallbacks.map((fallback) => ({ ...fallback })),
    }));
}

function safeCollection(helper, collectionKey) {
  if (!helper || typeof helper.collection !== "function") {
    return {
      selectedPresetId: "",
      selectedFixedId: "",
      workingCopies: [],
      activeWorkingCopyId: "",
      presets: [],
    };
  }
  const collection = helper.collection(collectionKey);
  return isPlainObject(collection)
    ? {
        selectedPresetId: trimString(collection.selectedPresetId),
        selectedFixedId: trimString(collection.selectedFixedId),
        workingCopies: Array.isArray(collection.workingCopies) ? collection.workingCopies : [],
        activeWorkingCopyId: trimString(collection.activeWorkingCopyId),
        presets: Array.isArray(collection.presets) ? collection.presets : [],
      }
    : {
        selectedPresetId: "",
        selectedFixedId: "",
        workingCopies: [],
        activeWorkingCopyId: "",
        presets: [],
      };
}

function safeFixedPresets(helper, entry, adapter, translate) {
  let fixed = [];
  if (adapter && typeof helper?.fixedPresets === "function") {
    fixed = helper.fixedPresets(entry.collectionKey);
  }
  if ((!Array.isArray(fixed) || !fixed.length) && !adapter) {
    fixed = presetPreviewHelper()?.fixedPresets?.(entry.collectionKey);
  }
  if ((!Array.isArray(fixed) || !fixed.length) && !adapter) {
    fixed = entry.fixedFallbacks.map((fallback) => ({
      id: fallback.id,
      name: translate(fallback.labelKey, fallback.labelFallback),
      payload: null,
    }));
  }
  return fixed
    .map((preset, index) => {
      const id = trimString(preset?.id) || `fixed_${index + 1}`;
      return {
        id,
        label: trimString(preset?.name) || id,
        payload: preset?.payload ?? null,
      };
    })
    .filter((preset) => preset.id);
}

function sourceLabel(source, fixedOptions, savedOptions) {
  const normalized = normalizeSource(source);
  if (normalized.kind === "fixed") {
    return fixedOptions.find((option) => option.id === normalized.id)?.label || normalized.id;
  }
  if (normalized.kind === "preset") {
    return savedOptions.find((option) => option.id === normalized.id)?.label || normalized.id;
  }
  return "";
}

function collectionHasExplicitState(collection) {
  return Boolean(
    collection.selectedPresetId
      || collection.selectedFixedId
      || collection.activeWorkingCopyId
      || collection.workingCopies.length
      || collection.presets.length,
  );
}

export function filterPresetQuickSwitchOptions(options, query) {
  const normalizedQuery = trimString(query).toLowerCase();
  if (!normalizedQuery) {
    return Array.isArray(options) ? options.slice() : [];
  }
  return (Array.isArray(options) ? options : []).filter((option) => (
    trimString(option.searchText || option.label).toLowerCase().includes(normalizedQuery)
  ));
}

export function buildPresetQuickSwitchRow(helper, entry, translate = defaultTranslate) {
  const normalizedEntry = normalizePresetQuickSwitchEntry(entry);
  if (!normalizedEntry) {
    return null;
  }
  const adapter = typeof helper?.collectionAdapter === "function"
    ? helper.collectionAdapter(normalizedEntry.collectionKey)
    : null;
  const collection = safeCollection(helper, normalizedEntry.collectionKey);
  const label = translate(
    normalizedEntry.labelKey || adapter?.quickSwitchLabelKey || adapter?.titleKey,
    normalizedEntry.labelFallback || adapter?.quickSwitchLabelFallback || adapter?.titleFallback,
  );
  const fixedPresets = safeFixedPresets(helper, normalizedEntry, adapter, translate);
  const fixedOptions = fixedPresets.map((preset) => ({
    collectionKey: normalizedEntry.collectionKey,
    key: optionKey("fixed", preset.id),
    kind: "fixed",
    id: preset.id,
    source: { kind: "fixed", id: preset.id },
    label: preset.label,
    status: translate("presets.status.default", "Default"),
    statusTone: "default",
    payload: preset.payload,
  }));
  const savedOptions = collection.presets
    .map((preset) => {
      const id = trimString(preset?.id);
      if (!id) {
        return null;
      }
      return {
        collectionKey: normalizedEntry.collectionKey,
        key: optionKey("preset", id),
        kind: "preset",
        id,
        source: { kind: "preset", id },
        label: trimString(preset.name) || id,
        status: translate("presets.status.saved", "Saved"),
        statusTone: "saved",
        payload: preset.payload ?? null,
      };
    })
    .filter(Boolean);
  const dirtyWorkingCopies = collection.workingCopies
    .filter((workingCopy) => workingCopy?.modified === true && workingCopy?.payload)
    .map((workingCopy) => {
      const source = normalizeSource(workingCopy.source || workingCopy.origin);
      const sourceName = sourceLabel(source, fixedOptions, savedOptions);
      return {
        collectionKey: normalizedEntry.collectionKey,
        key: optionKey("work", workingCopy.id),
        kind: "working",
        id: workingCopy.id,
        source,
        label: translate(
          "presets.current.modified_from",
          "Modified: {$name}",
          { name: sourceName || translate("presets.current.modified", "Modified current preset") },
        ),
        status: translate("presets.status.modified", "Modified"),
        statusTone: "modified",
        payload: workingCopy.payload,
        active: workingCopy.id === collection.activeWorkingCopyId,
      };
    });
  const options = [
    ...fixedOptions,
    ...savedOptions,
    ...dirtyWorkingCopies,
  ].map((option) => ({
    ...option,
    searchText: `${option.label} ${option.id}`,
  }));

  let selectedOption = dirtyWorkingCopies.find((option) => option.active) || null;
  if (!selectedOption && collection.selectedPresetId) {
    selectedOption = savedOptions.find((option) => option.id === collection.selectedPresetId) || null;
  }
  if (!selectedOption && collection.selectedFixedId) {
    selectedOption = fixedOptions.find((option) => option.id === collection.selectedFixedId) || null;
  }
  if (!selectedOption && !collectionHasExplicitState(collection)) {
    selectedOption = fixedOptions[0] || null;
  }

  return {
    collectionKey: normalizedEntry.collectionKey,
    label,
    emptyText: translate("presets.quick_switch.empty", "No presets available."),
    searchPlaceholder: translate("presets.quick_switch.search", "Search presets..."),
    selectedLabel: selectedOption?.label || translate("presets.quick_switch.not_selected", "Not selected"),
    selectedStatus: selectedOption?.status || translate("presets.quick_switch.not_selected", "Not selected"),
    selectedStatusTone: selectedOption?.statusTone || "none",
    selectedOptionKey: selectedOption?.key || "",
    adapter,
    options,
  };
}

export function buildPresetQuickSwitchRows(
  helper,
  entries = presetQuickSwitchEntries(),
  translate = defaultTranslate,
) {
  return (Array.isArray(entries) ? entries : [])
    .map((entry) => buildPresetQuickSwitchRow(helper, entry, translate))
    .filter(Boolean);
}

export function applyPresetQuickSwitchOption(helper, option) {
  if (!helper || !option) {
    return null;
  }
  const collectionKey = normalizeCollectionKey(option.collectionKey);
  if (!collectionKey) {
    return null;
  }
  if (option.kind === "preset" && typeof helper.activatePreset === "function") {
    return helper.activatePreset(collectionKey, option.id);
  }
  if (option.kind === "fixed" && typeof helper.activateFixedPreset === "function") {
    return helper.activateFixedPreset(collectionKey, option.id);
  }
  if (option.kind === "working" && typeof helper.activateWorkingCopy === "function") {
    return helper.activateWorkingCopy(collectionKey, option.id);
  }
  if (option.kind === "working") {
    return option.payload || null;
  }
  return null;
}

function createTextElement(tagName, className, text) {
  const element = document.createElement(tagName);
  if (className) {
    element.className = className;
  }
  element.textContent = text;
  return element;
}

function appendChildren(parent, children) {
  for (const child of children) {
    if (child) {
      parent.append(child);
    }
  }
}

function createIconElement(alias, className) {
  const normalizedAlias = trimString(alias);
  if (!normalizedAlias) {
    return null;
  }
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("class", `fishy-icon ${className || ""}`.trim());
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("width", "100%");
  use.setAttribute("height", "100%");
  use.setAttribute("href", `${iconSpriteUrl()}#fishy-${normalizedAlias}`);
  svg.append(use);
  return svg;
}

function renderPresetOptionPreview(container, row, option) {
  if (!(container instanceof HTMLElement)) {
    return;
  }
  presetPreviewHelper()?.render?.(container, {
    collectionKey: option?.collectionKey || row?.collectionKey,
    adapter: row?.adapter,
    item: {
      collectionKey: option?.collectionKey,
      key: option?.key,
      kind: option?.kind,
      id: option?.id,
      name: option?.label,
      source: option?.source,
      payload: cloneJson(option?.payload),
    },
    payload: cloneJson(option?.payload),
    previewSize: 200,
    variant: "quick-switch",
    errorMessage: "fishy preset quick-switch preview render failed",
  });
}

function createPresetOptionContent(row, option) {
  const root = document.createElement("span");
  root.className = "fishy-preset-quick-switch__preset-option";
  const shell = presetPreviewHelper()?.createShell?.({
    shellTag: "span",
    viewportTag: "span",
    previewTag: "span",
    shellClassName: "fishy-preset-quick-switch__preview",
    cardKey: option?.key || "",
    ariaHidden: true,
  });
  if (shell?.preview && shell?.shell) {
    renderPresetOptionPreview(shell.preview, row, option);
  }
  const label = createTextElement("span", "fishy-preset-quick-switch__option-name", option?.label || "");
  appendChildren(root, [shell?.shell, label]);
  return root;
}

function createCatalogTemplate(role, row, option) {
  const template = document.createElement("template");
  template.dataset.role = role;
  template.dataset.value = option.key;
  if (role === "selected-content") {
    template.dataset.label = option.label;
    template.dataset.searchText = option.searchText;
  }
  template.content.append(createPresetOptionContent(row, option));
  return template;
}

function focusWithoutScroll(element) {
  if (!(element instanceof HTMLElement) || typeof element.focus !== "function") {
    return false;
  }
  try {
    element.focus({ preventScroll: true });
  } catch (_error) {
    element.focus();
  }
  return true;
}

function presetSelectorFromEvent(event) {
  const dropdown = event?.target;
  return dropdown instanceof Element && dropdown.matches("fishy-searchable-dropdown[data-role='selector']")
    ? dropdown
    : null;
}

export class FishyPresetQuickSwitch extends HTMLElementBase {
  constructor() {
    super();
    this.instanceId = nextQuickSwitchInstanceId;
    nextQuickSwitchInstanceId += 1;
    this.heldUserMenus = new Map();
    this.lastPointerDown = null;
    this.handleDocumentPointerDown = this.handleDocumentPointerDown.bind(this);
    this.handleDropdownOpen = this.handleDropdownOpen.bind(this);
    this.handleDropdownClose = this.handleDropdownClose.bind(this);
    this.handleSelectionChange = this.handleSelectionChange.bind(this);
    this.handleLanguageChange = this.handleLanguageChange.bind(this);
    this.handleEntriesChange = this.handleEntriesChange.bind(this);
  }

  connectedCallback() {
    if (this.dataset.presetQuickSwitchReady === "true") {
      return;
    }
    this.dataset.presetQuickSwitchReady = "true";
    this.classList.add("fishy-preset-quick-switch");
    this.addEventListener("change", this.handleSelectionChange);
    this.addEventListener(SEARCHABLE_DROPDOWN_OPEN_EVENT, this.handleDropdownOpen);
    this.addEventListener(SEARCHABLE_DROPDOWN_CLOSE_EVENT, this.handleDropdownClose);
    globalThis.document?.addEventListener?.("pointerdown", this.handleDocumentPointerDown, true);
    globalThis.window?.addEventListener?.(LANGUAGE_CHANGE_EVENT, this.handleLanguageChange);
    globalThis.window?.addEventListener?.(ENTRIES_CHANGE_EVENT, this.handleEntriesChange);
    this.render();
    this.renderAfterSearchableDropdownUpgrade();
  }

  disconnectedCallback() {
    this.removeEventListener("change", this.handleSelectionChange);
    this.removeEventListener(SEARCHABLE_DROPDOWN_OPEN_EVENT, this.handleDropdownOpen);
    this.removeEventListener(SEARCHABLE_DROPDOWN_CLOSE_EVENT, this.handleDropdownClose);
    globalThis.document?.removeEventListener?.("pointerdown", this.handleDocumentPointerDown, true);
    this.releaseAllUserMenus();
    globalThis.window?.removeEventListener?.(LANGUAGE_CHANGE_EVENT, this.handleLanguageChange);
    globalThis.window?.removeEventListener?.(ENTRIES_CHANGE_EVENT, this.handleEntriesChange);
  }

  translate(key, fallback, vars = {}) {
    return defaultTranslate(key, fallback, vars);
  }

  rows() {
    return buildPresetQuickSwitchRows(presetHelper(), presetQuickSwitchEntries(), this.translate.bind(this));
  }

  render() {
    const rows = this.rows();
    this.replaceChildren();
    if (!rows.length) {
      this.append(
        createTextElement(
          "p",
          "fishy-preset-quick-switch__empty text-xs text-base-content/55",
          this.translate("presets.quick_switch.empty", "No presets available."),
        ),
      );
      return;
    }
    const list = document.createElement("div");
    list.className = "fishy-preset-quick-switch__rows";
    for (const row of rows) {
      list.append(this.renderRow(row));
    }
    this.append(list);
  }

  renderAfterSearchableDropdownUpgrade() {
    const registry = globalThis.customElements;
    const DropdownConstructor = registry?.get?.(SEARCHABLE_DROPDOWN_TAG_NAME);
    if (typeof DropdownConstructor === "function") {
      const hasStaleDropdown = Array.from(this.querySelectorAll(SEARCHABLE_DROPDOWN_TAG_NAME))
        .some((node) => !(node instanceof DropdownConstructor));
      if (hasStaleDropdown) {
        queueMicrotask(() => {
          if (this.isConnected) {
            this.render();
          }
        });
      }
      return;
    }
    if (typeof registry?.whenDefined !== "function") {
      return;
    }
    registry.whenDefined(SEARCHABLE_DROPDOWN_TAG_NAME).then(() => {
      if (this.isConnected) {
        this.render();
      }
    });
  }

  renderRow(row) {
    const shell = document.createElement("section");
    shell.className = "fishy-preset-quick-switch__row";
    shell.dataset.collectionKey = row.collectionKey;

    const header = document.createElement("div");
    header.className = "fishy-preset-quick-switch__row-header";
    const label = createTextElement("div", "fishy-preset-quick-switch__label", row.label);
    header.append(label);
    shell.append(header);

    shell.append(this.renderDropdown(row));
    return shell;
  }

  renderDropdown(row) {
    const inputId = `fishy-preset-quick-switch-${this.instanceId}-${row.collectionKey}`;
    const panelId = `${inputId}-panel`;
    const searchInputId = `${inputId}-search`;
    const selectedOption = row.options.find((option) => option.key === row.selectedOptionKey) || null;

    const dropdown = document.createElement("fishy-searchable-dropdown");
    dropdown.className = "fishy-preset-quick-switch__dropdown relative block w-full";
    dropdown.dataset.role = "selector";
    dropdown.dataset.collectionKey = row.collectionKey;
    dropdown.setAttribute("input-id", inputId);
    dropdown.setAttribute("label", row.selectedLabel);
    dropdown.setAttribute("value", row.selectedOptionKey);
    dropdown.setAttribute("placeholder", row.searchPlaceholder);
    dropdown.setAttribute("panel-mode", "detached");
    dropdown.setAttribute("panel-min-width", "panel");
    dropdown.setAttribute("panel-width", "22rem");

    const hiddenInput = document.createElement("input");
    hiddenInput.id = inputId;
    hiddenInput.type = "hidden";
    hiddenInput.value = row.selectedOptionKey;
    dropdown.append(hiddenInput);

    const trigger = document.createElement("button");
    trigger.type = "button";
    trigger.dataset.role = "trigger";
    trigger.className = "fishy-preset-quick-switch__trigger";
    trigger.setAttribute("aria-haspopup", "listbox");
    trigger.setAttribute("aria-expanded", "false");
    trigger.setAttribute("aria-controls", panelId);
    const selectedContent = document.createElement("span");
    selectedContent.dataset.role = "selected-content";
    selectedContent.className = "fishy-preset-quick-switch__selected-content";
    if (selectedOption) {
      selectedContent.append(createPresetOptionContent(row, selectedOption));
    } else {
      selectedContent.append(createTextElement("span", "fishy-preset-quick-switch__option-name", row.selectedLabel));
    }
    trigger.append(selectedContent);
    const caret = createIconElement("caret-down", "fishy-preset-quick-switch__caret size-4 opacity-60");
    if (caret) {
      trigger.append(caret);
    }
    dropdown.append(trigger);

    const panel = document.createElement("div");
    panel.id = panelId;
    panel.dataset.role = "panel";
    panel.className = "fishy-preset-quick-switch__panel absolute left-0 top-0 z-50 w-full min-w-full max-w-full";
    panel.hidden = true;
    const panelShell = document.createElement("div");
    panelShell.className = "fishy-preset-quick-switch__panel-shell";
    const searchLabel = document.createElement("label");
    searchLabel.className = "fishy-preset-quick-switch__search-label";
    const searchIcon = createIconElement("search-field", "size-4 opacity-60");
    if (searchIcon) {
      searchLabel.append(searchIcon);
    }
    const searchInput = document.createElement("input");
    searchInput.id = searchInputId;
    searchInput.dataset.role = "search-input";
    searchInput.type = "search";
    searchInput.className = "fishy-preset-quick-switch__search-input";
    searchInput.placeholder = row.searchPlaceholder;
    searchInput.autocomplete = "off";
    searchInput.spellcheck = false;
    searchLabel.append(searchInput);
    const resultsWrapper = document.createElement("div");
    resultsWrapper.className = "fishy-preset-quick-switch__results-shell";
    const results = document.createElement("ul");
    results.dataset.role = "results";
    results.tabIndex = -1;
    results.className = "menu menu-sm fishy-preset-quick-switch__results";
    if (!row.options.length) {
      const emptyItem = document.createElement("li");
      emptyItem.className = "menu-disabled";
      emptyItem.append(createTextElement("span", "", row.emptyText));
      results.append(emptyItem);
    }
    resultsWrapper.append(results);
    panelShell.append(searchLabel, resultsWrapper);
    panel.append(panelShell);
    dropdown.append(panel);

    const catalog = document.createElement("div");
    catalog.dataset.role = "selected-content-catalog";
    catalog.hidden = true;
    for (const option of row.options) {
      catalog.append(
        createCatalogTemplate("selected-content", row, option),
        createCatalogTemplate("option-content", row, option),
      );
    }
    dropdown.append(catalog);

    return dropdown;
  }

  applyOptionValue(collectionKey, optionKeyValue) {
    const option = this.findOption(collectionKey, optionKeyValue);
    if (!option) {
      return;
    }
    const focusAnchor = this.closest?.(".dropdown-content") || this;
    focusWithoutScroll(focusAnchor);
    const result = applyPresetQuickSwitchOption(presetHelper(), option);
    if (!result) {
      toastHelper()?.error?.(this.translate("presets.error.apply", "Preset apply failed."));
      return;
    }
    toastHelper()?.success?.(
      this.translate("presets.toast.applied", 'Applied "{$name}".', { name: option.label }),
    );
    this.render();
    const trigger = Array.from(this.querySelectorAll(".fishy-preset-quick-switch__row"))
      .find((row) => row?.dataset?.collectionKey === option.collectionKey)
      ?.querySelector?.("[data-role='trigger']");
    if (!focusWithoutScroll(trigger)) {
      focusWithoutScroll(focusAnchor);
    }
    this.releaseAllUserMenus();
  }

  findOption(collectionKey, optionKeyValue) {
    const normalizedCollectionKey = normalizeCollectionKey(collectionKey);
    const normalizedOptionKey = trimString(optionKeyValue);
    if (!normalizedCollectionKey || !normalizedOptionKey) {
      return null;
    }
    const row = this.rows().find((entry) => entry.collectionKey === normalizedCollectionKey);
    return row?.options.find((option) => option.key === normalizedOptionKey) || null;
  }

  handleSelectionChange(event) {
    const dropdown = presetSelectorFromEvent(event);
    if (!dropdown || !this.contains(dropdown)) {
      return;
    }
    this.applyOptionValue(dropdown.dataset.collectionKey, dropdown.getAttribute("value"));
  }

  handleDropdownOpen(event) {
    const dropdown = presetSelectorFromEvent(event);
    if (!dropdown || !this.contains(dropdown)) {
      return;
    }
    this.holdUserMenu(dropdown.closest?.(".dropdown"));
  }

  handleDropdownClose(event) {
    const dropdown = presetSelectorFromEvent(event);
    if (!dropdown) {
      return;
    }
    const menu = dropdown.closest?.(".dropdown");
    if (this.recentPointerWasInsideMenu(menu)) {
      return;
    }
    this.releaseUserMenu(menu);
  }

  handleDocumentPointerDown(event) {
    const target = event?.target;
    if (!(target instanceof Node)) {
      return;
    }
    const insideMenus = new Set();
    for (const menu of this.heldUserMenus.keys()) {
      if (menu.contains(target)) {
        insideMenus.add(menu);
      }
    }
    const insideDetachedPanel = target instanceof Element
      && Boolean(target.closest(".fishy-preset-quick-switch__panel"));
    this.lastPointerDown = {
      time: Date.now(),
      insideDetachedPanel,
      insideMenus,
    };
    if (!this.heldUserMenus.size || insideMenus.size || insideDetachedPanel) {
      return;
    }
    this.releaseAllUserMenus();
  }

  recentPointerWasInsideMenu(menu) {
    if (!(menu instanceof HTMLElement) || !this.lastPointerDown) {
      return false;
    }
    return Date.now() - this.lastPointerDown.time < 350
      && this.lastPointerDown.insideMenus?.has?.(menu);
  }

  holdUserMenu(menu) {
    if (!(menu instanceof HTMLElement)) {
      return;
    }
    const held = this.heldUserMenus.get(menu) || {
      count: 0,
      hadOpenClass: menu.classList.contains("dropdown-open"),
    };
    held.count += 1;
    menu.classList.add("dropdown-open");
    this.heldUserMenus.set(menu, held);
  }

  releaseUserMenu(menu) {
    if (!(menu instanceof HTMLElement)) {
      return;
    }
    const held = this.heldUserMenus.get(menu);
    if (!held) {
      return;
    }
    held.count -= 1;
    if (held.count > 0) {
      this.heldUserMenus.set(menu, held);
      return;
    }
    if (!held.hadOpenClass) {
      menu.classList.remove("dropdown-open");
    }
    this.heldUserMenus.delete(menu);
  }

  releaseAllUserMenus() {
    for (const [menu, held] of this.heldUserMenus.entries()) {
      if (!held.hadOpenClass) {
        menu.classList.remove("dropdown-open");
      }
    }
    this.heldUserMenus.clear();
  }

  handleLanguageChange() {
    this.render();
  }

  handleEntriesChange() {
    this.render();
  }
}

function installPresetQuickSwitchRegistry() {
  const target = globalThis.window;
  if (!target || target.__fishystuffPresetQuickSwitch) {
    return;
  }
  target.__fishystuffPresetQuickSwitch = Object.freeze({
    ENTRIES_CHANGE_EVENT,
    defaultEntries: DEFAULT_PRESET_QUICK_SWITCH_ENTRIES,
    entries: presetQuickSwitchEntries,
    registerEntry: registerPresetQuickSwitchEntry,
    sync(node) {
      const target = node?.matches?.(TAG_NAME) ? node : node?.closest?.(TAG_NAME);
      target?.render?.();
    },
  });
}

export function registerPresetQuickSwitch(registry = globalThis.customElements) {
  installPresetQuickSwitchRegistry();
  if (globalThis.window?.customElements) {
    registerSearchableDropdown();
  }
  if (!registry || typeof registry.define !== "function") {
    return;
  }
  if (!registry.get(TAG_NAME)) {
    registry.define(TAG_NAME, FishyPresetQuickSwitch);
  }
}

installPresetQuickSwitchRegistry();
registerPresetQuickSwitch();
