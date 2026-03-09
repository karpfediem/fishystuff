{ inputs, pkgs, ... }: {
  name = "site";
  packages = with pkgs; [
    just
    (inputs.zine.packages.${system}.default.override { 
          zigPreferMusl = true;
    })
    tailwindcss
    watchexec
  ];

  languages.javascript.enable = true;
  languages.javascript.bun.enable = true;
}
