{
  lib,
  writeText,
  activeRelease,
  apiClosure ? null,
  cdnRuntimeClosure ? null,
  cluster,
  doltBranchContext,
  doltCommit,
  doltMode ? "read_only",
  doltRepository ? "fishystuff/fishystuff",
  doltServiceClosure ? null,
  environment,
  generation ? 1,
  gitRev,
  hostKey,
  hostName ? hostKey,
  hostRole ? "single-site",
  mode ? "validate",
  releaseGeneration ? generation,
  retainedReleases ? [ ],
  serve ? false,
  siteClosure ? null,
}:
let
  storePathString = value: builtins.unsafeDiscardStringContext "${value}";
  closure =
    gcrootName: value:
    {
      enabled = value != null;
      store_path = if value == null then "" else storePathString value;
      gcroot_path =
        if value == null then "" else "/var/lib/fishystuff/gitops/gcroots/${activeRelease}/${gcrootName}";
    };
  payload = {
    inherit cluster generation mode;
    hosts.${hostKey} = {
      enabled = true;
      role = hostRole;
      hostname = hostName;
    };
    releases.${activeRelease} = {
      generation = releaseGeneration;
      git_rev = gitRev;
      dolt_commit = doltCommit;
      closures = {
        api = closure "api" apiClosure;
        site = closure "site" siteClosure;
        cdn_runtime = closure "cdn-runtime" cdnRuntimeClosure;
        dolt_service = closure "dolt-service" doltServiceClosure;
      };
      dolt = {
        repository = doltRepository;
        commit = doltCommit;
        branch_context = doltBranchContext;
        mode = doltMode;
      };
    };
    environments.${environment} = {
      enabled = true;
      strategy = "single_active";
      host = hostKey;
      active_release = activeRelease;
      retained_releases = retainedReleases;
      inherit serve;
    };
  };
in
assert lib.assertMsg (cluster != "") "gitops desired state requires cluster";
assert lib.assertMsg (environment != "") "gitops desired state requires environment";
assert lib.assertMsg (activeRelease != "") "gitops desired state requires activeRelease";
assert lib.assertMsg (generation > 0) "gitops desired state requires positive generation";
assert lib.assertMsg (releaseGeneration > 0) "gitops desired state requires positive releaseGeneration";
assert lib.assertMsg (gitRev != "") "gitops desired state requires gitRev";
assert lib.assertMsg (doltCommit != "") "gitops desired state requires doltCommit";
assert lib.assertMsg (doltBranchContext != "") "gitops desired state requires doltBranchContext";
assert lib.assertMsg (!serve || mode != "validate") "validate-mode desired state must not request serve";
writeText "fishystuff-gitops-${environment}.desired.json" (
  builtins.toJSON payload + "\n"
)
