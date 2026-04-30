export const LANGUAGE_CONFIG = Object.freeze({
  defaultContentLang: "en-US",
  defaultLocale: "en-US",
  defaultApiLang: "en",
  contentLanguages: Object.freeze([
    Object.freeze({ code: "en-US", pathPrefix: "/" }),
    Object.freeze({ code: "de-DE", pathPrefix: "/de-DE/" }),
  ]),
  localeLanguages: Object.freeze(["en-US", "de-DE", "ko-KR"]),
  apiLanguages: Object.freeze(["en", "de", "fr", "sp"]),
});
