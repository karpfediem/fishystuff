{ pkgs, lib ? pkgs.lib }:
{
  api = lib.modules.importApply ./fishystuff-api.nix { inherit pkgs; };
  dolt = lib.modules.importApply ./fishystuff-dolt.nix { inherit pkgs; };
  edge = lib.modules.importApply ./fishystuff-edge.nix { inherit pkgs; };
  grafana = lib.modules.importApply ./fishystuff-grafana.nix { inherit pkgs; };
  jaeger = lib.modules.importApply ./fishystuff-jaeger.nix { inherit pkgs; };
  loki = lib.modules.importApply ./fishystuff-loki.nix { inherit pkgs; };
  otel-collector = lib.modules.importApply ./fishystuff-otel-collector.nix { inherit pkgs; };
  prometheus = lib.modules.importApply ./fishystuff-prometheus.nix { inherit pkgs; };
  vector = lib.modules.importApply ./fishystuff-vector.nix { inherit pkgs; };
}
