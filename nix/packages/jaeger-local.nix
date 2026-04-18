{ stdenvNoCC, fetchurl, lib }:

stdenvNoCC.mkDerivation rec {
  pname = "jaeger-local";
  version = "2.17.0";

  src = fetchurl {
    url = "https://github.com/jaegertracing/jaeger/releases/download/v${version}/jaeger-${version}-linux-amd64.tar.gz";
    hash = "sha256-Wwkqq69hnNLo1wn6RIWTaFviRxdACtXfkiLwXycJ/GY=";
  };

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    find jaeger-${version}-linux-amd64 -maxdepth 1 -type f -perm -0100 -exec cp {} $out/bin/ \;
    runHook postInstall
  '';
}
