(function () {
  const GENERATED = window.__fishystuffGeneratedI18n || {};
  const CONFIG = GENERATED.config || {};
  const CATALOGS = GENERATED.catalogs || {};
  const PAGE_MANIFEST = GENERATED.pageManifest || {};
  const UI_SETTINGS_KEY = "fishystuff.ui.settings.v1";
  const LOCALE_PATH = ["app", "language", "locale"];
  const API_LANG_PATH = ["app", "language", "apiLang"];
  const AUTO_VALUE = "__auto__";
  const CHANGE_EVENT = "fishystuff:languagechange";

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
    return Array.isArray(CONFIG.apiLanguages) ? CONFIG.apiLanguages.map(trimString).filter(Boolean) : [];
  }

  function defaultContentLang() {
    return trimString(CONFIG.defaultContentLang) || "en-US";
  }

  function defaultLocale() {
    return trimString(CONFIG.defaultLocale) || defaultContentLang();
  }

  function defaultApiLang() {
    return trimString(CONFIG.defaultApiLang) || "en";
  }

  function currentContentLang() {
    const doc = document.documentElement;
    const explicit = trimString(doc.getAttribute("data-content-lang") || doc.lang);
    return explicit || defaultContentLang();
  }

  function defaultLocaleForContent(contentLang) {
    return localeLanguages().includes(contentLang) ? contentLang : defaultLocale();
  }

  function defaultApiLangForLocale(locale) {
    const normalized = trimString(locale).toLowerCase();
    const baseLanguage = normalized.split(/[-_]/)[0];
    if (baseLanguage && apiLanguages().includes(baseLanguage)) {
      return baseLanguage;
    }
    return defaultApiLang();
  }

  function resolveLocale(contentLang, setting) {
    const normalizedSetting = trimString(setting);
    if (normalizedSetting && localeLanguages().includes(normalizedSetting)) {
      return normalizedSetting;
    }
    return defaultLocaleForContent(contentLang);
  }

  function resolveApiLang(locale, setting) {
    const normalizedSetting = trimString(setting);
    if (normalizedSetting && apiLanguages().includes(normalizedSetting)) {
      return normalizedSetting;
    }
    return defaultApiLangForLocale(locale);
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

  function renderLanguagePanels(root = document) {
    const panels = Array.from(root.querySelectorAll("[data-language-panel]"))
      .filter((panel) => panel instanceof HTMLElement);
    if (!panels.length) {
      return;
    }
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
    renderPanels: renderLanguagePanels,
    resolveContentUrl,
    t,
  });

  document.addEventListener("DOMContentLoaded", init);
})();
