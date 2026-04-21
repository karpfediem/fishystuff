{ pkgs, repoRoot }:
let
  fineGrainedTree = pkgs.callPackage ./fine-grained-tree.nix { };
in
fineGrainedTree {
  name = "minimap-source-tiles";
  src = repoRoot + "/data/scratch/minimap/source_tiles";
  fileFilter = relativePath: pkgs.lib.hasSuffix ".png" relativePath;
  bucketPrefixLength = 1;
}
