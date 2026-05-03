import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  assetDataUrl,
  buildHtmlDocument,
  mimeTypeForPath,
} from "./build-embed-images.mjs";

test("mimeTypeForPath recognizes embed image inputs", () => {
  assert.equal(mimeTypeForPath("logo.png"), "image/png");
  assert.equal(mimeTypeForPath("logo.webp"), "image/webp");
  assert.equal(mimeTypeForPath("logo.svg"), "image/svg+xml");
  assert.equal(mimeTypeForPath("logo.bin"), "application/octet-stream");
});

test("buildHtmlDocument inlines the embed logo data", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "fishystuff-embed-test-"));
  try {
    const logoPath = path.join(tempDir, "logo.png");
    fs.writeFileSync(logoPath, Buffer.from("fake-png"));

    const html = buildHtmlDocument({
      locale: "en-US",
      tagline: "Fishing <fast>",
      logoPath,
    });

    assert.match(html, /src="data:image\/png;base64,/);
    assert.doesNotMatch(html, /src="file:/);
    assert.match(html, /Fishing &lt;fast&gt;/);
    assert.equal(assetDataUrl(logoPath), "data:image/png;base64,ZmFrZS1wbmc=");
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
