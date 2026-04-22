(function () {
  const ICON_SPRITE_URL = "/img/icons.svg?v=20260422-1";
  const CALCULATOR_DATA_STORAGE_KEY = "fishystuff.calculator.data.v1";
  const CALCULATOR_UI_STORAGE_KEY = "fishystuff.calculator.ui.v1";
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const CALCULATOR_PERSIST_EXCLUDE_SIGNAL_PATTERN = /^(_loading|_calc(?:\.|$)|_live(?:\.|$)|_defaults(?:\.|$))/;
  const CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN =
    /^(_loading|_calc(?:\.|$)|_live(?:\.|$)|_defaults(?:\.|$)|_calculator_ui(?:\.|$))/;
  const CALCULATOR_ACTION_SIGNAL_PATTERN = /^_calculator_actions(?:\.|$)/;
  const CALCULATOR_TOP_LEVEL_TABS = new Set([
    "overview",
    "inputs",
    "distribution",
    "loot",
    "gear",
    "pets",
    "overlay",
    "debug",
  ]);
  const CALCULATOR_DEFAULT_PINNED_SECTIONS = Object.freeze(["overview"]);
  const CALCULATOR_DISTRIBUTION_TABS = new Set(["groups", "silver", "loot_flow", "target_fish"]);
  const CALCULATOR_ACTION_DEFAULTS = Object.freeze({
    copyUrlToken: 0,
    copyShareToken: 0,
    clearToken: 0,
  });
  const DEFAULT_CALCULATOR_LOCALE = "en-US";
  const BREAKDOWN_SECTION_KEYS = Object.freeze(["inputs", "composition"]);

  const calculatorState = {
    persistBinding: null,
    actionBinding: null,
    uiStateRestored: false,
  };
  const calculatorPetCatalogState = {
    bound: false,
    catalog: null,
    syncQueued: false,
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

  const emitInputChangeEvents = (element) => {
    if (!(element instanceof EventTarget)) {
      return;
    }
    element.dispatchEvent(new Event("input", { bubbles: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
  };

  const calculatorPetCatalog = () => {
    if (calculatorPetCatalogState.catalog) {
      return calculatorPetCatalogState.catalog;
    }
    const script = document.getElementById("calculator-pet-catalog-data");
    if (!(script instanceof HTMLScriptElement)) {
      return null;
    }
    try {
      const parsed = JSON.parse(script.textContent || "{}");
      const pets = Array.isArray(parsed.pets) ? parsed.pets : [];
      const tiers = Array.isArray(parsed.tiers) ? parsed.tiers : [];
      const specials = Array.isArray(parsed.specials) ? parsed.specials : [];
      const talents = Array.isArray(parsed.talents) ? parsed.talents : [];
      const skills = Array.isArray(parsed.skills) ? parsed.skills : [];
      const petByKey = new Map(pets.map((pet) => [String(pet.key || ""), pet]));
      const tierByKey = new Map(tiers.map((tier) => [String(tier.key || ""), tier]));
      const specialByKey = new Map(specials.map((option) => [String(option.key || ""), option]));
      const talentByKey = new Map(talents.map((option) => [String(option.key || ""), option]));
      const skillByKey = new Map(skills.map((option) => [String(option.key || ""), option]));
      calculatorPetCatalogState.catalog = {
        ...parsed,
        pets,
        tiers,
        specials,
        talents,
        skills,
        petByKey,
        tierByKey,
        specialByKey,
        talentByKey,
        skillByKey,
      };
      return calculatorPetCatalogState.catalog;
    } catch (error) {
      console.error("Error parsing calculator pet catalog:", error);
      return null;
    }
  };

  const petDropdownTemplate = (option) => {
    const template = document.createElement("template");
    template.dataset.role = "selected-content";
    template.dataset.value = String(option?.key ?? "");
    template.dataset.label = String(option?.label ?? "");
    template.dataset.searchText = String(option?.label ?? "");
    const label = document.createElement("span");
    label.className = "truncate font-medium";
    label.textContent = String(option?.label ?? "");
    template.content.append(label);
    return template;
  };

  const replaceLocalDropdownOptions = (dropdown, options, boundInput, allowNone) => {
    if (!(dropdown instanceof HTMLElement)) {
      return;
    }
    const catalog = dropdown.querySelector('[data-role="selected-content-catalog"]');
    if (!(catalog instanceof HTMLElement)) {
      return;
    }
    const noneTemplate = allowNone
      ? catalog.querySelector('template[data-role="selected-content"][data-value=""]')
      : null;
    const noneOption = noneTemplate instanceof HTMLTemplateElement
      ? {
        key: "",
        label: String(noneTemplate.getAttribute("data-label") || "").trim(),
      }
      : null;
    const normalizedOptions = Array.isArray(options)
      ? options
        .filter((option) => option && typeof option === "object")
        .map((option) => ({
          key: String(option.key || ""),
          label: String(option.label || option.key || ""),
        }))
      : [];
    const nextOptions = [];
    if (allowNone && noneOption) {
      nextOptions.push(noneOption);
    }
    for (const option of normalizedOptions) {
      if (!option.key && allowNone) {
        continue;
      }
      nextOptions.push(option);
    }
    catalog.replaceChildren(...nextOptions.map(petDropdownTemplate));
    if (typeof dropdown.refreshResults === "function") {
      dropdown.refreshResults();
    }
    if (boundInput instanceof HTMLInputElement) {
      emitInputChangeEvents(boundInput);
    }
  };

  const syncPetSkillCheckboxes = (skillsRoot, allowedKeys) => {
    if (!(skillsRoot instanceof HTMLElement)) {
      return;
    }
    const select = skillsRoot.querySelector('select[data-role="bound-select"]');
    if (!(select instanceof HTMLSelectElement)) {
      return;
    }
    const allowed = allowedKeys instanceof Set ? allowedKeys : null;
    let changed = false;
    for (const option of Array.from(select.options)) {
      const visible = !allowed || allowed.has(option.value);
      option.hidden = !visible;
      option.disabled = !visible;
      if (!visible && option.selected) {
        option.selected = false;
        changed = true;
      }
    }
    for (const checkbox of skillsRoot.querySelectorAll("input[data-checkbox-group-option]")) {
      if (!(checkbox instanceof HTMLInputElement)) {
        continue;
      }
      const visible = !allowed || allowed.has(checkbox.value);
      const label = checkbox.closest("label");
      if (label instanceof HTMLElement) {
        label.hidden = !visible;
      }
      checkbox.disabled = !visible;
      if (!visible && checkbox.checked) {
        checkbox.checked = false;
      }
    }
    if (changed) {
      emitInputChangeEvents(select);
    } else {
      emitInputChangeEvents(select);
    }
  };

  const petSlotElements = (slot) => ({
    petInput: document.getElementById(`calculator-pet${slot}-pet-value`),
    tierInput: document.getElementById(`calculator-pet${slot}-tier-value`),
    specialInput: document.getElementById(`calculator-pet${slot}-special-value`),
    talentInput: document.getElementById(`calculator-pet${slot}-talent-value`),
    petDropdown: document.getElementById(`calculator-pet${slot}-pet-picker`),
    tierDropdown: document.getElementById(`calculator-pet${slot}-tier-picker`),
    specialDropdown: document.getElementById(`calculator-pet${slot}-special-picker`),
    talentDropdown: document.getElementById(`calculator-pet${slot}-talent-picker`),
    skillsRoot: document.getElementById(`pet${slot}_skills`),
  });

  const highestTierKey = (tiers) => (
    Array.isArray(tiers)
      ? tiers
        .map((tier) => String(tier?.key ?? "").trim())
        .filter(Boolean)
        .sort((left, right) => Number(right) - Number(left))[0] || ""
      : ""
  );

  const syncPetSlotControls = (slot, catalog) => {
    const elements = petSlotElements(slot);
    if (!(elements.petInput instanceof HTMLInputElement) || !(elements.tierInput instanceof HTMLInputElement)) {
      return;
    }

    const selectedPet = catalog.petByKey.get(String(elements.petInput.value || "").trim()) || null;
    const tierOptions = selectedPet
      ? (Array.isArray(selectedPet.tiers) ? selectedPet.tiers : []).map((tier) => ({
        key: String(tier.key || ""),
        label: String(tier.label || tier.key || ""),
      }))
      : catalog.tiers;
    let selectedTier = String(elements.tierInput.value || "").trim();
    if (selectedPet) {
      const allowedTiers = new Set(tierOptions.map((tier) => tier.key));
      if (!allowedTiers.has(selectedTier)) {
        const fallbackTier = highestTierKey(tierOptions);
        if (selectedTier !== fallbackTier) {
          elements.tierInput.value = fallbackTier;
          selectedTier = fallbackTier;
          emitInputChangeEvents(elements.tierInput);
        }
      }
    }

    const selectedTierEntry = selectedPet
      ? (Array.isArray(selectedPet.tiers) ? selectedPet.tiers : []).find(
        (tier) => String(tier?.key || "") === selectedTier,
      ) || null
      : null;
    const specialOptions = selectedTierEntry
      ? (Array.isArray(selectedTierEntry.specials) ? selectedTierEntry.specials : [])
        .map((key) => catalog.specialByKey.get(String(key || "")))
        .filter(Boolean)
      : catalog.specials;
    const talentOptions = selectedTierEntry
      ? (Array.isArray(selectedTierEntry.talents) ? selectedTierEntry.talents : [])
        .map((key) => catalog.talentByKey.get(String(key || "")))
        .filter(Boolean)
      : catalog.talents;
    const skillOptions = selectedTierEntry
      ? (Array.isArray(selectedTierEntry.skills) ? selectedTierEntry.skills : [])
        .map((key) => catalog.skillByKey.get(String(key || "")))
        .filter(Boolean)
      : catalog.skills;

    if (selectedTierEntry && elements.specialInput instanceof HTMLInputElement) {
      const allowedSpecials = new Set(specialOptions.map((option) => String(option.key || "")));
      if (!allowedSpecials.has(String(elements.specialInput.value || "").trim())) {
        elements.specialInput.value = "";
        emitInputChangeEvents(elements.specialInput);
      }
    }
    if (selectedTierEntry && elements.talentInput instanceof HTMLInputElement) {
      const allowedTalents = new Set(talentOptions.map((option) => String(option.key || "")));
      const currentTalent = String(elements.talentInput.value || "").trim();
      if (!allowedTalents.has(currentTalent)) {
        const nextTalent = talentOptions.length === 1
          ? String(talentOptions[0].key || "")
          : "";
        if (currentTalent !== nextTalent) {
          elements.talentInput.value = nextTalent;
          emitInputChangeEvents(elements.talentInput);
        }
      }
    }

    replaceLocalDropdownOptions(elements.tierDropdown, tierOptions, elements.tierInput, false);
    replaceLocalDropdownOptions(elements.specialDropdown, specialOptions, elements.specialInput, true);
    replaceLocalDropdownOptions(elements.talentDropdown, talentOptions, elements.talentInput, true);
    syncPetSkillCheckboxes(
      elements.skillsRoot,
      selectedTierEntry
        ? new Set(skillOptions.map((option) => String(option.key || "")))
        : new Set(catalog.skills.map((option) => String(option.key || ""))),
    );
  };

  const syncCalculatorPetControls = () => {
    const catalog = calculatorPetCatalog();
    if (!catalog) {
      return;
    }
    const slotCount = Number(catalog.slots) > 0 ? Number(catalog.slots) : 5;
    for (let slot = 1; slot <= slotCount; slot += 1) {
      syncPetSlotControls(slot, catalog);
    }
  };

  const queueCalculatorPetControlSync = () => {
    if (calculatorPetCatalogState.syncQueued) {
      return;
    }
    calculatorPetCatalogState.syncQueued = true;
    queueMicrotask(() => {
      calculatorPetCatalogState.syncQueued = false;
      syncCalculatorPetControls();
    });
  };

  function bindPetCatalogListener() {
    if (calculatorPetCatalogState.bound) {
      return;
    }
    const petsRoot = document.getElementById("pets");
    if (!(petsRoot instanceof HTMLElement)) {
      return;
    }
    petsRoot.addEventListener("input", queueCalculatorPetControlSync);
    petsRoot.addEventListener("change", queueCalculatorPetControlSync);
    document.addEventListener(DATASTAR_SIGNAL_PATCH_EVENT, queueCalculatorPetControlSync);
    calculatorPetCatalogState.bound = true;
    queueCalculatorPetControlSync();
  }

  function calculatorSurfaceLanguage() {
    const current = languageHelper()?.current?.() || {};
    const locale = String(current.locale || document.documentElement.lang || "en-US").trim();
    const localeKey = locale.toLowerCase();
    const apiLang = String(current.apiLang || "").trim().toLowerCase();
    const resolvedApiLang = apiLang === "ko" ? "ko" : (localeKey.startsWith("ko") ? "ko" : "en");
    if (localeKey.startsWith("ko")) {
      return {
        locale: "ko-KR",
        apiLang: resolvedApiLang,
        lang: resolvedApiLang,
      };
    }
    if (localeKey.startsWith("de")) {
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

  function datastarPersistHelper() {
    const helper = window.__fishystuffDatastarPersist;
    return helper && typeof helper.createDebouncedSignalPatchPersistor === "function"
      ? helper
      : null;
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
      return null;
    }
    const combined = storedData && typeof storedData === "object" ? { ...storedData } : {};
    if (storedUi && typeof storedUi === "object") {
      combined._calculator_ui = storedUi;
    }
    return combined;
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

  const normalizePinnedSections = (
    value,
    fallback = CALCULATOR_DEFAULT_PINNED_SECTIONS,
  ) => {
    const normalizeList = (entries) => compactStringArray(entries)
      .filter((entry) => CALCULATOR_TOP_LEVEL_TABS.has(entry));
    if (Array.isArray(value)) {
      return normalizeList(value);
    }
    return normalizeList(fallback);
  };

  const normalizeCalculatorUiState = (value, legacyDistributionTab = "") => {
    const current = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    const topLevelTab = String(current.top_level_tab || "overview").trim();
    const distributionTab = String(
      current.distribution_tab || legacyDistributionTab || "groups",
    ).trim();
    return {
      top_level_tab: CALCULATOR_TOP_LEVEL_TABS.has(topLevelTab)
        ? topLevelTab
        : "overview",
      distribution_tab: CALCULATOR_DISTRIBUTION_TABS.has(distributionTab)
        ? distributionTab
        : "groups",
      pinned_sections: normalizePinnedSections(current.pinned_sections),
    };
  };

  const cloneCalculatorSignals = (value) => JSON.parse(JSON.stringify(value));
  const normalizeBooleanFlag = (value, fallback = false) =>
    value == null ? fallback : value === true || value === "true" || value === 1 || value === "1";

  function normalizeSectionId(sectionId) {
    return String(sectionId ?? "").trim();
  }

  function pinnedSectionIndex(pinnedSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    return normalizePinnedSections(pinnedSections).indexOf(normalizedSection);
  }

  function isPinnedSection(pinnedSections, sectionId) {
    return pinnedSectionIndex(pinnedSections, sectionId) >= 0;
  }

  function togglePinnedSection(pinnedSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    if (!CALCULATOR_TOP_LEVEL_TABS.has(normalizedSection)) {
      return normalizePinnedSections(pinnedSections);
    }
    const next = normalizePinnedSections(pinnedSections);
    const currentIndex = next.indexOf(normalizedSection);
    if (currentIndex >= 0) {
      next.splice(currentIndex, 1);
      return next;
    }
    next.push(normalizedSection);
    return next;
  }

  function pinSection(pinnedSections, sectionId) {
    const normalizedSection = normalizeSectionId(sectionId);
    if (!CALCULATOR_TOP_LEVEL_TABS.has(normalizedSection)) {
      return normalizePinnedSections(pinnedSections);
    }
    const next = normalizePinnedSections(pinnedSections);
    if (next.includes(normalizedSection)) {
      return next;
    }
    next.push(normalizedSection);
    return next;
  }

  function placePinnedSection(pinnedSections, sectionId, targetSectionId, position) {
    const normalizedSection = normalizeSectionId(sectionId);
    const normalizedTarget = normalizeSectionId(targetSectionId);
    const normalizedPosition = position === "before" ? "before" : "after";
    if (!CALCULATOR_TOP_LEVEL_TABS.has(normalizedSection)) {
      return normalizePinnedSections(pinnedSections);
    }
    if (!normalizedTarget || normalizedTarget === normalizedSection) {
      return pinSection(pinnedSections, normalizedSection);
    }
    const next = normalizePinnedSections(pinnedSections)
      .filter((entry) => entry !== normalizedSection);
    const targetIndex = next.indexOf(normalizedTarget);
    if (targetIndex < 0) {
      next.push(normalizedSection);
      return next;
    }
    next.splice(targetIndex + (normalizedPosition === "after" ? 1 : 0), 0, normalizedSection);
    return next;
  }

  function movePinnedSection(pinnedSections, sectionId, direction) {
    const normalizedSection = normalizeSectionId(sectionId);
    const normalizedDirection = Number(direction);
    const next = normalizePinnedSections(pinnedSections);
    const currentIndex = next.indexOf(normalizedSection);
    if (currentIndex < 0 || !Number.isFinite(normalizedDirection) || normalizedDirection === 0) {
      return next;
    }
    const targetIndex = currentIndex + (normalizedDirection < 0 ? -1 : 1);
    if (targetIndex < 0 || targetIndex >= next.length) {
      return next;
    }
    const [movingSection] = next.splice(currentIndex, 1);
    next.splice(targetIndex, 0, movingSection);
    return next;
  }

  function canMovePinnedSection(pinnedSections, sectionId, direction) {
    const normalizedDirection = Number(direction);
    const currentIndex = pinnedSectionIndex(pinnedSections, sectionId);
    const totalPinned = normalizePinnedSections(pinnedSections).length;
    if (currentIndex < 0 || !Number.isFinite(normalizedDirection) || normalizedDirection === 0) {
      return false;
    }
    const targetIndex = currentIndex + (normalizedDirection < 0 ? -1 : 1);
    return targetIndex >= 0 && targetIndex < totalPinned;
  }

  function calculatorSectionVisible(sectionId, topLevelTab, pinnedSections) {
    const normalizedSection = normalizeSectionId(sectionId);
    return isPinnedSection(pinnedSections, normalizedSection) || normalizeSectionId(topLevelTab) === normalizedSection;
  }

  function calculatorSectionOrder(sectionId, topLevelTab, pinnedSections) {
    const pinned = normalizePinnedSections(pinnedSections);
    const normalizedSection = normalizeSectionId(sectionId);
    const pinIndex = pinned.indexOf(normalizedSection);
    if (pinIndex >= 0) {
      return pinIndex;
    }
    if (normalizeSectionId(topLevelTab) === normalizedSection) {
      return pinned.length;
    }
    return pinned.length + 1;
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
    };
    for (const [legacyKey, canonicalKey] of Object.entries(aliases)) {
      if (!(canonicalKey in current) && legacyKey in current) {
        current[canonicalKey] = current[legacyKey];
      }
      delete current[legacyKey];
    }
    const legacyDistributionTab = String(current._distribution_tab ?? "").trim();
    delete current._distribution_tab;
    current._calculator_ui = normalizeCalculatorUiState(current._calculator_ui, legacyDistributionTab);
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
    for (const petKey of ["pet1", "pet2", "pet3", "pet4", "pet5"]) {
      if (!current[petKey] || typeof current[petKey] !== "object" || Array.isArray(current[petKey])) {
        continue;
      }
      current[petKey] = {
        ...current[petKey],
        skills: compactStringArray(current[petKey].skills),
      };
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

  const clearCalculatorDataStorage = () => {
    localStorage.removeItem(CALCULATOR_DATA_STORAGE_KEY);
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

  function calculatorEvalUrl() {
    const language = calculatorSurfaceLanguage();
    return window.__fishystuffResolveApiUrl(
      `/api/v1/calculator/datastar/eval?lang=${language.apiLang}&locale=${encodeURIComponent(language.locale)}`,
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
    const lead = current.active
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

  function clearCalculator(signals) {
    const current = signals && typeof signals === "object"
      ? signals
      : signalStore.signalObject();
    const persistedUi = current && typeof current === "object"
      ? persistedCalculatorUiSignals(current)
      : null;
    clearCalculatorDataStorage();
    const defaults = current && typeof current === "object"
      ? current._defaults
      : null;
    if (!defaults || typeof defaults !== "object" || Array.isArray(defaults)) {
      if (persistedUi) {
        localStorage.setItem(CALCULATOR_UI_STORAGE_KEY, JSON.stringify(persistedUi));
      }
      return;
    }
    Object.assign(current, cloneCalculatorSignals(defaults));
    if (persistedUi) {
      current._calculator_ui = cloneCalculatorSignals(persistedUi);
      localStorage.setItem(CALCULATOR_UI_STORAGE_KEY, JSON.stringify(persistedUi));
    }
    syncSignalsFromSharedUserOverlays(current);
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
        clearToken: () => {
          clearCalculator(current);
          window.__fishystuffToast.info(calculatorText("toast.cleared"));
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
    bindPersistListener();
    bindActionListener();
    const storedSignals = loadStoredSignals();
    if (storedSignals && typeof storedSignals === "object") {
      Object.assign(signals, canonicalizeStoredSignals(storedSignals));
    }
    syncSignalsFromSharedUserOverlays(signals);
    const appRoot = document.getElementById?.("calculator");
    if (appRoot && languageHelper()) {
      languageHelper().apply(appRoot);
    }
    bindPetCatalogListener();
    calculatorState.uiStateRestored = true;
  }

  function persistCalculator(signals) {
    const shared = sharedUserOverlays();
    if (shared) {
      shared.setOverlaySignals(signals.overlay);
      shared.setPriceOverrides(signals.priceOverrides);
    }
    const persistedData = persistedCalculatorSignals(signals);
    const persistedUi = persistedCalculatorUiSignals(signals);
    localStorage.setItem(CALCULATOR_DATA_STORAGE_KEY, JSON.stringify(persistedData));
    localStorage.setItem(CALCULATOR_UI_STORAGE_KEY, JSON.stringify(persistedUi));
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
    initUrl: calculatorInitUrl,
    evalUrl: calculatorEvalUrl,
    evalSignalPatchFilter: calculatorEvalSignalPatchFilter,
    signalObject() {
      return signalStore.signalObject();
    },
    patchSignals(patch) {
      signalStore.patchSignals(patch);
      document.dispatchEvent(new CustomEvent(DATASTAR_SIGNAL_PATCH_EVENT, {
        detail: cloneCalculatorSignals(patch),
      }));
    },
    restore: restoreCalculator,
    liveCalc: liveCalculator,
    togglePinnedSection,
    pinSection,
    placePinnedSection,
    movePinnedSection,
    canMovePinnedSection,
    isPinnedSection,
    pinnedSectionIndex,
    sectionVisible: calculatorSectionVisible,
    sectionOrder: calculatorSectionOrder,
  };
})();
