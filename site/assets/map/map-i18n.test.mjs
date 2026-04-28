import { test } from "bun:test";
import assert from "node:assert/strict";

import { languageReady } from "./map-i18n.js";

function installLanguageHelper(helper) {
  globalThis.window = {
    ...(globalThis.window || {}),
    __fishystuffLanguage: helper,
  };
  globalThis.__fishystuffLanguage = helper;
}

test("languageReady does not block when the current data language is resolved", async () => {
  installLanguageHelper({
    current() {
      return {
        apiLang: "en",
        apiLangSetting: "",
      };
    },
    ready: new Promise(() => {}),
  });

  assert.equal((await languageReady()).apiLang, "en");
});

test("languageReady waits when an explicit data language setting still needs metadata", async () => {
  let resolveReady = () => {};
  let apiLang = "en";
  const ready = new Promise((resolve) => {
    resolveReady = resolve;
  });
  installLanguageHelper({
    current() {
      return {
        apiLang,
        apiLangSetting: "de",
      };
    },
    ready,
  });

  let settled = false;
  const pendingReady = languageReady().then((snapshot) => {
    settled = true;
    return snapshot;
  });
  await Promise.resolve();
  assert.equal(settled, false);

  apiLang = "de";
  resolveReady();
  assert.equal((await pendingReady).apiLang, "de");
});
