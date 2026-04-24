import { test } from "bun:test";
import assert from "node:assert/strict";

import { rewriteZineContentDirPaths, rewriteZineHostUrl } from "./write-zine-config.mjs";
import { resolvePublicBaseUrls } from "./write-runtime-config.mjs";

test("zine config host uses the resolved public site base", () => {
  const { publicSiteBaseUrl } = resolvePublicBaseUrls({
    FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
  });
  const next = rewriteZineHostUrl(
    'Multilingual {\n  .host_url = "https://fishystuff.fish",\n}\n',
    publicSiteBaseUrl,
  );

  assert.match(next, /\.host_url = "https:\/\/beta\.fishystuff\.fish",/);
});

test("zine config content dirs can be redirected to generated shells", () => {
  const next = rewriteZineContentDirPaths(
    [
      "Multilingual {",
      '  .locales = [',
      "    {",
      '      .code = "en-US",',
      '      .content_dir_path = "content/en-US",',
      "    },",
      "    {",
      '      .code = "de-DE",',
      '      .content_dir_path = "content/de-DE",',
      "    },",
      "  ],",
      "}",
      "",
    ].join("\n"),
    ".generated/content",
  );

  assert.match(next, /\.content_dir_path = "\.generated\/content\/en-US",/);
  assert.match(next, /\.content_dir_path = "\.generated\/content\/de-DE",/);
});
