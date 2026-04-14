(function () {
  const ICON_SPRITE_URL = "/img/icons.svg?v=20260330-1";
  const CALCULATOR_STORAGE_KEY = "calculator";
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const CALCULATOR_PERSIST_EXCLUDE_SIGNAL_PATTERN = /^(_loading|_calc(?:\.|$)|_live(?:\.|$)|_defaults(?:\.|$))/;
  const CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN =
    /^(_loading|_calc(?:\.|$)|_live(?:\.|$)|_defaults(?:\.|$)|_calculator_ui(?:\.|$))/;
  const CALCULATOR_ACTION_SIGNAL_PATTERN = /^_calculator_actions(?:\.|$)/;
  const CALCULATOR_DISTRIBUTION_TABS = new Set(["groups", "silver", "loot_flow", "target_fish"]);
  const CALCULATOR_ACTION_DEFAULTS = Object.freeze({
    copyUrlToken: 0,
    copyShareToken: 0,
    clearToken: 0,
  });

  const calculatorState = {
    persistBinding: null,
    actionBinding: null,
    uiStateRestored: false,
  };

  const signalStore = window.__fishystuffDatastarState.createPageSignalStore();
  const calculatorActionTokens =
    window.__fishystuffDatastarState.createCounterTokenController(
      CALCULATOR_ACTION_DEFAULTS,
    );

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

  const calculatorLang = document.documentElement.lang?.trim().toLowerCase().startsWith("ko")
    ? "ko"
    : "en";
  const urlParams = new URLSearchParams(window.location.search);
  const presetQueryParam = urlParams.get("preset");

  const loadStoredSignals = () => {
    const raw = localStorage.getItem(CALCULATOR_STORAGE_KEY);
    if (!raw) {
      return null;
    }
    try {
      const parsed = JSON.parse(raw);
      return parsed && typeof parsed === "object" ? parsed : null;
    } catch (error) {
      console.error("Error parsing stored calculator state:", error);
      return null;
    }
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

  const cloneCalculatorSignals = (value) => JSON.parse(JSON.stringify(value));

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
    if (
      !current._calculator_ui
      || typeof current._calculator_ui !== "object"
      || Array.isArray(current._calculator_ui)
    ) {
      current._calculator_ui = {};
    }
    const distributionTab = String(
      current._calculator_ui.distribution_tab || legacyDistributionTab || "groups",
    ).trim();
    current._calculator_ui = {
      ...current._calculator_ui,
      distribution_tab: CALCULATOR_DISTRIBUTION_TABS.has(distributionTab)
        ? distributionTab
        : "groups",
    };
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
    const persisted = Object.fromEntries(
      Object.entries(current).filter(([key]) => !key.startsWith("_")),
    );
    persisted._calculator_ui = cloneCalculatorSignals(current._calculator_ui);
    return persisted;
  };

  const sharedCalculatorSignals = (signals) =>
    Object.fromEntries(
      Object.entries(canonicalizeStoredSignals(signals)).filter(
        ([key]) => !key.startsWith("_") && key !== "debug",
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

  const clearSignals = () => {
    localStorage.removeItem(CALCULATOR_STORAGE_KEY);
  };

  if (presetQueryParam) {
    try {
      const jsonString = LZString.decompressFromEncodedURIComponent(presetQueryParam);
      JSON.parse(jsonString);
      localStorage.setItem(CALCULATOR_STORAGE_KEY, jsonString);

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
    const label = unit === "minutes"
      ? (normalized === 1 ? "minute" : "minutes")
      : unit === "hours"
        ? (normalized === 1 ? "hour" : "hours")
        : unit === "days"
          ? (normalized === 1 ? "day" : "days")
          : (normalized === 1 ? "week" : "weeks");
    return `${calculatorTrimFloat(normalized)} ${label}`;
  };
  const calculatorAbundanceLabel = (resources) => {
    const value = calculatorNumber(resources);
    if (value <= 14) {
      return "Exhausted";
    }
    if (value <= 45) {
      return "Low";
    }
    if (value <= 70) {
      return "Average";
    }
    return "Abundant";
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
  const calculatorBreakdownSectionRows = (raw, sectionLabel) => {
    const payload = calculatorParseBreakdown(raw);
    if (!payload || !Array.isArray(payload.sections)) {
      return [];
    }
    const section = payload.sections.find((candidate) => String(candidate?.label ?? "") === sectionLabel);
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
      if (replaceSections && Array.isArray(replaceSections[sectionLabel])) {
        section.rows = replaceSections[sectionLabel].map((row) => ({ ...row }));
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
        "Bite Time",
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
          "Auto-Fishing Time",
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
        "Catch Time",
        catchTimeRaw,
        percentCatch,
        "#d27746",
        "color-mix(in srgb, #d27746 74%, var(--color-base-content) 24%)",
        catchBreakdown,
      ),
      calculatorTimelineSegment(
        "Time Saved",
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
    return window.__fishystuffResolveApiUrl(`/api/v1/calculator/datastar/init?lang=${calculatorLang}`);
  }

  function calculatorEvalUrl() {
    return window.__fishystuffResolveApiUrl(`/api/v1/calculator/datastar/eval?lang=${calculatorLang}`);
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
      ? "Active | "
      : `${calc.auto_fish_time_reduction_text ?? "0%"} AFR | `;
    return (
      `[FishyStuff Calculator Preset | ${lead}${calc.item_drr_text ?? "0%"} Item DRR | ${calc.zone_name ?? current.zone}](`
      + calculatorPresetUrl(current)
      + ")"
    );
  }

  function clearCalculator(signals) {
    clearSignals();
    const current = signals && typeof signals === "object"
      ? signals
      : signalStore.signalObject();
    const defaults = current && typeof current === "object"
      ? current._defaults
      : null;
    if (!defaults || typeof defaults !== "object" || Array.isArray(defaults)) {
      return;
    }
    Object.assign(current, cloneCalculatorSignals(defaults));
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
            success: "Preset URL copied.",
          });
        },
        copyShareToken: () => {
          window.__fishystuffToast.copyText(calculatorShareText(current), {
            success: "Share text copied.",
          });
        },
        clearToken: () => {
          clearCalculator(current);
          window.__fishystuffToast.info("Calculator cleared.");
        },
      },
    );
  }

  function restoreCalculator(signals) {
    signalStore.connect(signals);
    bindPersistListener();
    bindActionListener();
    const storedSignals = loadStoredSignals();
    if (storedSignals && typeof storedSignals === "object") {
      Object.assign(signals, canonicalizeStoredSignals(storedSignals));
    }
    calculatorState.uiStateRestored = true;
  }

  function persistCalculator(signals) {
    const payload = persistedCalculatorSignals(signals);
    localStorage.setItem(CALCULATOR_STORAGE_KEY, JSON.stringify(payload));
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
        casts_title: `Average Casts (${currentTimespanText})`,
        durability_loss_title: `Average Durability Loss (${currentTimespanText})`,
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
    const sessionDurationDetail = `${currentTimespanText} = ${calculatorTrimFloat(sessionSeconds)} seconds`;
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
      "Inputs",
    ).filter((row) => String(row?.label ?? "") !== "Average casts");
    const lootFishPerHourInputRows = calculatorBreakdownSectionRows(
      current.stat_breakdowns?.loot_fish_per_hour,
      "Inputs",
    ).filter((row) => String(row?.label ?? "") !== "Average total fishing time");
    const lootGroupProfitRows = calculatorBreakdownSectionRows(
      current.stat_breakdowns?.loot_total_profit,
      "Inputs",
    );
    const scaledLootGroupProfitValues = canScaleProfitRows
      ? lootGroupProfitRows.map((row) => calculatorScaleSilverText(row?.value_text, profitScale))
      : lootGroupProfitRows.map((row) => String(row?.value_text ?? "").trim()).filter(Boolean);

    statBreakdowns.total_time = calculatorUpdateBreakdown(current.stat_breakdowns?.total_time, {
      valueText: calculatorFmt2(totalTimeRaw),
      summaryText: active
        ? "Active mode uses bite time plus active catch time."
        : "AFK mode uses bite time, passive auto-fishing time, and AFK catch time.",
      formulaText: active
        ? "Average total = Average bite time + Active catch time."
        : "Average total = Average bite time + Auto-Fishing Time + AFK catch time.",
      formulaTerms: active
        ? [
            calculatorBreakdownFormulaTerm("Average total", calculatorFmt2(totalTimeRaw)),
            calculatorBreakdownFormulaTerm("Average bite time", calculatorFmt2(biteTimeRaw)),
            calculatorBreakdownFormulaTerm("Active catch time", calculatorFmt2(activeCatchTimeRaw)),
          ]
        : [
            calculatorBreakdownFormulaTerm("Average total", calculatorFmt2(totalTimeRaw)),
            calculatorBreakdownFormulaTerm("Average bite time", calculatorFmt2(biteTimeRaw)),
            calculatorBreakdownFormulaTerm("Auto-Fishing Time", calculatorFmt2(autoFishTimeRaw)),
            calculatorBreakdownFormulaTerm("AFK catch time", calculatorFmt2(afkCatchTimeRaw)),
          ],
      replaceSections: {
        Inputs: active
          ? [
              calculatorBreakdownRow(
                "Average bite time",
                calculatorFmt2(biteTimeRaw),
                "Effective average bite time after level and abundance modifiers.",
                calculatorBreakdownFormulaPart("Average bite time", 1),
              ),
              calculatorBreakdownRow(
                "Active catch time",
                calculatorFmt2(activeCatchTimeRaw),
                "Manual catch-time input used in active mode.",
                calculatorBreakdownFormulaPart("Active catch time", 2),
              ),
            ]
          : [
              calculatorBreakdownRow(
                "Average bite time",
                calculatorFmt2(biteTimeRaw),
                "Effective average bite time after level and abundance modifiers.",
                calculatorBreakdownFormulaPart("Average bite time", 1),
              ),
              calculatorBreakdownRow(
                "Auto-Fishing Time",
                calculatorFmt2(autoFishTimeRaw),
                "Passive waiting phase after AFR is applied.",
                calculatorBreakdownFormulaPart("Auto-Fishing Time", 2),
              ),
              calculatorBreakdownRow(
                "AFK catch time",
                calculatorFmt2(afkCatchTimeRaw),
                "Manual catch-time input used in AFK mode.",
                calculatorBreakdownFormulaPart("AFK catch time", 3),
              ),
            ],
        Composition: [
          calculatorBreakdownRow(
            "Average total",
            calculatorFmt2(totalTimeRaw),
            "Average fishing cycle duration used for downstream casts and loot calculations.",
          ),
        ],
      },
    });
    statBreakdowns.bite_time = calculatorUpdateBreakdown(current.stat_breakdowns?.bite_time, {
      valueText: calculatorFmt2(biteTimeRaw),
      formulaTerms: [
        calculatorBreakdownFormulaTerm("Average bite time", calculatorFmt2(biteTimeRaw)),
        calculatorBreakdownFormulaTerm("Zone average bite time", calculatorFmt2(zoneBiteAvgRaw)),
        calculatorBreakdownFormulaTerm("Level factor", calculatorFactorText(factorLevel)),
        calculatorBreakdownFormulaTerm("Abundance factor", calculatorFactorText(factorResources)),
      ],
      replaceSections: {
        Inputs: [
          calculatorBreakdownRow(
            "Zone average bite time",
            calculatorFmt2(zoneBiteAvgRaw),
            `Derived from ${zoneName} zone bite-time metadata.`,
            calculatorBreakdownFormulaPart("Zone average bite time", 1),
          ),
          calculatorBreakdownRow(
            "Level factor",
            calculatorFactorText(factorLevel),
            `Fishing level ${normalizedLevel} reduces the base bite window.`,
            calculatorBreakdownFormulaPart("Level factor", 2),
          ),
          calculatorBreakdownRow(
            "Abundance factor",
            calculatorFactorText(factorResources),
            `Resources ${calculatorTrimFloat(normalizedResources)}% (${abundanceLabel}) scale the bite window.`,
            calculatorBreakdownFormulaPart("Abundance factor", 3),
          ),
        ],
        Composition: [
          calculatorBreakdownRow(
            "Average bite time",
            calculatorFmt2(biteTimeRaw),
            "Used in the total fishing time calculation.",
          ),
        ],
      },
    });
    statBreakdowns.auto_fish_time = calculatorUpdateBreakdown(
      current.stat_breakdowns?.auto_fish_time,
      {
        valueText: calculatorFmt2(autoFishTimeRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Auto-Fishing Time", calculatorFmt2(autoFishTimeRaw)),
          calculatorBreakdownFormulaTerm("Baseline auto-fishing time", "180"),
          calculatorBreakdownFormulaTerm("Applied AFR", autoFishTimeReductionText),
          calculatorBreakdownFormulaTerm("Minimum auto-fishing time", "60"),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Baseline auto-fishing time",
              "180",
              "Backend keeps the passive AFK baseline even when Active Fishing is enabled.",
              calculatorBreakdownFormulaPart("Baseline auto-fishing time", 1),
            ),
            calculatorBreakdownRow(
              "Applied AFR",
              autoFishTimeReductionText,
              "Capped AFR used by the passive auto-fishing timer.",
              calculatorBreakdownFormulaPart("Applied AFR", 2),
            ),
            calculatorBreakdownRow(
              "Minimum auto-fishing time",
              "60",
              "The passive timer cannot go below 60 seconds.",
              calculatorBreakdownFormulaPart("Minimum auto-fishing time", 3),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Auto-Fishing Time",
              calculatorFmt2(autoFishTimeRaw),
              "Used only in AFK total fishing time calculations.",
            ),
          ],
        },
      },
    );
    statBreakdowns.catch_time = calculatorUpdateBreakdown(
      current.stat_breakdowns?.catch_time,
      {
        formulaText: active
          ? "Catch time = Active catch time."
          : "Catch time = AFK catch time.",
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Catch time", calculatorFmt2(catchTimeRaw)),
          calculatorBreakdownFormulaTerm(
            active ? "Active catch time" : "AFK catch time",
            calculatorFmt2(catchTimeRaw),
          ),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              active ? "Active catch time" : "AFK catch time",
              calculatorFmt2(catchTimeRaw),
              active
                ? "Manual catch-time input used in active mode."
                : "Manual catch-time input used after the passive auto-fishing timer finishes.",
              calculatorBreakdownFormulaPart(
                active ? "Active catch time" : "AFK catch time",
                1,
              ),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Catch time",
              calculatorFmt2(catchTimeRaw),
              "Used in the total fishing time calculation.",
            ),
          ],
        },
      },
    );
    statBreakdowns.time_saved = calculatorUpdateBreakdown(
      current.stat_breakdowns?.time_saved,
      {
        valueText: `${calculatorFmt2(percentImprovement)}%`,
        summaryText: Math.max(0, unoptimizedTimeRaw - totalTimeRaw) > 0
          ? "Share of the unoptimized baseline cycle removed by bite-time reduction and the current fishing mode timings."
          : "No time is currently being saved versus the unoptimized baseline cycle.",
        formulaText: "Time saved = Average unoptimized time - Average total fishing time.; Saved share = Time saved / Average unoptimized time.",
        formulaTerms: [
          calculatorBreakdownFormulaTerm(
            "Time saved",
            calculatorFmt2(Math.max(0, unoptimizedTimeRaw - totalTimeRaw)),
          ),
          calculatorBreakdownFormulaTerm(
            "Average unoptimized time",
            calculatorFmt2(unoptimizedTimeRaw),
          ),
          calculatorBreakdownFormulaTerm(
            "Average total fishing time",
            calculatorFmt2(totalTimeRaw),
          ),
          calculatorBreakdownFormulaTerm("Saved share", `${calculatorFmt2(percentImprovement)}%`),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Average unoptimized time",
              calculatorFmt2(unoptimizedTimeRaw),
              active
                ? "Baseline active cycle using zone average bite time plus active catch time."
                : "Baseline AFK cycle using zone average bite time, 180-second auto-fishing, and AFK catch time.",
              calculatorBreakdownFormulaPart("Average unoptimized time", 1),
            ),
            calculatorBreakdownRow(
              "Average total fishing time",
              calculatorFmt2(totalTimeRaw),
              "Current optimized cycle duration after level, abundance, and AFR effects.",
              calculatorBreakdownFormulaPart("Average total fishing time", 2),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Time saved",
              calculatorFmt2(Math.max(0, unoptimizedTimeRaw - totalTimeRaw)),
              "Absolute seconds removed from the unoptimized baseline cycle.",
            ),
            calculatorBreakdownRow(
              "Saved share",
              `${calculatorFmt2(percentImprovement)}%`,
              "Portion of the baseline cycle represented by the saved time segment.",
            ),
          ],
        },
      },
    );
    statBreakdowns.casts_average = calculatorUpdateBreakdown(
      current.stat_breakdowns?.casts_average,
      {
        title: `Average Casts (${currentTimespanText})`,
        valueText: calculatorFmt2(castsAverageRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Average casts", calculatorFmt2(castsAverageRaw)),
          calculatorBreakdownFormulaTerm("Session seconds", calculatorTrimFloat(sessionSeconds)),
          calculatorBreakdownFormulaTerm("Average total fishing time", calculatorFmt2(totalTimeRaw)),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Session duration",
              currentTimespanText,
              sessionDurationDetail,
              calculatorBreakdownFormulaPart("Session duration", 1),
            ),
            calculatorBreakdownRow(
              "Average total fishing time",
              calculatorFmt2(totalTimeRaw),
              "Average cycle duration used as the denominator.",
              calculatorBreakdownFormulaPart("Average total fishing time", 2),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Average casts",
              calculatorFmt2(castsAverageRaw),
              "Average completed casts for the selected session duration.",
            ),
          ],
        },
      },
    );
    statBreakdowns.durability_loss_average = calculatorUpdateBreakdown(
      current.stat_breakdowns?.durability_loss_average,
      {
        title: `Average Durability Loss (${currentTimespanText})`,
        valueText: calculatorFmt2(durabilityLossAverageRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Average loss", calculatorFmt2(durabilityLossAverageRaw)),
          calculatorBreakdownFormulaTerm("Average casts", calculatorFmt2(castsAverageRaw)),
          calculatorBreakdownFormulaTerm(
            "Chance to consume durability",
            chanceToConsumeDurabilityText,
          ),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Average casts",
              calculatorFmt2(castsAverageRaw),
              `Average casts for ${currentTimespanText}.`,
              calculatorBreakdownFormulaPart("Average casts", 1),
            ),
            calculatorBreakdownRow(
              "Chance to consume durability",
              chanceToConsumeDurabilityText,
              "Final per-cast consumption chance.",
              calculatorBreakdownFormulaPart("Chance to consume durability", 2),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Average loss",
              calculatorFmt2(durabilityLossAverageRaw),
              "Expected durability consumed over the current session duration.",
            ),
          ],
        },
      },
    );
    statBreakdowns.zone_bite_min = calculatorUpdateBreakdown(
      current.stat_breakdowns?.zone_bite_min,
      {
        valueText: calculatorFmt2(zoneBiteMinRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Zone Bite Min", calculatorFmt2(zoneBiteMinRaw)),
          calculatorBreakdownFormulaTerm(
            "selected zone minimum bite-time entry",
            calculatorFmt2(zoneBiteMinRaw),
          ),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow("Selected zone", calculatorFmt2(zoneBiteMinRaw), zoneName),
          ],
        },
      },
    );
    statBreakdowns.zone_bite_avg = calculatorUpdateBreakdown(
      current.stat_breakdowns?.zone_bite_avg,
      {
        valueText: calculatorFmt2(zoneBiteAvgRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Zone Bite Average", calculatorFmt2(zoneBiteAvgRaw)),
          calculatorBreakdownFormulaTerm("Zone Bite Min", calculatorFmt2(zoneBiteMinRaw)),
          calculatorBreakdownFormulaTerm("Zone Bite Max", calculatorFmt2(zoneBiteMaxRaw)),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Zone min",
              calculatorFmt2(zoneBiteMinRaw),
              zoneName,
              calculatorBreakdownFormulaPart("Zone min", 1),
            ),
            calculatorBreakdownRow(
              "Zone max",
              calculatorFmt2(zoneBiteMaxRaw),
              zoneName,
              calculatorBreakdownFormulaPart("Zone max", 2),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Zone average",
              calculatorFmt2(zoneBiteAvgRaw),
              "Base zone average before level and abundance scaling.",
            ),
          ],
        },
      },
    );
    statBreakdowns.zone_bite_max = calculatorUpdateBreakdown(
      current.stat_breakdowns?.zone_bite_max,
      {
        valueText: calculatorFmt2(zoneBiteMaxRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Zone Bite Max", calculatorFmt2(zoneBiteMaxRaw)),
          calculatorBreakdownFormulaTerm(
            "selected zone maximum bite-time entry",
            calculatorFmt2(zoneBiteMaxRaw),
          ),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow("Selected zone", calculatorFmt2(zoneBiteMaxRaw), zoneName),
          ],
        },
      },
    );
    statBreakdowns.effective_bite_min = calculatorUpdateBreakdown(
      current.stat_breakdowns?.effective_bite_min,
      {
        valueText: calculatorFmt2(effectiveBiteMinRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Effective Bite Min", calculatorFmt2(effectiveBiteMinRaw)),
          calculatorBreakdownFormulaTerm("Zone Bite Min", calculatorFmt2(zoneBiteMinRaw)),
          calculatorBreakdownFormulaTerm("Level factor", calculatorFactorText(factorLevel)),
          calculatorBreakdownFormulaTerm("Abundance factor", calculatorFactorText(factorResources)),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Zone min",
              calculatorFmt2(zoneBiteMinRaw),
              zoneName,
              calculatorBreakdownFormulaPart("Zone min", 1),
            ),
            calculatorBreakdownRow(
              "Level factor",
              calculatorFactorText(factorLevel),
              `Fishing level ${normalizedLevel} modifier.`,
              calculatorBreakdownFormulaPart("Level factor", 2),
            ),
            calculatorBreakdownRow(
              "Abundance factor",
              calculatorFactorText(factorResources),
              `Resources ${calculatorTrimFloat(normalizedResources)}% (${abundanceLabel})`,
              calculatorBreakdownFormulaPart("Abundance factor", 3),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Effective min",
              calculatorFmt2(effectiveBiteMinRaw),
              "Lower end of the current effective bite window.",
            ),
          ],
        },
      },
    );
    statBreakdowns.effective_bite_avg = calculatorUpdateBreakdown(
      current.stat_breakdowns?.effective_bite_avg,
      {
        valueText: calculatorFmt2(biteTimeRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Effective Bite Average", calculatorFmt2(biteTimeRaw)),
          calculatorBreakdownFormulaTerm("Zone Bite Average", calculatorFmt2(zoneBiteAvgRaw)),
          calculatorBreakdownFormulaTerm("Level factor", calculatorFactorText(factorLevel)),
          calculatorBreakdownFormulaTerm("Abundance factor", calculatorFactorText(factorResources)),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Zone average bite time",
              calculatorFmt2(zoneBiteAvgRaw),
              `Derived from ${zoneName} zone bite-time metadata.`,
              calculatorBreakdownFormulaPart("Zone average bite time", 1),
            ),
            calculatorBreakdownRow(
              "Level factor",
              calculatorFactorText(factorLevel),
              `Fishing level ${normalizedLevel} reduces the base bite window.`,
              calculatorBreakdownFormulaPart("Level factor", 2),
            ),
            calculatorBreakdownRow(
              "Abundance factor",
              calculatorFactorText(factorResources),
              `Resources ${calculatorTrimFloat(normalizedResources)}% (${abundanceLabel}) scale the bite window.`,
              calculatorBreakdownFormulaPart("Abundance factor", 3),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Effective average",
              calculatorFmt2(biteTimeRaw),
              "Matches the Average Bite Time stat.",
            ),
          ],
        },
      },
    );
    statBreakdowns.effective_bite_max = calculatorUpdateBreakdown(
      current.stat_breakdowns?.effective_bite_max,
      {
        valueText: calculatorFmt2(effectiveBiteMaxRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Effective Bite Max", calculatorFmt2(effectiveBiteMaxRaw)),
          calculatorBreakdownFormulaTerm("Zone Bite Max", calculatorFmt2(zoneBiteMaxRaw)),
          calculatorBreakdownFormulaTerm("Level factor", calculatorFactorText(factorLevel)),
          calculatorBreakdownFormulaTerm("Abundance factor", calculatorFactorText(factorResources)),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Zone max",
              calculatorFmt2(zoneBiteMaxRaw),
              zoneName,
              calculatorBreakdownFormulaPart("Zone max", 1),
            ),
            calculatorBreakdownRow(
              "Level factor",
              calculatorFactorText(factorLevel),
              `Fishing level ${normalizedLevel} modifier.`,
              calculatorBreakdownFormulaPart("Level factor", 2),
            ),
            calculatorBreakdownRow(
              "Abundance factor",
              calculatorFactorText(factorResources),
              `Resources ${calculatorTrimFloat(normalizedResources)}% (${abundanceLabel})`,
              calculatorBreakdownFormulaPart("Abundance factor", 3),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Effective max",
              calculatorFmt2(effectiveBiteMaxRaw),
              "Upper end of the current effective bite window.",
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_total_catches = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_total_catches,
      {
        title: `Expected Catches (${currentTimespanText})`,
        valueText: calculatorFmt2(lootTotalCatchesRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Expected catches", calculatorFmt2(lootTotalCatchesRaw)),
          calculatorBreakdownFormulaTerm("Average casts", calculatorFmt2(castsAverageRaw)),
          calculatorBreakdownFormulaTerm("Applied fish multiplier", fishMultiplierText),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Average casts",
              calculatorFmt2(castsAverageRaw),
              `Average casts during ${currentTimespanText}.`,
              calculatorBreakdownFormulaPart("Average casts", 1),
            ),
            ...lootTotalCatchInputRows.map((row) => ({
              ...row,
              formula_part: "Applied fish multiplier",
              formula_part_order: 2,
            })),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Expected catches",
              calculatorFmt2(lootTotalCatchesRaw),
              "Expected catches for the selected session duration.",
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_fish_per_hour = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_fish_per_hour,
      {
        valueText: calculatorFmt2(lootFishPerHourRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Catches / hour", calculatorFmt2(lootFishPerHourRaw)),
          calculatorBreakdownFormulaTerm("Average total fishing time", calculatorFmt2(totalTimeRaw)),
          calculatorBreakdownFormulaTerm("Applied fish multiplier", fishMultiplierText),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              "Average total fishing time",
              calculatorFmt2(totalTimeRaw),
              "Average seconds per full fishing cycle.",
              calculatorBreakdownFormulaPart("Average total fishing time", 1),
            ),
            ...lootFishPerHourInputRows.map((row) => ({
              ...row,
              formula_part: "Applied fish multiplier",
              formula_part_order: 2,
            })),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Catches / hour",
              calculatorFmt2(lootFishPerHourRaw),
              "Expected hourly catch throughput.",
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_total_profit = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_total_profit,
      {
        title: `Expected Profit (${currentTimespanText})`,
        valueText: calculatorFmtSilver(lootTotalProfitRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Expected profit", calculatorFmtSilver(lootTotalProfitRaw)),
          calculatorBreakdownFormulaTerm(
            "Group expected silver",
            calculatorJoinFormulaTermValues(scaledLootGroupProfitValues, " + ", "0"),
          ),
        ],
        rowUpdates: canScaleProfitRows
          ? Object.fromEntries(
              [calculatorParseBreakdown(current.stat_breakdowns?.loot_total_profit)]
                .filter(Boolean)
                .flatMap((payload) => Array.isArray(payload.sections) ? payload.sections : [])
                .filter((section) => String(section?.label ?? "") === "Inputs")
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
          Composition: [
            calculatorBreakdownRow(
              "Trade sale multiplier",
              String(current.trade_sale_multiplier_text ?? "").trim(),
              "Current sale multiplier after trade settings.",
            ),
            calculatorBreakdownRow(
              "Expected profit",
              calculatorFmtSilver(lootTotalProfitRaw),
              "Expected silver across the selected session duration.",
            ),
          ],
        },
      },
    );
    statBreakdowns.loot_profit_per_hour = calculatorUpdateBreakdown(
      current.stat_breakdowns?.loot_profit_per_hour,
      {
        valueText: calculatorFmtSilver(lootProfitPerHourRaw),
        formulaTerms: [
          calculatorBreakdownFormulaTerm("Profit / hour", calculatorFmtSilver(lootProfitPerHourRaw)),
          calculatorBreakdownFormulaTerm("Expected profit", calculatorFmtSilver(lootTotalProfitRaw)),
          calculatorBreakdownFormulaTerm("Session hours", sessionHoursText),
        ],
        replaceSections: {
          Inputs: [
            calculatorBreakdownRow(
              `Expected profit (${currentTimespanText})`,
              calculatorFmtSilver(lootTotalProfitRaw),
              "Expected silver over the current session duration.",
              calculatorBreakdownFormulaPart("Expected profit", 1),
            ),
            calculatorBreakdownRow(
              "Session duration",
              currentTimespanText,
              sessionDurationDetail,
              calculatorBreakdownFormulaPart("Session hours", 2),
            ),
          ],
          Composition: [
            calculatorBreakdownRow(
              "Profit / hour",
              calculatorFmtSilver(lootProfitPerHourRaw),
              "Expected hourly silver throughput.",
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
      casts_title: `Average Casts (${currentTimespanText})`,
      casts_average: calculatorFmt2(castsAverageRaw),
      durability_loss_title: `Average Durability Loss (${currentTimespanText})`,
      durability_loss_average: calculatorFmt2(durabilityLossAverageRaw),
      loot_total_catches: calculatorFmt2(lootTotalCatchesRaw),
      loot_fish_per_hour: calculatorFmt2(lootFishPerHourRaw),
      loot_fish_multiplier_text: fishMultiplierText,
      loot_total_profit: calculatorFmtSilver(lootTotalProfitRaw),
      loot_profit_per_hour: calculatorFmtSilver(lootProfitPerHourRaw),
      timespan_text: currentTimespanText,
      bite_time_title: `Bite Time: ${calculatorFmt2(biteTimeRaw)}s (${calculatorFmt2(percentBite)}%)`,
      auto_fish_time_title: `Auto-Fishing Time: ${calculatorFmt2(autoFishTimeRaw)}s (${calculatorFmt2(percentAF)}%)`,
      catch_time_title: `Catch Time: ${calculatorFmt2(catchTimeRaw)}s (${calculatorFmt2(percentCatch)}%)`,
      unoptimized_time_title: `Average Unoptimized Time: ${calculatorFmt2(unoptimizedTimeRaw)}s (${calculatorFmt2(percentImprovement)}%)`,
      show_auto_fishing: !active,
      percent_bite: calculatorFmt2(percentBite),
      percent_af: calculatorFmt2(percentAF),
      percent_catch: calculatorFmt2(percentCatch),
    };
  }

  window.__fishystuffCalculator = {
    iconSpriteUrl: ICON_SPRITE_URL,
    lang: calculatorLang,
    initUrl: calculatorInitUrl,
    evalUrl: calculatorEvalUrl,
    evalSignalPatchFilter: calculatorEvalSignalPatchFilter,
    signalObject() {
      return signalStore.signalObject();
    },
    restore: restoreCalculator,
    liveCalc: liveCalculator,
  };
})();
