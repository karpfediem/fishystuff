{ inputs, pkgs, ... }: {
  name = "site";
  packages = with pkgs; [
    just
    inputs.zine.packages.${system}.default
    tailwindcss
    watchexec
  ];

  languages.javascript.enable = true;
  languages.javascript.bun.enable = true;
}
