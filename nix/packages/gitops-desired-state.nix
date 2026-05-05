{
  lib,
  writeText,
  activeRelease ? null,
  admissionProbe ? null,
  apiService ? "",
  apiUpstream ? "",
  apiClosure ? null,
  cdnRuntimeClosure ? null,
  cluster,
  doltBranchContext,
  doltCommit,
  doltCacheDir ? "",
  doltMaterialization ? "metadata_only",
  doltMode ? "read_only",
  doltRepository ? "fishystuff/fishystuff",
  doltReleaseRef ? "",
  doltRemoteUrl ? "",
  doltServiceClosure ? null,
  environment,
  generation ? 1,
  gitRev,
  hostKey,
  hostName ? hostKey,
  hostRole ? "single-site",
  mode ? "validate",
  retainedReleaseObjects ? [ ],
  releaseGeneration ? generation,
  retainedReleases ? [ ],
  serve ? false,
  siteClosure ? null,
  transition ? null,
}:
let
  storePathString = value: builtins.unsafeDiscardStringContext "${value}";
  optionalStorePath = value: if value == null then "" else storePathString value;
  remoteUrlContainsUserinfo =
    url:
    !lib.hasPrefix "file://" url && builtins.match "[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@.*" url != null;
  releaseMaterial = {
    generation = releaseGeneration;
    git_rev = gitRev;
    dolt_commit = doltCommit;
    dolt_repository = doltRepository;
    dolt_branch_context = doltBranchContext;
    dolt_mode = doltMode;
    api = optionalStorePath apiClosure;
    site = optionalStorePath siteClosure;
    cdn_runtime = optionalStorePath cdnRuntimeClosure;
    dolt_service = optionalStorePath doltServiceClosure;
  };
  derivedRelease =
    "release-${builtins.substring 0 16 (builtins.hashString "sha256" (builtins.toJSON releaseMaterial))}";
  releaseId = if activeRelease == null then derivedRelease else activeRelease;
  admissionProbeKind = if admissionProbe == null then "" else admissionProbe.kind or "";
  admissionProbeTimeoutMs = if admissionProbe == null then 0 else admissionProbe.timeout_ms or 0;
  admissionProbeExpectedStatus = if admissionProbe == null then 0 else admissionProbe.expected_status or 0;
  admissionProbeIsDoltSql = admissionProbeKind == "dolt_sql_scalar";
  admissionProbeIsHttpStatus = admissionProbeKind == "http_status";
  admissionProbeIsHttpJsonScalar = admissionProbeKind == "http_json_scalar";
  admissionProbeIsHttp = admissionProbeIsHttpStatus || admissionProbeIsHttpJsonScalar;
  closure =
    releaseKey: gcrootName: value:
    {
      enabled = value != null;
      store_path = optionalStorePath value;
      gcroot_path =
        if value == null then "" else "/var/lib/fishystuff/gitops/gcroots/${releaseKey}/${gcrootName}";
    };
  mkRelease =
    release:
    {
      generation = release.generation;
      git_rev = release.gitRev;
      dolt_commit = release.doltCommit;
      closures = {
        api = closure release.releaseId "api" (release.apiClosure or null);
        site = closure release.releaseId "site" (release.siteClosure or null);
        cdn_runtime = closure release.releaseId "cdn-runtime" (release.cdnRuntimeClosure or null);
        dolt_service = closure release.releaseId "dolt-service" (release.doltServiceClosure or null);
      };
      dolt = {
        repository = release.doltRepository or doltRepository;
        commit = release.doltCommit;
        branch_context = release.doltBranchContext or doltBranchContext;
        mode = release.doltMode or doltMode;
        materialization = release.doltMaterialization or doltMaterialization;
        remote_url = release.doltRemoteUrl or doltRemoteUrl;
        cache_dir = release.doltCacheDir or doltCacheDir;
        release_ref = release.doltReleaseRef or doltReleaseRef;
      };
    };
  retainedReleaseIds =
    if retainedReleases != [ ] then retainedReleases else map (release: release.releaseId) retainedReleaseObjects;
  retainedReleaseAttrs =
    map (release: {
      name = release.releaseId;
      value = mkRelease release;
    }) retainedReleaseObjects;
  activeReleaseAttr = {
    name = releaseId;
    value = mkRelease {
      inherit
        releaseId
        doltCommit
        doltCacheDir
        doltMaterialization
        doltRepository
        doltBranchContext
        doltMode
        doltReleaseRef
        doltRemoteUrl
        gitRev
        apiClosure
        siteClosure
        cdnRuntimeClosure
        doltServiceClosure
        ;
      generation = releaseGeneration;
    };
  };
  environmentPayload = {
    enabled = true;
    strategy = "single_active";
    host = hostKey;
    active_release = releaseId;
    retained_releases = retainedReleaseIds;
    inherit serve;
  } // lib.optionalAttrs (apiUpstream != "") {
    api_upstream = apiUpstream;
  } // lib.optionalAttrs (apiService != "") {
    api_service = apiService;
  } // lib.optionalAttrs (admissionProbe != null) {
    admission_probe = admissionProbe;
  } // lib.optionalAttrs (transition != null) {
    inherit transition;
  };
  payload = {
    inherit cluster generation mode;
    hosts.${hostKey} = {
      enabled = true;
      role = hostRole;
      hostname = hostName;
    };
    releases = builtins.listToAttrs ([ activeReleaseAttr ] ++ retainedReleaseAttrs);
    environments.${environment} = environmentPayload;
  };
