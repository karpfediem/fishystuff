import { test } from "bun:test";
import assert from "node:assert/strict";

import { isBetaDeploymentSite, resolveBrandAssets } from "./brand-assets.mjs";

test("isBetaDeploymentSite matches beta sibling hosts", () => {
  assert.equal(isBetaDeploymentSite("https://beta.fishystuff.fish"), true);
  assert.equal(isBetaDeploymentSite("https://beta.preview.example.com"), true);
  assert.equal(isBetaDeploymentSite("https://fishystuff.fish"), false);
  assert.equal(isBetaDeploymentSite("not a url"), false);
});

test("resolveBrandAssets defaults to the existing dolphin brand", () => {
  const assets = resolveBrandAssets({
    FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://fishystuff.fish",
  });

  assert.equal(assets.variant, "default");
  assert.equal(assets.heroLogoUrl, "/img/logo.png");
  assert.equal(assets.navLogoUrl, "/img/logo-32.png");
  assert.equal(assets.navLogoSrcset, "/img/logo-32.png 1x, /img/logo-64.png 2x");
  assert.match(assets.embedLogoPath.replaceAll("\\", "/"), /\/site\/assets\/img\/logo\.png$/);
});

test("resolveBrandAssets switches beta to the sourced betta item icon", () => {
  const assets = resolveBrandAssets({
    FISHYSTUFF_PUBLIC_SITE_BASE_URL: "https://beta.fishystuff.fish",
  });

  assert.equal(assets.variant, "betta");
  assert.equal(assets.heroLogoUrl, "https://cdn.beta.fishystuff.fish/images/items/00820996.webp");
  assert.equal(assets.navLogoUrl, "https://cdn.beta.fishystuff.fish/images/items/00820996.webp");
  assert.equal(assets.navLogoSrcset, "");
  assert.match(assets.embedLogoPath.replaceAll("\\", "/"), /\/data\/data\/FishIcons\/00820996\.png$/);
});
