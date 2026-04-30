(function () {
  const CHANGE_EVENT = "fishystuff:preset-previews-changed";
  const ICON_SPRITE_FALLBACK_URL = "";
  const CALCULATOR_LAYOUT_COLLECTION_KEY = "calculator-layouts";
  const CALCULATOR_PRESET_COLLECTION_KEY = "calculator-presets";
  const MAP_PRESET_COLLECTION_KEY = "map-presets";
  const FISHYDEX_PRESET_COLLECTION_KEY = "fishydex-presets";

  const CALCULATOR_SECTION_TABS = Object.freeze([
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
  const CALCULATOR_SECTION_ICON_BY_ID = Object.freeze({
    mode: "fish-fill",
    overview: "information-fill",
    zone: "fullscreen-fill",
    bite_time: "stopwatch-2-fill",
    catch_time: "stopwatch-fill",
    session: "time-fill",
    distribution: "chart-pie-2-fill",
    loot: "trending-up-fill",
    trade: "wheel-fill",
    gear: "gear-fill",
    food: "dinner-fill",
    buffs: "arrows-up-fill",
    pets: "paw-fill",
    overlay: "edit-4-fill",
    debug: "bug-fill",
  });
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
  const DEFAULT_CALCULATOR_PRESET_PREVIEW_PAYLOAD = Object.freeze({
    active: false,
    fishingMode: "rod",
    level: 0,
    resources: 100,
    zone: "",
    food: Object.freeze([]),
    buff: Object.freeze([]),
  });
  const DEFAULT_MAP_PRESET_PAYLOAD = Object.freeze({
    version: 1,
    windowUi: Object.freeze({}),
    layers: Object.freeze({
      expandedLayerIds: Object.freeze([]),
      hoverFactsVisibleByLayer: Object.freeze({}),
    }),
    search: Object.freeze({
      query: "",
      expression: Object.freeze({ operator: "or", children: Object.freeze([]) }),
      selectedTerms: Object.freeze([]),
    }),
    bridgedUi: Object.freeze({
      diagnosticsOpen: false,
      showPoints: true,
      showPointIcons: true,
      viewMode: "2d",
      pointIconScale: 2,
    }),
    bridgedFilters: Object.freeze({
      layerIdsVisible: Object.freeze(["bookmarks", "fish_evidence", "zone_mask", "minimap"]),
      layerIdsOrdered: Object.freeze([]),
      layerFilterBindingIdsDisabledByLayer: Object.freeze({}),
      layerOpacities: Object.freeze({}),
      layerClipMasks: Object.freeze({ fish_evidence: "zone_mask" }),
      layerWaypointConnectionsVisible: Object.freeze({}),
      layerWaypointLabelsVisible: Object.freeze({}),
      layerPointIconsVisible: Object.freeze({}),
      layerPointIconScales: Object.freeze({}),
    }),
    view: Object.freeze({ viewMode: "2d", camera: Object.freeze({}) }),
  });
  const DEFAULT_FISHYDEX_PRESET_PAYLOAD = Object.freeze({
    caughtIds: Object.freeze([]),
    favouriteIds: Object.freeze([]),
  });

  const adapters = new Map();

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

  function cloneJson(value) {
    if (value == null) {
      return value;
    }
    return JSON.parse(JSON.stringify(value));
  }

  function formatText(text, vars = {}) {
    return String(text ?? "").replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) => {
      return Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : "";
    });
  }

  function translatedText(key, vars = {}) {
    const normalizedKey = trimString(key);
    if (!normalizedKey) {
      return "";
    }
    const helper = globalThis.window?.__fishystuffLanguage;
    if (helper && typeof helper.t === "function") {
      const translated = trimString(helper.t(normalizedKey, vars));
      if (translated && translated !== normalizedKey) {
        return translated;
      }
    }
    return formatText(normalizedKey, vars);
  }

  function documentFor(container) {
    return container?.ownerDocument || globalThis.document || null;
  }

  function appendChildren(parent, children) {
    for (const child of children) {
      if (child) {
        parent.append(child);
      }
    }
  }

  function createElement(doc, tagName, className = "") {
    const element = doc.createElement(tagName);
    if (className) {
      element.className = className;
    }
    return element;
  }

  function createSvgElement(doc, tagName) {
    return doc.createElementNS("http://www.w3.org/2000/svg", tagName);
  }

  function hasRenderableContainer(container) {
    return Boolean(container)
      && typeof container.replaceChildren === "function"
      && typeof container.append === "function";
  }

  function iconSpriteUrl() {
    return trimString(globalThis.window?.__fishystuffCalculator?.iconSpriteUrl) || ICON_SPRITE_FALLBACK_URL;
  }

  function iconHref(alias) {
    const normalizedAlias = trimString(alias);
    return normalizedAlias ? `${iconSpriteUrl()}#fishy-${normalizedAlias}` : "";
  }

  function fixedPreset(id, name, payload) {
    return { id, name, payload: cloneJson(payload) };
  }

  function normalizeUniqueSectionIds(sectionIds, availableSectionIds = CALCULATOR_SECTION_TABS) {
    const available = new Set((Array.isArray(availableSectionIds) ? availableSectionIds : [])
      .map(trimString)
      .filter(Boolean));
    const seen = new Set();
    const normalized = [];
    for (const entry of Array.isArray(sectionIds) ? sectionIds : []) {
      const sectionId = trimString(entry);
      if (!sectionId || seen.has(sectionId) || (available.size && !available.has(sectionId))) {
        continue;
      }
      seen.add(sectionId);
      normalized.push(sectionId);
    }
    return normalized;
  }

  function normalizeCustomLayout(layout, fallback = CALCULATOR_DEFAULT_CUSTOM_LAYOUT) {
    const fallbackRows = Array.isArray(fallback?.[0]?.[0])
      ? fallback
      : normalizeUniqueSectionIds(fallback).map((sectionId) => [[sectionId]]);
    const rows = Array.isArray(layout) ? layout : fallbackRows;
    const seen = new Set();
    const normalized = [];
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
          const sectionId = trimString(entry);
          if (!sectionId || seen.has(sectionId) || !CALCULATOR_SECTION_TABS.includes(sectionId)) {
            continue;
          }
          seen.add(sectionId);
          normalizedColumn.push(sectionId);
        }
        if (normalizedColumn.length) {
          normalizedRow.push(normalizedColumn);
        }
      }
      if (normalizedRow.length) {
        normalized.push(normalizedRow);
      }
    }
    if (Array.isArray(layout)) {
      return normalized;
    }
    if (normalized.length) {
      return normalized;
    }
    return normalizeUniqueSectionIds(CALCULATOR_DEFAULT_CUSTOM_SECTIONS).map((sectionId) => [[sectionId]]);
  }

  function normalizeCalculatorLayoutPresetPayload(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      custom_layout: normalizeCustomLayout(source.custom_layout),
    };
  }

  function defaultCalculatorLayoutPresetPayload() {
    return normalizeCalculatorLayoutPresetPayload({
      custom_layout: CALCULATOR_DEFAULT_CUSTOM_LAYOUT,
    });
  }

  function calculatorLayoutPresetTitleIconAlias(payload) {
    const layoutPreset = normalizeCalculatorLayoutPresetPayload(payload);
    for (const row of layoutPreset.custom_layout) {
      for (const column of Array.isArray(row) ? row : []) {
        for (const sectionId of Array.isArray(column) ? column : []) {
          const alias = CALCULATOR_SECTION_ICON_BY_ID[trimString(sectionId)];
          if (alias) {
            return alias;
          }
        }
      }
    }
    return "";
  }

  function calculatorLayoutPreviewModel(payload) {
    const layoutPreset = normalizeCalculatorLayoutPresetPayload(payload);
    const layout = Array.isArray(layoutPreset.custom_layout) ? layoutPreset.custom_layout : [];
    const previewWidth = 96;
    const paddingX = 4;
    const paddingY = 4;
    const topGutter = 10;
    const bottomGutter = 10;
    const cardHeight = 14;
    const columnGap = 6;
    const stackGap = 4;
    const rowGap = 10;
    const iconSize = 8;
    const innerWidth = previewWidth - paddingX * 2;
    const boxes = [];
    let cursorY = paddingY + topGutter;

    for (const row of layout) {
      const normalizedRow = (Array.isArray(row) ? row : [])
        .map((column) => Array.isArray(column) ? column : [])
        .filter((column) => column.length > 0);
      if (!normalizedRow.length) {
        continue;
      }
      let cursorX = paddingX;
      let rowHeight = cardHeight;
      const columnCount = normalizedRow.length;
      const boxWidth = (innerWidth - columnGap * (columnCount - 1)) / columnCount;
      for (const column of normalizedRow) {
        let columnY = cursorY;
        let columnHeight = 0;
        for (let index = 0; index < column.length; index += 1) {
          const sectionId = trimString(column[index]);
          boxes.push({
            x: cursorX,
            y: columnY,
            width: boxWidth,
            height: cardHeight,
            iconHref: iconHref(CALCULATOR_SECTION_ICON_BY_ID[sectionId]),
          });
          columnY += cardHeight + stackGap;
          columnHeight += cardHeight + (index < column.length - 1 ? stackGap : 0);
        }
        rowHeight = Math.max(rowHeight, columnHeight || cardHeight);
        cursorX += boxWidth + columnGap;
      }
      cursorY += rowHeight + rowGap;
    }

    const lastRowBottom = boxes.length
      ? boxes.reduce((bottom, box) => Math.max(bottom, box.y + box.height), 0)
      : paddingY + topGutter + cardHeight;
    return {
      previewWidth,
      contentHeight: Math.max(
        paddingY * 2 + topGutter + bottomGutter + cardHeight,
        lastRowBottom + bottomGutter + paddingY,
      ),
      paddingX,
      boxes,
      iconSize,
    };
  }

  function renderCalculatorLayoutPreview(container, context = {}) {
    if (!hasRenderableContainer(container)) {
      return;
    }
    const doc = documentFor(container);
    if (!doc) {
      return;
    }
    const model = context.previewModel || calculatorLayoutPreviewModel(context.payload);
    container.replaceChildren();
    const svg = createSvgElement(doc, "svg");
    svg.setAttribute("class", "fishy-preset-manager__preview-svg");
    svg.setAttribute("width", model.previewWidth);
    svg.setAttribute("height", model.contentHeight);
    svg.setAttribute("viewBox", `0 0 ${model.previewWidth} ${model.contentHeight}`);
    svg.setAttribute("pointer-events", "none");

    for (const box of model.boxes) {
      const group = createSvgElement(doc, "g");
      group.setAttribute("class", "fishy-preset-manager__preview-card-group");
      group.setAttribute("pointer-events", "none");
      const rect = createSvgElement(doc, "rect");
      rect.setAttribute("class", "fishy-preset-manager__preview-card");
      rect.setAttribute("x", box.x);
      rect.setAttribute("y", box.y);
      rect.setAttribute("width", box.width);
      rect.setAttribute("height", box.height);
      rect.setAttribute("rx", 3);
      rect.setAttribute("ry", 3);
      rect.setAttribute("pointer-events", "none");
      group.append(rect);
      if (box.iconHref) {
        const icon = createSvgElement(doc, "use");
        icon.setAttribute("class", "fishy-preset-manager__preview-icon");
        icon.setAttribute("href", box.iconHref);
        icon.setAttribute("x", box.x + (box.width - model.iconSize) / 2);
        icon.setAttribute("y", box.y + (box.height - model.iconSize) / 2);
        icon.setAttribute("width", model.iconSize);
        icon.setAttribute("height", model.iconSize);
        icon.setAttribute("pointer-events", "none");
        group.append(icon);
      }
      svg.append(group);
    }
    container.append(svg);
  }

  function normalizeFishingMode(mode) {
    const normalized = trimString(mode).toLowerCase();
    return normalized === "hotspot" || normalized === "harpoon" ? normalized : "rod";
  }

  function normalizeCalculatorPresetPayload(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      ...cloneJson(DEFAULT_CALCULATOR_PRESET_PREVIEW_PAYLOAD),
      ...cloneJson(source),
      fishingMode: normalizeFishingMode(source.fishingMode),
      active: source.active === true,
      food: Array.isArray(source.food) ? cloneJson(source.food) : [],
      buff: Array.isArray(source.buff) ? cloneJson(source.buff) : [],
    };
  }

  function calculatorPresetTitleIconAlias(context = {}) {
    const payload = normalizeCalculatorPresetPayload(context.payload || context);
    const mode = normalizeFishingMode(payload.fishingMode);
    if (mode === "harpoon") {
      return "wheel-fill";
    }
    if (mode === "hotspot") {
      return "fish-fill";
    }
    return "nav-calculator";
  }

  function calculatorPresetPreviewModel(payload) {
    const normalized = normalizeCalculatorPresetPayload(payload);
    const mode = normalizeFishingMode(normalized.fishingMode);
    const modeLabel = mode === "harpoon" ? "Harpoon" : mode === "hotspot" ? "Hotspot" : "Rod";
    return {
      rows: [
        [modeLabel, normalized.active ? "Active" : "AFK"],
        [
          `Lv ${Number.parseInt(normalized.level ?? 0, 10) || 0}`,
          `${Number.parseInt(normalized.resources ?? 0, 10) || 0}%`,
        ],
        [trimString(normalized.zone) || "No zone"],
        [
          `${Array.isArray(normalized.food) ? normalized.food.length : 0} food`,
          `${Array.isArray(normalized.buff) ? normalized.buff.length : 0} buffs`,
        ],
      ],
    };
  }

  function normalizeMapViewMode(value) {
    return value === "3d" ? "3d" : "2d";
  }

  function mergePlainBranch(defaultBranch, sourceBranch) {
    return {
      ...cloneJson(defaultBranch),
      ...(isPlainObject(sourceBranch) ? cloneJson(sourceBranch) : {}),
    };
  }

  function normalizeMapPresetPayload(value) {
    const source = isPlainObject(value) ? value : {};
    const bridgedUi = mergePlainBranch(DEFAULT_MAP_PRESET_PAYLOAD.bridgedUi, source.bridgedUi || source._map_bridged?.ui);
    const sourceView = isPlainObject(source.view)
      ? source.view
      : isPlainObject(source.session?.view)
        ? source.session.view
        : source._map_session?.view;
    const view = mergePlainBranch(DEFAULT_MAP_PRESET_PAYLOAD.view, sourceView);
    view.viewMode = normalizeMapViewMode(view.viewMode || bridgedUi.viewMode);
    bridgedUi.viewMode = normalizeMapViewMode(bridgedUi.viewMode || view.viewMode);
    return {
      version: 1,
      windowUi: mergePlainBranch(DEFAULT_MAP_PRESET_PAYLOAD.windowUi, source.windowUi || source._map_ui?.windowUi),
      layers: mergePlainBranch(DEFAULT_MAP_PRESET_PAYLOAD.layers, source.layers || source._map_ui?.layers),
      search: mergePlainBranch(DEFAULT_MAP_PRESET_PAYLOAD.search, source.search || source._map_ui?.search),
      bridgedUi,
      bridgedFilters: mergePlainBranch(
        DEFAULT_MAP_PRESET_PAYLOAD.bridgedFilters,
        source.bridgedFilters || source._map_bridged?.filters,
      ),
      view,
    };
  }

  function mapPresetTitleIconAlias(context = {}) {
    return normalizeMapPresetPayload(context.payload || context).bridgedUi.viewMode === "3d"
      ? "cube-view"
      : "map-view";
  }

  function mapPresetPreviewModel(payload) {
    const normalized = normalizeMapPresetPayload(payload);
    const visibleLayers = Array.isArray(normalized.bridgedFilters?.layerIdsVisible)
      ? normalized.bridgedFilters.layerIdsVisible.length
      : 0;
    const query = trimString(normalized.search?.query);
    return {
      rows: [
        [normalized.bridgedUi?.viewMode === "3d" ? "3D" : "2D", `${visibleLayers} layers`],
        [normalized.bridgedUi?.showPoints === false ? "Points off" : "Points on"],
        [query || "No search"],
      ],
    };
  }

  function normalizeFishIds(values) {
    const rawValues = Array.isArray(values)
      ? values
      : values === undefined || values === null
        ? []
        : [values];
    const unique = new Set();
    for (const raw of rawValues) {
      const fishId = Number.parseInt(raw, 10);
      if (Number.isInteger(fishId) && fishId > 0) {
        unique.add(fishId);
      }
    }
    return Array.from(unique).sort((left, right) => left - right);
  }

  function normalizeFishydexPresetPayload(value) {
    const source = isPlainObject(value) ? value : {};
    return {
      caughtIds: normalizeFishIds(source.caughtIds),
      favouriteIds: normalizeFishIds(source.favouriteIds),
    };
  }

  function fishydexPresetTitleIconAlias(context = {}) {
    const normalized = normalizeFishydexPresetPayload(context.payload || context);
    if (normalized.favouriteIds.length) {
      return "heart-fill";
    }
    if (normalized.caughtIds.length) {
      return "check-badge-solid";
    }
    return "nav-dex";
  }

  function fishydexPresetPreviewModel(payload) {
    const normalized = normalizeFishydexPresetPayload(payload);
    const trackedCount = new Set([
      ...normalized.caughtIds,
      ...normalized.favouriteIds,
    ]).size;
    return {
      rows: [
        [
          translatedText("fishydex.presets.preview.caught", { count: normalized.caughtIds.length }),
          translatedText("fishydex.presets.preview.favourite", { count: normalized.favouriteIds.length }),
        ],
        [
          translatedText("fishydex.presets.preview.tracked", { count: trackedCount }),
        ],
      ],
    };
  }

  function renderSummaryPreview(container, context = {}) {
    if (!hasRenderableContainer(container)) {
      return;
    }
    const doc = documentFor(container);
    if (!doc) {
      return;
    }
    const rows = Array.isArray(context.previewModel?.rows) ? context.previewModel.rows : [];
    container.replaceChildren();
    const root = createElement(doc, "div", "fishy-preset-manager__summary-preview");
    for (const row of rows) {
      const rowElement = createElement(doc, "div", "fishy-preset-manager__summary-preview-row");
      for (const part of Array.isArray(row) ? row : []) {
        const chip = createElement(doc, "span", "fishy-preset-manager__summary-preview-chip");
        chip.textContent = part;
        rowElement.append(chip);
      }
      root.append(rowElement);
    }
    container.append(root);
  }

  function normalizeAdapterFixedPresets(collectionKey, adapter) {
    const entries = typeof adapter?.fixedPresets === "function" ? adapter.fixedPresets() : [];
    return (Array.isArray(entries) ? entries : [])
      .map((entry, index) => {
        if (!isPlainObject(entry)) {
          return null;
        }
        const id = trimString(entry.id) || `fixed_${index + 1}`;
        const name = trimString(entry.name) || `Fixed ${index + 1}`;
        const normalizer = typeof adapter.normalizePayload === "function"
          ? adapter.normalizePayload
          : (payload) => (isPlainObject(payload) ? payload : {});
        return fixedPreset(id, name, normalizer(entry.payload));
      })
      .filter(Boolean);
  }

  function renderFallbackPreview(container) {
    if (!hasRenderableContainer(container)) {
      return;
    }
    const doc = documentFor(container);
    if (!doc) {
      return;
    }
    const fallback = createElement(doc, "span", "fishy-preset-manager__preview-fallback");
    for (let index = 0; index < 3; index += 1) {
      fallback.append(createElement(doc, "span", "fishy-preset-manager__preview-fallback-bar"));
    }
    container.replaceChildren(fallback);
  }

  function normalizePreviewContext(context = {}) {
    const collectionKey = normalizeCollectionKey(context.collectionKey || context.item?.collectionKey);
    const item = isPlainObject(context.item) ? cloneJson(context.item) : null;
    const rawPayload = context.payload ?? item?.payload ?? null;
    return {
      ...context,
      collectionKey,
      item,
      payload: rawPayload,
      previewSize: Number.isFinite(Number(context.previewSize)) ? Number(context.previewSize) : 200,
      variant: trimString(context.variant) || "default",
    };
  }

  function renderPreview(container, context = {}) {
    const normalizedContext = normalizePreviewContext(context);
    const renderer = adapter(normalizedContext.collectionKey);
    if (!renderer || !hasRenderableContainer(container) || !normalizedContext.payload) {
      renderFallbackPreview(container);
      return false;
    }
    try {
      const payload = typeof renderer.normalizePayload === "function"
        ? renderer.normalizePayload(normalizedContext.payload)
        : normalizedContext.payload;
      const previewModel = typeof renderer.previewModel === "function"
        ? renderer.previewModel(payload, normalizedContext)
        : null;
      renderer.renderPreview(container, {
        ...normalizedContext,
        item: normalizedContext.item,
        payload: cloneJson(payload),
        previewModel: cloneJson(previewModel),
      });
      if (container.childNodes?.length) {
        return true;
      }
    } catch (error) {
      console.error(trimString(context.errorMessage) || "fishy preset preview render failed", error);
    }
    renderFallbackPreview(container);
    return false;
  }

  function createShell(options = {}) {
    const doc = globalThis.document;
    if (!doc) {
      return null;
    }
    const shellTag = trimString(options.shellTag) || "div";
    const viewportTag = trimString(options.viewportTag) || shellTag;
    const previewTag = trimString(options.previewTag) || viewportTag;
    const shell = createElement(
      doc,
      shellTag,
      trimString(`${trimString(options.shellClassName)} fishy-preset-manager__preset-preview-shell`),
    );
    if (options.ariaHidden === true) {
      shell.setAttribute("aria-hidden", "true");
    }
    const viewport = createElement(doc, viewportTag, "fishy-preset-manager__preset-preview-viewport");
    const preview = createElement(doc, previewTag, "fishy-preset-manager__preset-preview");
    if (trimString(options.cardKey)) {
      preview.dataset.cardKey = trimString(options.cardKey);
    }
    viewport.append(preview);
    shell.append(viewport);
    return { shell, viewport, preview };
  }

  function registerAdapter(collectionKey, previewAdapter) {
    const key = normalizeCollectionKey(collectionKey);
    if (!key || !isPlainObject(previewAdapter)) {
      throw new Error("Preset preview adapter requires a collection key and adapter object.");
    }
    adapters.set(key, { ...previewAdapter });
    if (typeof CustomEvent === "function") {
      globalThis.window?.dispatchEvent?.(
        new CustomEvent(CHANGE_EVENT, { detail: { collectionKey: key } }),
      );
    }
    return adapter(key);
  }

  function adapter(collectionKey) {
    const key = normalizeCollectionKey(collectionKey);
    return key ? adapters.get(key) || null : null;
  }

  function fixedPresets(collectionKey) {
    const previewAdapter = adapter(collectionKey);
    return previewAdapter ? normalizeAdapterFixedPresets(collectionKey, previewAdapter) : [];
  }

  function titleIconAlias(collectionKey, context = {}) {
    const key = normalizeCollectionKey(collectionKey || context.collectionKey || context.item?.collectionKey);
    const resolver = adapter(key);
    if (!resolver) {
      return "";
    }
    try {
      return trimString(resolver.titleIconAlias({
        ...context,
        collectionKey: key,
        item: isPlainObject(context.item) ? cloneJson(context.item) : null,
        payload: cloneJson(context.payload ?? context.item?.payload),
      }));
    } catch (error) {
      console.error("fishy preset title icon resolution failed", error);
      return "";
    }
  }

  registerAdapter(CALCULATOR_LAYOUT_COLLECTION_KEY, {
    fixedPresets() {
      return [fixedPreset("default", "Default", defaultCalculatorLayoutPresetPayload())];
    },
    normalizePayload: normalizeCalculatorLayoutPresetPayload,
    previewModel: calculatorLayoutPreviewModel,
    titleIconAlias(context = {}) {
      return calculatorLayoutPresetTitleIconAlias(context.payload || context);
    },
    renderPreview: renderCalculatorLayoutPreview,
  });

  registerAdapter(CALCULATOR_PRESET_COLLECTION_KEY, {
    fixedPresets() {
      return [fixedPreset("default", "Default calculator", DEFAULT_CALCULATOR_PRESET_PREVIEW_PAYLOAD)];
    },
    normalizePayload: normalizeCalculatorPresetPayload,
    previewModel: calculatorPresetPreviewModel,
    titleIconAlias: calculatorPresetTitleIconAlias,
    renderPreview: renderSummaryPreview,
  });

  registerAdapter(MAP_PRESET_COLLECTION_KEY, {
    fixedPresets() {
      return [fixedPreset("default", "Default map", DEFAULT_MAP_PRESET_PAYLOAD)];
    },
    normalizePayload: normalizeMapPresetPayload,
    previewModel: mapPresetPreviewModel,
    titleIconAlias: mapPresetTitleIconAlias,
    renderPreview: renderSummaryPreview,
  });

  registerAdapter(FISHYDEX_PRESET_COLLECTION_KEY, {
    fixedPresets() {
      return [fixedPreset(
        "default",
        translatedText("fishydex.presets.default"),
        DEFAULT_FISHYDEX_PRESET_PAYLOAD,
      )];
    },
    normalizePayload: normalizeFishydexPresetPayload,
    previewModel: fishydexPresetPreviewModel,
    titleIconAlias: fishydexPresetTitleIconAlias,
    renderPreview: renderSummaryPreview,
  });

  const target = globalThis.window || globalThis;
  target.__fishystuffPresetPreviews = Object.freeze({
    CHANGE_EVENT,
    adapter,
    fixedPresets,
    registerAdapter,
    render: renderPreview,
    renderFallback: renderFallbackPreview,
    createShell,
    titleIconAlias,
  });
}());