in
assert lib.assertMsg (cluster != "") "gitops desired state requires cluster";
assert lib.assertMsg (environment != "") "gitops desired state requires environment";
assert lib.assertMsg (
  apiUpstream == "" || !lib.hasSuffix "/" apiUpstream
) "gitops apiUpstream must not end with /";
assert lib.assertMsg (
  apiUpstream == "" || !remoteUrlContainsUserinfo apiUpstream
) "gitops apiUpstream must not contain embedded credentials";
assert lib.assertMsg (
  apiService == "" || apiUpstream != ""
) "gitops apiService requires apiUpstream";
assert lib.assertMsg (
  apiService == "" || !(lib.hasSuffix ".service" apiService)
) "gitops apiService expects a mgmt svc name, not a systemd unit filename";
assert lib.assertMsg (activeRelease == null || activeRelease != "") "gitops desired state activeRelease override must not be empty";
assert lib.assertMsg (generation > 0) "gitops desired state requires positive generation";
assert lib.assertMsg (releaseGeneration > 0) "gitops desired state requires positive releaseGeneration";
assert lib.assertMsg (gitRev != "") "gitops desired state requires gitRev";
assert lib.assertMsg (doltCommit != "") "gitops desired state requires doltCommit";
assert lib.assertMsg (doltBranchContext != "") "gitops desired state requires doltBranchContext";
assert lib.assertMsg (doltMode == "read_only") "gitops desired state requires doltMode = read_only";
assert lib.assertMsg (
  doltMaterialization == "metadata_only"
  || doltMaterialization == "fetch_pin"
  || doltMaterialization == "replica_pin"
  || doltMaterialization == "snapshot"
) "gitops desired state has unsupported doltMaterialization";
assert lib.assertMsg (
  doltMaterialization != "fetch_pin" || doltRemoteUrl != ""
) "fetch_pin dolt materialization requires doltRemoteUrl";
assert lib.assertMsg (
  !remoteUrlContainsUserinfo doltRemoteUrl
) "gitops desired state doltRemoteUrl must not contain embedded credentials";
assert lib.assertMsg (
  mode == "validate" || doltMaterialization != "replica_pin"
) "replica_pin dolt materialization is validate-only until implemented";
assert lib.assertMsg (
  mode == "validate" || doltMaterialization != "snapshot"
) "snapshot dolt materialization is validate-only until implemented";
assert lib.assertMsg (
  admissionProbe == null || builtins.isAttrs admissionProbe
) "gitops admissionProbe must be an attribute set";
assert lib.assertMsg (
  admissionProbe == null || admissionProbe ? kind
) "gitops admissionProbe requires kind";
assert lib.assertMsg (
  admissionProbe == null || lib.elem admissionProbeKind [
    "dolt_sql_scalar"
    "http_status"
    "http_json_scalar"
  ]
) "gitops admissionProbe kind must be dolt_sql_scalar, http_status, or http_json_scalar";
assert lib.assertMsg (
  admissionProbe == null
  || (
    admissionProbeIsDoltSql
    && (mode == "vm-test" || mode == "vm-test-closures")
  )
  || (
    admissionProbeIsHttp
    && (mode == "vm-test" || mode == "vm-test-closures" || mode == "local-apply")
  )
) "gitops admissionProbe mode is unsupported for this probe kind";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsDoltSql || doltMaterialization == "fetch_pin"
) "gitops Dolt SQL admissionProbe requires fetch_pin Dolt materialization";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsDoltSql || (admissionProbe.query or "") != ""
) "gitops Dolt SQL admissionProbe requires query";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsDoltSql || admissionProbe ? expected_scalar
) "gitops Dolt SQL admissionProbe requires expected_scalar";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttp || (admissionProbe.probe_name or "") != ""
) "gitops HTTP admissionProbe requires probe_name";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttp || (admissionProbe.url or "") != ""
) "gitops HTTP admissionProbe requires url";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttp || !serve || apiUpstream != ""
) "gitops serving HTTP admissionProbe requires apiUpstream";
assert lib.assertMsg (
  admissionProbe == null
  || !admissionProbeIsHttp
  || apiUpstream == ""
  || (admissionProbe.url or "") == apiUpstream
  || lib.hasPrefix "${apiUpstream}/" (admissionProbe.url or "")
  || lib.hasPrefix "${apiUpstream}?" (admissionProbe.url or "")
) "gitops HTTP admissionProbe url must target apiUpstream when apiUpstream is set";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttp || (admissionProbeExpectedStatus >= 100 && admissionProbeExpectedStatus <= 599)
) "gitops HTTP admissionProbe expected_status must be between 100 and 599";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttp || admissionProbeTimeoutMs == 0 || (admissionProbeTimeoutMs >= 1 && admissionProbeTimeoutMs <= 30000)
) "gitops HTTP admissionProbe timeout_ms must be between 1 and 30000";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttpJsonScalar || admissionProbe ? expected_scalar
) "gitops HTTP JSON scalar admissionProbe requires expected_scalar";
assert lib.assertMsg (
  admissionProbe == null || !admissionProbeIsHttpJsonScalar || admissionProbe ? json_pointer
) "gitops HTTP JSON scalar admissionProbe requires json_pointer";
assert lib.assertMsg (
  lib.all (release: release ? releaseId && release.releaseId != "") retainedReleaseObjects
) "gitops retained release objects require releaseId";
assert lib.assertMsg (
  lib.all (release: release ? generation && release.generation > 0) retainedReleaseObjects
) "gitops retained release objects require positive generation";
assert lib.assertMsg (
  lib.all (release: release ? gitRev && release.gitRev != "") retainedReleaseObjects
) "gitops retained release objects require gitRev";
assert lib.assertMsg (
  lib.all (release: release ? doltCommit && release.doltCommit != "") retainedReleaseObjects
) "gitops retained release objects require doltCommit";
assert lib.assertMsg (
  lib.all (release: (release.doltMode or doltMode) == "read_only") retainedReleaseObjects
) "gitops retained release objects require doltMode = read_only";
assert lib.assertMsg (
  lib.all (release: mode == "validate" || (release.doltMaterialization or doltMaterialization) != "replica_pin") retainedReleaseObjects
) "gitops retained release objects cannot use replica_pin outside validate mode";
assert lib.assertMsg (
  lib.all (release: mode == "validate" || (release.doltMaterialization or doltMaterialization) != "snapshot") retainedReleaseObjects
) "gitops retained release objects cannot use snapshot outside validate mode";
assert lib.assertMsg (
  lib.all (release: !remoteUrlContainsUserinfo (release.doltRemoteUrl or doltRemoteUrl)) retainedReleaseObjects
) "gitops retained release objects must not use credential-bearing Dolt remote URLs";
assert lib.assertMsg (
  retainedReleases == [ ] || retainedReleases == map (release: release.releaseId) retainedReleaseObjects
) "gitops retainedReleases must match retainedReleaseObjects when both are provided";
assert lib.assertMsg (
  !(lib.elem releaseId retainedReleaseIds)
) "gitops retained rollback releases must not include the active release";
assert lib.assertMsg (
  retainedReleaseIds == lib.unique retainedReleaseIds
) "gitops retained rollback releases must be unique";
assert lib.assertMsg (
  transition == null || builtins.isAttrs transition
) "gitops transition must be an attribute set";
assert lib.assertMsg (
  transition == null || (transition ? kind && lib.isString transition.kind)
) "gitops transition requires string kind";
assert lib.assertMsg (
  transition == null || (transition ? from_release && lib.isString transition.from_release)
) "gitops transition requires string from_release";
assert lib.assertMsg (
  transition == null || (transition ? reason && lib.isString transition.reason)
) "gitops transition requires string reason";
assert lib.assertMsg (
  transition == null || lib.elem (transition.kind or "") [
    "candidate"
    "activate"
    "rollback"
  ]
) "gitops transition kind must be candidate, activate, or rollback";
assert lib.assertMsg (
  transition == null || (transition.kind or "") != "rollback" || serve
) "gitops rollback transition requires serve = true";
assert lib.assertMsg (
  transition == null || (transition.kind or "") != "rollback" || (transition.from_release or "") != ""
) "gitops rollback transition requires from_release";
assert lib.assertMsg (
  transition == null
  || (transition.kind or "") != "rollback"
  || lib.elem transition.from_release retainedReleaseIds
) "gitops rollback transition from_release must remain retained";
assert lib.assertMsg (!serve || mode != "validate") "validate-mode desired state must not request serve";
assert lib.assertMsg (!serve || retainedReleaseIds != [ ]) "serving desired state requires at least one retained rollback release";
assert lib.assertMsg (!serve || apiClosure != null) "serving desired state requires apiClosure";
assert lib.assertMsg (!serve || siteClosure != null) "serving desired state requires siteClosure";
assert lib.assertMsg (!serve || cdnRuntimeClosure != null) "serving desired state requires cdnRuntimeClosure";
assert lib.assertMsg (!serve || doltServiceClosure != null) "serving desired state requires doltServiceClosure";
writeText "fishystuff-gitops-${environment}-${releaseId}.desired.json" (
  builtins.toJSON payload + "\n"
)
