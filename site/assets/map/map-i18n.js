function trimString(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function replaceVars(message, vars = {}) {
  return String(message || "").replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) =>
    Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : "",
  );
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function languageHelper() {
  const helper = globalThis.window?.__fishystuffLanguage ?? globalThis.__fishystuffLanguage;
  return helper && typeof helper.t === "function" ? helper : null;
}

function generatedI18nPayload() {
  return globalThis.window?.__fishystuffGeneratedI18n ?? globalThis.__fishystuffGeneratedI18n ?? null;
}

function catalogForLocale(locale = "") {
  const payload = generatedI18nPayload();
  const catalogs = isPlainObject(payload?.catalogs) ? payload.catalogs : null;
  if (!catalogs) {
    return null;
  }
  const requestedLocale = trimString(locale);
  if (requestedLocale && isPlainObject(catalogs[requestedLocale])) {
    return catalogs[requestedLocale];
  }
  const defaultLocale = trimString(payload?.config?.defaultLocale) || "en-US";
  return isPlainObject(catalogs[defaultLocale]) ? catalogs[defaultLocale] : null;
}

function mapKey(key) {
  const normalized = trimString(key);
  if (!normalized) {
    return "";
  }
  return normalized.startsWith("map.") ? normalized : `map.${normalized}`;
}

function translatedText(key, vars = {}, options = {}) {
  const resolvedKey = trimString(key);
  if (!resolvedKey) {
    return "";
  }
  const normalizedVars = isPlainObject(vars) ? vars : {};
  const locale = trimString(options?.locale);
  const helper = languageHelper();
  if (helper) {
    const translated = helper.t(resolvedKey, normalizedVars, locale ? { locale } : {});
    if (translated !== resolvedKey) {
      return String(translated);
    }
  }
  const raw = catalogForLocale(locale)?.[resolvedKey];
  return raw === undefined ? resolvedKey : replaceVars(raw, normalizedVars);
}

export function siteText(key, vars = {}, options = {}) {
  return translatedText(key, vars, options);
}

export function mapText(key, vars = {}, options = {}) {
  return translatedText(mapKey(key), vars, options);
}

export function mapCountText(key, count, vars = {}, options = {}) {
  const normalizedCount = Number(count);
  const normalizedVars = isPlainObject(vars) ? vars : {};
  return mapText(`${key}.${normalizedCount === 1 ? "one" : "other"}`, {
    ...normalizedVars,
    count,
  }, options);
}
