{ lib, runCommandLocal }:
{
  name,
  src,
  fileFilter ? (_relativePath: true),
  bucketPrefixLength ? 1,
}:
let
  srcString = toString src;
  allFiles = lib.filesystem.listFilesRecursive src;
  relativePathFor = file: lib.removePrefix "${srcString}/" (toString file);
  selectedFiles =
    builtins.filter (
      file: fileFilter (relativePathFor file)
    ) allFiles;
  bucketFor = relativePath: builtins.substring 0 bucketPrefixLength (builtins.hashString "sha256" relativePath);
  buckets = lib.sort (a: b: a < b) (lib.unique (map (file: bucketFor (relativePathFor file)) selectedFiles));
  bucketTrees = map (
    bucket:
    builtins.path {
      path = src;
      name = "${name}-bucket-${bucket}";
      filter =
        path: type:
        let
          pathString = toString path;
          relativePath =
            if pathString == srcString then
              ""
            else
              lib.removePrefix "${srcString}/" pathString;
        in
        if relativePath == "" then
          true
        else if type == "directory" then
          true
        else
          fileFilter relativePath && bucketFor relativePath == bucket;
    }
  ) buckets;
  escapedLinks = map (
    bucketTree:
    ''
      bucket_root=${lib.escapeShellArg (toString bucketTree)}
      while IFS= read -r -d "" relative_path; do
        relative_path="''${relative_path#./}"
        source_path="$bucket_root/$relative_path"
        target="$out/$relative_path"
        if [[ -d "$source_path" ]]; then
          mkdir -p "$target"
        else
          mkdir -p "$(dirname "$target")"
          ln -sfn "$source_path" "$target"
        fi
      done < <(cd "$bucket_root" && find . -mindepth 1 -print0)
    ''
  ) bucketTrees;
in
runCommandLocal name { } ''
  mkdir -p "$out"
  ${lib.concatStringsSep "\n" escapedLinks}
''
