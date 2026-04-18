{ stdenvNoCC, fetchurl }:

stdenvNoCC.mkDerivation rec {
  pname = "jaeger-local";
  version = "1.75.0";

  src = fetchurl {
    url = "https://github.com/jaegertracing/jaeger/releases/download/v${version}/jaeger-${version}-linux-amd64.tar.gz";
    hash = "sha256-ZUzr/Wyc/V6Inlo/VhtJYEy0FaR/xyDSa/EficSeWS8=";
  };

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp jaeger-${version}-linux-amd64/jaeger-all-in-one $out/bin/
    runHook postInstall
  '';
}
