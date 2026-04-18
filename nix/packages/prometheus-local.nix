{ stdenvNoCC, fetchurl, lib }:

stdenvNoCC.mkDerivation rec {
  pname = "prometheus-local";
  version = "3.11.2";

  src = fetchurl {
    url = "https://github.com/prometheus/prometheus/releases/download/v${version}/prometheus-${version}.linux-amd64.tar.gz";
    hash = "sha256-9kPqHukNEJMpMC0nvdsfsuUmVbH6hOnib5pvNA2hRKY=";
  };

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp prometheus-${version}.linux-amd64/prometheus $out/bin/
    cp prometheus-${version}.linux-amd64/promtool $out/bin/
    runHook postInstall
  '';
}
