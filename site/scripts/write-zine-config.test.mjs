import test from "node:test";
import assert from "node:assert/strict";

import { rewriteZineHostUrl } from "./write-zine-config.mjs";
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
