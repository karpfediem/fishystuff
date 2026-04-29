(function () {
  const GENERATED = window.__fishystuffGeneratedI18n || {};
  const CONFIG = GENERATED.config || {};
  const CATALOGS = GENERATED.catalogs || {};
  const PAGE_MANIFEST = GENERATED.pageManifest || {};
  const UI_SETTINGS_KEY = "fishystuff.ui.settings.v1";
  const DATA_LANGUAGES_CACHE_KEY = "fishystuff.data.languages.v1";
  const LOCALE_PATH = ["app", "language", "locale"];
  const API_LANG_PATH = ["app", "language", "apiLang"];
  const AUTO_VALUE = "__auto__";
  const CHANGE_EVENT = "fishystuff:languagechange";
  const LANGUAGE_DETAILS_BOUND_ATTR = "data-language-refresh-bound";
  let runtimeApiLanguages = initialApiLanguages();
  let dataLanguagesPromise = null;

  function trimString(value) {
    return String(value ?? "").trim();
  }

  function isPlainObject(value) {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      return false;
    }
    const prototype = Object.getPrototypeOf(value);
    return prototype === Object.prototype || prototype === null;
  }

  function normalizePathParts(path) {
    return Array.isArray(path)
      ? path.map(trimString).filter(Boolean)
      : trimString(path).split(".").map(trimString).filter(Boolean);
  }

  function readJsonStorage(key, fallback) {
    try {
      const raw = localStorage.getItem(key);
      if (!raw) {
        return fallback;
      }
      const parsed = JSON.parse(raw);
      return parsed === undefined ? fallback : parsed;
    } catch (_error) {
      return fallback;
    }
  }

  function isDataLangCode(value) {
    const code = trimString(value);
    return /^[a-z0-9_]+$/.test(code);
  }

  function normalizeApiLanguages(values) {
    if (!Array.isArray(values)) {
      return [];
    }
    const seen = new Set();
    const languages = [];
    for (const value of values) {
      const code = trimString(value);
      if (!isDataLangCode(code) || seen.has(code)) {
        continue;
      }
      seen.add(code);
      languages.push(code);
    }
    return languages;
  }

  function initialApiLanguages() {
    const cached = normalizeApiLanguages(readJsonStorage(DATA_LANGUAGES_CACHE_KEY, []));
    return cached.length ? cached : normalizeApiLanguages(CONFIG.apiLanguages);
  }

  function writeDataLanguagesCache(languages) {
    try {
      localStorage.setItem(DATA_LANGUAGES_CACHE_KEY, JSON.stringify(languages));
    } catch (_error) {
    }
  }

  function readStoredPath(path) {
    const store = window.__fishystuffUiSettings;
    if (store && typeof store.get === "function") {
      return store.get(path, "");
    }
    const settings = readJsonStorage(UI_SETTINGS_KEY, {});
    let current = settings;
    for (const part of normalizePathParts(path)) {
      if (!isPlainObject(current) || !(part in current)) {
        return "";
      }
      current = current[part];
    }
    return current;
  }

  function writeStoredPath(path, value) {
    const store = window.__fishystuffUiSettings;
    const normalizedValue = trimString(value);
    if (store && typeof store.set === "function" && typeof store.remove === "function") {
      if (normalizedValue) {
        store.set(path, normalizedValue);
      } else {
        store.remove(path);
      }
      return;
    }
    const settings = readJsonStorage(UI_SETTINGS_KEY, {});
    const next = isPlainObject(settings) ? { ...settings } : {};
    let cursor = next;
    const parts = normalizePathParts(path);
    for (const part of parts.slice(0, -1)) {
      cursor[part] = isPlainObject(cursor[part]) ? { ...cursor[part] } : {};
      cursor = cursor[part];
    }
    if (!parts.length) {
      return;
    }
    if (normalizedValue) {
      cursor[parts[parts.length - 1]] = normalizedValue;
    } else {
      delete cursor[parts[parts.length - 1]];
    }
    try {
      localStorage.setItem(UI_SETTINGS_KEY, JSON.stringify(next));
    } catch (_error) {
    }
  }

  function contentLanguages() {
    return Array.isArray(CONFIG.contentLanguages) ? CONFIG.contentLanguages : [];
  }

  function localeLanguages() {
    return Array.isArray(CONFIG.localeLanguages) ? CONFIG.localeLanguages.map(trimString).filter(Boolean) : [];
  }

  function apiLanguages() {
    return runtimeApiLanguages.slice();
  }

  function defaultContentLang() {
    return trimString(CONFIG.defaultContentLang) || "en-US";
  }

  function defaultLocale() {
    return trimString(CONFIG.defaultLocale) || defaultContentLang();
  }

  function defaultApiLang() {
    const configured = trimString(CONFIG.defaultApiLang);
    if (configured && runtimeApiLanguages.includes(configured)) {
      return configured;
    }
    return runtimeApiLanguages[0] || configured || "en";
  }

  function currentContentLang() {
    const doc = document.documentElement;
    const explicit = trimString(doc.getAttribute("data-content-lang") || doc.lang);
    return explicit || defaultContentLang();
  }

  function defaultLocaleForContent(contentLang) {
    return localeLanguages().includes(contentLang) ? contentLang : defaultLocale();
  }

  function resolveLocale(contentLang, setting) {
    const normalizedSetting = trimString(setting);
    if (normalizedSetting && localeLanguages().includes(normalizedSetting)) {
      return normalizedSetting;
    }
    return defaultLocaleForContent(contentLang);
  }

  function resolveApiLang(_locale, setting) {
    const normalizedSetting = trimString(setting);
    if (normalizedSetting && apiLanguages().includes(normalizedSetting)) {
      return normalizedSetting;
    }
    return defaultApiLang();
  }

  function shouldLoadDataLanguages(state = snapshot()) {
    const setting = trimString(state.apiLangSetting);
    return Boolean(setting && isDataLangCode(setting) && !apiLanguages().includes(setting));
  }

  function replaceVars(message, vars = {}) {
    return String(message || "").replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, function (_match, name) {
      return Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : "";
    });
  }

  function snapshot() {
    const contentLang = currentContentLang();
    const localeSetting = trimString(readStoredPath(LOCALE_PATH));
    const locale = resolveLocale(contentLang, localeSetting);
    const apiLangSetting = trimString(readStoredPath(API_LANG_PATH));
    const apiLang = resolveApiLang(locale, apiLangSetting);
    return Object.freeze({
      contentLang,
      locale,
      apiLang,
      localeSetting,
      apiLangSetting,
      localeSource: localeSetting ? "explicit" : "auto",
      apiLangSource: apiLangSetting ? "explicit" : "auto",
    });
  }

  function sameSnapshot(left, right) {
    return Boolean(left && right)
      && left.contentLang === right.contentLang
      && left.locale === right.locale
      && left.apiLang === right.apiLang
      && left.localeSetting === right.localeSetting
      && left.apiLangSetting === right.apiLangSetting;
  }

  function apiUrl(pathname) {
    if (typeof window.__fishystuffResolveApiUrl === "function") {
      return window.__fishystuffResolveApiUrl(pathname);
    }
    const configured = window.__fishystuffRuntimeConfig || {};
    const baseUrl = trimString(window.__fishystuffApiBaseUrl || configured.apiBaseUrl);
    if (!baseUrl) {
      return pathname;
    }
    try {
      return new URL(pathname, baseUrl).toString();
    } catch (_error) {
      return pathname;
    }
  }

  function updateRuntimeApiLanguages(languages) {
    const normalized = normalizeApiLanguages(languages);
    if (!normalized.length) {
      return false;
    }
    if (normalized.join("\n") === runtimeApiLanguages.join("\n")) {
      writeDataLanguagesCache(normalized);
      return false;
    }
    runtimeApiLanguages = normalized;
    writeDataLanguagesCache(normalized);
    return true;
  }

  async function loadDataLanguages() {
    if (dataLanguagesPromise) {
      return dataLanguagesPromise;
    }
    dataLanguagesPromise = (async () => {
      if (typeof fetch !== "function") {
        return apiLanguages();
      }
      let response;
      try {
        response = await fetch(apiUrl("/api/v1/meta"));
      } catch (_error) {
        return apiLanguages();
      }
      if (!response || !response.ok || typeof response.json !== "function") {
        return apiLanguages();
      }
      let meta;
      try {
        meta = await response.json();
      } catch (_error) {
        return apiLanguages();
      }
      const before = snapshot();
      if (updateRuntimeApiLanguages(meta && meta.data_languages)) {
        const after = snapshot();
        apply(document);
        renderLanguagePanels(document);
        if (!sameSnapshot(before, after)) {
          dispatchChange(after);
        }
      }
      return apiLanguages();
    })();
    return dataLanguagesPromise;
  }

  function languageReady() {
    const current = snapshot();
    if (!shouldLoadDataLanguages(current)) {
      return Promise.resolve(current);
    }
    return loadDataLanguages()
      .catch(() => apiLanguages())
      .then(() => snapshot());
  }

  function catalogForLocale(locale) {
    return isPlainObject(CATALOGS[locale]) ? CATALOGS[locale] : {};
  }

  function t(key, vars = {}, options = {}) {
    const requestedLocale = trimString(options.locale) || snapshot().locale;
    const preferred = catalogForLocale(requestedLocale);
    const fallback = catalogForLocale(defaultLocale());
    const raw = preferred[key] ?? fallback[key];
    if (raw === undefined) {
      return key;
    }
    return replaceVars(raw, vars);
  }

  function rootPathForContentLang(code) {
    const match = contentLanguages().find((language) => trimString(language.code) === trimString(code));
    if (!match) {
      return "/";
    }
    const prefix = trimString(match.pathPrefix || "/");
    return prefix === "/" ? "/" : `${prefix.replace(/\/+$/, "")}/`;
  }

  function normalizeRouteKey(pathname) {
    let normalized = trimString(pathname || "/");
    if (!normalized.startsWith("/")) {
      normalized = `/${normalized}`;
    }
    normalized = normalized.replace(/\/{2,}/g, "/");
    if (normalized !== "/" && !normalized.endsWith("/")) {
      normalized = `${normalized}/`;
    }
    return normalized;
  }

  function routeState(pathname) {
    const normalizedPath = normalizeRouteKey(pathname);
    const prefixed = contentLanguages()
      .filter((language) => trimString(language.pathPrefix) && trimString(language.pathPrefix) !== "/")
      .slice()
      .sort((left, right) => trimString(right.pathPrefix).length - trimString(left.pathPrefix).length);
    for (const language of prefixed) {
      const prefix = trimString(language.pathPrefix).replace(/\/+$/, "");
      if (normalizedPath === prefix || normalizedPath.startsWith(`${prefix}/`)) {
        const relative = normalizedPath.slice(prefix.length) || "/";
        return {
          contentLang: trimString(language.code),
          routeKey: normalizeRouteKey(relative),
        };
      }
    }
    return {
      contentLang: defaultContentLang(),
      routeKey: normalizedPath,
    };
  }

  function resolveContentUrl(targetContentLang, pathname = window.location.pathname) {
    const route = routeState(pathname);
    const variants = PAGE_MANIFEST[route.routeKey];
    if (variants && variants[targetContentLang]) {
      return variants[targetContentLang];
    }
    if (variants && variants[defaultContentLang()]) {
      return variants[defaultContentLang()];
    }
    return rootPathForContentLang(targetContentLang);
  }

  function dispatchChange(detail) {
    window.dispatchEvent(new CustomEvent(CHANGE_EVENT, { detail }));
  }

  function apply(root = document) {
    if (!root || typeof root.querySelectorAll !== "function") {
      return;
    }
    for (const element of root.querySelectorAll("[data-i18n-text]")) {
      element.textContent = t(trimString(element.getAttribute("data-i18n-text")));
    }
    for (const element of root.querySelectorAll("*")) {
      if (!element.attributes) {
        continue;
      }
      for (const attribute of Array.from(element.attributes)) {
        if (!attribute.name.startsWith("data-i18n-attr-")) {
          continue;
        }
        const targetName = attribute.name.slice("data-i18n-attr-".length);
        element.setAttribute(targetName, t(trimString(attribute.value)));
      }
    }
  }

  function buildOption(value, label) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    return option;
  }

  function translatedOptionLabel(prefix, code) {
    const normalizedCode = trimString(code);
    if (!normalizedCode) {
      return "";
    }
    const key = `${prefix}.${normalizedCode}`;
    const translated = t(key);
    return translated === key ? normalizedCode : translated;
  }

  function renderSelectField({ id, label, help, value, options, onChange }) {
    const fieldset = document.createElement("fieldset");
    fieldset.className = "fieldset rounded-box border border-base-300 bg-base-100 p-3";

    const legend = document.createElement("legend");
    legend.className = "fieldset-legend text-sm";
    legend.textContent = label;
    fieldset.appendChild(legend);

    const select = document.createElement("select");
    select.id = id;
    select.className = "select select-sm select-bordered w-full";
    for (const option of options) {
      select.appendChild(buildOption(option.value, option.label));
    }
    select.value = value;
    select.addEventListener("change", onChange);
    fieldset.appendChild(select);

    const note = document.createElement("p");
    note.className = "mt-2 text-xs text-base-content/70";
    note.textContent = help;
    fieldset.appendChild(note);

    return fieldset;
  }

  function updateLanguageLabels(root, state) {
    const current = state || snapshot();
    const localeLabel = translatedOptionLabel("language.option.locale", current.locale);
    const contentLabel = translatedOptionLabel("language.option.content", current.contentLang);
    const apiLabel = translatedOptionLabel("language.option.api", current.apiLang);
    for (const element of root.querySelectorAll("[data-language-current-label]")) {
      element.textContent = localeLabel;
    }
    for (const element of root.querySelectorAll("[data-language-content-label]")) {
      element.textContent = contentLabel;
    }
    for (const element of root.querySelectorAll("[data-language-api-label]")) {
      element.textContent = apiLabel;
    }
  }

  function languagePanels(root = document) {
    const panels = Array.from(root.querySelectorAll("[data-language-panel]"))
      .filter((panel) => panel instanceof HTMLElement);
    return panels;
  }

  function panelDetails(panel) {
    return typeof panel.closest === "function" ? panel.closest("details") : null;
  }

  function isVisibleLanguagePanel(panel) {
    const details = panelDetails(panel);
    return !details || Boolean(details.open);
  }

  function bindLanguagePanelRefresh(panels) {
    for (const panel of panels) {
      const details = panelDetails(panel);
      if (!details || details.hasAttribute(LANGUAGE_DETAILS_BOUND_ATTR)) {
        continue;
      }
      details.setAttribute(LANGUAGE_DETAILS_BOUND_ATTR, "true");
      details.addEventListener("toggle", () => {
        if (!details.open) {
          return;
        }
        loadDataLanguages()
          .then(() => renderLanguagePanels(document))
          .catch(() => {});
      });
    }
  }

  function maybeLoadDataLanguagesForVisiblePanels(panels) {
    if (dataLanguagesPromise || !panels.some(isVisibleLanguagePanel)) {
      return;
    }
    loadDataLanguages().catch(() => {});
  }

  function renderLanguagePanels(root = document) {
    const panels = languagePanels(root);
    if (!panels.length) {
      return;
    }
    bindLanguagePanelRefresh(panels);
    const state = snapshot();
    panels.forEach((panel, index) => {
      const instanceId = trimString(panel.getAttribute("data-language-panel-id")) || String(index + 1);
      panel.replaceChildren();

      panel.appendChild(renderSelectField({
        id: `content-language-select-${instanceId}`,
        label: t("language.menu.content"),
        help: t("language.menu.content_help"),
        value: state.contentLang,
        options: contentLanguages().map((language) => ({
          value: trimString(language.code),
          label: translatedOptionLabel("language.option.content", language.code),
        })),
        onChange(event) {
          const next = trimString(event.target.value);
          if (next && next !== state.contentLang) {
            window.location.assign(resolveContentUrl(next));
          }
        },
      }));

      panel.appendChild(renderSelectField({
        id: `locale-language-select-${instanceId}`,
        label: t("language.menu.locale"),
        help: t("language.menu.locale_help"),
        value: state.localeSetting || AUTO_VALUE,
        options: [
          { value: AUTO_VALUE, label: t("language.option.auto.locale") },
          ...localeLanguages().map((code) => ({
            value: code,
            label: translatedOptionLabel("language.option.locale", code),
          })),
        ],
        onChange(event) {
          const next = trimString(event.target.value);
          writeStoredPath(LOCALE_PATH, next === AUTO_VALUE ? "" : next);
          window.location.reload();
        },
      }));

      panel.appendChild(renderSelectField({
        id: `api-language-select-${instanceId}`,
        label: t("language.menu.api"),
        help: t("language.menu.api_help"),
        value: state.apiLangSetting || AUTO_VALUE,
        options: [
          { value: AUTO_VALUE, label: t("language.option.auto.api") },
          ...apiLanguages().map((code) => ({
            value: code,
            label: translatedOptionLabel("language.option.api", code),
          })),
        ],
        onChange(event) {
          const next = trimString(event.target.value);
          writeStoredPath(API_LANG_PATH, next === AUTO_VALUE ? "" : next);
          window.location.reload();
        },
      }));
    });

    updateLanguageLabels(root, state);
    maybeLoadDataLanguagesForVisiblePanels(panels);
  }

  function init() {
    const current = snapshot();
    apply(document);
    renderLanguagePanels(document);
    dispatchChange(current);
  }

  window.__fishystuffLanguage = Object.freeze({
    autoValue: AUTO_VALUE,
    event: CHANGE_EVENT,
    apply,
    current: snapshot,
    dataLanguages: apiLanguages,
    renderPanels: renderLanguagePanels,
    resolveContentUrl,
    ready: languageReady(),
    refreshDataLanguages: loadDataLanguages,
    t,
  });

  document.addEventListener("DOMContentLoaded", init);
})();
