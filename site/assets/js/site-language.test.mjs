import { test } from "bun:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const SITE_LANGUAGE_SOURCE = fs.readFileSync(
  new URL("./site-language.js", import.meta.url),
  "utf8",
);

const UI_SETTINGS_KEY = "fishystuff.ui.settings.v1";
const DATA_LANGUAGES_CACHE_KEY = "fishystuff.data.languages.v1";

class MemoryStorage {
  constructor(initial = {}) {
    this.map = new Map(Object.entries(initial));
  }

  getItem(key) {
    return this.map.has(key) ? this.map.get(key) : null;
  }

  setItem(key, value) {
    this.map.set(key, String(value));
  }

  removeItem(key) {
    this.map.delete(key);
  }
}

function listenersFor(map, type) {
  if (!map.has(type)) {
    map.set(type, []);
  }
  return map.get(type);
}

function createContext(options = {}) {
  const localStorage = new MemoryStorage(options.localStorage || {});
  const documentListeners = new Map();
  const windowListeners = new Map();
  const languageEvents = [];
  const documentElement = {
    lang: options.lang || "en-US",
    getAttribute(name) {
      if (name === "data-content-lang") {
        return options.contentLang || "";
      }
      if (name === "lang") {
        return this.lang;
      }
      return "";
    },
  };
  const document = {
    documentElement,
    addEventListener(type, listener) {
      listenersFor(documentListeners, type).push(listener);
    },
    dispatchEvent(event) {
      for (const listener of documentListeners.get(event.type) || []) {
        listener(event);
      }
    },
    querySelectorAll() {
      return [];
    },
  };
  const window = {
    document,
    localStorage,
    location: {
      pathname: options.pathname || "/calculator/",
    },
    __fishystuffGeneratedI18n: {
      config: {
        defaultContentLang: "en-US",
        defaultLocale: "en-US",
        defaultApiLang: "en",
        contentLanguages: [
          { code: "en-US", pathPrefix: "/" },
          { code: "de-DE", pathPrefix: "/de-DE/" },
        ],
        localeLanguages: ["en-US", "de-DE", "ko-KR"],
        apiLanguages: ["en"],
      },
      catalogs: {
        "en-US": {
          "language.option.api.en": "English",
          "language.option.api.kr": "Korean",
        },
      },
      pageManifest: {},
    },
    __fishystuffResolveApiUrl(path) {
      return `https://api.fishystuff.fish${path}`;
    },
    addEventListener(type, listener) {
      listenersFor(windowListeners, type).push(listener);
    },
    dispatchEvent(event) {
      if (event.type === "fishystuff:languagechange") {
        languageEvents.push(event.detail);
      }
      for (const listener of windowListeners.get(event.type) || []) {
        listener(event);
      }
    },
  };
  class CustomEvent {
    constructor(type, init = {}) {
      this.type = type;
      this.detail = init.detail;
    }
  }
  const context = {
    window,
    document,
    localStorage,
    fetch: options.fetch || (async (url) => ({
      ok: true,
      async json() {
        return options.metaResponse || { data_languages: ["en"] };
      },
      url,
    })),
    CustomEvent,
    URL,
    JSON,
    Object,
    Array,
    String,
    Set,
    RegExp,
  };
  vm.createContext(context);
  vm.runInContext(SITE_LANGUAGE_SOURCE, context);
  return { context, document, documentListeners, languageEvents, localStorage, window };
}

function uiSettings(apiLang) {
  return JSON.stringify({
    app: {
      language: {
        apiLang,
      },
    },
  });
}

test("site language loads data languages from API metadata", async () => {
  const env = createContext({
    metaResponse: { data_languages: ["en", "kr", "jp", "ko-KR", "kr"] },
  });

  await env.window.__fishystuffLanguage.ready;

  assert.deepEqual(env.window.__fishystuffLanguage.dataLanguages(), ["en", "kr", "jp"]);
  assert.deepEqual(JSON.parse(env.localStorage.getItem(DATA_LANGUAGES_CACHE_KEY)), ["en", "kr", "jp"]);
});

test("site language uses available data codes without locale aliasing", async () => {
  const staleAlias = createContext({
    localStorage: {
      [DATA_LANGUAGES_CACHE_KEY]: JSON.stringify(["en", "kr"]),
      [UI_SETTINGS_KEY]: uiSettings("ko-KR"),
    },
    lang: "ko-KR",
    metaResponse: { data_languages: ["en", "kr"] },
  });

  assert.equal(staleAlias.window.__fishystuffLanguage.current().locale, "ko-KR");
  assert.equal(staleAlias.window.__fishystuffLanguage.current().apiLang, "en");
  await staleAlias.window.__fishystuffLanguage.ready;
  assert.equal(staleAlias.window.__fishystuffLanguage.current().apiLang, "en");

  const directDataCode = createContext({
    localStorage: {
      [DATA_LANGUAGES_CACHE_KEY]: JSON.stringify(["en", "kr"]),
      [UI_SETTINGS_KEY]: uiSettings("kr"),
    },
    lang: "ko-KR",
    metaResponse: { data_languages: ["en", "kr"] },
  });

  assert.equal(directDataCode.window.__fishystuffLanguage.current().locale, "ko-KR");
  assert.equal(directDataCode.window.__fishystuffLanguage.current().apiLang, "kr");
  await directDataCode.window.__fishystuffLanguage.ready;
  assert.equal(directDataCode.window.__fishystuffLanguage.current().apiLang, "kr");
});
