{ inputs, lib, ... }: [
  (final: prev: {
    zigpkgs = inputs.zig.packages.${prev.system};
    zig = inputs.zig.packages.${prev.system}."2024-12-30";
    zine = inputs.zine.packages.${prev.system}.default;
  })
]
