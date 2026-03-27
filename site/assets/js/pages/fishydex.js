(function () {
  const DEX_UI_STORAGE_KEY = "fishystuff.fishydex.ui.v1";
  const DEX_PROGRESS_PANEL_UI_KEY = "dex.panels.progress.collapsed";
  const DEX_FILTER_PANEL_UI_KEY = "dex.panels.filter.collapsed";
  const CAUGHT_STORAGE_KEY = "fishystuff.fishydex.caught.v1";
  const FAVOURITES_STORAGE_KEY = "fishystuff.fishydex.favourites.v1";
  const GRADE_COLOR_ORDER = ["red", "yellow", "blue", "green", "white", "unknown"];
  const GRADE_FILTER_COLOR_ORDER = ["red", "yellow", "blue", "green", "white"];
  const GRADE_LABELS = {
    red: "Red",
    yellow: "Yellow",
    blue: "Blue",
    green: "Green",
    white: "White",
    unknown: "Unknown",
  };
  const METHOD_ORDER = ["rod", "harpoon"];
  const SORT_FIELD_ORDER = ["name", "price"];
  const SORT_DIRECTION_ORDER = ["asc", "desc"];
  const SILVER_FORMATTER = new Intl.NumberFormat();
  const state = {
    signals: null,
    uiSettingsUnsubscribe: null,
    persistedUiJson: "",
    uiStateRestored: false,
    pendingViewportRestore: null,
    pendingViewportFrame: 0,
    stampTimers: new Map(),
    fishById: new Map(),
    renderKey: "",
    gridBound: false,
    detailsBound: false,
    suppressCardAnimation: false,
  };

  function spriteIconMarkup(name, className, inline) {
    const classes = ["fishy-icon"];
    if (inline) {
      classes.push("fishy-icon--inline");
    }
    if (className) {
      classes.push(className);
    }
    return `<svg class="${classes.join(" ")}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260326-6#fishy-${name}"></use></svg>`;
  }

  const ICON_HEART_FILL = spriteIconMarkup("heart-fill", "size-6", true);
  const ICON_HEART_LINE = spriteIconMarkup("heart-line", "size-6", true);
  const ICON_CAUGHT_FILL = spriteIconMarkup("check-badge-solid", "size-7", true);
  const ICON_CAUGHT_LINE = spriteIconMarkup("check-circle-dash-line", "size-7", true);
  const ICON_COIN_STACK = spriteIconMarkup("coin-stack", "", false);

  function signalObject() {
    return state.signals && typeof state.signals === "object" ? state.signals : null;
  }

  function patchSignals(patch) {
    const signals = signalObject();
    if (!signals || !patch || typeof patch !== "object") {
      return;
    }
    Object.assign(signals, patch);
  }

  function sharedUiSettingsStore() {
    const store = window.__fishystuffUiSettings;
    return store && typeof store.get === "function" && typeof store.set === "function"
      ? store
      : null;
  }

  function panelUiKey(panelId) {
    return panelId === "progress" ? DEX_PROGRESS_PANEL_UI_KEY : DEX_FILTER_PANEL_UI_KEY;
  }

  function normalizeBooleanFlag(value) {
    return value === true || value === "true" || value === 1 || value === "1";
  }

  function readPanelCollapsedState(panelId) {
    const store = sharedUiSettingsStore();
    return normalizeBooleanFlag(store ? store.get(panelUiKey(panelId), false) : false);
  }

  function persistPanelCollapsed(panelId, collapsed) {
    const store = sharedUiSettingsStore();
    if (!store) {
      return;
    }
    store.set(panelUiKey(panelId), Boolean(collapsed));
  }

  function connect(signals) {
    state.signals = signals && typeof signals === "object" ? signals : null;
    if (typeof state.uiSettingsUnsubscribe === "function") {
      state.uiSettingsUnsubscribe();
      state.uiSettingsUnsubscribe = null;
    }

    const store = sharedUiSettingsStore();
    if (!store || typeof store.subscribe !== "function") {
      return;
    }

    state.uiSettingsUnsubscribe = store.subscribe(function () {
      const signalsObject = signalObject();
      if (!signalsObject) {
        return;
      }
      const progressCollapsed = readPanelCollapsedState("progress");
      const filterCollapsed = readPanelCollapsedState("filter");
      if (
        normalizeBooleanFlag(signalsObject._progress_panel_collapsed) === progressCollapsed
        && normalizeBooleanFlag(signalsObject._filter_panel_collapsed) === filterCollapsed
      ) {
        return;
      }
      Object.assign(signalsObject, {
        _progress_panel_collapsed: progressCollapsed,
        _filter_panel_collapsed: filterCollapsed,
      });
    });
  }

  function normalizeStoredFishIds(value) {
    let ids = [];
    if (Array.isArray(value)) {
      ids = value;
    } else if (value && typeof value === "object") {
      ids = Object.entries(value)
        .filter(function (entry) {
          return entry[1];
        })
        .map(function (entry) {
          return entry[0];
        });
    }

    const unique = new Set();
    for (const raw of ids) {
      const fishId = Number.parseInt(String(raw), 10);
      if (Number.isInteger(fishId) && fishId > 0) {
        unique.add(fishId);
      }
    }
    return Array.from(unique).sort(function (left, right) {
      return left - right;
    });
  }

  function parseCaughtJson(raw) {
    return normalizeStoredFishIds(JSON.parse(raw));
  }

  function parseFavouriteJson(raw) {
    return normalizeStoredFishIds(JSON.parse(raw));
  }

  function normalizeMethod(value) {
    const normalized = String(value || "").trim().toLowerCase();
    if (normalized === "harpoon") {
      return "harpoon";
    }
    if (normalized === "rod") {
      return "rod";
    }
    return "unknown";
  }

  function normalizeMethodList(values) {
    const rawValues = Array.isArray(values)
      ? values
      : values === undefined || values === null
        ? []
        : [values];
    const selected = new Set();
    for (const raw of rawValues) {
      const method = normalizeMethod(raw);
      if (method !== "unknown") {
        selected.add(method);
      }
    }
    return METHOD_ORDER.filter(function (method) {
      return selected.has(method);
    });
  }

  function normalizeSortField(value) {
    const normalized = String(value || "price").trim().toLowerCase();
    return SORT_FIELD_ORDER.includes(normalized) ? normalized : "price";
  }

  function normalizeSortDirection(value) {
    const normalized = String(value || "desc").trim().toLowerCase();
    return SORT_DIRECTION_ORDER.includes(normalized) ? normalized : "desc";
  }

  function normalizeCaughtFilter(value) {
    const normalized = String(value || "all").trim().toLowerCase();
    return normalized === "caught" || normalized === "missing" ? normalized : "all";
  }

  function normalizeOrderedValues(values, order) {
    const selected = new Set(Array.isArray(values) ? values.map(String) : []);
    return order.filter(function (value) {
      return selected.has(value);
    });
  }

  function normalizeGradeFilters(values) {
    return normalizeOrderedValues(values, GRADE_FILTER_COLOR_ORDER);
  }

  function normalizeMethodFilters(values) {
    return normalizeOrderedValues(values, METHOD_ORDER);
  }

  function storedUiSignals(signals) {
    return {
      search_query: String((signals && signals.search_query) || "").trim(),
      caught_filter: normalizeCaughtFilter(signals && signals.caught_filter),
      favourite_filter: normalizeBooleanFlag(signals && signals.favourite_filter),
      grade_filters: normalizeGradeFilters(signals && signals.grade_filters),
      method_filters: normalizeMethodFilters(signals && signals.method_filters),
      show_dried: normalizeBooleanFlag(signals && signals.show_dried),
      sort_field: normalizeSortField(signals && signals.sort_field),
      sort_direction: normalizeSortDirection(signals && signals.sort_direction),
    };
  }

  function persistUi() {
    const signals = signalObject();
    if (!signals || !state.uiStateRestored) {
      return;
    }
    try {
      const json = JSON.stringify(storedUiSignals(signals));
      if (json === state.persistedUiJson) {
        return;
      }
      localStorage.setItem(DEX_UI_STORAGE_KEY, json);
      state.persistedUiJson = json;
    } catch (_error) {
      patchSignals({ _status_message: "localStorage is unavailable; progress is not persisted." });
    }
  }

  function loadUiStateFromStorage() {
    let patch = storedUiSignals({});
    let statusMessage = "";
    try {
      const raw = localStorage.getItem(DEX_UI_STORAGE_KEY);
      if (raw) {
        try {
          patch = storedUiSignals(JSON.parse(raw));
        } catch (_error) {
          localStorage.removeItem(DEX_UI_STORAGE_KEY);
          statusMessage = "Reset corrupted local fishydex filters.";
        }
      }
    } catch (_error) {
      statusMessage = "localStorage is unavailable; progress is not persisted.";
    }
    state.persistedUiJson = JSON.stringify(patch);
    return { patch, statusMessage };
  }

  function loadCaughtIdsFromStorage() {
    let caughtIds = [];
    let statusMessage = "";
    try {
      const raw = localStorage.getItem(CAUGHT_STORAGE_KEY);
      if (raw) {
        try {
          caughtIds = parseCaughtJson(raw);
        } catch (_error) {
          localStorage.removeItem(CAUGHT_STORAGE_KEY);
          statusMessage = "Reset corrupted local fishydex progress.";
        }
      }
    } catch (_error) {
      statusMessage = "localStorage is unavailable; progress is not persisted.";
    }
    return { caughtIds, statusMessage };
  }

  function loadFavouriteIdsFromStorage() {
    let favouriteIds = [];
    let statusMessage = "";
    try {
      const raw = localStorage.getItem(FAVOURITES_STORAGE_KEY);
      if (raw) {
        try {
          favouriteIds = parseFavouriteJson(raw);
        } catch (_error) {
          localStorage.removeItem(FAVOURITES_STORAGE_KEY);
          statusMessage = "Reset corrupted local fishydex favourites.";
        }
      }
    } catch (_error) {
      statusMessage = "localStorage is unavailable; progress is not persisted.";
    }
    return { favouriteIds, statusMessage };
  }

  function persistCaughtIds(caughtIds) {
    try {
      localStorage.setItem(CAUGHT_STORAGE_KEY, JSON.stringify(normalizeStoredFishIds(caughtIds)));
    } catch (_error) {
      patchSignals({ _status_message: "localStorage is unavailable; progress is not persisted." });
    }
  }

  function persistFavouriteIds(favouriteIds) {
    try {
      localStorage.setItem(FAVOURITES_STORAGE_KEY, JSON.stringify(normalizeStoredFishIds(favouriteIds)));
    } catch (_error) {
      patchSignals({ _status_message: "localStorage is unavailable; progress is not persisted." });
    }
  }

  function restore(signals) {
    connect(signals);
    const uiState = loadUiStateFromStorage();
    const caughtState = loadCaughtIdsFromStorage();
    const favouriteState = loadFavouriteIdsFromStorage();
    Object.assign(signals, uiState.patch, {
      fish: [],
      count: 0,
      revision: "",
      caught_ids: caughtState.caughtIds,
      favourite_ids: favouriteState.favouriteIds,
      selected_fish_id: null,
      _progress_panel_collapsed: readPanelCollapsedState("progress"),
      _filter_panel_collapsed: readPanelCollapsedState("filter"),
      _status_message:
        uiState.statusMessage
        || caughtState.statusMessage
        || favouriteState.statusMessage
        || "",
    });
    state.uiStateRestored = true;
  }

  function toggleFishIds(values, fishId) {
    const next = new Set(normalizeStoredFishIds(values));
    if (next.has(fishId)) {
      next.delete(fishId);
    } else {
      next.add(fishId);
    }
    return Array.from(next).sort(function (left, right) {
      return left - right;
    });
  }

  function toggleGradeFilters(values, grade) {
    const next = new Set(normalizeGradeFilters(values));
    if (next.has(grade)) {
      next.delete(grade);
    } else {
      next.add(grade);
    }
    return GRADE_FILTER_COLOR_ORDER.filter(function (value) {
      return next.has(value);
    });
  }

  function toggleMethodFilters(values, method) {
    const next = new Set(normalizeMethodFilters(values));
    if (next.has(method)) {
      next.delete(method);
    } else {
      next.add(method);
    }
    return METHOD_ORDER.filter(function (value) {
      return next.has(value);
    });
  }

  function queueStamp(key, fishId) {
    const signals = signalObject();
    if (!signals || !key || typeof key !== "string") {
      return;
    }
    const currentTimer = state.stampTimers.get(key);
    if (currentTimer) {
      window.clearTimeout(currentTimer);
      state.stampTimers.delete(key);
    }
    signals[key] = Number.isInteger(fishId) ? fishId : null;
    if (!Number.isInteger(fishId)) {
      return;
    }
    const timer = window.setTimeout(function () {
      const activeSignals = signalObject();
      if (activeSignals && activeSignals[key] === fishId) {
        activeSignals[key] = null;
      }
      state.stampTimers.delete(key);
    }, 420);
    state.stampTimers.set(key, timer);
  }

  function rememberViewport(fishId, event) {
    const trigger = event && event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
    const card = trigger ? trigger.closest("[data-fish-id]") : null;
    if (!(card instanceof HTMLElement) || !Number.isInteger(fishId)) {
      state.pendingViewportRestore = null;
      return;
    }
    const action = trigger.dataset.action || "";
    state.pendingViewportRestore = {
      fishId: fishId,
      anchorTop: trigger.getBoundingClientRect().top,
      scrollX: window.scrollX,
      scrollY: window.scrollY,
      restoreFocusSelector: action ? `[data-action="${action}"]` : "",
    };
  }

  function scheduleViewportRestore() {
    if (!state.pendingViewportRestore) {
      return;
    }
    if (state.pendingViewportFrame) {
      window.cancelAnimationFrame(state.pendingViewportFrame);
    }
    state.pendingViewportFrame = window.requestAnimationFrame(function () {
      state.pendingViewportFrame = 0;
      const pending = state.pendingViewportRestore;
      state.pendingViewportRestore = null;
      if (!pending) {
        return;
      }
      const nextCard = document.querySelector(`[data-fish-id="${pending.fishId}"]`);
      if (!(nextCard instanceof HTMLElement)) {
        window.scrollTo(pending.scrollX, pending.scrollY);
        return;
      }
      const nextAnchor = pending.restoreFocusSelector
        ? nextCard.querySelector(pending.restoreFocusSelector)
        : nextCard;
      if (nextAnchor instanceof HTMLElement) {
        nextAnchor.focus({ preventScroll: true });
        const delta = nextAnchor.getBoundingClientRect().top - pending.anchorTop;
        if (Math.abs(delta) > 1.5 || Math.abs(pending.scrollX - window.scrollX) > 0.5) {
          window.scrollTo(pending.scrollX, window.scrollY + delta);
        }
        return;
      }
      window.scrollTo(pending.scrollX, pending.scrollY);
    });
  }

  function fishItemId(entry) {
    const value = Number(entry && entry.item_id);
    return Number.isInteger(value) ? value : 0;
  }

  function fishEncyclopediaId(entry) {
    const value = Number(entry && entry.encyclopedia_id);
    return Number.isInteger(value) && value > 0 ? value : null;
  }

  function filterGradeForEntry(entry) {
    if (!entry) {
      return "unknown";
    }
    if (entry.is_prize === true || entry.grade === "Prize") {
      return "red";
    }
    if (entry.grade === "Rare") {
      return "yellow";
    }
    if (entry.grade === "HighQuality") {
      return "blue";
    }
    if (entry.grade === "General") {
      return "green";
    }
    if (entry.grade === "Trash") {
      return "white";
    }
    return "unknown";
  }

  function gradeLabelForKey(value) {
    return GRADE_LABELS[value] || GRADE_LABELS.unknown;
  }

  function entryCatchMethods(entry) {
    if (!entry) {
      return [];
    }
    const methods = Array.isArray(entry.catch_methods)
      ? normalizeMethodList(entry.catch_methods)
      : normalizeMethodList(entry.catch_method);
    return methods.length ? methods : ["rod"];
  }

  function entryIsDried(entry) {
    return normalizeBooleanFlag(entry && entry.is_dried);
  }

  function compareFishNames(left, right) {
    return String((left && left.name) || "").localeCompare(String((right && right.name) || ""), undefined, {
      sensitivity: "base",
      numeric: true,
    });
  }

  function entryVendorPrice(entry) {
    const amount = Number(entry && entry.vendor_price);
    return Number.isFinite(amount) && amount > 0 ? amount : null;
  }

  function compareFishEntries(left, right, sortField, sortDirection) {
    if (sortField === "price") {
      const leftPrice = entryVendorPrice(left);
      const rightPrice = entryVendorPrice(right);
      if (leftPrice === null && rightPrice !== null) {
        return 1;
      }
      if (leftPrice !== null && rightPrice === null) {
        return -1;
      }
      if (leftPrice !== null && rightPrice !== null && leftPrice !== rightPrice) {
        return sortDirection === "desc" ? rightPrice - leftPrice : leftPrice - rightPrice;
      }
    }
    const nameOrder = compareFishNames(left, right);
    if (nameOrder !== 0) {
      return sortDirection === "desc" && sortField === "name" ? -nameOrder : nameOrder;
    }
    return fishItemId(left) - fishItemId(right);
  }

  function fishRenderSignature(fish) {
    let hash = 2166136261;
    for (const entry of fish) {
      const raw = entry
        ? `${fishItemId(entry)}|${fishEncyclopediaId(entry) || 0}|${entry.grade || ""}|${entry.is_prize === true ? 1 : 0}|${entry.name || ""}|${entryIsDried(entry) ? 1 : 0}|${entryCatchMethods(entry).join(",")}|${entry.vendor_price || 0}`
        : "null";
      for (let index = 0; index < raw.length; index += 1) {
        hash ^= raw.charCodeAt(index);
        hash = Math.imul(hash, 16777619);
      }
    }
    return `${fish.length}:${(hash >>> 0).toString(16)}`;
  }

  function createElement(tagName, className, textContent) {
    const element = document.createElement(tagName);
    if (className) {
      element.className = className;
    }
    if (textContent !== undefined) {
      element.textContent = textContent;
    }
    return element;
  }

  function formatSilver(value) {
    const amount = Number(value);
    if (!Number.isFinite(amount) || amount <= 0) {
      return "Unavailable";
    }
    return SILVER_FORMATTER.format(amount);
  }

  function populateVendorPrice(element, value) {
    if (!(element instanceof HTMLElement)) {
      return;
    }
    const icon = createElement("span", "fishydex-price-icon");
    icon.innerHTML = ICON_COIN_STACK;
    const amount = createElement("span", "fishydex-price-value", formatSilver(value));
    element.replaceChildren(icon, amount);
  }

  function createVendorPriceElement(tagName, className, value) {
    const element = createElement(tagName, className);
    populateVendorPrice(element, value);
    return element;
  }

  function fishItemIconPath(itemId) {
    if (!Number.isInteger(itemId) || itemId <= 0) {
      return "";
    }
    return `/images/FishIcons/${String(itemId).padStart(8, "0")}.png`;
  }

  function fishEncyclopediaIconPath(encyclopediaId) {
    if (!Number.isInteger(encyclopediaId) || encyclopediaId <= 0) {
      return "";
    }
    return `/images/FishIcons/IC_0${encyclopediaId}.png`;
  }

  function cdnUrl(path) {
    if (typeof window.__fishystuffResolveCdnUrl === "function") {
      return window.__fishystuffResolveCdnUrl(path);
    }
    const runtimeConfig = window.__fishystuffRuntimeConfig || {};
    const base = String(runtimeConfig.cdnBaseUrl || "https://cdn.fishystuff.fish").replace(/\/+$/, "");
    const raw = String(path || "").trim();
    if (!raw) {
      return "";
    }
    if (raw.startsWith("/")) {
      return `${base}${raw}`;
    }
    return `${base}/${raw.replace(/^\/+/, "")}`;
  }

  function fishItemIconUrl(itemId) {
    if (typeof window.__fishystuffResolveFishItemIconUrl === "function") {
      return window.__fishystuffResolveFishItemIconUrl(itemId);
    }
    const path = fishItemIconPath(itemId);
    return path ? cdnUrl(path) : "";
  }

  function fishEncyclopediaIconUrl(encyclopediaId) {
    if (typeof window.__fishystuffResolveFishEncyclopediaIconUrl === "function") {
      return window.__fishystuffResolveFishEncyclopediaIconUrl(encyclopediaId);
    }
    const path = fishEncyclopediaIconPath(encyclopediaId);
    return path ? cdnUrl(path) : "";
  }

  function setImageWithPlaceholder(image, placeholder, src, alt) {
    if (!(image instanceof HTMLImageElement) || !(placeholder instanceof HTMLElement)) {
      return;
    }
    image.onload = null;
    image.onerror = null;
    image.dataset.expectedSrc = src || "";
    if (!src) {
      image.hidden = true;
      image.removeAttribute("src");
      image.alt = "";
      placeholder.hidden = false;
      return;
    }
    placeholder.hidden = true;
    image.hidden = false;
    image.alt = alt;
    image.onload = function () {
      if (image.dataset.expectedSrc === src) {
        placeholder.hidden = true;
        image.hidden = false;
      }
    };
    image.onerror = function () {
      if (image.dataset.expectedSrc !== src) {
        return;
      }
      image.onload = null;
      image.onerror = null;
      image.hidden = true;
      image.removeAttribute("src");
      image.alt = "";
      placeholder.hidden = false;
    };
    image.src = src;
  }

  function setElementText(element, text) {
    if (element) {
      element.textContent = text;
    }
  }

  function setElementLink(element, text, href) {
    if (!element) {
      return;
    }
    if (!href) {
      element.textContent = text;
      return;
    }
    const link = document.createElement("a");
    link.className = "fishydex-details-link link link-hover";
    link.href = href;
    link.target = "_blank";
    link.rel = "noreferrer noopener";
    link.textContent = text;
    element.replaceChildren(link);
  }

  function bdolyticsItemUrl(itemKey) {
    if (!Number.isInteger(itemKey)) {
      return "";
    }
    return `https://bdolytics.com/en/NA/db/item/${itemKey}`;
  }

  function currentStamp(snapshot, key, fishId) {
    return Number.isInteger(fishId) && snapshot[key] === fishId;
  }

  function renderEmptyState(hasActiveFilters) {
    const empty = createElement("div", "fishydex-empty card card-dash bg-base-100");
    const body = createElement("div", "card-body items-center");
    const title = createElement("h3", "fishydex-empty-title", "No fish match this filter.");
    const detail = createElement(
      "p",
      "fishydex-subtle",
      hasActiveFilters
        ? "Try a broader search or clear some filters."
        : "The fish catalog is empty."
    );
    body.append(title, detail);
    empty.appendChild(body);
    return empty;
  }

  function renderFishCard(fish, caughtSet, favouriteSet, snapshot, animationIndex, animateCards) {
    const itemId = fishItemId(fish);
    const fishName = fish.name || `Fish ${itemId}`;
    const isCaught = caughtSet.has(itemId);
    const isFavourite = favouriteSet.has(itemId);
    const card = createElement("article", "fishydex-card card card-border bg-base-100");
    card.dataset.fishId = String(itemId);
    if (animateCards) {
      card.classList.add("is-entering");
      card.style.setProperty("--fishydex-card-delay", `${Math.min(animationIndex, 11) * 24}ms`);
    }
    if (isCaught) {
      card.classList.add("is-caught");
    }

    const openButton = createElement("button", "fishydex-card-open");
    openButton.type = "button";
    openButton.dataset.action = "open-details";
    openButton.setAttribute("aria-haspopup", "dialog");
    openButton.setAttribute("aria-label", `Open details for ${fishName}`);

    const content = createElement("div", "fishydex-card-content card-body");
    const top = createElement("div", "fishydex-card-top");
    const actions = createElement("div", "fishydex-card-actions");

    const favouriteButton = createElement(
      "button",
      `fishydex-favourite-button btn btn-sm btn-circle btn-ghost${isFavourite ? " is-favourite" : ""}${currentStamp(snapshot, "_favourite_stamp_fish_id", itemId) ? " is-stamping" : ""}`
    );
    favouriteButton.type = "button";
    favouriteButton.dataset.action = "toggle-favourite";
    favouriteButton.setAttribute("aria-pressed", isFavourite ? "true" : "false");
    favouriteButton.setAttribute(
      "aria-label",
      `${isFavourite ? "Remove" : "Add"} ${fishName} ${isFavourite ? "from" : "to"} favourites`
    );
    favouriteButton.innerHTML = isFavourite ? ICON_HEART_FILL : ICON_HEART_LINE;

    const caughtButton = createElement(
      "button",
      `fishydex-caught-button btn btn-sm btn-circle btn-ghost${isCaught ? " is-caught" : ""}${currentStamp(snapshot, "_caught_stamp_fish_id", itemId) ? " is-stamping" : ""}`
    );
    caughtButton.type = "button";
    caughtButton.dataset.action = "toggle-caught";
    caughtButton.setAttribute("aria-pressed", isCaught ? "true" : "false");
    caughtButton.setAttribute("aria-label", `Mark ${fishName} as ${isCaught ? "not caught" : "caught"}`);
    caughtButton.innerHTML = isCaught ? ICON_CAUGHT_FILL : ICON_CAUGHT_LINE;

    actions.append(favouriteButton, caughtButton);
    top.appendChild(actions);

    const main = createElement("div", "fishydex-card-main");
    const iconWrap = createElement("div", `fishydex-icon-wrap grade-${filterGradeForEntry(fish)}`);
    const icon = createElement("img", "fishydex-icon");
    icon.loading = "lazy";
    const placeholder = createElement("div", "fishydex-placeholder", "?");
    setImageWithPlaceholder(icon, placeholder, fishItemIconUrl(itemId), `${fishName} icon`);
    iconWrap.append(icon, placeholder);

    main.appendChild(iconWrap);
    main.appendChild(createElement("div", "fishydex-name", fishName));
    main.appendChild(createVendorPriceElement("div", "fishydex-price fishydex-card-price", entryVendorPrice(fish)));

    content.append(top, main);
    card.append(openButton, content);
    return card;
  }

  function renderGroup(grade, fish, caughtSet, favouriteSet, snapshot, animationIndex, animateCards) {
    if (!fish.length) {
      return null;
    }
    const section = createElement("fieldset", "fishydex-group card card-border bg-base-100");
    const legend = createElement("legend", "fishydex-group-title fieldset-legend ml-6 px-2", gradeLabelForKey(grade));
    const body = createElement("div", "card-body pt-0");
    const header = createElement("div", "fishydex-group-header");
    header.appendChild(createElement("span", "fishydex-group-count badge badge-ghost", `${fish.length} fish`));

    const grid = createElement("div", "fishydex-card-grid");
    let nextAnimationIndex = animationIndex;
    for (const entry of fish) {
      grid.appendChild(renderFishCard(entry, caughtSet, favouriteSet, snapshot, nextAnimationIndex, animateCards));
      nextAnimationIndex += 1;
    }

    body.append(header, grid);
    section.append(legend, body);
    return {
      section: section,
      nextAnimationIndex: nextAnimationIndex,
    };
  }

  function toggleCaught(fishId) {
    const signals = signalObject();
    if (!signals || !Number.isInteger(fishId)) {
      return;
    }
    const caughtIds = toggleFishIds(signals.caught_ids, fishId);
    const isCaught = caughtIds.includes(fishId);
    state.suppressCardAnimation = true;
    persistCaughtIds(caughtIds);
    queueStamp("_caught_stamp_fish_id", isCaught ? fishId : null);
    patchSignals({
      caught_ids: caughtIds,
      _status_message: "",
    });
  }

  function toggleFavourite(fishId) {
    const signals = signalObject();
    if (!signals || !Number.isInteger(fishId)) {
      return;
    }
    const favouriteIds = toggleFishIds(signals.favourite_ids, fishId);
    const isFavourite = favouriteIds.includes(fishId);
    state.suppressCardAnimation = true;
    persistFavouriteIds(favouriteIds);
    queueStamp("_favourite_stamp_fish_id", isFavourite ? fishId : null);
    patchSignals({
      favourite_ids: favouriteIds,
      _status_message: "",
    });
  }

  function bindGridClicks() {
    if (state.gridBound) {
      return;
    }
    const grid = document.getElementById("fishydex-grid");
    if (!(grid instanceof HTMLElement)) {
      return;
    }
    grid.addEventListener("click", function (event) {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }
      const card = target.closest("[data-fish-id]");
      if (!(card instanceof HTMLElement)) {
        return;
      }
      const fishId = Number.parseInt(card.dataset.fishId || "", 10);
      if (!Number.isInteger(fishId)) {
        return;
      }

      const favouriteButton = target.closest("[data-action='toggle-favourite']");
      if (favouriteButton instanceof HTMLElement) {
        rememberViewport(fishId, { currentTarget: favouriteButton });
        toggleFavourite(fishId);
        return;
      }

      const caughtButton = target.closest("[data-action='toggle-caught']");
      if (caughtButton instanceof HTMLElement) {
        rememberViewport(fishId, { currentTarget: caughtButton });
        toggleCaught(fishId);
        return;
      }

      if (target.closest("[data-action='open-details']")) {
        openDetails(fishId);
      }
    });
    state.gridBound = true;
  }

  function fishMetaById(fishId) {
    if (!Number.isInteger(fishId)) {
      return null;
    }
    const fish = state.fishById.get(fishId);
    if (!fish) {
      return null;
    }
    const signals = signalObject();
    const caughtIds = normalizeStoredFishIds(signals && signals.caught_ids);
    const favouriteIds = normalizeStoredFishIds(signals && signals.favourite_ids);
    return {
      fishId: fishId,
      itemId: fishItemId(fish),
      encyclopediaId: fishEncyclopediaId(fish),
      name: fish.name || `Fish ${fishId}`,
      grade: filterGradeForEntry(fish),
      isDried: entryIsDried(fish),
      catchMethods: entryCatchMethods(fish),
      vendorPrice: entryVendorPrice(fish),
      caught: caughtIds.includes(fishId),
      favourite: favouriteIds.includes(fishId),
    };
  }

  function renderDetails() {
    const modal = document.getElementById("fishydex-details");
    if (!(modal instanceof HTMLElement)) {
      return;
    }
    const signals = signalObject();
    const selectedFishId = signals && Number.isInteger(signals.selected_fish_id)
      ? signals.selected_fish_id
      : null;
    const meta = fishMetaById(selectedFishId);
    if (!meta) {
      modal.classList.remove("modal-open");
      modal.hidden = true;
      return;
    }

    modal.hidden = false;
    modal.classList.add("modal-open");

    const title = document.getElementById("fishydex-details-title");
    const favouriteToggle = document.getElementById("fishydex-details-favourite-toggle");
    const caughtToggle = document.getElementById("fishydex-details-caught-toggle");
    const favouriteBadge = document.getElementById("fishydex-details-favourite");
    const caughtBadge = document.getElementById("fishydex-details-caught");
    const gradeBadge = document.getElementById("fishydex-details-grade");
    const rodBadge = document.getElementById("fishydex-details-method-rod");
    const harpoonBadge = document.getElementById("fishydex-details-method-harpoon");
    const driedBadge = document.getElementById("fishydex-details-dried-badge");
    const itemKey = document.getElementById("fishydex-details-item-key");
    const vendorPrice = document.getElementById("fishydex-details-vendor-price");
    const spotsNote = document.getElementById("fishydex-details-spots-note");
    const iconFrame = document.getElementById("fishydex-details-icon-frame");
    const icon = document.getElementById("fishydex-details-icon");
    const iconPlaceholder = document.getElementById("fishydex-details-placeholder");
    const guideImage = document.getElementById("fishydex-details-guide-image");
    const guidePlaceholder = document.getElementById("fishydex-details-guide-placeholder");

    setElementText(title, meta.name);

    if (favouriteToggle instanceof HTMLButtonElement) {
      favouriteToggle.className = `fishydex-favourite-button btn btn-sm btn-circle btn-ghost${meta.favourite ? " is-favourite" : ""}${currentStamp(signals, "_favourite_stamp_fish_id", meta.fishId) ? " is-stamping" : ""}`;
      favouriteToggle.dataset.favouriteState = meta.favourite ? "active" : "inactive";
      favouriteToggle.setAttribute("aria-pressed", meta.favourite ? "true" : "false");
      favouriteToggle.setAttribute(
        "aria-label",
        `${meta.favourite ? "Remove" : "Add"} ${meta.name} ${meta.favourite ? "from" : "to"} favourites`
      );
      favouriteToggle.innerHTML = meta.favourite ? ICON_HEART_FILL : ICON_HEART_LINE;
    }

    if (caughtToggle instanceof HTMLButtonElement) {
      caughtToggle.className = `fishydex-caught-button btn btn-sm btn-circle btn-ghost${meta.caught ? " is-caught" : ""}${currentStamp(signals, "_caught_stamp_fish_id", meta.fishId) ? " is-stamping" : ""}`;
      caughtToggle.dataset.caughtState = meta.caught ? "caught" : "uncaught";
      caughtToggle.setAttribute("aria-pressed", meta.caught ? "true" : "false");
      caughtToggle.setAttribute("aria-label", `Mark ${meta.name} as ${meta.caught ? "not caught" : "caught"}`);
      caughtToggle.innerHTML = meta.caught ? ICON_CAUGHT_FILL : ICON_CAUGHT_LINE;
    }

    if (favouriteBadge instanceof HTMLElement) {
      favouriteBadge.hidden = !meta.favourite;
    }
    if (caughtBadge instanceof HTMLElement) {
      caughtBadge.className = meta.caught
        ? "fishydex-caught badge badge-soft badge-success"
        : "fishydex-grade badge badge-soft grade-unknown";
      caughtBadge.textContent = meta.caught ? "Caught" : "Not Caught";
    }
    if (gradeBadge instanceof HTMLElement) {
      gradeBadge.className = `fishydex-grade badge badge-soft grade-${meta.grade}`;
      gradeBadge.textContent = gradeLabelForKey(meta.grade);
    }
    if (rodBadge instanceof HTMLElement) {
      rodBadge.hidden = !meta.catchMethods.includes("rod");
    }
    if (harpoonBadge instanceof HTMLElement) {
      harpoonBadge.hidden = !meta.catchMethods.includes("harpoon");
    }
    if (driedBadge instanceof HTMLElement) {
      driedBadge.hidden = !meta.isDried;
    }

    setElementLink(itemKey, String(meta.itemId), bdolyticsItemUrl(meta.itemId));
    if (vendorPrice instanceof HTMLElement) {
      vendorPrice.className = "fishydex-price fishydex-details-price";
      populateVendorPrice(vendorPrice, meta.vendorPrice);
    }
    setElementText(
      spotsNote,
      "Planned input: evidence locations mapped back to fishing zones, then ranked by rarity and bite-time behavior in each zone."
    );

    if (iconFrame instanceof HTMLElement) {
      iconFrame.className = `fishydex-details-icon-wrap grade-${meta.grade}`;
    }
    if (icon instanceof HTMLImageElement && iconPlaceholder instanceof HTMLElement) {
      setImageWithPlaceholder(icon, iconPlaceholder, fishItemIconUrl(meta.itemId), `${meta.name} icon`);
    }
    if (guideImage instanceof HTMLImageElement && guidePlaceholder instanceof HTMLElement) {
      setImageWithPlaceholder(
        guideImage,
        guidePlaceholder,
        fishEncyclopediaIconUrl(meta.encyclopediaId),
        `${meta.name} guide image`
      );
    }
  }

  function closeDetails(options) {
    const restoreFocus = !options || options.restoreFocus !== false;
    const signals = signalObject();
    const fishId = signals && Number.isInteger(signals.selected_fish_id) ? signals.selected_fish_id : null;
    patchSignals({ selected_fish_id: null });
    if (!restoreFocus || !Number.isInteger(fishId)) {
      return;
    }
    window.requestAnimationFrame(function () {
      const target = document.querySelector(`[data-fish-id="${fishId}"] [data-action="open-details"]`);
      if (target instanceof HTMLElement) {
        target.focus({ preventScroll: true });
      }
    });
  }

  function openDetails(fishId) {
    if (!fishMetaById(fishId)) {
      return;
    }
    patchSignals({ selected_fish_id: fishId });
  }

  function bindDetailsControls() {
    if (state.detailsBound) {
      return;
    }
    const modal = document.getElementById("fishydex-details");
    if (!(modal instanceof HTMLElement)) {
      return;
    }
    modal.addEventListener("click", function (event) {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }
      if (target.closest("[data-action='close-details']")) {
        closeDetails();
        return;
      }
      if (target.closest("[data-action='toggle-favourite-details']")) {
        const signals = signalObject();
        const fishId = signals && Number.isInteger(signals.selected_fish_id) ? signals.selected_fish_id : null;
        if (Number.isInteger(fishId)) {
          toggleFavourite(fishId);
        }
        return;
      }
      if (target.closest("[data-action='toggle-caught-details']")) {
        const signals = signalObject();
        const fishId = signals && Number.isInteger(signals.selected_fish_id) ? signals.selected_fish_id : null;
        if (Number.isInteger(fishId)) {
          toggleCaught(fishId);
        }
      }
    });
    document.addEventListener("keydown", function (event) {
      const signals = signalObject();
      if (event.key === "Escape" && signals && Number.isInteger(signals.selected_fish_id)) {
        closeDetails();
      }
    });
    state.detailsBound = true;
  }

  function sync(snapshot) {
    bindGridClicks();
    bindDetailsControls();
    persistUi();

    const fish = Array.isArray(snapshot.fish) ? snapshot.fish : [];
    const caughtIds = normalizeStoredFishIds(snapshot.caught_ids);
    const favouriteIds = normalizeStoredFishIds(snapshot.favourite_ids);
    const caughtFilter = normalizeCaughtFilter(snapshot.caught_filter);
    const favouriteFilter = normalizeBooleanFlag(snapshot.favourite_filter);
    const gradeFilters = normalizeGradeFilters(snapshot.grade_filters);
    const methodFilters = normalizeMethodFilters(snapshot.method_filters);
    const showDried = normalizeBooleanFlag(snapshot.show_dried);
    const sortField = normalizeSortField(snapshot.sort_field);
    const sortDirection = normalizeSortDirection(snapshot.sort_direction);
    const searchQuery = String(snapshot.search_query || "").trim().toLowerCase();
    const isLoading = normalizeBooleanFlag(snapshot._loading);

    state.fishById = new Map(
      fish
        .filter(function (entry) {
          return entry && Number.isInteger(entry.item_id);
        })
        .map(function (entry) {
          return [entry.item_id, entry];
        })
    );

    const catalogEntries = fish.filter(function (entry) {
      return entry && (showDried || !entryIsDried(entry));
    });
    const filtered = catalogEntries.filter(function (entry) {
      const haystack = `${fishItemId(entry)} ${entry.name || ""}`.toLowerCase();
      if (searchQuery && !haystack.includes(searchQuery)) {
        return false;
      }
      if (caughtFilter === "caught" && !caughtIds.includes(fishItemId(entry))) {
        return false;
      }
      if (caughtFilter === "missing" && caughtIds.includes(fishItemId(entry))) {
        return false;
      }
      if (favouriteFilter && !favouriteIds.includes(fishItemId(entry))) {
        return false;
      }
      if (gradeFilters.length > 0 && !gradeFilters.includes(filterGradeForEntry(entry))) {
        return false;
      }
      if (methodFilters.length > 0) {
        const methods = entryCatchMethods(entry);
        if (!methodFilters.every(function (method) { return methods.includes(method); })) {
          return false;
        }
      }
      return true;
    });

    const sorted = filtered.slice().sort(function (left, right) {
      return compareFishEntries(left, right, sortField, sortDirection);
    });

    const supportsGradeFilter = fish.some(function (entry) {
      return entry && (entry.is_prize !== null || entry.grade);
    });
    const supportsMethodFilter = fish.some(function (entry) {
      return entry && entryCatchMethods(entry).length > 0;
    });
    const supportsDriedFilter = fish.some(function (entry) {
      return entry && entryIsDried(entry);
    });

    const gradeProgress = {
      red: { total: 0, caught: 0 },
      yellow: { total: 0, caught: 0 },
      blue: { total: 0, caught: 0 },
      green: { total: 0, caught: 0 },
      white: { total: 0, caught: 0 },
    };
    for (const entry of catalogEntries) {
      const grade = filterGradeForEntry(entry);
      if (!gradeProgress[grade]) {
        continue;
      }
      gradeProgress[grade].total += 1;
      if (caughtIds.includes(fishItemId(entry))) {
        gradeProgress[grade].caught += 1;
      }
    }

    const renderKey = [
      snapshot.revision || "",
      fishRenderSignature(fish),
      searchQuery,
      caughtFilter,
      favouriteFilter ? "1" : "0",
      gradeFilters.join(","),
      methodFilters.join(","),
      showDried ? "1" : "0",
      sortField,
      sortDirection,
      caughtIds.join(","),
      favouriteIds.join(","),
      snapshot._caught_stamp_fish_id || "",
      snapshot._favourite_stamp_fish_id || "",
    ].join("|");

    if (renderKey !== state.renderKey) {
      state.renderKey = renderKey;
      const grid = document.getElementById("fishydex-grid");
      if (grid instanceof HTMLElement) {
        const fragment = document.createDocumentFragment();
        const animateCards = !state.suppressCardAnimation;
        let animationIndex = 0;
        for (const grade of GRADE_COLOR_ORDER) {
          const groupEntries = sorted.filter(function (entry) {
            return filterGradeForEntry(entry) === grade;
          });
          const rendered = renderGroup(
            grade,
            groupEntries,
            new Set(caughtIds),
            new Set(favouriteIds),
            snapshot,
            animationIndex,
            animateCards
          );
          if (rendered) {
            fragment.appendChild(rendered.section);
            animationIndex = rendered.nextAnimationIndex;
          }
        }
        if (!sorted.length && !isLoading) {
          fragment.appendChild(
            renderEmptyState(
              Boolean(searchQuery)
              || caughtFilter !== "all"
              || favouriteFilter
              || gradeFilters.length > 0
              || methodFilters.length > 0
            )
          );
        }
        grid.replaceChildren(fragment);
      }
      state.suppressCardAnimation = false;
    }

    patchSignals({
      catalog_count: catalogEntries.length,
      total_count: fish.length,
      visible_count: sorted.length,
      caught_count: catalogEntries.reduce(function (count, entry) {
        return count + (caughtIds.includes(fishItemId(entry)) ? 1 : 0);
      }, 0),
      red_total_count: gradeProgress.red.total,
      red_caught_count: gradeProgress.red.caught,
      yellow_total_count: gradeProgress.yellow.total,
      yellow_caught_count: gradeProgress.yellow.caught,
      blue_total_count: gradeProgress.blue.total,
      blue_caught_count: gradeProgress.blue.caught,
      green_total_count: gradeProgress.green.total,
      green_caught_count: gradeProgress.green.caught,
      white_total_count: gradeProgress.white.total,
      white_caught_count: gradeProgress.white.caught,
      supports_grade_filter: supportsGradeFilter,
      supports_method_filter: supportsMethodFilter,
      supports_dried_filter: supportsDriedFilter,
      _api_error_message: fish.length > 0 ? "" : snapshot._api_error_message,
      _api_error_hint: fish.length > 0 ? "" : snapshot._api_error_hint,
    });

    renderDetails();
    scheduleViewportRestore();
  }

  function downloadJson(filename, text) {
    const blob = new Blob([text], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }

  function showToast(tone, message) {
    const toast = window.__fishystuffToast;
    if (!toast || !message) {
      return;
    }
    const handler = typeof toast[tone] === "function"
      ? toast[tone]
      : typeof toast.show === "function"
        ? function (value) {
            toast.show({ tone: tone, message: value });
          }
        : null;
    if (handler) {
      handler(message);
    }
  }

  async function exportCaught(caughtIds) {
    const normalized = normalizeStoredFishIds(caughtIds);
    const text = JSON.stringify(normalized, null, 2);
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(text);
        patchSignals({ _status_message: `Copied ${normalized.length} caught fish IDs.` });
        showToast("success", `Copied ${normalized.length} caught fish IDs.`);
        return;
      }
    } catch (_error) {
    }
    downloadJson("fishystuff-fishydex-caught.json", text);
    patchSignals({ _status_message: `Downloaded ${normalized.length} caught fish IDs.` });
    showToast("info", `Downloaded ${normalized.length} caught fish IDs.`);
  }

  function importCaught() {
    const raw = window.prompt("Paste caught fish JSON (array of IDs or an id:true map).");
    if (raw === null) {
      return;
    }
    try {
      const caughtIds = parseCaughtJson(raw);
      persistCaughtIds(caughtIds);
      patchSignals({
        caught_ids: caughtIds,
        _status_message: `Imported ${caughtIds.length} caught fish IDs.`,
        _api_error_message: "",
        _api_error_hint: "",
      });
      showToast("success", `Imported ${caughtIds.length} caught fish IDs.`);
    } catch (_error) {
      patchSignals({
        _status_message: "Import failed. Paste a JSON array of fish IDs or a map like {\"8474\": true}.",
      });
      showToast("error", "Import failed. Paste a JSON array of fish IDs or a map like {\"8474\": true}.");
    }
  }

  function apiUrl(pathname) {
    const candidates = [
      window.__fishystuffApiBaseUrl,
      window.__fishystuffRuntimeConfig && window.__fishystuffRuntimeConfig.apiBaseUrl,
      "https://api.fishystuff.fish",
    ];
    for (const value of candidates) {
      const explicitBase = String(value || "").trim();
      if (explicitBase) {
        return new URL(pathname, explicitBase).toString();
      }
    }
    return new URL(pathname, "https://api.fishystuff.fish").toString();
  }

  function fishApiUrl() {
    return apiUrl("/api/v1/fish");
  }

  function handleDatastarEvent(event) {
    const detail = event && event.detail ? event.detail : null;
    if (!detail) {
      return;
    }
    if (detail.type === "finished") {
      patchSignals({
        _api_error_message: "",
        _api_error_hint: "",
      });
      return;
    }
    if (detail.type === "error") {
      const status = detail.argsRaw && detail.argsRaw.status;
      patchSignals({
        _api_error_message: status
          ? `Fish API request failed (HTTP ${status}).`
          : "Fish API request failed.",
        _api_error_hint:
          "If this page is on a different origin than the API, confirm the API CORS allowlist includes this site.",
      });
      return;
    }
    if (detail.type === "retrying") {
      patchSignals({
        _api_error_message: "Fish API request is retrying.",
        _api_error_hint: detail.argsRaw && detail.argsRaw.message
          ? String(detail.argsRaw.message)
          : "The request could not be completed cleanly.",
      });
    }
  }

  document.addEventListener("datastar-fetch", handleDatastarEvent);

  window.Fishydex = {
    restore: restore,
    persistUi: persistUi,
    persistPanelCollapsed: persistPanelCollapsed,
    toggleFishIds: toggleFishIds,
    toggleGradeFilters: toggleGradeFilters,
    toggleMethodFilters: toggleMethodFilters,
    persistCaughtIds: persistCaughtIds,
    persistFavouriteIds: persistFavouriteIds,
    queueStamp: queueStamp,
    rememberViewport: rememberViewport,
    exportCaught: exportCaught,
    importCaught: importCaught,
    sync: sync,
    fishApiUrl: fishApiUrl,
    toggleCaught: toggleCaught,
    toggleFavourite: toggleFavourite,
    openDetails: openDetails,
    closeDetails: closeDetails,
  };
})();
