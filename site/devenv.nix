{ inputs, pkgs, ... }: {
  name = "site";
  packages = with pkgs; [
    just
    inputs.zine.packages.${system}.default
  ];
}
