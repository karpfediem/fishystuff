{ runCommandLocal }:
let
  assetsSrc = builtins.path {
    path = ../../site/assets;
    name = "fishystuff-site-assets-src";
  };
  contentSrc = builtins.path {
    path = ../../site/content;
    name = "fishystuff-site-content-src";
  };
  i18nSrc = builtins.path {
    path = ../../site/i18n;
    name = "fishystuff-site-i18n-src";
  };
  layoutsSrc = builtins.path {
    path = ../../site/layouts;
    name = "fishystuff-site-layouts-src";
  };
  scriptsSrc = builtins.path {
    path = ../../site/scripts;
    name = "fishystuff-site-scripts-src";
  };
  packageJson = builtins.path {
    path = ../../site/package.json;
    name = "fishystuff-site-package.json";
  };
  bunLock = builtins.path {
    path = ../../site/bun.lock;
    name = "fishystuff-site-bun.lock";
  };
  tailwindInput = builtins.path {
    path = ../../site/tailwind.input.css;
    name = "fishystuff-site-tailwind.input.css";
  };
  zineConfig = builtins.path {
    path = ../../site/zine.ziggy;
    name = "fishystuff-site-zine.ziggy";
  };
in
runCommandLocal "fishystuff-site-src" { } ''
  mkdir -p "$out"
  cp -r ${assetsSrc} "$out/assets"
  cp -r ${contentSrc} "$out/content"
  cp -r ${i18nSrc} "$out/i18n"
  cp -r ${layoutsSrc} "$out/layouts"
  cp -r ${scriptsSrc} "$out/scripts"
  cp ${packageJson} "$out/package.json"
  cp ${bunLock} "$out/bun.lock"
  cp ${tailwindInput} "$out/tailwind.input.css"
  cp ${zineConfig} "$out/zine.ziggy"
''
