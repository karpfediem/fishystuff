{ lib, config, pkgs, ... }: {
  name = "map";
  packages = with pkgs; [
    glslang # or shaderc
    vulkan-headers
    vulkan-loader
    vulkan-tools
    vulkan-validation-layers
    steam-run
    glfw
    glfw-wayland
    wayland-protocols
  ];
  languages = {
    zig = {
      enable = true;
      package = pkgs.zigpkgs.mach-latest;
    };
  };
  env = {
    LD_LIBRARY_PATH = "${pkgs.wayland}/lib:${pkgs.glfw}/lib:${pkgs.freetype}/lib:${pkgs.vulkan-loader}/lib:${pkgs.vulkan-validation-layers}/lib";
    VULKAN_SDK = "${pkgs.vulkan-headers}";
    VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  };
}
