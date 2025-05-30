{ lib, config, pkgs, ... }: {
  name = "map";
  packages = with pkgs; [
    glslang # or shaderc
    vulkan-headers
    vulkan-loader
    vulkan-tools
    vulkan-validation-layers
    steam-run
  ];
  languages = {
    zig = {
      enable = true;
      package = pkgs.zigpkgs.mach-latest;
    };
  };
}
