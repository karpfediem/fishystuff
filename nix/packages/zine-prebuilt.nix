{
  lib,
  fetchurl,
  stdenvNoCC,
}:
let
  releaseAssets = {
    aarch64-linux = {
      asset = "aarch64-linux-musl.tar.xz";
      hash = "sha256-lBOc2l5fjJ48cA0HdqQ0ws3u0VSMy/W6HyzhFM78YQI=";
    };
    x86_64-linux = {
      asset = "x86_64-linux-musl.tar.xz";
      hash = "sha256-urP0fgfvuBJkKPgDXs9G5DNElltDCq5fiW1lp0f1wZw=";
    };
  };
  system = stdenvNoCC.hostPlatform.system;
  release =
    releaseAssets.${system} or (throw "zine-prebuilt: unsupported system ${system}");
  version = "0.11.2";
in
stdenvNoCC.mkDerivation {
  pname = "zine";
  inherit version;

  src = fetchurl {
    url = "https://github.com/kristoff-it/zine/releases/download/v${version}/${release.asset}";
    hash = release.hash;
  };

  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;
  dontPatchELF = true;
  dontFixup = true;

  installPhase = ''
    runHook preInstall

    mkdir extracted
    tar -xf "$src" -C extracted
    install -Dm755 extracted/zine "$out/bin/zine"

    runHook postInstall
  '';

  # Upstream's musl build is statically linked, so a simple execution check is
  # enough to prove the fetched binary runs on NixOS without extra patching.
  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck

    "$out/bin/zine" version 2>&1 | grep -qx "v${version}"

    runHook postInstallCheck
  '';

  meta = {
    description = "Static site generator for SuperHTML and SuperMD";
    homepage = "https://zine-ssg.io/";
    license = lib.licenses.mit;
    mainProgram = "zine";
    platforms = builtins.attrNames releaseAssets;
    sourceProvenance = with lib.sourceTypes; [ binaryNativeCode ];
  };
}
