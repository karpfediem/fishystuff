{ lib, ... }: {
  allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [
    "steam-unwrapped"
    "steam-run"
  ];
}
