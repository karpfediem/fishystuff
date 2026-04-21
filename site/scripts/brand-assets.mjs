import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { resolvePublicBaseUrls } from "./write-runtime-config.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const siteDir = path.resolve(scriptDir, "..");

const BETTA_ICON_CDN_PATH = "/images/items/00820996.webp";
const BETTA_ICON_SOURCE_PATH = path.join(repoRoot, "data", "data", "FishIcons", "00820996.png");
const DEFAULT_EMBED_LOGO_PATH = path.join(siteDir, "assets", "img", "logo.png");

export function isBetaDeploymentSite(baseUrl) {
  try {
    const hostname = new URL(String(baseUrl ?? "")).hostname.toLowerCase();
    return hostname === "beta.fishystuff.fish" || hostname.startsWith("beta.");
  } catch {
    return false;
  }
}

export function resolveBrandAssets(env = process.env) {
  const { publicSiteBaseUrl, publicCdnBaseUrl } = resolvePublicBaseUrls(env);
  const configuredBettaEmbedLogoPath = env.FISHYSTUFF_BETTA_EMBED_LOGO_PATH || BETTA_ICON_SOURCE_PATH;
  const bettaEmbedLogoPath = fs.existsSync(configuredBettaEmbedLogoPath)
    ? configuredBettaEmbedLogoPath
    : DEFAULT_EMBED_LOGO_PATH;
  if (isBetaDeploymentSite(publicSiteBaseUrl)) {
    const bettaIconUrl = `${publicCdnBaseUrl}${BETTA_ICON_CDN_PATH}`;
    return {
      variant: "betta",
      heroLogoUrl: bettaIconUrl,
      navLogoUrl: bettaIconUrl,
      navLogoSrcset: "",
      embedLogoPath: bettaEmbedLogoPath,
    };
  }

  return {
    variant: "default",
    heroLogoUrl: "/img/logo.png",
    navLogoUrl: "/img/logo-32.png",
    navLogoSrcset: "/img/logo-32.png 1x, /img/logo-64.png 2x",
    embedLogoPath: DEFAULT_EMBED_LOGO_PATH,
  };
}
