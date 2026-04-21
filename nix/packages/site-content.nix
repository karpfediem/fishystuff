{
  lib,
  stdenv,
  stdenvNoCC,
  bun,
  chromium,
  imagemagick,
  nodejs,
  python3Packages,
  siteSrc,
  writableTmpDirAsHomeHook,
  woff2,
  zine,
  deploymentEnvironment,
  mapAssetCacheKey,
  publicApiBaseUrl,
  publicCdnBaseUrl,
  publicSiteBaseUrl,
  publicTelemetryBaseUrl,
}:
let
  apiTailwindRoutesSrc = builtins.path {
    path = ../../api/fishystuff_server/src/routes;
    name = "fishystuff-site-tailwind-api-routes-src";
  };
  siteNodeModules = stdenvNoCC.mkDerivation {
    pname = "fishystuff-site-node-modules";
    version = "1";
    src = siteSrc;

    impureEnvVars = lib.fetchers.proxyImpureEnvVars ++ [
      "GIT_PROXY_COMMAND"
      "SOCKS_SERVER"
    ];

    nativeBuildInputs = [
      bun
      writableTmpDirAsHomeHook
    ];

    dontConfigure = true;

    buildPhase = ''
      runHook preBuild

      export BUN_INSTALL_CACHE_DIR="$(mktemp -d)"

      bun install \
        --frozen-lockfile \
        --ignore-scripts \
        --no-progress

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p "$out"
      cp -r node_modules "$out/"

      runHook postInstall
    '';

    dontFixup = true;

    outputHash = "sha256-0J3kJTwRpWprqH0HDcximh/p/Fv1Q6FxCWvaapa07d8=";
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
  };
in
stdenvNoCC.mkDerivation {
  pname = "fishystuff-site-content";
  version = "1";
  src = siteSrc;

  nativeBuildInputs = [
    bun
    chromium
    imagemagick
    nodejs
    python3Packages.fonttools
    writableTmpDirAsHomeHook
    woff2
    zine
  ];

  dontConfigure = true;

  FISHYSTUFF_PUBLIC_SITE_BASE_URL = publicSiteBaseUrl;
  FISHYSTUFF_PUBLIC_API_BASE_URL = publicApiBaseUrl;
  FISHYSTUFF_PUBLIC_CDN_BASE_URL = publicCdnBaseUrl;
  FISHYSTUFF_PUBLIC_TELEMETRY_BASE_URL = publicTelemetryBaseUrl;
  FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY = mapAssetCacheKey;
  FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT = deploymentEnvironment;
  LD_LIBRARY_PATH = lib.makeLibraryPath [ stdenv.cc.cc.lib ];

  buildPhase = ''
    runHook preBuild

    cp -r ${siteNodeModules}/node_modules ./node_modules
    patchShebangs ./node_modules/.bin
    mkdir -p ../api/fishystuff_server/src
    cp -r ${apiTailwindRoutesSrc} ../api/fishystuff_server/src/routes

    bun run content-shells:build
    bun run i18n:build
    bun run datastar:build
    bun run d3:build
    bun run otel:build
    bun run images:build
    bun run icons:build
    bun run tailwind:scan
    bun --bun ./node_modules/@tailwindcss/cli/dist/index.mjs \
      -i tailwind.input.css \
      -o assets/css/site.css
    bun run embed:build

    rm -rf .release-out
    mkdir -p .release-out
    backup_path="$(mktemp .zine.ziggy.backup.XXXXXX)"
    generated_path="$(mktemp .zine.ziggy.generated.XXXXXX)"
    cp zine.ziggy "$backup_path"
    node ./scripts/write-zine-config.mjs \
      --template "$backup_path" \
      --out "$generated_path" \
      --generated-content-root ".generated/content"
    cp "$generated_path" zine.ziggy
    zine release --output "$PWD/.release-out"
    mv "$backup_path" zine.ziggy
    rm -f "$generated_path"

    FISHYSTUFF_WEB_FONT_OUTPUT_ROOT="$PWD/.release-out/css/fonts" \
      bash ./scripts/build-web-fonts.sh
    bun run ./scripts/write-runtime-config.mjs --out "$PWD/.release-out/runtime-config.js"

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p "$out"
    cp -r .release-out/. "$out/"

    runHook postInstall
  '';
}
