(function () {
  const ICON_SPRITE_URL = "/img/icons.svg?v=20260423-3";
  const CALCULATOR_DATA_STORAGE_KEY = "fishystuff.calculator.data.v1";
  const CALCULATOR_UI_STORAGE_KEY = "fishystuff.calculator.ui.v1";
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const CALCULATOR_REACTIVE_SIGNAL_PATCH_EVENT = "fishystuff-calculator-patch-signals";
  const CALCULATOR_PERSIST_EXCLUDE_SIGNAL_PATTERN =
    /^_(?:loading|calc|live|defaults|user_presets|preset_manager_ui)(?:\.|$)/;
  const CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN = /^_/;
  const CALCULATOR_PACK_LEADER_SIGNAL_PATTERN = /^pet[1-5]\.packLeader$/;
  const CALCULATOR_PET_CARD_SIGNAL_PATTERN = /^pet[1-5](?:\.|$)/;
  const CALCULATOR_TARGET_FISH_SELECT_SIGNAL_PATTERN = /^zone$/;
  const CALCULATOR_ACTION_SIGNAL_PATTERN = /^_calculator_actions(?:\.|$)/;
  const CALCULATOR_LAYOUT_UI_SIGNAL_PATTERN = /^_calculator_ui(?:\.|$)/;
  const CALCULATOR_PRESET_SIGNAL_FILTER = {
    include: /^(?!debug(?:\.|$)|overlay(?:\.|$)|_)[^.]+(?:\.|$)/,
  };
  const CALCULATOR_SECTION_TABS = new Set([
    "mode",
    "overview",
    "zone",
    "bite_time",
    "catch_time",
    "session",
    "distribution",
    "loot",
    "trade",
    "gear",
    "food",
    "buffs",
    "pets",
    "overlay",
    "debug",
  ]);
  const CALCULATOR_WORKSPACE_TABS = new Set([
    "basics",
    "loadout",
    "loot",
    "trade",
    "advanced",
    "custom",
  ]);
  const CALCULATOR_DEFAULT_WORKSPACE_TAB = "basics";
  const CALCULATOR_CUSTOM_WORKSPACE_TAB = "custom";
  const CALCULATOR_WORKSPACE_SECTIONS = Object.freeze({
    basics: Object.freeze(["overview", "zone", "session", "bite_time"]),
    loadout: Object.freeze(["gear", "food", "buffs", "pets"]),
    loot: Object.freeze(["distribution", "loot"]),
    trade: Object.freeze(["trade"]),
    advanced: Object.freeze(["mode", "catch_time", "overlay", "debug"]),
  });
  const CALCULATOR_PRESET_COLLECTION_KEY = "calculator-presets";
  const CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY = "calculator-layouts";
  const CALCULATOR_DEFAULT_CUSTOM_SECTIONS = Object.freeze([
    "overview",
    "zone",
    "session",
    "bite_time",
    "loot",
  ]);
  const CALCULATOR_DEFAULT_CUSTOM_LAYOUT = Object.freeze([
    Object.freeze([Object.freeze(["overview"])]),
    Object.freeze([Object.freeze(["zone"]), Object.freeze(["session"])]),
    Object.freeze([Object.freeze(["bite_time"]), Object.freeze(["loot"])]),
  ]);
  const CALCULATOR_DISTRIBUTION_TABS = new Set(["groups", "silver", "loot_flow", "target_fish"]);
  const CALCULATOR_ACTION_DEFAULTS = Object.freeze({
    copyUrlToken: 0,
    copyShareToken: 0,
    saveCalculatorToken: 0,
    discardCalculatorToken: 0,
    saveLayoutToken: 0,
    discardLayoutToken: 0,
  });
  const DEFAULT_CALCULATOR_LOCALE = "en-US";
  const BREAKDOWN_SECTION_KEYS = Object.freeze(["inputs", "composition"]);

  const calculatorState = {
    persistBinding: null,
    evalPatchBinding: null,
    actionBinding: null,
    layoutPresetBinding: null,
    calculatorPresetBinding: null,
    uiStateRestored: false,
    calculatorPresetAdapterBound: false,
    layoutPresetAdapterBound: false,
    pendingCalculatorPresetRestore: false,
    pendingLayoutPresetRestore: false,
    pendingCalculatorDataState: null,
    pendingCalculatorUiState: null,
    pendingEvalNeedsPetCards: null,
    pendingEvalNeedsTargetFishSelect: null,
    reactivePatchApplied: false,
  };
  const calculatorPetUiState = {
    imageFallbackBound: false,
  };

  const signalStore = window.__fishystuffDatastarState.createPageSignalStore();
  const calculatorActionTokens =
    window.__fishystuffDatastarState.createCounterTokenController(
      CALCULATOR_ACTION_DEFAULTS,
    );

  function languageHelper() {
    const helper = window.__fishystuffLanguage;
    return helper && typeof helper.current === "function" && typeof helper.t === "function"
      ? helper
      : null;
  }

  function languageReady() {
    const ready = languageHelper()?.ready;
    return ready && typeof ready.then === "function" ? ready.catch(() => {}) : Promise.resolve();
  }

  const replacePetImageWithFallback = (image) => {
    if (!(image instanceof HTMLImageElement)) {
      return;
    }
    const frame = image.closest(".fishy-calculator-pet-option__frame");
    if (!(frame instanceof HTMLElement)) {
      return;
    }
    const fallback = document.createElement("span");
    fallback.className = "fishy-calculator-pet-option__fallback fishy-item-icon-fallback";
    fallback.textContent = String(
      image.dataset.fallbackLabel || image.getAttribute("data-fallback-label") || image.alt || "?",
    )
      .trim()
      .charAt(0)
      .toUpperCase() || "?";
    frame.replaceChildren(fallback);
  };

  function bindPetImageFallbackListener() {
    if (calculatorPetUiState.imageFallbackBound) {
      return;
    }
    document.addEventListener("error", (event) => {
      if (!(event.target instanceof HTMLImageElement)) {
        return;
      }
      if (!event.target.classList.contains("fishy-calculator-pet-option__image")) {
        return;
      }
      replacePetImageWithFallback(event.target);
    }, true);
    calculatorPetUiState.imageFallbackBound = true;
  }

  function calculatorSurfaceLanguage() {
    const current = languageHelper()?.current?.() || {};
    const locale = String(current.locale || document.documentElement.lang || "en-US").trim();
    const localeKey = locale.toLowerCase();
    const localeBase = localeKey.split(/[-_]/)[0];
    const apiLang = String(current.apiLang || "").trim().toLowerCase();
    const resolvedApiLang = apiLang || "en";
    if (localeBase === "ko") {
      return {
        locale: "ko-KR",
        apiLang: resolvedApiLang,
        lang: resolvedApiLang,
      };
    }
    if (localeBase === "de") {
      return {
        locale: "de-DE",
        apiLang: resolvedApiLang,
        lang: resolvedApiLang,
      };
    }
    return {
      locale: "en-US",
      apiLang: resolvedApiLang,
      lang: resolvedApiLang,
    };
  }

  function calculatorText(key, vars = {}, options = {}) {
    const helper = languageHelper();
    if (!helper) {
      return `calculator.${key}`;
    }
    return helper.t(`calculator.${key}`, vars, {
      locale: options.locale || calculatorSurfaceLanguage().locale,
    });
  }

  function normalizeFishingMode(mode) {
    const normalized = String(mode ?? "").trim().toLowerCase();
    return normalized === "hotspot" || normalized === "harpoon" ? normalized : "rod";
  }

  function effectiveActivity(mode, active) {
    return normalizeFishingMode(mode) === "harpoon" || Boolean(active);
  }

  function breakdownSectionLabel(key) {
    return calculatorText(`breakdown.section.${key}`);
  }

  function breakdownLabel(key, vars = {}) {
    return calculatorText(`breakdown.label.${key}`, vars);
  }

  function breakdownSummary(key, vars = {}) {
    return calculatorText(`breakdown.summary.${key}`, vars);
  }

  function breakdownDetail(key, vars = {}) {
    return calculatorText(`breakdown.detail.${key}`, vars);
  }

  function breakdownFormula(key, vars = {}) {
    return calculatorText(`breakdown.formula.${key}`, vars);
  }

  function breakdownTitle(key, vars = {}) {
    return calculatorText(`breakdown.title.${key}`, vars);
  }

  function calculatorTitle(key, vars = {}) {
    return calculatorText(`title.${key}`, vars);
  }

  function timelineLabel(key) {
    return calculatorText(`timeline.${key}`);
  }

  function uniqueTextVariants(values) {
    return Array.from(
      new Set(
        values
          .map((value) => String(value ?? "").trim())
          .filter(Boolean),
      ),
    );
  }

  function breakdownSectionAliases(key) {
    return uniqueTextVariants([
      breakdownSectionLabel(key),
      calculatorText(`breakdown.section.${key}`, {}, { locale: DEFAULT_CALCULATOR_LOCALE }),
    ]);
  }

  function breakdownLabelAliases(key, vars = {}) {
    return uniqueTextVariants([
      breakdownLabel(key, vars),
      calculatorText(`breakdown.label.${key}`, vars, { locale: DEFAULT_CALCULATOR_LOCALE }),
    ]);
  }

  function breakdownSectionKey(label) {
    const normalized = String(label ?? "").trim();
    return BREAKDOWN_SECTION_KEYS.find((key) => breakdownSectionAliases(key).includes(normalized)) || "";
  }

  function breakdownLabelMatches(label, key, vars = {}) {
    const normalized = String(label ?? "").trim();
    return breakdownLabelAliases(key, vars).includes(normalized);
  }

  function sharedUserOverlays() {
    const helper = window.__fishystuffUserOverlays;
    return helper && typeof helper.overlaySignals === "function" && typeof helper.priceOverrides === "function"
      ? helper
      : null;
  }

  function sharedUserPresets() {
    const helper = window.__fishystuffUserPresets;
    return helper
      && typeof helper.registerCollectionAdapter === "function"
      && typeof helper.capturePayload === "function"
      && typeof helper.activatePreset === "function"
      && typeof helper.trackCurrentPayload === "function"
      && typeof helper.currentActionState === "function"
      ? helper
      : null;
  }

  function datastarPersistHelper() {
    const helper = window.__fishystuffDatastarPersist;
    return helper && typeof helper.createDebouncedSignalPatchPersistor === "function"
      ? helper
      : null;
  }

  const isPlainObject = (value) => value && typeof value === "object" && !Array.isArray(value);

  function mergeCalculatorSignalPatch(target, patch) {
    if (!isPlainObject(target) || !isPlainObject(patch)) {
      return target;
    }
    for (const [key, value] of Object.entries(patch)) {
      if (isPlainObject(value) && isPlainObject(target[key])) {
        mergeCalculatorSignalPatch(target[key], value);
      } else {
        target[key] = cloneCalculatorSignals(value);
      }
    }
    return target;
  }

  function assignCalculatorSignalPatch(target, patch, options = {}) {
    if (!isPlainObject(target) || !isPlainObject(patch)) {
      return target;
    }
    if (options.replaceCalculatorData === true) {
      for (const key of Object.keys(target)) {
        if (!key.startsWith("_") && key !== "overlay" && !(key in patch)) {
          delete target[key];
        }
      }
    }
    if (options.replaceTopLevel === true || options.replaceCalculatorData === true) {
      for (const [key, value] of Object.entries(patch)) {
        target[key] = cloneCalculatorSignals(value);
      }
      return target;
    }
    return mergeCalculatorSignalPatch(target, patch);
  }

  function readSignalPath(source, path) {
    return String(path ?? "")
      .split(".")
      .filter(Boolean)
      .reduce((current, key) => (
        current && typeof current === "object" && key in current ? current[key] : undefined
      ), source);
  }

  function writeControlValue(control, value) {
    if (!(control instanceof HTMLElement)) {
      return;
    }
    if (control instanceof HTMLInputElement) {
      if (control.type === "checkbox") {
        control.checked = typeof value === "string" ? value === control.value : Boolean(value);
        return;
      }
      if (control.type === "radio") {
        control.checked = value === (typeof value === "number" ? Number(control.value) : control.value);
        return;
      }
      control.value = value == null ? "" : String(value);
      return;
    }
    if (control instanceof HTMLSelectElement) {
      if (control.multiple) {
        const selected = new Set((Array.isArray(value) ? value : []).map(String));
        for (const option of Array.from(control.options)) {
          option.selected = selected.has(option.value);
        }
        return;
      }
      control.value = value == null ? "" : String(value);
      return;
    }
    if (control instanceof HTMLTextAreaElement) {
      control.value = value == null ? "" : String(value);
    }
  }

  function syncBoundCalculatorControls(patch) {
    if (!isPlainObject(patch) || typeof document.querySelectorAll !== "function") {
      return;
    }
    for (const control of Array.from(document.querySelectorAll("[data-bind]"))) {
      const path = String(control.getAttribute("data-bind") || "").trim();
      if (!path) {
        continue;
      }
      const value = readSignalPath(patch, path);
      if (value !== undefined) {
        writeControlValue(control, value);
      }
    }
  }

  function signalPatchLeafPaths(patch, prefix = "") {
    if (!isPlainObject(patch)) {
      return [];
    }
    const paths = [];
    for (const [key, value] of Object.entries(patch)) {
      const path = prefix ? `${prefix}.${key}` : key;
      if (isPlainObject(value)) {
        paths.push(...signalPatchLeafPaths(value, path));
      } else {
        paths.push(path);
      }
    }
    return paths;
  }

  function patchMatchesCalculatorEvalFilter(patch) {
    const helper = datastarPersistHelper();
    const patchMatches = helper && typeof helper.patchMatchesSignalFilter === "function"
      ? helper.patchMatchesSignalFilter
      : null;
    return patchMatches
      ? patchMatches(patch, calculatorEvalSignalPatchFilter())
      : signalPatchLeafPaths(patch).some((path) => !CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN.test(path));
  }

  function calculatorEvalPatchPaths(patch) {
    return signalPatchLeafPaths(patch)
      .filter((path) => !CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN.test(path));
  }

  function clearPendingEvalElementPatches() {
    calculatorState.pendingEvalNeedsPetCards = null;
    calculatorState.pendingEvalNeedsTargetFishSelect = null;
  }

  function calculatorEvalOptionsForPatch(patch) {
    const paths = calculatorEvalPatchPaths(patch);
    const touchesPetCards = paths.some((path) => CALCULATOR_PET_CARD_SIGNAL_PATTERN.test(path))
      && !paths.every((path) => CALCULATOR_PACK_LEADER_SIGNAL_PATTERN.test(path));
    return {
      includePetCards: touchesPetCards,
      includeTargetFishSelect: paths.some((path) => CALCULATOR_TARGET_FISH_SELECT_SIGNAL_PATTERN.test(path)),
    };
  }

  function noteCalculatorEvalPatch(patch) {
    if (!calculatorState.uiStateRestored) {
      return;
    }
    if (!patchMatchesCalculatorEvalFilter(patch)) {
      return;
    }
    const paths = calculatorEvalPatchPaths(patch);
    if (!paths.length) {
      return;
    }
    const options = calculatorEvalOptionsForPatch(patch);
    if (options.includeTargetFishSelect) {
      calculatorState.pendingEvalNeedsTargetFishSelect = true;
    }
    if (!options.includePetCards) {
      if (calculatorState.pendingEvalNeedsPetCards !== true) {
        calculatorState.pendingEvalNeedsPetCards = false;
      }
      return;
    }
    calculatorState.pendingEvalNeedsPetCards = true;
  }

  function bindPersistListener() {
    if (calculatorState.persistBinding) {
      return;
    }
    const helper = datastarPersistHelper();
    if (!helper) {
      return;
    }
    calculatorState.persistBinding = helper.createDebouncedSignalPatchPersistor({
      delayMs: 150,
      isReady() {
        return calculatorState.uiStateRestored;
      },
      filter: {
        exclude: CALCULATOR_PERSIST_EXCLUDE_SIGNAL_PATTERN,
      },
      persist() {
        const signals = signalStore.signalObject();
        if (!signals) {
          return;
        }
        persistCalculator(signals);
      },
    });
    calculatorState.persistBinding.bind();
  }

  function bindEvalPatchListener() {
    if (calculatorState.evalPatchBinding) {
      return;
    }
    const handleSignalPatch = (event) => {
      noteCalculatorEvalPatch(event && event.detail ? event.detail : null);
    };
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    calculatorState.evalPatchBinding = {
      dispose() {
        document.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
      },
    };
  }

  function bindActionListener() {
    if (calculatorState.actionBinding) {
      return;
    }
    const helper = datastarPersistHelper();
    const patchMatches = helper && typeof helper.patchMatchesSignalFilter === "function"
      ? helper.patchMatchesSignalFilter
      : null;
    if (!patchMatches) {
      return;
    }
    const handleSignalPatch = (event) => {
      if (!calculatorState.uiStateRestored) {
        return;
      }
      const patch = event && event.detail ? event.detail : null;
      if (!patchMatches(patch, { include: CALCULATOR_ACTION_SIGNAL_PATTERN })) {
        return;
      }
      const signals = signalStore.signalObject();
      if (!signals) {
        return;
      }
      syncCalculatorActions(signals);
    };
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    calculatorState.actionBinding = {
      dispose() {
        document.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
      },
    };
  }

  function bindLayoutPresetListener() {
    if (calculatorState.layoutPresetBinding) {
      return;
    }
    const helper = datastarPersistHelper();
    const patchMatches = helper && typeof helper.patchMatchesSignalFilter === "function"
      ? helper.patchMatchesSignalFilter
      : null;
    if (!patchMatches) {
      return;
    }
    const handleSignalPatch = (event) => {
      if (!calculatorState.uiStateRestored) {
        return;
      }
      const patch = event && event.detail ? event.detail : null;
      const signals = signalStore.signalObject();
      if (!signals) {
        return;
      }
      const hasPendingLayoutRestore = calculatorState.pendingLayoutPresetRestore
        || calculatorState.pendingCalculatorUiState;
      if (hasPendingLayoutRestore && calculatorRestoreReadyPatch(patch)) {
        calculatorState.pendingLayoutPresetRestore = false;
        const pendingUiState = calculatorState.pendingCalculatorUiState;
        calculatorState.pendingCalculatorUiState = null;
        if (pendingUiState) {
          signals._calculator_ui = cloneCalculatorSignals(pendingUiState);
        }
        const applied = pendingUiState
          ? null
          : applyStoredCalculatorLayoutPresetState(signals);
        if (pendingUiState) {
          trackCalculatorLayoutPresetCurrent(signals);
        }
        if ((applied || pendingUiState) && typeof window.__fishystuffCalculator?.patchSignals === "function") {
          window.__fishystuffCalculator.patchSignals({
            _calculator_ui: cloneCalculatorSignals(signals._calculator_ui),
          }, {
            eval: false,
            replaceTopLevel: true,
          });
          return;
        }
      }
      if (hasPendingLayoutRestore && calculatorInitPatch(patch)) {
        return;
      }
      const layoutPatch = patchMatches(patch, { include: CALCULATOR_LAYOUT_UI_SIGNAL_PATTERN });
      if (!layoutPatch) {
        return;
      }
      if (calculatorState.pendingCalculatorUiState) {
        calculatorState.pendingCalculatorUiState = cloneCalculatorSignals(signals._calculator_ui);
      }
      trackCalculatorLayoutPresetCurrent(signals);
    };
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    calculatorState.layoutPresetBinding = {
      dispose() {
        document.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
      },
    };
  }

  function bindCalculatorPresetListener() {
    if (calculatorState.calculatorPresetBinding) {
      return;
    }
    const helper = datastarPersistHelper();
    const patchMatches = helper && typeof helper.patchMatchesSignalFilter === "function"
      ? helper.patchMatchesSignalFilter
      : null;
    if (!patchMatches) {
      return;
    }
    const handleSignalPatch = (event) => {
      if (!calculatorState.uiStateRestored) {
        return;
      }
      const patch = event && event.detail ? event.detail : null;
      const signals = signalStore.signalObject();
      if (!signals) {
        return;
      }
      if (calculatorState.pendingCalculatorPresetRestore && calculatorRestoreReadyPatch(patch)) {
        calculatorState.pendingCalculatorPresetRestore = false;
        const pendingCalculatorData = calculatorState.pendingCalculatorDataState;
        calculatorState.pendingCalculatorDataState = null;
        if (pendingCalculatorData) {
          const restoredPatch = calculatorPresetPatch(signals, pendingCalculatorData);
          if (typeof window.__fishystuffCalculator?.patchSignals === "function") {
            window.__fishystuffCalculator.patchSignals(restoredPatch, {
              eval: true,
              replaceCalculatorData: true,
            });
          } else {
            Object.assign(signals, cloneCalculatorSignals(restoredPatch));
          }
          return;
        }
        const applied = applyStoredCalculatorPresetState(signals);
        if (applied && typeof window.__fishystuffCalculator?.patchSignals === "function") {
          window.__fishystuffCalculator.patchSignals(
            calculatorPresetPatch(signals, applied.payload),
            {
              eval: true,
              replaceCalculatorData: true,
            },
          );
          return;
        }
      }
      if (calculatorState.pendingCalculatorPresetRestore && calculatorInitPatch(patch)) {
        return;
      }
      if (calculatorInitPatch(patch)) {
        trackCalculatorPresetCurrent(signals);
        return;
      }
      if (!patchMatches(patch, CALCULATOR_PRESET_SIGNAL_FILTER)) {
        return;
      }
      trackCalculatorPresetCurrent(signals);
    };
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    calculatorState.calculatorPresetBinding = {
      dispose() {
        document.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
      },
    };
  }

  const urlParams = new URLSearchParams(window.location.search);
  const presetQueryParam = urlParams.get("preset");

  const loadStoredJson = (storageKey, label) => {
    const raw = localStorage.getItem(storageKey);
    if (!raw) {
      return null;
    }
    try {
      const parsed = JSON.parse(raw);
      return parsed && typeof parsed === "object" ? parsed : null;
    } catch (error) {
      console.error(`Error parsing stored ${label}:`, error);
      return null;
    }
  };

  const loadStoredSignals = () => {
    const storedData = loadStoredJson(CALCULATOR_DATA_STORAGE_KEY, "calculator data");
    const storedUi = loadStoredJson(CALCULATOR_UI_STORAGE_KEY, "calculator UI state");
    if (!storedData && !storedUi) {
      return {
        signals: null,
        hasStoredData: false,
        hasStoredUi: false,
      };
    }
    const combined = storedData && typeof storedData === "object" ? { ...storedData } : {};
    if (storedUi && typeof storedUi === "object") {
      combined._calculator_ui = storedUi;
    }
    return {
      signals: combined,
      hasStoredData: Boolean(storedData),
      hasStoredUi: Boolean(storedUi),
    };
  };

  const compactStringArray = (value) => {
    if (!Array.isArray(value)) {
      return [];
    }
    const seen = new Set();
    const out = [];
    for (const entry of value) {
      const normalized = String(entry ?? "").trim();
      if (!normalized || seen.has(normalized)) {
        continue;
      }
      seen.add(normalized);
      out.push(normalized);
    }
    return out;
  };

  const canonicalizePetSignals = (value) => {
    const current = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    return {
      ...current,
      packLeader: normalizeBooleanFlag(current.packLeader, false),
      skills: compactStringArray(current.skills),
    };
  };

  const normalizeCustomSections = (
    value,
    fallback = CALCULATOR_DEFAULT_CUSTOM_SECTIONS,
  ) => {
    const normalizeList = (entries) => compactStringArray(entries)
      .filter((entry) => CALCULATOR_SECTION_TABS.has(entry));
    if (Array.isArray(value)) {
      return normalizeList(value);
    }
    return normalizeList(fallback);
  };

  const flattenCustomLayout = (value) => {
    const seen = new Set();
    const rows = Array.isArray(value) ? value : [];
    const out = [];
    for (const row of rows) {
      if (!Array.isArray(row)) {
        continue;
      }
      for (const column of row) {
        if (!Array.isArray(column)) {
          continue;
        }
        for (const entry of column) {
          const normalized = String(entry ?? "").trim();
          if (!normalized || !CALCULATOR_SECTION_TABS.has(normalized) || seen.has(normalized)) {
            continue;
          }
          seen.add(normalized);
          out.push(normalized);
        }
      }
    }
    return out;
  };

  const normalizeCustomLayout = (
    value,
    fallback = CALCULATOR_DEFAULT_CUSTOM_LAYOUT,
  ) => {
    const fallbackRows = Array.isArray(fallback?.[0]?.[0])
      ? fallback
      : normalizeCustomSections(fallback).map((sectionId) => [[sectionId]]);
    const rows = Array.isArray(value) ? value : fallbackRows;
    const seen = new Set();
    const out = [];
    for (const row of rows) {
      if (!Array.isArray(row)) {
        continue;
      }
      const normalizedRow = [];
      for (const column of row) {
        if (!Array.isArray(column)) {
          continue;
        }
        const normalizedColumn = [];
        for (const entry of column) {
          const normalized = String(entry ?? "").trim();
          if (!normalized || !CALCULATOR_SECTION_TABS.has(normalized) || seen.has(normalized)) {
            continue;
          }
          seen.add(normalized);
          normalizedColumn.push(normalized);
        }
        if (normalizedColumn.length) {
          normalizedRow.push(normalizedColumn);
        }
      }
      if (normalizedRow.length) {
        out.push(normalizedRow);
      }
    }
    if (Array.isArray(value)) {
      return out;
    }
    if (out.length) {
      return out;
    }
    const normalizedFallback = normalizeCustomSections(fallbackRows.flat(2));
    return normalizedFallback.length
      ? normalizedFallback.map((sectionId) => [[sectionId]])
      : normalizeCustomSections(CALCULATOR_DEFAULT_CUSTOM_SECTIONS)
        .map((sectionId) => [[sectionId]]);
  };

  const normalizeCalculatorWorkspaceTab = (value) => {
    const normalized = String(value ?? "").trim();
    if (CALCULATOR_WORKSPACE_TABS.has(normalized)) {
      return normalized;
    }
    return CALCULATOR_DEFAULT_WORKSPACE_TAB;
  };

  const customSectionsFromUiState = (value) => {
    if (Array.isArray(value)) {
      return normalizeCustomSections(value);
    }
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      return normalizeCustomSections(undefined);
    }
    if (Object.prototype.hasOwnProperty.call(value, "custom_layout")) {
      return flattenCustomLayout(value.custom_layout);
    }
    return flattenCustomLayout(normalizeCustomLayout(undefined));
  };

  const customLayoutFromUiState = (value) => {
    if (Array.isArray(value)) {
      return Array.isArray(value[0]?.[0])
        ? normalizeCustomLayout(value)
        : normalizeCustomLayout(undefined, value);
    }
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      return normalizeCustomLayout(undefined);
    }
    if (Object.prototype.hasOwnProperty.call(value, "custom_layout")) {
      return normalizeCustomLayout(value.custom_layout);
    }
    return normalizeCustomLayout(undefined);
  };

  const uiStateWithCustomLayout = (value, layout) => {
    const current = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    const customLayout = normalizeCustomLayout(layout);
    return {
      ...current,
      custom_layout: customLayout,
      custom_sections: flattenCustomLayout(customLayout),
    };
  };

  const defaultCalculatorLayoutUiState = (value) => {
    const current = normalizeCalculatorUiState(value);
    return {
      ...uiStateWithCustomLayout(current, CALCULATOR_DEFAULT_CUSTOM_LAYOUT),
    };
  };

  const normalizeCalculatorUiState = (value) => {
    const current = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    const distributionTab = String(
      current.distribution_tab || "groups",
    ).trim();
    const customLayout = customLayoutFromUiState(current);
    const normalized = {
      workspace_tab: normalizeCalculatorWorkspaceTab(current.workspace_tab),
      distribution_tab: CALCULATOR_DISTRIBUTION_TABS.has(distributionTab)
        ? distributionTab
        : "groups",
      custom_layout: customLayout,
      custom_sections: flattenCustomLayout(customLayout),
    };
    return normalized;
  };

  const cloneCalculatorSignals = (value) => JSON.parse(JSON.stringify(value));
  const normalizeBooleanFlag = (value, fallback = false) =>
    value == null ? fallback : value === true || value === "true" || value === 1 || value === "1";
  const petSkillLimitForTier = (tier) => {
    switch (String(tier ?? "").trim()) {
      case "3":
        return 2;
      case "4":
      case "5":
        return 3;
      default:
        return 1;
    }
  };

  function petSkillSlots(tier, ...values) {
    const selected = [];
    const limit = petSkillLimitForTier(tier);
    for (const value of values.slice(0, limit)) {
      const normalized = String(value ?? "").trim();
      if (normalized && !selected.includes(normalized)) {
        selected.push(normalized);
      }
    }
    return selected;
  }

  function normalizeSectionId(sectionId) {
    return String(sectionId ?? "").trim();
  }

  function customSectionIndex(customSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    return customSectionsFromUiState(customSections).indexOf(normalizedSection);
  }

  function isCustomSection(customSections, sectionId) {
    return customSectionIndex(customSections, sectionId) >= 0;
  }

  function cloneCustomLayout(layout) {
    return normalizeCustomLayout(layout).map((row) => row.map((column) => [...column]));
  }

  function compactCustomLayout(layout) {
    return layout
      .map((row) => row
        .map((column) => column.filter(Boolean))
        .filter((column) => column.length))
      .filter((row) => row.length);
  }

  function removeSectionFromCustomLayout(layout, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    return compactCustomLayout(
      cloneCustomLayout(layout).map((row) => row
        .map((column) => column.filter((entry) => entry !== normalizedSection))),
    );
  }

  function appendSectionRow(layout, sectionId) {
    return [
      ...cloneCustomLayout(layout),
      [[sectionId]],
    ];
  }

  function toggleCustomSection(customSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    if (!CALCULATOR_SECTION_TABS.has(normalizedSection)) {
      return Array.isArray(customSections)
        ? normalizeCustomSections(customSections)
        : normalizeCalculatorUiState(customSections);
    }
    const uiState = !Array.isArray(customSections)
      && customSections
      && typeof customSections === "object"
      ? normalizeCalculatorUiState(customSections)
      : null;
    const currentLayout = customLayoutFromUiState(customSections);
    if (flattenCustomLayout(currentLayout).includes(normalizedSection)) {
      const nextLayout = removeSectionFromCustomLayout(currentLayout, normalizedSection);
      return uiState ? uiStateWithCustomLayout(uiState, nextLayout) : flattenCustomLayout(nextLayout);
    }
    const nextLayout = appendSectionRow(currentLayout, normalizedSection);
    return uiState ? uiStateWithCustomLayout(uiState, nextLayout) : flattenCustomLayout(nextLayout);
  }

  function assignCustomUiState(targetUiState, nextUiState) {
    if (!targetUiState || typeof targetUiState !== "object" || Array.isArray(targetUiState)) {
      return normalizeCalculatorUiState(nextUiState);
    }
    const normalized = normalizeCalculatorUiState({
      ...targetUiState,
      ...(nextUiState && typeof nextUiState === "object" && !Array.isArray(nextUiState)
        ? nextUiState
        : {}),
    });
    targetUiState.workspace_tab = normalized.workspace_tab;
    targetUiState.distribution_tab = normalized.distribution_tab;
    targetUiState.custom_layout = normalized.custom_layout;
    targetUiState.custom_sections = normalized.custom_sections;
    return targetUiState;
  }

  function toggleCustomSectionInPlace(uiState, sectionId) {
    return assignCustomUiState(uiState, toggleCustomSection(uiState, sectionId));
  }

  function removeCustomSection(customSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    const uiState = !Array.isArray(customSections)
      && customSections
      && typeof customSections === "object"
      ? normalizeCalculatorUiState(customSections)
      : null;
    const nextLayout = removeSectionFromCustomLayout(customLayoutFromUiState(customSections), normalizedSection);
    return uiState ? uiStateWithCustomLayout(uiState, nextLayout) : flattenCustomLayout(nextLayout);
  }

  function removeCustomSectionInPlace(uiState, sectionId) {
    return assignCustomUiState(uiState, removeCustomSection(uiState, sectionId));
  }

  function resetCalculatorLayout(uiState) {
    return defaultCalculatorLayoutUiState(uiState);
  }

  function resetCalculatorLayoutInPlace(uiState) {
    return assignCustomUiState(uiState, resetCalculatorLayout(uiState));
  }

  function normalizeCalculatorLayoutPresetPayload(value) {
    const current = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    return {
      custom_layout: customLayoutFromUiState(current),
    };
  }

  function calculatorLayoutPresetPayload(uiState) {
    return normalizeCalculatorLayoutPresetPayload(uiState);
  }

  function applyCalculatorLayoutPreset(uiState, payload) {
    const current = normalizeCalculatorUiState(uiState);
    const layoutPreset = normalizeCalculatorLayoutPresetPayload(payload);
    return assignCustomUiState(current, {
      custom_layout: layoutPreset.custom_layout,
      custom_sections: flattenCustomLayout(layoutPreset.custom_layout),
    });
  }

  function applyCalculatorLayoutPresetInPlace(uiState, payload) {
    return assignCustomUiState(uiState, applyCalculatorLayoutPreset(uiState, payload));
  }

  function defaultCalculatorLayoutPresetPayload() {
    return normalizeCalculatorLayoutPresetPayload({
      custom_layout: CALCULATOR_DEFAULT_CUSTOM_LAYOUT,
    });
  }

  function presetPreviewHelper() {
    return window.__fishystuffPresetPreviews || null;
  }

  function presetPreviewTitleIconAlias(collectionKey, payload) {
    return presetPreviewHelper()?.titleIconAlias?.(collectionKey, { payload }) || "";
  }

  function renderSharedPresetPreview(collectionKey, container, context = {}) {
    presetPreviewHelper()?.render?.(container, {
      ...context,
      collectionKey,
    });
  }

  function dispatchCalculatorSignalPatch(patch) {
    document.dispatchEvent(new CustomEvent(DATASTAR_SIGNAL_PATCH_EVENT, {
      detail: cloneCalculatorSignals(patch),
    }));
  }

  function applyReactiveSignalPatch(signals, detail) {
    const current = signals && typeof signals === "object"
      ? signals
      : signalStore.signalObject();
    const patch = detail && Object.prototype.hasOwnProperty.call(detail, "patch")
      ? detail.patch
      : detail;
    if (!current || !isPlainObject(patch)) {
      return null;
    }
    const normalizedPatch = cloneCalculatorSignals(patch);
    calculatorState.reactivePatchApplied = true;
    signalStore.connect(current);
    assignCalculatorSignalPatch(current, normalizedPatch, {
      replaceCalculatorData: detail?.replaceCalculatorData === true,
      replaceTopLevel: detail?.replaceTopLevel === true,
    });
    syncBoundCalculatorControls(normalizedPatch);
    dispatchCalculatorSignalPatch(normalizedPatch);
    return current;
  }

  function patchCalculatorSignals(patch, options = {}) {
    if (!isPlainObject(patch)) {
      return null;
    }
    const normalizedPatch = cloneCalculatorSignals(patch);
    const detail = {
      patch: normalizedPatch,
      eval: options.eval === true || (options.eval !== false && patchMatchesCalculatorEvalFilter(normalizedPatch)),
      replaceCalculatorData: options.replaceCalculatorData === true,
      replaceTopLevel: options.replaceTopLevel === true,
    };
    calculatorState.reactivePatchApplied = false;
    if (typeof window.dispatchEvent === "function") {
      window.dispatchEvent(new CustomEvent(CALCULATOR_REACTIVE_SIGNAL_PATCH_EVENT, { detail }));
    }
    if (!calculatorState.reactivePatchApplied) {
      applyReactiveSignalPatch(signalStore.signalObject(), detail);
    }
    return signalStore.signalObject();
  }

  function replaceCalculatorPresetSignals(signals, payload) {
    if (!signals || typeof signals !== "object") {
      return null;
    }
    const patch = calculatorPresetPatch(signals, payload);
    patchCalculatorSignals(patch, {
      eval: true,
      replaceCalculatorData: true,
    });
    return calculatorPresetPayload({
      ...signals,
      ...patch,
    });
  }

  function replaceCalculatorUiSignals(signals, nextUiState) {
    if (!signals || typeof signals !== "object") {
      return null;
    }
    const patch = {
      _calculator_ui: cloneCalculatorSignals(nextUiState),
    };
    patchCalculatorSignals(patch, {
      eval: false,
      replaceTopLevel: true,
    });
    return signals._calculator_ui;
  }

  function bindCalculatorPresetAdapter() {
    if (calculatorState.calculatorPresetAdapterBound) {
      return;
    }
    const helper = sharedUserPresets();
    if (!helper) {
      return;
    }
    helper.registerCollectionAdapter(CALCULATOR_PRESET_COLLECTION_KEY, {
      titleKey: "calculator.presets.title",
      titleFallback: "Calculator presets",
      openLabelKey: "calculator.presets.open",
      openLabelFallback: "Calculator Presets",
      managerIconAlias: "settings-6-fill",
      fileBaseName: "fishystuff-calculator-presets",
      defaultPresetName(index) {
        return calculatorText("presets.default_name", {
          index: String(index),
        });
      },
      fixedPresets() {
        const signals = signalStore.signalObject();
        const payload = defaultCalculatorPresetPayload(signals);
        return Object.keys(payload).length
          ? [{
              id: "default",
              name: calculatorText("presets.default"),
              payload,
            }]
          : [];
      },
      normalizePayload: normalizeCalculatorPresetPayload,
      titleIconAlias({ payload }) {
        return presetPreviewTitleIconAlias(CALCULATOR_PRESET_COLLECTION_KEY, payload);
      },
      renderPreview(container, context) {
        renderSharedPresetPreview(CALCULATOR_PRESET_COLLECTION_KEY, container, context);
      },
      capture() {
        const signals = signalStore.signalObject();
        return signals && typeof signals === "object" && calculatorPresetDefaultsReady(signals)
          ? calculatorPresetPayload(signals)
          : null;
      },
      apply(payload) {
        const signals = signalStore.signalObject();
        return replaceCalculatorPresetSignals(signals, payload);
      },
    });
    calculatorState.calculatorPresetAdapterBound = true;
  }

  function bindCalculatorLayoutPresetAdapter() {
    if (calculatorState.layoutPresetAdapterBound) {
      return;
    }
    const helper = sharedUserPresets();
    if (!helper) {
      return;
    }
    helper.registerCollectionAdapter(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY, {
      titleKey: "calculator.layout_presets.title",
      titleFallback: "Workspace presets",
      openLabelKey: "calculator.layout_presets.open",
      openLabelFallback: "Workspace Presets",
      fileBaseName: "fishystuff-calculator-layouts",
      defaultPresetName(index) {
        return calculatorText("layout_presets.default_name", {
          index: String(index),
        });
      },
      fixedPresets() {
        return [{
          id: "default",
          name: calculatorText("layout_presets.default"),
          payload: defaultCalculatorLayoutPresetPayload(),
        }];
      },
      normalizePayload: normalizeCalculatorLayoutPresetPayload,
      titleIconAlias({ payload }) {
        return presetPreviewTitleIconAlias(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY, payload);
      },
      renderPreview(container, context) {
        renderSharedPresetPreview(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY, container, context);
      },
      capture() {
        const signals = signalStore.signalObject();
        return signals && typeof signals === "object"
          ? calculatorLayoutPresetPayload(signals._calculator_ui)
          : null;
      },
      apply(payload) {
        const signals = signalStore.signalObject();
        if (!signals || typeof signals !== "object") {
          return null;
        }
        const nextUiState = applyCalculatorLayoutPreset(signals._calculator_ui, payload);
        return replaceCalculatorUiSignals(signals, nextUiState);
      },
    });
    calculatorState.layoutPresetAdapterBound = true;
  }

  function trackCalculatorLayoutPresetCurrent(signals) {
    const helper = sharedUserPresets();
    if (!helper) {
      return null;
    }
    const payload = calculatorLayoutPresetPayload(signals?._calculator_ui);
    const tracked = helper.trackCurrentPayload(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY, {
      payload,
    });
    helper.refreshDatastar?.();
    return tracked;
  }

  function trackCalculatorPresetCurrent(signals) {
    const helper = sharedUserPresets();
    if (!helper || !calculatorPresetDefaultsReady(signals)) {
      return null;
    }
    const payload = calculatorPresetPayload(signals);
    const tracked = helper.trackCurrentPayload(CALCULATOR_PRESET_COLLECTION_KEY, {
      payload,
    });
    helper.refreshDatastar?.();
    return tracked;
  }

  function presetCollectionActionSnapshot(userPresetsSnapshot, collectionKey) {
    const key = String(collectionKey ?? "").trim();
    if (!key) {
      return null;
    }
    const collections = userPresetsSnapshot?.collections;
    return collections && typeof collections === "object" ? collections[key] || null : null;
  }

  function presetCollectionCanSave(userPresetsSnapshot, collectionKey) {
    return Boolean(presetCollectionActionSnapshot(userPresetsSnapshot, collectionKey)?.canSave);
  }

  function presetCollectionCanDiscard(userPresetsSnapshot, collectionKey) {
    return Boolean(presetCollectionActionSnapshot(userPresetsSnapshot, collectionKey)?.canDiscard);
  }

  function discardPresetCurrent(collectionKey) {
    const helper = sharedUserPresets();
    if (!helper || typeof helper.discardCurrent !== "function") {
      return null;
    }
    const actionState = helper.currentActionState(collectionKey, {
      refresh: true,
      patchDatastar: false,
    });
    if (!actionState.canDiscard) {
      return null;
    }
    const result = helper.discardCurrent(collectionKey, { refreshCurrent: false });
    return result?.current ? null : result;
  }

  function savePresetCurrent(collectionKey) {
    const helper = sharedUserPresets();
    if (!helper || typeof helper.saveCurrent !== "function") {
      return null;
    }
    const actionState = helper.currentActionState(collectionKey, {
      refresh: true,
      patchDatastar: false,
    });
    if (!actionState.canSave) {
      return null;
    }
    return helper.saveCurrent(collectionKey);
  }

  function presetText(key, vars = {}) {
    const helper = languageHelper();
    const normalizedKey = String(key ?? "").trim();
    if (!helper || !normalizedKey) {
      return normalizedKey;
    }
    return helper.t(normalizedKey, vars, {
      locale: calculatorSurfaceLanguage().locale,
    });
  }

  function showPresetSaveToast(result) {
    const savedPreset = result?.preset;
    if (!savedPreset) {
      return;
    }
    const key = result.action === "created" ? "presets.toast.created" : "presets.toast.saved";
    const message = presetText(key, { name: savedPreset.name || "" });
    const toast = window.__fishystuffToast;
    if (typeof toast?.success === "function") {
      toast.success(message);
      return;
    }
    toast?.info?.(message);
  }

  function showPresetActionError(_error, fallbackKey) {
    const message = presetText(fallbackKey);
    const toast = window.__fishystuffToast;
    if (typeof toast?.error === "function") {
      toast.error(message);
      return;
    }
    toast?.info?.(message);
  }

  function calculatorInitPatch(patch) {
    return Boolean(
      patch
      && typeof patch === "object"
      && (
        Object.prototype.hasOwnProperty.call(patch, "_defaults")
        || Object.prototype.hasOwnProperty.call(patch, "_loading")
      ),
    );
  }

  function calculatorRestoreReadyPatch(patch) {
    return Boolean(
      patch
      && typeof patch === "object"
      && Object.prototype.hasOwnProperty.call(patch, "_loading")
    );
  }

  function applyStoredCalculatorLayoutPresetState(signals) {
    const helper = sharedUserPresets();
    if (!signals || typeof signals !== "object") {
      return null;
    }
    const storedCollection = helper?.snapshot?.()?.collections?.[CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY];
    if (!storedCollection?.activeWorkingCopyId) {
      return null;
    }
    const activeWorkingCopy = helper?.activeWorkingCopy?.(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY);
    if (activeWorkingCopy?.payload) {
      const nextUiState = applyCalculatorLayoutPreset(signals._calculator_ui, activeWorkingCopy.payload);
      signals._calculator_ui = cloneCalculatorSignals(nextUiState);
      return activeWorkingCopy;
    }
    return null;
  }

  function applyStoredCalculatorPresetState(signals, options = {}) {
    const helper = sharedUserPresets();
    if (!signals || typeof signals !== "object") {
      return null;
    }
    const storedCollection = helper?.snapshot?.()?.collections?.[CALCULATOR_PRESET_COLLECTION_KEY];
    if (!storedCollection?.activeWorkingCopyId) {
      return null;
    }
    const activeWorkingCopy = helper?.activeWorkingCopy?.(CALCULATOR_PRESET_COLLECTION_KEY);
    const activeSource = activeWorkingCopy?.source || activeWorkingCopy?.origin || {};
    const hasActiveModifications = Boolean(helper?.current?.(CALCULATOR_PRESET_COLLECTION_KEY)?.payload);
    const shouldPreferStoredData = Boolean(
      options.hasStoredData
        && !hasActiveModifications
        && activeSource.kind === "fixed"
        && activeSource.id === "default",
    );
    if (!shouldPreferStoredData && activeWorkingCopy?.payload) {
      Object.assign(signals, cloneCalculatorSignals(calculatorPresetPatch(signals, activeWorkingCopy.payload)));
      return activeWorkingCopy;
    }
    return null;
  }

  function addCustomSection(customSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    if (!CALCULATOR_SECTION_TABS.has(normalizedSection)) {
      return Array.isArray(customSections)
        ? normalizeCustomSections(customSections)
        : normalizeCalculatorUiState(customSections);
    }
    const uiState = !Array.isArray(customSections)
      && customSections
      && typeof customSections === "object"
      ? normalizeCalculatorUiState(customSections)
      : null;
    const nextLayout = cloneCustomLayout(customLayoutFromUiState(customSections));
    if (flattenCustomLayout(nextLayout).includes(normalizedSection)) {
      return uiState ? uiStateWithCustomLayout(uiState, nextLayout) : flattenCustomLayout(nextLayout);
    }
    const customLayout = appendSectionRow(nextLayout, normalizedSection);
    return uiState ? uiStateWithCustomLayout(uiState, customLayout) : flattenCustomLayout(customLayout);
  }

  function placeCustomSection(customSections, sectionId, targetSectionId, position) {
    const normalizedSection = normalizeSectionId(sectionId);
    const normalizedTarget = normalizeSectionId(targetSectionId);
    const normalizedPosition = position === "before" ? "before" : "after";
    if (!CALCULATOR_SECTION_TABS.has(normalizedSection)) {
      return Array.isArray(customSections)
        ? normalizeCustomSections(customSections)
        : normalizeCalculatorUiState(customSections);
    }
    if (!normalizedTarget || normalizedTarget === normalizedSection) {
      return addCustomSection(customSections, normalizedSection);
    }
    const uiState = !Array.isArray(customSections)
      && customSections
      && typeof customSections === "object"
      ? normalizeCalculatorUiState(customSections)
      : null;
    const nextLayout = removeSectionFromCustomLayout(customLayoutFromUiState(customSections), normalizedSection);
    for (let rowIndex = 0; rowIndex < nextLayout.length; rowIndex += 1) {
      for (let columnIndex = 0; columnIndex < nextLayout[rowIndex].length; columnIndex += 1) {
        const targetIndex = nextLayout[rowIndex][columnIndex].indexOf(normalizedTarget);
        if (targetIndex < 0) {
          continue;
        }
        nextLayout[rowIndex][columnIndex].splice(
          targetIndex + (normalizedPosition === "after" ? 1 : 0),
          0,
          normalizedSection,
        );
        return uiState ? uiStateWithCustomLayout(uiState, nextLayout) : flattenCustomLayout(nextLayout);
      }
    }
    const appendedLayout = appendSectionRow(nextLayout, normalizedSection);
    return uiState ? uiStateWithCustomLayout(uiState, appendedLayout) : flattenCustomLayout(appendedLayout);
  }

  function calculatorWorkspaceTab(uiState) {
    const current = normalizeCalculatorUiState(uiState);
    return normalizeCalculatorWorkspaceTab(current.workspace_tab);
  }

  function calculatorSectionVisibleInWorkspace(sectionId, uiState) {
    const normalizedSection = normalizeSectionId(sectionId);
    const current = normalizeCalculatorUiState(uiState);
    const workspaceTab = calculatorWorkspaceTab(current);
    if (workspaceTab === CALCULATOR_CUSTOM_WORKSPACE_TAB) {
      return customSectionsFromUiState(current).includes(normalizedSection);
    }
    return (CALCULATOR_WORKSPACE_SECTIONS[workspaceTab] || []).includes(normalizedSection);
  }

  const canonicalizeStoredSignals = (signals) => {
    const current = { ...(signals ?? {}) };
    const aliases = {
      _active: "active",
      _debug: "debug",
      _level: "level",
      _resources: "resources",
      _catchTimeActive: "catchTimeActive",
      _catchTimeAfk: "catchTimeAfk",
      _timespanAmount: "timespanAmount",
      _timespanUnit: "timespanUnit",
      mode: "fishingMode",
    };
    for (const [legacyKey, canonicalKey] of Object.entries(aliases)) {
      if (!(canonicalKey in current) && legacyKey in current) {
        current[canonicalKey] = current[legacyKey];
      }
      delete current[legacyKey];
    }
    delete current._distribution_tab;
    current._calculator_ui = normalizeCalculatorUiState(current._calculator_ui);
    if (!("discardGrade" in current)) {
      if (current.discardRareFish || current.discardPrizeFish) {
        current.discardGrade = "yellow";
      } else if (current.discardHighQualityFish) {
        current.discardGrade = "blue";
      } else if (current.discardGeneralFish) {
        current.discardGrade = "green";
      } else if (current.discardTrashFish) {
        current.discardGrade = "white";
      } else {
        current.discardGrade = "none";
      }
    }
    delete current.discardTrashFish;
    delete current.discardGeneralFish;
    delete current.discardHighQualityFish;
    delete current.discardRareFish;
    delete current.discardPrizeFish;
    current.fishingMode = normalizeFishingMode(current.fishingMode);
    const validDiscardGrades = new Set(["none", "white", "green", "blue", "yellow"]);
    if (!validDiscardGrades.has(String(current.discardGrade ?? "").trim().toLowerCase())) {
      current.discardGrade = "none";
    } else {
      current.discardGrade = String(current.discardGrade).trim().toLowerCase();
    }
    if (
      !current.priceOverrides
      || typeof current.priceOverrides !== "object"
      || Array.isArray(current.priceOverrides)
    ) {
      current.priceOverrides = {};
    }
    current.priceOverrides = Object.fromEntries(
      Object.entries(current.priceOverrides)
        .map(([key, value]) => {
          const normalizedKey = String(key).trim().replace(/^item:/, "");
          if (!/^\d+$/.test(normalizedKey) || !value || typeof value !== "object" || Array.isArray(value)) {
            return null;
          }
          const tradeValueRaw = value.tradePriceCurvePercent;
          const basePriceRaw = value.basePrice;
          const tradePriceCurvePercent = Number(tradeValueRaw);
          const basePrice = Number(basePriceRaw);
          const normalizedValue = {};
          if (Number.isFinite(tradePriceCurvePercent)) {
            normalizedValue.tradePriceCurvePercent = Math.max(0, tradePriceCurvePercent);
          }
          if (Number.isFinite(basePrice)) {
            normalizedValue.basePrice = Math.max(0, basePrice);
          }
          if (Object.keys(normalizedValue).length === 0) {
            return null;
          }
          return [normalizedKey, normalizedValue];
        })
        .filter(Boolean),
    );
    current.outfit = compactStringArray(current.outfit);
    current.food = compactStringArray(current.food);
    current.buff = compactStringArray(current.buff);
    let packLeaderSeen = false;
    for (const petKey of ["pet1", "pet2", "pet3", "pet4", "pet5"]) {
      if (!current[petKey] || typeof current[petKey] !== "object" || Array.isArray(current[petKey])) {
        continue;
      }
      const normalizedPet = canonicalizePetSignals(current[petKey]);
      normalizedPet.packLeader = normalizedPet.packLeader && !packLeaderSeen;
      packLeaderSeen ||= normalizedPet.packLeader;
      current[petKey] = normalizedPet;
      for (let index = 0; index < 3; index += 1) {
        current[`_${petKey}_skill_slot${index + 1}`] = String(normalizedPet.skills?.[index] ?? "");
      }
    }
    return current;
  };

  const persistedCalculatorSignals = (signals) => {
    const current = canonicalizeStoredSignals(signals);
    return Object.fromEntries(
      Object.entries(current).filter(
        ([key]) => !key.startsWith("_") && key !== "overlay",
      ),
    );
  };

  function calculatorPresetBasePayload(signals = signalStore.signalObject()) {
    return signals?._defaults && typeof signals._defaults === "object"
      ? persistedCalculatorSignals(signals._defaults)
      : {};
  }

  function calculatorPresetDefaultsReady(signals = signalStore.signalObject()) {
    return Object.keys(calculatorPresetBasePayload(signals)).length > 0;
  }

  function normalizeCalculatorPresetPayload(value) {
    return {
      ...calculatorPresetBasePayload(),
      ...persistedCalculatorSignals(value && typeof value === "object" ? value : {}),
    };
  }

  function calculatorPresetPayload(signals = signalStore.signalObject()) {
    return normalizeCalculatorPresetPayload(signals);
  }

  function defaultCalculatorPresetPayload(signals) {
    return signals?._defaults && typeof signals._defaults === "object"
      ? persistedCalculatorSignals(signals._defaults)
      : calculatorPresetBasePayload(signals);
  }

  function calculatorControlSignalPatch(payload) {
    const patch = cloneCalculatorSignals(payload && typeof payload === "object" ? payload : {});
    if ("resources" in patch) {
      patch._resources = patch.resources;
    }
    if ("outfit" in patch) {
      patch._outfit_slots = compactStringArray(patch.outfit);
    }
    if ("food" in patch) {
      patch._food_slots = compactStringArray(patch.food);
    }
    if ("buff" in patch) {
      patch._buff_slots = compactStringArray(patch.buff);
    }
    for (const petKey of ["pet1", "pet2", "pet3", "pet4", "pet5"]) {
      const pet = patch[petKey];
      if (!pet || typeof pet !== "object" || Array.isArray(pet)) {
        continue;
      }
      const skills = compactStringArray(pet.skills);
      for (let index = 0; index < 3; index += 1) {
        patch[`_${petKey}_skill_slot${index + 1}`] = String(skills[index] ?? "");
      }
    }
    return patch;
  }

  function calculatorPresetPatch(signals, payload) {
    return calculatorControlSignalPatch({
      ...defaultCalculatorPresetPayload(signals),
      ...normalizeCalculatorPresetPayload(payload),
    });
  }

  const persistedCalculatorUiSignals = (signals) => {
    const current = canonicalizeStoredSignals(signals);
    return cloneCalculatorSignals(current._calculator_ui);
  };

  const sharedCalculatorSignals = (signals) =>
    Object.fromEntries(
      Object.entries(canonicalizeStoredSignals(signals)).filter(
        ([key]) => !key.startsWith("_") && key !== "debug" && key !== "overlay",
      ),
    );

  const presetURL = (signals) => {
    const payload = JSON.stringify(sharedCalculatorSignals(signals));
    return (
      window.location.origin
      + window.location.pathname
      + "?preset="
      + LZString.compressToEncodedURIComponent(payload)
    );
  };

  if (presetQueryParam) {
    try {
      const jsonString = LZString.decompressFromEncodedURIComponent(presetQueryParam);
      JSON.parse(jsonString);
      localStorage.setItem(CALCULATOR_DATA_STORAGE_KEY, jsonString);

      urlParams.delete("preset");
      const newQueryString = urlParams.toString();
      const newUrl =
        window.location.origin
        + window.location.pathname
        + (newQueryString ? "?" + newQueryString : "");
      window.location.replace(newUrl);
    } catch (error) {
      console.error("Error importing preset:", error);
    }
  }

  const calculatorNumber = (value) => {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  };

  const calculatorFmt2 = (value) => calculatorNumber(value).toFixed(2);
  const calculatorFmtSilver = (value) =>
    Math.max(0, Math.round(calculatorNumber(value))).toLocaleString();
  const calculatorTrimFloat = (value) => calculatorFmt2(value).replace(/\.?0+$/, "");
  const calculatorPercentText = (value) => `${calculatorTrimFloat(value)}%`;
  const calculatorFactorText = (value) => `×${calculatorTrimFloat(value)}`;
  const calculatorPercentage = (value, total) => {
    const safeTotal = calculatorNumber(total);
    if (safeTotal <= 0) {
      return 0;
    }
    return (calculatorNumber(value) / safeTotal) * 100;
  };
  const calculatorTimespanSeconds = (amount, unit) => {
    const unitSeconds = unit === "minutes"
      ? 60
      : unit === "hours"
        ? 3600
        : unit === "days"
          ? 86400
          : 604800;
    return Math.max(0, calculatorNumber(amount)) * unitSeconds;
  };
  const calculatorTimespanText = (amount, unit) => {
    const normalized = Math.max(0, calculatorNumber(amount));
    const normalizedUnit = unit === "minutes"
      ? "minute"
      : unit === "hours"
        ? "hour"
        : unit === "days"
          ? "day"
          : "week";
    const label = calculatorText(`timespan.unit.${normalizedUnit}.${normalized === 1 ? "one" : "other"}`);
    return `${calculatorTrimFloat(normalized)} ${label}`;
  };
  const calculatorAbundanceLabel = (resources) => {
    const value = calculatorNumber(resources);
    if (value <= 14) {
      return calculatorText("resource.exhausted");
    }
    if (value <= 45) {
      return calculatorText("resource.low");
    }
    if (value <= 70) {
      return calculatorText("resource.average");
    }
    return calculatorText("resource.abundant");
  };
  const calculatorBreakdownRow = (label, valueText, detailText, extra = {}) => ({
    ...extra,
    label,
    value_text: valueText,
    detail_text: detailText,
  });
  const calculatorBreakdownFormulaPart = (formulaPart, formulaPartOrder) => ({
    formula_part: formulaPart,
    formula_part_order: formulaPartOrder,
  });
  const calculatorBreakdownFormulaTerm = (label, valueText, aliases = []) => ({
    label,
    value_text: valueText,
    aliases,
  });
  const calculatorJoinFormulaTermValues = (values, separator = ", ", fallback = "0") => {
    const parts = Array.isArray(values)
      ? values
        .map((value) => String(value ?? "").trim())
        .filter(Boolean)
      : [];
    return parts.length ? parts.join(separator) : fallback;
  };
  const calculatorParseBreakdown = (value) => {
    const raw = String(value ?? "").trim();
    if (!raw) {
      return null;
    }
    try {
      const parsed = JSON.parse(raw);
      return parsed && typeof parsed === "object" && !Array.isArray(parsed)
        ? parsed
        : null;
    } catch {
      return null;
    }
  };
  const calculatorBreakdownSectionRows = (raw, sectionKey) => {
    const payload = calculatorParseBreakdown(raw);
    if (!payload || !Array.isArray(payload.sections)) {
      return [];
    }
    const targetKey = String(sectionKey ?? "").trim();
    const section = payload.sections.find((candidate) => breakdownSectionKey(candidate?.label) === targetKey);
    return Array.isArray(section?.rows)
      ? section.rows.map((row) => ({ ...row }))
      : [];
  };
  const calculatorStringifyBreakdown = (payload, fallback = "") => {
    try {
      return JSON.stringify(payload);
    } catch {
      return fallback;
    }
  };
  const calculatorUpdateBreakdown = (raw, options = {}) => {
    const payload = calculatorParseBreakdown(raw);
    if (!payload) {
      return String(raw ?? "");
    }
    const nextPayload = {
      ...payload,
      sections: Array.isArray(payload.sections)
        ? payload.sections.map((section) => ({
            ...section,
            rows: Array.isArray(section?.rows)
              ? section.rows.map((row) => ({ ...row }))
              : [],
          }))
        : [],
    };
    for (const section of nextPayload.sections) {
      const normalizedKey = breakdownSectionKey(section?.label);
      if (normalizedKey) {
        section.label = breakdownSectionLabel(normalizedKey);
      }
    }
    if ("title" in options) {
      nextPayload.title = options.title;
    }
    if ("valueText" in options) {
      nextPayload.value_text = options.valueText;
    }
    if ("summaryText" in options) {
      nextPayload.summary_text = options.summaryText;
    }
    if ("formulaText" in options) {
      nextPayload.formula_text = options.formulaText;
    }
    if ("formulaTerms" in options) {
      nextPayload.formula_terms = Array.isArray(options.formulaTerms)
        ? options.formulaTerms.map((term) => ({
            ...term,
            aliases: Array.isArray(term?.aliases) ? [...term.aliases] : [],
          }))
        : [];
    }
    const replaceSections = options.replaceSections && typeof options.replaceSections === "object"
      ? options.replaceSections
      : null;
    const rowUpdates = options.rowUpdates && typeof options.rowUpdates === "object"
      ? options.rowUpdates
      : null;
    for (const section of nextPayload.sections) {
      const sectionLabel = String(section?.label ?? "");
      const sectionKey = breakdownSectionKey(sectionLabel);
      const replacementRows = replaceSections
        ? (
          (sectionKey && Array.isArray(replaceSections[sectionKey]) ? replaceSections[sectionKey] : null)
          || (Array.isArray(replaceSections[sectionLabel]) ? replaceSections[sectionLabel] : null)
        )
        : null;
      if (replacementRows) {
        section.rows = replacementRows.map((row) => ({ ...row }));
        continue;
      }
      if (!rowUpdates || !Array.isArray(section.rows)) {
        continue;
      }
      for (const row of section.rows) {
        const update = rowUpdates[String(row?.label ?? "")];
        if (!update || typeof row !== "object") {
          continue;
        }
        if ("valueText" in update) {
          row.value_text = update.valueText;
        }
        if ("detailText" in update) {
          row.detail_text = update.detailText;
        }
      }
    }
    return calculatorStringifyBreakdown(nextPayload, String(raw ?? ""));
  };
  const calculatorScaleSilverText = (valueText, ratio) => (
    calculatorFmtSilver(calculatorNumber(String(valueText ?? "").replace(/,/g, "")) * ratio)
  );
  const calculatorTimelineSegment = (
    label,
    valueSeconds,
    widthPct,
    fillColor,
    strokeColor,
    breakdown,
  ) => ({
    label,
    value_text: `${calculatorFmt2(valueSeconds)}s`,
    detail_text: `${calculatorFmt2(widthPct)}%`,
    width_pct: Math.max(0, calculatorNumber(widthPct)),
    fill_color: fillColor,
    stroke_color: strokeColor,
    breakdown,
  });
  const calculatorTimelineChart = ({
    active,
    biteTimeRaw,
    autoFishTimeRaw,
    catchTimeRaw,
    totalTimeRaw,
    zoneBiteAvgRaw,
    biteBreakdown,
    autoBreakdown,
    catchBreakdown,
    timeSavedBreakdown,
  }) => {
    const unoptimizedTimeRaw = zoneBiteAvgRaw + (active ? catchTimeRaw : catchTimeRaw + 180);
    const percentBite = calculatorPercentage(biteTimeRaw, unoptimizedTimeRaw);
    const percentAF = active ? 0 : calculatorPercentage(autoFishTimeRaw, unoptimizedTimeRaw);
    const percentCatch = calculatorPercentage(catchTimeRaw, unoptimizedTimeRaw);
    const percentSaved = Math.max(
      0,
      100 - calculatorPercentage(totalTimeRaw, unoptimizedTimeRaw),
    );
    const timeSavedRaw = Math.max(0, unoptimizedTimeRaw - totalTimeRaw);
    const segments = [
      calculatorTimelineSegment(
        timelineLabel("bite_time"),
        biteTimeRaw,
        percentBite,
        "#46d2a7",
        "color-mix(in srgb, #46d2a7 72%, var(--color-base-content) 22%)",
        biteBreakdown,
      ),
    ];
    if (!active) {
      segments.push(
        calculatorTimelineSegment(
          timelineLabel("auto_fishing_time"),
          autoFishTimeRaw,
          percentAF,
          "#4e7296",
          "color-mix(in srgb, #4e7296 76%, var(--color-base-content) 24%)",
          autoBreakdown,
        ),
      );
    }
    segments.push(
      calculatorTimelineSegment(
        timelineLabel("catch_time"),
        catchTimeRaw,
        percentCatch,
        "#d27746",
        "color-mix(in srgb, #d27746 74%, var(--color-base-content) 24%)",
        catchBreakdown,
      ),
      calculatorTimelineSegment(
        timelineLabel("time_saved"),
        timeSavedRaw,
        percentSaved,
        "color-mix(in oklab, var(--color-base-100) 55%, var(--color-base-content) 10%)",
        "color-mix(in oklab, var(--color-base-content) 16%, transparent)",
        timeSavedBreakdown,
      ),
    );
    return { segments };
  };

  function calculatorInitUrl() {
    const language = calculatorSurfaceLanguage();
    return window.__fishystuffResolveApiUrl(
      `/api/v1/calculator/datastar/init?lang=${language.apiLang}&locale=${encodeURIComponent(language.locale)}`,
    );
  }

  function calculatorEvalUrl(patch = null) {
    const language = calculatorSurfaceLanguage();
    const patchOptions = patch && typeof patch === "object"
      ? calculatorEvalOptionsForPatch(patch)
      : null;
    const includePetCards = patchOptions
      ? patchOptions.includePetCards
      : calculatorState.pendingEvalNeedsPetCards !== false;
    const includeTargetFishSelect = patchOptions
      ? patchOptions.includeTargetFishSelect
      : calculatorState.pendingEvalNeedsTargetFishSelect === true;
    clearPendingEvalElementPatches();
    const petCardsParam = includePetCards ? "" : "&pet_cards=false";
    const targetFishSelectParam = includeTargetFishSelect ? "&target_fish_select=true" : "";
    return window.__fishystuffResolveApiUrl(
      `/api/v1/calculator/datastar/eval?lang=${language.apiLang}&locale=${encodeURIComponent(language.locale)}${petCardsParam}${targetFishSelectParam}`,
    );
  }

  function calculatorEvalSignalPatchFilter() {
    return {
      exclude: CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN,
    };
  }

  function calculatorPresetUrl(signals) {
    return presetURL(signals);
  }

  function calculatorShareText(signals) {
    const current = signals ?? {};
    const calc = current._calc ?? {};
    const lead = effectiveActivity(current.fishingMode, current.active)
      ? calculatorText("share.active_lead")
      : calculatorText("share.afr_lead", {
          afr: calc.auto_fish_time_reduction_text ?? "0%",
        });
    return calculatorText("share.link", {
      lead,
      item_drr: calc.item_drr_text ?? "0%",
      zone: calc.zone_name ?? current.zone,
      url: calculatorPresetUrl(current),
    });
  }

  function syncCalculatorActions(signals) {
    const current = signals && typeof signals === "object"
      ? signals
      : signalStore.signalObject();
    if (!current || typeof current !== "object") {
      return;
    }
    calculatorActionTokens.consume(
      current._calculator_actions,
      {
        copyUrlToken: () => {
          window.__fishystuffToast.copyText(calculatorPresetUrl(current), {
            success: calculatorText("toast.preset_url_copied"),
          });
        },
        copyShareToken: () => {
          window.__fishystuffToast.copyText(calculatorShareText(current), {
            success: calculatorText("toast.share_copied"),
          });
        },
        saveCalculatorToken: () => {
          try {
            showPresetSaveToast(savePresetCurrent(CALCULATOR_PRESET_COLLECTION_KEY));
          } catch (error) {
            showPresetActionError(error, "presets.error.save");
          }
        },
        discardCalculatorToken: () => {
          try {
            if (discardPresetCurrent(CALCULATOR_PRESET_COLLECTION_KEY)) {
              window.__fishystuffToast.info(presetText("presets.toast.discarded"));
            }
          } catch (error) {
            showPresetActionError(error, "presets.error.discard");
          }
        },
        saveLayoutToken: () => {
          try {
            showPresetSaveToast(savePresetCurrent(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY));
          } catch (error) {
            showPresetActionError(error, "presets.error.save");
          }
        },
        discardLayoutToken: () => {
          try {
            if (discardPresetCurrent(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY)) {
              window.__fishystuffToast.info(presetText("presets.toast.discarded"));
            }
          } catch (error) {
            showPresetActionError(error, "presets.error.discard");
          }
        },
      },
    );
  }

  function syncSignalsFromSharedUserOverlays(signals) {
    const shared = sharedUserOverlays();
    if (!shared || !signals || typeof signals !== "object") {
      return;
    }
    shared.mergeLegacyPriceOverrides(signals.priceOverrides);
    signals.overlay = shared.overlaySignals();
    signals.priceOverrides = shared.priceOverrides();
  }

  function restoreCalculator(signals) {
    signalStore.connect(signals);
    bindCalculatorPresetAdapter();
    bindCalculatorLayoutPresetAdapter();
    sharedUserPresets()?.bindDatastar?.(signals);
    bindPersistListener();
    bindEvalPatchListener();
    bindActionListener();
    bindCalculatorPresetListener();
    bindLayoutPresetListener();
    const storedState = loadStoredSignals();
    const storedSignals = storedState.signals;
    let restoredUiState = null;
    if (storedSignals && typeof storedSignals === "object") {
      const restoredSignals = canonicalizeStoredSignals(storedSignals);
      restoredUiState = normalizeCalculatorUiState(restoredSignals._calculator_ui);
      Object.assign(signals, restoredSignals);
    }
    syncSignalsFromSharedUserOverlays(signals);
    const restoredCalculatorPresetState = applyStoredCalculatorPresetState(signals, {
      hasStoredData: storedState.hasStoredData,
    });
    const pendingCalculatorDataState = storedState.hasStoredData && !restoredCalculatorPresetState
      ? persistedCalculatorSignals(signals)
      : null;
    const restoredLayoutPresetState = applyStoredCalculatorLayoutPresetState(signals);
    const trackedCalculatorPresetState = trackCalculatorPresetCurrent(signals);
    const trackedLayoutPresetState = trackCalculatorLayoutPresetCurrent(signals);
    calculatorState.pendingCalculatorPresetRestore = Boolean(restoredCalculatorPresetState)
      || Boolean(pendingCalculatorDataState)
      || trackedCalculatorPresetState?.kind === "current";
    calculatorState.pendingLayoutPresetRestore = Boolean(restoredLayoutPresetState)
      || trackedLayoutPresetState?.kind === "current";
    calculatorState.pendingCalculatorDataState = pendingCalculatorDataState
      ? cloneCalculatorSignals(pendingCalculatorDataState)
      : null;
    calculatorState.pendingCalculatorUiState = restoredUiState && !restoredLayoutPresetState
      ? cloneCalculatorSignals(signals._calculator_ui)
      : null;
    const appRoot = document.getElementById?.("calculator");
    if (appRoot && languageHelper()) {
      languageHelper().apply(appRoot);
    }
    bindPetImageFallbackListener();
    calculatorState.uiStateRestored = true;
    clearPendingEvalElementPatches();
  }

  function persistCalculator(signals) {
    const shared = sharedUserOverlays();
    if (shared) {
      shared.setOverlaySignals(signals.overlay);
      shared.setPriceOverrides(signals.priceOverrides);
    }
    trackCalculatorPresetCurrent(signals);
    trackCalculatorLayoutPresetCurrent(signals);
    const persistedData = persistedCalculatorSignals(signals);
    const persistedUi = persistedCalculatorUiSignals(signals);
    localStorage.setItem(CALCULATOR_DATA_STORAGE_KEY, JSON.stringify(persistedData));
    localStorage.setItem(CALCULATOR_UI_STORAGE_KEY, JSON.stringify(persistedUi));
  }

  function blurActiveElement() {
    const activeElement = document.activeElement;
    if (activeElement instanceof HTMLElement && typeof activeElement.blur === "function") {
      activeElement.blur();
    }
  }

  function syncPackLeaderInputs(slot, checked) {
    const selector = "input[data-pet-pack-leader]";
    const inputs = typeof document.querySelectorAll === "function"
      ? Array.from(document.querySelectorAll(selector))
      : [];
    for (const input of inputs) {
      if (!(input instanceof HTMLInputElement)) {
        continue;
      }
      const inputSlot = Number.parseInt(input.getAttribute("data-pet-pack-leader-slot") || "", 10);
      if (checked) {
        input.checked = inputSlot === slot;
      } else if (inputSlot === slot) {
        input.checked = false;
      }
    }
  }

  function applyPackLeaderChange(input, slot) {
    const normalizedSlot = Number.parseInt(String(slot ?? ""), 10);
    if (!Number.isInteger(normalizedSlot) || normalizedSlot < 1 || normalizedSlot > 5) {
      return;
    }
    const signals = signalStore.signalObject();
    if (!signals) {
      return;
    }
    const targetKey = `pet${normalizedSlot}`;
    const targetPet = signals[targetKey];
    const tierFiveAvailable = targetPet
      && typeof targetPet === "object"
      && !Array.isArray(targetPet)
      && String(targetPet.tier ?? "").trim() === "5";
    const checked = tierFiveAvailable && Boolean(input?.checked);
    syncPackLeaderInputs(normalizedSlot, checked);
    const patch = {};
    for (let index = 1; index <= 5; index += 1) {
      const key = `pet${index}`;
      const pet = signals[key];
      if (!pet || typeof pet !== "object" || Array.isArray(pet)) {
        continue;
      }
      const tierFivePet = String(pet.tier ?? "").trim() === "5";
      const nextValue = tierFivePet
        ? (index === normalizedSlot ? checked : (checked ? false : Boolean(pet.packLeader)))
        : false;
      if (Boolean(pet.packLeader) !== nextValue) {
        patch[key] = { packLeader: nextValue };
      }
    }
    if (Object.keys(patch).length > 0) {
      window.__fishystuffCalculator.patchSignals(patch);
    }
  }

  function liveCalculator(
    level,
    resources,
    active,
    catchTimeActive,
    catchTimeAfk,
    timespanAmount,
    timespanUnit,
    calc,
  ) {
    const current = calc ?? {};
    const zoneBiteMinRaw = calculatorNumber(current.zone_bite_min);
    const zoneBiteMaxRaw = calculatorNumber(current.zone_bite_max);
    const currentTimespanText = calculatorTimespanText(timespanAmount, timespanUnit);
    const zoneBiteAvgRaw = (zoneBiteMinRaw + zoneBiteMaxRaw) / 2;
    const normalizedLevel = Math.max(0, Math.min(5, calculatorNumber(level)));
    const normalizedResources = Math.max(0, Math.min(100, calculatorNumber(resources)));
    if (!String(current.zone_bite_min ?? "").trim() && !String(current.zone_bite_max ?? "").trim()) {
      return {
        ...current,
        abundance_label: calculatorAbundanceLabel(normalizedResources),
        timespan_text: currentTimespanText,
        casts_title: calculatorTitle("casts_average", { timespan: currentTimespanText }),
        durability_loss_title: calculatorTitle("durability_loss_average", { timespan: currentTimespanText }),
        show_auto_fishing: !active,
        zone_bite_avg: current.zone_bite_avg ?? "0.00",
        effective_bite_avg: current.effective_bite_avg ?? current.bite_time ?? "0.00",
        percent_bite: current.percent_bite ?? "0.00",
        percent_af: current.percent_af ?? "0.00",
        percent_catch: current.percent_catch ?? "0.00",
        fishing_timeline_chart: current.fishing_timeline_chart ?? { segments: [] },
      };
    }
    const factorLevel = 1 - [0.15, 0.30, 0.35, 0.40, 0.45, 0.50][normalizedLevel];
    const factorResources = 2 - (normalizedResources / 100);
    const biteFactor = factorLevel * factorResources;
    const effectiveBiteMinRaw = zoneBiteMinRaw * biteFactor;
    const effectiveBiteMaxRaw = zoneBiteMaxRaw * biteFactor;
    const biteTimeRaw = zoneBiteAvgRaw * biteFactor;
    const activeCatchTimeRaw = Math.max(0, calculatorNumber(catchTimeActive));
    const afkCatchTimeRaw = Math.max(0, calculatorNumber(catchTimeAfk));
    const autoFishTimeRaw = active ? 0 : calculatorNumber(current.auto_fish_time);
    const catchTimeRaw = active ? activeCatchTimeRaw : afkCatchTimeRaw;
    const totalTimeRaw = active
      ? biteTimeRaw + activeCatchTimeRaw
      : biteTimeRaw + autoFishTimeRaw + afkCatchTimeRaw;
    const unoptimizedTimeRaw = zoneBiteAvgRaw + (active ? activeCatchTimeRaw : afkCatchTimeRaw + 180);
    const percentBite = calculatorPercentage(biteTimeRaw, unoptimizedTimeRaw);
    const percentAF = calculatorPercentage(autoFishTimeRaw, unoptimizedTimeRaw);
    const percentCatch = calculatorPercentage(catchTimeRaw, unoptimizedTimeRaw);
    const percentImprovement = 100 - calculatorPercentage(totalTimeRaw, unoptimizedTimeRaw);
    const castsAverageRaw = totalTimeRaw > 0
      ? calculatorTimespanSeconds(timespanAmount, timespanUnit) / totalTimeRaw
      : 0;
    const chanceToReduceRaw = calculatorNumber(
      String(current.chance_to_consume_durability_text ?? "").replace("%", ""),
    ) / 100;
    const durabilityLossAverageRaw = castsAverageRaw * chanceToReduceRaw;
    const fishMultiplierRaw = Math.max(1, calculatorNumber(current.fish_multiplier_raw || 1));
    const lootTotalCatchesRaw = castsAverageRaw * fishMultiplierRaw;
    const lootFishPerHourRaw = totalTimeRaw > 0
      ? (3600 / totalTimeRaw) * fishMultiplierRaw
      : 0;
    const lootProfitPerCatchRaw = Math.max(
      0,
      calculatorNumber(current.loot_profit_per_catch_raw || 0),
    );
    const lootTotalProfitRaw = lootTotalCatchesRaw * lootProfitPerCatchRaw;
    const lootProfitPerHourRaw = lootFishPerHourRaw * lootProfitPerCatchRaw;
    const statBreakdowns = current.stat_breakdowns
      && typeof current.stat_breakdowns === "object"
      && !Array.isArray(current.stat_breakdowns)
      ? { ...current.stat_breakdowns }
      : {};
    const abundanceLabel = calculatorAbundanceLabel(normalizedResources);
    const sessionSeconds = calculatorTimespanSeconds(timespanAmount, timespanUnit);
    const sessionHoursText = calculatorTrimFloat(sessionSeconds / 3600);
    const sessionDurationDetail = breakdownDetail("session_duration_seconds", {
      timespan: currentTimespanText,
      seconds: calculatorTrimFloat(sessionSeconds),
    });
    const zoneName = String(current.zone_name ?? current.zone ?? "").trim();
    const chanceToConsumeDurabilityText =
      String(current.chance_to_consume_durability_text ?? "0.00%").trim() || "0.00%";
    const autoFishTimeReductionText =
      String(current.auto_fish_time_reduction_text ?? "0%").trim() || "0%";
    const fishMultiplierText = `×${calculatorTrimFloat(fishMultiplierRaw)}`;
    const previousTotalProfitRaw = calculatorNumber(
      String(current.loot_total_profit ?? "").replace(/,/g, ""),
    );
    const canScaleProfitRows = previousTotalProfitRaw > 0;
    const profitScale = canScaleProfitRows
      ? lootTotalProfitRaw / previousTotalProfitRaw
      : 0;
    const lootTotalCatchInputRows = calculatorBreakdownSectionRows(
      current.stat_breakdowns?.loot_total_catches,
      "inputs",
    ).filter((row) => !breakdownLabelMatches(row?.label, "average_casts"));
    const lootFishPerHourInputRows = calculatorBreakdownSectionRows(
      current.stat_breakdowns?.loot_fish_per_hour,
      "inputs",
    ).filter((row) => !breakdownLabelMatches(row?.label, "average_total_fishing_time"));
    const lootGroupProfitRows = calculatorBreakdownSectionRows(
      current.stat_breakdowns?.loot_total_profit,
      "inputs",
    );
    const scaledLootGroupProfitValues = canScaleProfitRows
      ? lootGroupProfitRows.map((row) => calculatorScaleSilverText(row?.value_text, profitScale))
      : lootGroupProfitRows.map((row) => String(row?.value_text ?? "").trim()).filter(Boolean);

    statBreakdowns.total_time = calculatorUpdateBreakdown(current.stat_breakdowns?.total_time, {
      title: breakdownTitle("total_time"),
      valueText: calculatorFmt2(totalTimeRaw),
      summaryText: active
        ? breakdownSummary("total_time.active")
        : breakdownSummary("total_time.afk"),
      formulaText: active
        ? breakdownFormula("total_time.active")
        : breakdownFormula("total_time.afk"),
      formulaTerms: active
        ? [
            calculatorBreakdownFormulaTerm(breakdownLabel("average_total"), calculatorFmt2(totalTimeRaw)),
            calculatorBreakdownFormulaTerm(breakdownLabel("average_bite_time"), calculatorFmt2(biteTimeRaw)),
            calculatorBreakdownFormulaTerm(breakdownLabel("active_catch_time"), calculatorFmt2(activeCatchTimeRaw)),
          ]
        : [
            calculatorBreakdownFormulaTerm(breakdownLabel("average_total"), calculatorFmt2(totalTimeRaw)),
            calculatorBreakdownFormulaTerm(breakdownLabel("average_bite_time"), calculatorFmt2(biteTimeRaw)),
            calculatorBreakdownFormulaTerm(breakdownLabel("auto_fishing_time"), calculatorFmt2(autoFishTimeRaw)),
            calculatorBreakdownFormulaTerm(breakdownLabel("afk_catch_time"), calculatorFmt2(afkCatchTimeRaw)),
          ],
      replaceSections: {
        inputs: active
          ? [
              calculatorBreakdownRow(
                breakdownLabel("average_bite_time"),
                calculatorFmt2(biteTimeRaw),
                breakdownDetail("effective_average_after_modifiers"),
                calculatorBreakdownFormulaPart(breakdownLabel("average_bite_time"), 1),
              ),
              calculatorBreakdownRow(
                breakdownLabel("active_catch_time"),
                calculatorFmt2(activeCatchTimeRaw),
                breakdownDetail("manual_catch_time_active"),
                calculatorBreakdownFormulaPart(breakdownLabel("active_catch_time"), 2),
              ),
            ]
          : [
              calculatorBreakdownRow(
                breakdownLabel("average_bite_time"),
                calculatorFmt2(biteTimeRaw),
                breakdownDetail("effective_average_after_modifiers"),
                calculatorBreakdownFormulaPart(breakdownLabel("average_bite_time"), 1),
              ),
              calculatorBreakdownRow(
                breakdownLabel("auto_fishing_time"),
                calculatorFmt2(autoFishTimeRaw),
                breakdownDetail("passive_waiting_after_afr"),
                calculatorBreakdownFormulaPart(breakdownLabel("auto_fishing_time"), 2),
              ),
              calculatorBreakdownRow(
                breakdownLabel("afk_catch_time"),
                calculatorFmt2(afkCatchTimeRaw),
                breakdownDetail("manual_catch_time_afk"),
                calculatorBreakdownFormulaPart(breakdownLabel("afk_catch_time"), 3),
              ),
            ],
        composition: [
          calculatorBreakdownRow(
            breakdownLabel("average_total"),
            calculatorFmt2(totalTimeRaw),
            breakdownDetail("average_cycle_downstream"),
          ),
        ],
      },
    });
    statBreakdowns.bite_time = calculatorUpdateBreakdown(current.stat_breakdowns?.bite_time, {
      title: breakdownTitle("bite_time"),
      valueText: calculatorFmt2(biteTimeRaw),
      formulaTerms: [
        calculatorBreakdownFormulaTerm(breakdownLabel("average_bite_time"), calculatorFmt2(biteTimeRaw)),
        calculatorBreakdownFormulaTerm(breakdownLabel("zone_average_bite_time"), calculatorFmt2(zoneBiteAvgRaw)),
        calculatorBreakdownFormulaTerm(breakdownLabel("level_factor"), calculatorFactorText(factorLevel)),
        calculatorBreakdownFormulaTerm(breakdownLabel("abundance_factor"), calculatorFactorText(factorResources)),
      ],
      replaceSections: {
        inputs: [
          calculatorBreakdownRow(
            breakdownLabel("zone_average_bite_time"),
            calculatorFmt2(zoneBiteAvgRaw),
            breakdownDetail("derived_from_zone_bite_metadata", { zone: zoneName }),
            calculatorBreakdownFormulaPart(breakdownLabel("zone_average_bite_time"), 1),
          ),
          calculatorBreakdownRow(
            breakdownLabel("level_factor"),
            calculatorFactorText(factorLevel),
            breakdownDetail("fishing_level_reduces_base_window", { level: normalizedLevel }),
            calculatorBreakdownFormulaPart(breakdownLabel("level_factor"), 2),
          ),
          calculatorBreakdownRow(
            breakdownLabel("abundance_factor"),
            calculatorFactorText(factorResources),
            breakdownDetail("resources_scale_bite_window", {
              resources: calculatorTrimFloat(normalizedResources),
              abundance: abundanceLabel,
            }),
            calculatorBreakdownFormulaPart(breakdownLabel("abundance_factor"), 3),
          ),
        ],
        composition: [
          calculatorBreakdownRow(
            breakdownLabel("average_bite_time"),
            calculatorFmt2(biteTimeRaw),
            breakdownDetail("used_in_total_fishing_time_calc"),
          ),
        ],
      },
    });
    statBreakdowns.auto_fish_time = calculatorUpdateBreakdown(
      current.stat_breakdowns?.auto_fish_time,
      {
        title: breakdownTitle("auto_fish_time"),
        valueText: calculatorFmt2(autoFishTimeRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("auto_fishing_time"), calculatorFmt2(autoFishTimeRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("baseline_auto_fishing_time"), "180"),
          calculatorBreakdownFormulaTerm(breakdownLabel("applied_afr"), autoFishTimeReductionText),
          calculatorBreakdownFormulaTerm(breakdownLabel("minimum_auto_fishing_time"), "60"),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("baseline_auto_fishing_time"),
              "180",
              breakdownDetail("backend_passive_afk_baseline"),
              calculatorBreakdownFormulaPart(breakdownLabel("baseline_auto_fishing_time"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("applied_afr"),
              autoFishTimeReductionText,
              breakdownDetail("capped_afr_passive_timer"),
              calculatorBreakdownFormulaPart(breakdownLabel("applied_afr"), 2),
            ),
            calculatorBreakdownRow(
              breakdownLabel("minimum_auto_fishing_time"),
              "60",
              breakdownDetail("passive_timer_minimum"),
              calculatorBreakdownFormulaPart(breakdownLabel("minimum_auto_fishing_time"), 3),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("auto_fishing_time"),
              calculatorFmt2(autoFishTimeRaw),
              breakdownDetail("used_only_in_afk_total_calc"),
            ),
          ],
        },
      },
    );
    statBreakdowns.catch_time = calculatorUpdateBreakdown(
      current.stat_breakdowns?.catch_time,
      {
        title: breakdownTitle("catch_time"),
        formulaText: active
          ? breakdownFormula("catch_time.active")
          : breakdownFormula("catch_time.afk"),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("catch_time"), calculatorFmt2(catchTimeRaw)),
          calculatorBreakdownFormulaTerm(
            breakdownLabel(active ? "active_catch_time" : "afk_catch_time"),
            calculatorFmt2(catchTimeRaw),
          ),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel(active ? "active_catch_time" : "afk_catch_time"),
              calculatorFmt2(catchTimeRaw),
              active
                ? breakdownDetail("manual_catch_time_active")
                : breakdownDetail("manual_catch_after_passive_timer"),
              calculatorBreakdownFormulaPart(
                breakdownLabel(active ? "active_catch_time" : "afk_catch_time"),
                1,
              ),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("catch_time"),
              calculatorFmt2(catchTimeRaw),
              breakdownDetail("used_in_total_fishing_time_calc"),
            ),
          ],
        },
      },
    );
    statBreakdowns.time_saved = calculatorUpdateBreakdown(
      current.stat_breakdowns?.time_saved,
      {
        title: breakdownTitle("time_saved"),
        valueText: `${calculatorFmt2(percentImprovement)}%`,
        summaryText: Math.max(0, unoptimizedTimeRaw - totalTimeRaw) > 0
          ? breakdownSummary("time_saved.some")
          : breakdownSummary("time_saved.none"),
        formulaText: breakdownFormula("time_saved"),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(
            breakdownLabel("time_saved"),
            calculatorFmt2(Math.max(0, unoptimizedTimeRaw - totalTimeRaw)),
          ),
          calculatorBreakdownFormulaTerm(
            breakdownLabel("average_unoptimized_time"),
            calculatorFmt2(unoptimizedTimeRaw),
          ),
          calculatorBreakdownFormulaTerm(
            breakdownLabel("average_total_fishing_time"),
            calculatorFmt2(totalTimeRaw),
          ),
          calculatorBreakdownFormulaTerm(breakdownLabel("saved_share"), `${calculatorFmt2(percentImprovement)}%`),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("average_unoptimized_time"),
              calculatorFmt2(unoptimizedTimeRaw),
              active
                ? breakdownDetail("baseline_active_cycle")
                : breakdownDetail("baseline_afk_cycle"),
              calculatorBreakdownFormulaPart(breakdownLabel("average_unoptimized_time"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("average_total_fishing_time"),
              calculatorFmt2(totalTimeRaw),
              breakdownDetail("current_optimized_cycle_duration"),
              calculatorBreakdownFormulaPart(breakdownLabel("average_total_fishing_time"), 2),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("time_saved"),
              calculatorFmt2(Math.max(0, unoptimizedTimeRaw - totalTimeRaw)),
              breakdownDetail("absolute_seconds_removed"),
            ),
            calculatorBreakdownRow(
              breakdownLabel("saved_share"),
              `${calculatorFmt2(percentImprovement)}%`,
              breakdownDetail("baseline_cycle_portion"),
            ),
          ],
        },
      },
    );
    statBreakdowns.casts_average = calculatorUpdateBreakdown(
      current.stat_breakdowns?.casts_average,
      {
        title: breakdownTitle("casts_average", { timespan: currentTimespanText }),
        valueText: calculatorFmt2(castsAverageRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("average_casts"), calculatorFmt2(castsAverageRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("session_seconds"), calculatorTrimFloat(sessionSeconds)),
          calculatorBreakdownFormulaTerm(breakdownLabel("average_total_fishing_time"), calculatorFmt2(totalTimeRaw)),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("session_duration"),
              currentTimespanText,
              sessionDurationDetail,
              calculatorBreakdownFormulaPart(breakdownLabel("session_duration"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("average_total_fishing_time"),
              calculatorFmt2(totalTimeRaw),
              breakdownDetail("denominator_average_cycle_duration"),
              calculatorBreakdownFormulaPart(breakdownLabel("average_total_fishing_time"), 2),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("average_casts"),
              calculatorFmt2(castsAverageRaw),
              breakdownDetail("average_completed_casts_session"),
            ),
          ],
        },
      },
    );
    statBreakdowns.durability_loss_average = calculatorUpdateBreakdown(
      current.stat_breakdowns?.durability_loss_average,
      {
        title: breakdownTitle("durability_loss_average", { timespan: currentTimespanText }),
        valueText: calculatorFmt2(durabilityLossAverageRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("average_loss"), calculatorFmt2(durabilityLossAverageRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("average_casts"), calculatorFmt2(castsAverageRaw)),
          calculatorBreakdownFormulaTerm(
            breakdownLabel("chance_to_consume_durability"),
            chanceToConsumeDurabilityText,
          ),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("average_casts"),
              calculatorFmt2(castsAverageRaw),
              breakdownDetail("average_casts_for_timespan", { timespan: currentTimespanText }),
              calculatorBreakdownFormulaPart(breakdownLabel("average_casts"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("chance_to_consume_durability"),
              chanceToConsumeDurabilityText,
              breakdownDetail("final_per_cast_consumption_chance"),
              calculatorBreakdownFormulaPart(breakdownLabel("chance_to_consume_durability"), 2),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("average_loss"),
              calculatorFmt2(durabilityLossAverageRaw),
              breakdownDetail("expected_durability_consumed_session"),
            ),
          ],
        },
      },
    );
    statBreakdowns.zone_bite_min = calculatorUpdateBreakdown(
      current.stat_breakdowns?.zone_bite_min,
      {
        title: breakdownTitle("zone_bite_min"),
        valueText: calculatorFmt2(zoneBiteMinRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_min"), calculatorFmt2(zoneBiteMinRaw)),
          calculatorBreakdownFormulaTerm(
            breakdownLabel("selected_zone_minimum_bite_time_entry"),
            calculatorFmt2(zoneBiteMinRaw),
          ),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(breakdownLabel("selected_zone"), calculatorFmt2(zoneBiteMinRaw), zoneName),
          ],
        },
      },
    );
    statBreakdowns.zone_bite_avg = calculatorUpdateBreakdown(
      current.stat_breakdowns?.zone_bite_avg,
      {
        title: breakdownTitle("zone_bite_avg"),
        valueText: calculatorFmt2(zoneBiteAvgRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_average"), calculatorFmt2(zoneBiteAvgRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_min"), calculatorFmt2(zoneBiteMinRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_max"), calculatorFmt2(zoneBiteMaxRaw)),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("zone_min"),
              calculatorFmt2(zoneBiteMinRaw),
              zoneName,
              calculatorBreakdownFormulaPart(breakdownLabel("zone_min"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("zone_max"),
              calculatorFmt2(zoneBiteMaxRaw),
              zoneName,
              calculatorBreakdownFormulaPart(breakdownLabel("zone_max"), 2),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("zone_bite_average"),
              calculatorFmt2(zoneBiteAvgRaw),
              breakdownDetail("base_zone_average_before_scaling"),
            ),
          ],
        },
      },
    );
    statBreakdowns.zone_bite_max = calculatorUpdateBreakdown(
      current.stat_breakdowns?.zone_bite_max,
      {
        title: breakdownTitle("zone_bite_max"),
        valueText: calculatorFmt2(zoneBiteMaxRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_max"), calculatorFmt2(zoneBiteMaxRaw)),
          calculatorBreakdownFormulaTerm(
            breakdownLabel("selected_zone_maximum_bite_time_entry"),
            calculatorFmt2(zoneBiteMaxRaw),
          ),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(breakdownLabel("selected_zone"), calculatorFmt2(zoneBiteMaxRaw), zoneName),
          ],
        },
      },
    );
    statBreakdowns.effective_bite_min = calculatorUpdateBreakdown(
      current.stat_breakdowns?.effective_bite_min,
      {
        title: breakdownTitle("effective_bite_min"),
        valueText: calculatorFmt2(effectiveBiteMinRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("effective_bite_min"), calculatorFmt2(effectiveBiteMinRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_min"), calculatorFmt2(zoneBiteMinRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("level_factor"), calculatorFactorText(factorLevel)),
          calculatorBreakdownFormulaTerm(breakdownLabel("abundance_factor"), calculatorFactorText(factorResources)),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("zone_min"),
              calculatorFmt2(zoneBiteMinRaw),
              zoneName,
              calculatorBreakdownFormulaPart(breakdownLabel("zone_min"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("level_factor"),
              calculatorFactorText(factorLevel),
              breakdownDetail("fishing_level_modifier", { level: normalizedLevel }),
              calculatorBreakdownFormulaPart(breakdownLabel("level_factor"), 2),
            ),
            calculatorBreakdownRow(
              breakdownLabel("abundance_factor"),
              calculatorFactorText(factorResources),
              breakdownDetail("resources_abundance", {
                resources: calculatorTrimFloat(normalizedResources),
                abundance: abundanceLabel,
              }),
              calculatorBreakdownFormulaPart(breakdownLabel("abundance_factor"), 3),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("effective_min"),
              calculatorFmt2(effectiveBiteMinRaw),
              breakdownDetail("lower_end_effective_window"),
            ),
          ],
        },
      },
    );
    statBreakdowns.effective_bite_avg = calculatorUpdateBreakdown(
      current.stat_breakdowns?.effective_bite_avg,
      {
        title: breakdownTitle("effective_bite_avg"),
        valueText: calculatorFmt2(biteTimeRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("effective_bite_average"), calculatorFmt2(biteTimeRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_average"), calculatorFmt2(zoneBiteAvgRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("level_factor"), calculatorFactorText(factorLevel)),
          calculatorBreakdownFormulaTerm(breakdownLabel("abundance_factor"), calculatorFactorText(factorResources)),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("zone_average_bite_time"),
              calculatorFmt2(zoneBiteAvgRaw),
              breakdownDetail("derived_from_zone_bite_metadata", { zone: zoneName }),
              calculatorBreakdownFormulaPart(breakdownLabel("zone_average_bite_time"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("level_factor"),
              calculatorFactorText(factorLevel),
              breakdownDetail("fishing_level_reduces_base_window", { level: normalizedLevel }),
              calculatorBreakdownFormulaPart(breakdownLabel("level_factor"), 2),
            ),
            calculatorBreakdownRow(
              breakdownLabel("abundance_factor"),
              calculatorFactorText(factorResources),
              breakdownDetail("resources_scale_bite_window", {
                resources: calculatorTrimFloat(normalizedResources),
                abundance: abundanceLabel,
              }),
              calculatorBreakdownFormulaPart(breakdownLabel("abundance_factor"), 3),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("effective_average"),
              calculatorFmt2(biteTimeRaw),
              breakdownDetail("matches_average_bite_time_stat"),
            ),
          ],
        },
      },
    );
    statBreakdowns.effective_bite_max = calculatorUpdateBreakdown(
      current.stat_breakdowns?.effective_bite_max,
      {
        title: breakdownTitle("effective_bite_max"),
        valueText: calculatorFmt2(effectiveBiteMaxRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("effective_bite_max"), calculatorFmt2(effectiveBiteMaxRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("zone_bite_max"), calculatorFmt2(zoneBiteMaxRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("level_factor"), calculatorFactorText(factorLevel)),
          calculatorBreakdownFormulaTerm(breakdownLabel("abundance_factor"), calculatorFactorText(factorResources)),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("zone_max"),
              calculatorFmt2(zoneBiteMaxRaw),
              zoneName,
              calculatorBreakdownFormulaPart(breakdownLabel("zone_max"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("level_factor"),
              calculatorFactorText(factorLevel),
              breakdownDetail("fishing_level_modifier", { level: normalizedLevel }),
              calculatorBreakdownFormulaPart(breakdownLabel("level_factor"), 2),
            ),
            calculatorBreakdownRow(
              breakdownLabel("abundance_factor"),
              calculatorFactorText(factorResources),
              breakdownDetail("resources_abundance", {
                resources: calculatorTrimFloat(normalizedResources),
                abundance: abundanceLabel,
              }),
              calculatorBreakdownFormulaPart(breakdownLabel("abundance_factor"), 3),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("effective_max"),
              calculatorFmt2(effectiveBiteMaxRaw),
              breakdownDetail("upper_end_effective_window"),
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_total_catches = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_total_catches,
      {
        title: breakdownTitle("loot_total_catches", { timespan: currentTimespanText }),
        valueText: calculatorFmt2(lootTotalCatchesRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("expected_catches"), calculatorFmt2(lootTotalCatchesRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("average_casts"), calculatorFmt2(castsAverageRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("applied_fish_multiplier"), fishMultiplierText),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("average_casts"),
              calculatorFmt2(castsAverageRaw),
              breakdownDetail("average_casts_during_timespan", { timespan: currentTimespanText }),
              calculatorBreakdownFormulaPart(breakdownLabel("average_casts"), 1),
            ),
            ...lootTotalCatchInputRows.map((row) => ({
              ...row,
              formula_part: breakdownLabel("applied_fish_multiplier"),
              formula_part_order: 2,
            })),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("expected_catches"),
              calculatorFmt2(lootTotalCatchesRaw),
              breakdownDetail("expected_catches_selected_session"),
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_fish_per_hour = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_fish_per_hour,
      {
        title: breakdownTitle("loot_fish_per_hour"),
        valueText: calculatorFmt2(lootFishPerHourRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("catches_per_hour"), calculatorFmt2(lootFishPerHourRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("average_total_fishing_time"), calculatorFmt2(totalTimeRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("applied_fish_multiplier"), fishMultiplierText),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("average_total_fishing_time"),
              calculatorFmt2(totalTimeRaw),
              breakdownDetail("average_seconds_full_cycle"),
              calculatorBreakdownFormulaPart(breakdownLabel("average_total_fishing_time"), 1),
            ),
            ...lootFishPerHourInputRows.map((row) => ({
              ...row,
              formula_part: breakdownLabel("applied_fish_multiplier"),
              formula_part_order: 2,
            })),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("catches_per_hour"),
              calculatorFmt2(lootFishPerHourRaw),
              breakdownDetail("expected_hourly_catch_throughput"),
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_total_profit = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_total_profit,
      {
        title: breakdownTitle("loot_total_profit", { timespan: currentTimespanText }),
        valueText: calculatorFmtSilver(lootTotalProfitRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("expected_profit"), calculatorFmtSilver(lootTotalProfitRaw)),
          calculatorBreakdownFormulaTerm(
            breakdownLabel("group_expected_silver"),
            calculatorJoinFormulaTermValues(scaledLootGroupProfitValues, " + ", "0"),
          ),
        ],
        rowUpdates: canScaleProfitRows
          ? Object.fromEntries(
              [calculatorParseBreakdown(current.stat_breakdowns?.loot_total_profit)]
                .filter(Boolean)
                .flatMap((payload) => Array.isArray(payload.sections) ? payload.sections : [])
                .filter((section) => breakdownSectionKey(section?.label) === "inputs")
                .flatMap((section) => Array.isArray(section.rows) ? section.rows : [])
                .map((row) => [
                  String(row?.label ?? ""),
                  {
                    valueText: calculatorScaleSilverText(row?.value_text, profitScale),
                  },
                ]),
            )
          : null,
        replaceSections: {
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("trade_sale_multiplier"),
              String(current.trade_sale_multiplier_text ?? "").trim(),
              breakdownDetail("current_sale_multiplier_after_trade_settings"),
            ),
            calculatorBreakdownRow(
              breakdownLabel("expected_profit"),
              calculatorFmtSilver(lootTotalProfitRaw),
              breakdownDetail("expected_silver_selected_session"),
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_profit_per_hour = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_profit_per_hour,
      {
        title: breakdownTitle("loot_profit_per_hour"),
        valueText: calculatorFmtSilver(lootProfitPerHourRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm(breakdownLabel("profit_per_hour"), calculatorFmtSilver(lootProfitPerHourRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("expected_profit"), calculatorFmtSilver(lootTotalProfitRaw)),
          calculatorBreakdownFormulaTerm(breakdownLabel("session_hours"), sessionHoursText),
        ],
        replaceSections: {
          inputs: [
            calculatorBreakdownRow(
              breakdownLabel("expected_profit_for_timespan", { timespan: currentTimespanText }),
              calculatorFmtSilver(lootTotalProfitRaw),
              breakdownDetail("expected_silver_current_session"),
              calculatorBreakdownFormulaPart(breakdownLabel("expected_profit"), 1),
            ),
            calculatorBreakdownRow(
              breakdownLabel("session_duration"),
              currentTimespanText,
              sessionDurationDetail,
              calculatorBreakdownFormulaPart(breakdownLabel("session_hours"), 2),
            ),
          ],
          composition: [
            calculatorBreakdownRow(
              breakdownLabel("profit_per_hour"),
              calculatorFmtSilver(lootProfitPerHourRaw),
              breakdownDetail("expected_hourly_silver_throughput"),
            ),
          ],
        },
      },
    );
    const fishingTimelineChart = calculatorTimelineChart({
      active,
      biteTimeRaw,
      autoFishTimeRaw,
      catchTimeRaw,
      totalTimeRaw,
      zoneBiteAvgRaw,
      biteBreakdown: calculatorParseBreakdown(statBreakdowns.bite_time),
      autoBreakdown: calculatorParseBreakdown(statBreakdowns.auto_fish_time),
      catchBreakdown: calculatorParseBreakdown(statBreakdowns.catch_time),
      timeSavedBreakdown: calculatorParseBreakdown(statBreakdowns.time_saved),
    });

    return {
      ...current,
      stat_breakdowns: statBreakdowns,
      fishing_timeline_chart: fishingTimelineChart,
      abundance_label: abundanceLabel,
      zone_bite_min: calculatorFmt2(zoneBiteMinRaw),
      zone_bite_max: calculatorFmt2(zoneBiteMaxRaw),
      zone_bite_avg: calculatorFmt2(zoneBiteAvgRaw),
      effective_bite_min: calculatorFmt2(effectiveBiteMinRaw),
      effective_bite_max: calculatorFmt2(effectiveBiteMaxRaw),
      effective_bite_avg: calculatorFmt2(biteTimeRaw),
      total_time: calculatorFmt2(totalTimeRaw),
      bite_time: calculatorFmt2(biteTimeRaw),
      auto_fish_time: calculatorFmt2(autoFishTimeRaw),
      casts_title: calculatorTitle("casts_average", { timespan: currentTimespanText }),
      casts_average: calculatorFmt2(castsAverageRaw),
      durability_loss_title: calculatorTitle("durability_loss_average", { timespan: currentTimespanText }),
      durability_loss_average: calculatorFmt2(durabilityLossAverageRaw),
      loot_total_catches: calculatorFmt2(lootTotalCatchesRaw),
      loot_fish_per_hour: calculatorFmt2(lootFishPerHourRaw),
      loot_fish_multiplier_text: fishMultiplierText,
      loot_total_profit: calculatorFmtSilver(lootTotalProfitRaw),
      loot_profit_per_hour: calculatorFmtSilver(lootProfitPerHourRaw),
      timespan_text: currentTimespanText,
      bite_time_title: calculatorTitle("bite_time", {
        seconds: calculatorFmt2(biteTimeRaw),
        percent: calculatorFmt2(percentBite),
      }),
      auto_fish_time_title: calculatorTitle("auto_fishing_time", {
        seconds: calculatorFmt2(autoFishTimeRaw),
        percent: calculatorFmt2(percentAF),
      }),
      catch_time_title: calculatorTitle("catch_time", {
        seconds: calculatorFmt2(catchTimeRaw),
        percent: calculatorFmt2(percentCatch),
      }),
      unoptimized_time_title: calculatorTitle("unoptimized_time", {
        seconds: calculatorFmt2(unoptimizedTimeRaw),
        percent: calculatorFmt2(percentImprovement),
      }),
      show_auto_fishing: !active,
      percent_bite: calculatorFmt2(percentBite),
      percent_af: calculatorFmt2(percentAF),
      percent_catch: calculatorFmt2(percentCatch),
    };
  }

  window.__fishystuffCalculator = {
    iconSpriteUrl: ICON_SPRITE_URL,
    lang: calculatorSurfaceLanguage().lang,
    locale: calculatorSurfaceLanguage().locale,
    apiLang: calculatorSurfaceLanguage().apiLang,
    ready: languageReady,
    initUrl: calculatorInitUrl,
    evalUrl: calculatorEvalUrl,
    evalSignalPatchFilter: calculatorEvalSignalPatchFilter,
    signalObject() {
      return signalStore.signalObject();
    },
    normalizeFishingMode,
    effectiveActivity,
    patchSignals(patch, options = {}) {
      patchCalculatorSignals(patch, options);
    },
    applyReactiveSignalPatch,
    applyPackLeaderChange,
    petSkillSlots,
    restore: restoreCalculator,
    liveCalc: liveCalculator,
    assignCustomUiState,
    calculatorPresetCollectionKey: CALCULATOR_PRESET_COLLECTION_KEY,
    calculatorPresetPayload,
    normalizeCalculatorPresetPayload,
    applyCalculatorPreset(signals, payload) {
      const current = signals && typeof signals === "object"
        ? signals
        : signalStore.signalObject();
      if (!current || typeof current !== "object") {
        return null;
      }
      const patch = calculatorPresetPatch(current, payload);
      patchCalculatorSignals(patch, {
        eval: true,
        replaceCalculatorData: true,
      });
      return calculatorPresetPayload({
        ...current,
        ...patch,
      });
    },
    layoutPresetCollectionKey: CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY,
    layoutPresetPayload: calculatorLayoutPresetPayload,
    normalizeLayoutPresetPayload: normalizeCalculatorLayoutPresetPayload,
    layoutPresetTitleIconAlias(payload) {
      return presetPreviewTitleIconAlias(CALCULATOR_LAYOUT_PRESET_COLLECTION_KEY, payload);
    },
    presetCollectionCanSave,
    presetCollectionCanDiscard,
    applyLayoutPreset: applyCalculatorLayoutPreset,
    applyLayoutPresetInPlace: applyCalculatorLayoutPresetInPlace,
    toggleCustomSection,
    toggleCustomSectionInPlace,
    removeCustomSection,
    removeCustomSectionInPlace,
    resetCalculatorLayout,
    resetCalculatorLayoutInPlace,
    addCustomSection,
    placeCustomSection,
    isCustomSection,
    customSectionIndex,
    workspaceTab: calculatorWorkspaceTab,
    normalizeWorkspaceTab: normalizeCalculatorWorkspaceTab,
    sectionVisibleInWorkspace: calculatorSectionVisibleInWorkspace,
    blurActiveElement,
  };
})();
