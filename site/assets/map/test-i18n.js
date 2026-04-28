import { readFileSync } from "node:fs";

const EN_US_CATALOG = JSON.parse(readFileSync(new URL("../../i18n/en-US.ziggy", import.meta.url), "utf8"));

function replaceVars(message, vars = {}) {
  return String(message || "").replace(/\{\s*\$([A-Za-z0-9_]+)\s*\}/g, (_match, name) =>
    Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : "",
  );
}

export function installMapTestI18n(globalRef = globalThis, options = {}) {
  const windowObject = globalRef.window && typeof globalRef.window === "object"
    ? globalRef.window
    : (globalRef.window = {});
  const payload = {
    config: {
      defaultLocale: "en-US",
    },
    catalogs: {
      "en-US": EN_US_CATALOG,
    },
  };
  windowObject.__fishystuffGeneratedI18n = payload;
  globalRef.__fishystuffGeneratedI18n = payload;
  windowObject.__fishystuffLanguage = {
    t(key, vars = {}) {
      const raw = EN_US_CATALOG[key];
      return raw === undefined ? key : replaceVars(raw, vars);
    },
    current() {
      return {
        contentLang: options.contentLang || "en-US",
        locale: options.locale || "en-US",
        apiLang: options.apiLang || "en",
        apiLangSetting: options.apiLangSetting || "",
      };
    },
    ready: Promise.resolve(),
  };
  globalRef.__fishystuffLanguage = windowObject.__fishystuffLanguage;
}
