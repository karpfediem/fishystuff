(function () {
  const ICON_SPRITE_URL = "/img/icons.svg?v=20260330-1";
  const CALCULATOR_STORAGE_KEY = "calculator";
  const DATASTAR_SIGNAL_PATCH_EVENT = "datastar-signal-patch";
  const CALCULATOR_PERSIST_EXCLUDE_SIGNAL_PATTERN = /^(_loading|_calc(?:\.|$)|_live(?:\.|$)|_defaults(?:\.|$))/;
  const CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN =
    /^(_loading|_calc(?:\.|$)|_live(?:\.|$)|_defaults(?:\.|$)|_calculator_ui(?:\.|$))/;
  const CALCULATOR_ACTION_SIGNAL_PATTERN = /^_calculator_actions(?:\.|$)/;
  const CALCULATOR_DISTRIBUTION_TABS = new Set(["groups", "silver", "loot_flow", "target_fish"]);

  const calculatorState = {
    persistBinding: null,
    actionBinding: null,
    uiStateRestored: false,
    handledActionTokens: {
      copyUrlToken: 0,
      copyShareToken: 0,
      clearToken: 0,
    },
  };

  const signalStore = window.__fishystuffDatastarState.createPageSignalStore();

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
        window.__fishystuffCalculator.persist(signals);
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
      window.__fishystuffCalculator.syncActions(signals);
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

  window.__fishystuffCalculator = {
    iconSpriteUrl: ICON_SPRITE_URL,
    lang: calculatorLang,
    initUrl() {
      return window.__fishystuffResolveApiUrl(`/api/v1/calculator/datastar/init?lang=${this.lang}`);
    },
    evalUrl() {
      return window.__fishystuffResolveApiUrl(`/api/v1/calculator/datastar/eval?lang=${this.lang}`);
    },
    connect(signals) {
      signalStore.connect(signals);
    },
    signalObject() {
      return signalStore.signalObject();
    },
    evalSignalPatchFilter() {
      return {
        exclude: CALCULATOR_EVAL_EXCLUDE_SIGNAL_PATTERN,
      };
    },
    presetUrl(signals) {
      return presetURL(signals);
    },
    shareText(signals) {
      const current = signals ?? {};
      const calc = current._calc ?? {};
      const lead = current.active
        ? "Active | "
        : `${calc.auto_fish_time_reduction_text ?? "0%"} AFR | `;
      return (
        `[FishyStuff Calculator Preset | ${lead}${calc.item_drr_text ?? "0%"} Item DRR | ${calc.zone_name ?? current.zone}](`
        + this.presetUrl(current)
        + ")"
      );
    },
    clear(signals) {
      clearSignals();
      const current = signals && typeof signals === "object"
        ? signals
        : this.signalObject();
      const defaults = current && typeof current === "object"
        ? current._defaults
        : null;
      if (!defaults || typeof defaults !== "object" || Array.isArray(defaults)) {
        return;
      }
      Object.assign(current, cloneCalculatorSignals(defaults));
    },
    actionState(signals) {
      const current = signals && typeof signals === "object"
        ? signals
        : this.signalObject();
      const raw = current && typeof current === "object"
        ? current._calculator_actions
        : null;
      return {
        copyUrlToken: Number.isFinite(raw?.copyUrlToken)
          ? Math.max(0, Math.trunc(raw.copyUrlToken))
          : 0,
        copyShareToken: Number.isFinite(raw?.copyShareToken)
          ? Math.max(0, Math.trunc(raw.copyShareToken))
          : 0,
        clearToken: Number.isFinite(raw?.clearToken)
          ? Math.max(0, Math.trunc(raw.clearToken))
          : 0,
      };
    },
    syncActions(signals) {
      const current = signals && typeof signals === "object"
        ? signals
        : this.signalObject();
      if (!current || typeof current !== "object") {
        return;
      }
      const nextTokens = this.actionState(current);
      const previousTokens = calculatorState.handledActionTokens;
      if (nextTokens.copyUrlToken > previousTokens.copyUrlToken) {
        window.__fishystuffToast.copyText(this.presetUrl(current), {
          success: "Preset URL copied.",
        });
      }
      if (nextTokens.copyShareToken > previousTokens.copyShareToken) {
        window.__fishystuffToast.copyText(this.shareText(current), {
          success: "Share text copied.",
        });
      }
      if (nextTokens.clearToken > previousTokens.clearToken) {
        this.clear(current);
        window.__fishystuffToast.info("Calculator cleared.");
      }
      calculatorState.handledActionTokens = nextTokens;
    },
    restore(signals) {
      this.connect(signals);
      bindPersistListener();
      bindActionListener();
      Object.assign(signals, canonicalizeStoredSignals(loadStoredSignals()));
      calculatorState.uiStateRestored = true;
    },
    persist(signals) {
      const payload = persistedCalculatorSignals(signals);
      localStorage.setItem(CALCULATOR_STORAGE_KEY, JSON.stringify(payload));
    },
    liveCalc(level, resources, active, catchTimeActive, catchTimeAfk, timespanAmount, timespanUnit, calc) {
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

      return {
        ...current,
        abundance_label: calculatorAbundanceLabel(normalizedResources),
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
        loot_fish_multiplier_text: `×${calculatorTrimFloat(fishMultiplierRaw)}`,
        loot_total_profit: calculatorFmtSilver(lootTotalProfitRaw),
        loot_profit_per_hour: calculatorFmtSilver(lootProfitPerHourRaw),
        timespan_text: currentTimespanText,
        bite_time_title: `Bitetime: ${calculatorFmt2(biteTimeRaw)}s (${calculatorFmt2(percentBite)}%)`,
        auto_fish_time_title: `Auto-Fishing Time: ${calculatorFmt2(autoFishTimeRaw)}s (${calculatorFmt2(percentAF)}%)`,
        catch_time_title: `Catch Time: ${calculatorFmt2(catchTimeRaw)}s (${calculatorFmt2(percentCatch)}%)`,
        unoptimized_time_title: `Average Unoptimized Time: ${calculatorFmt2(unoptimizedTimeRaw)}s (${calculatorFmt2(percentImprovement)}%)`,
        show_auto_fishing: !active,
        percent_bite: calculatorFmt2(percentBite),
        percent_af: calculatorFmt2(percentAF),
        percent_catch: calculatorFmt2(percentCatch),
      };
    },
  };
})();
