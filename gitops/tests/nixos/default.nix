{
  pkgs,
  mgmtPackage ? pkgs.mgmt,
  fishystuffDeployPackage,
  gitopsSrc,
  generatedServeFixture,
}:
{
  gitops-empty-unify = import ./empty-unify.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-single-host-candidate-vm = import ./single-host-candidate.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-dolt-fetch-pin-vm = import ./dolt-fetch-pin.nix {
    inherit fishystuffDeployPackage gitopsSrc mgmtPackage pkgs;
  };

  gitops-dolt-admission-pin-vm = import ./dolt-admission-pin.nix {
    inherit fishystuffDeployPackage gitopsSrc mgmtPackage pkgs;
  };

  gitops-served-retained-dolt-fetch-pin-vm = import ./served-retained-dolt-fetch-pin.nix {
    inherit fishystuffDeployPackage gitopsSrc mgmtPackage pkgs;
  };

  gitops-multi-environment-candidates-vm = import ./multi-environment-candidates.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-multi-environment-served-vm = import ./multi-environment-served.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-closure-roots-vm = import ./closure-roots.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-unused-release-closure-noop-vm = import ./unused-release-closure-noop.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-served-closure-roots-vm = import ./served-closure-roots.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-json-status-escaping-vm = import ./json-status-escaping.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-served-candidate-vm = import ./served-candidate.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-generated-served-candidate-vm = import ./generated-served-candidate.nix {
    inherit gitopsSrc mgmtPackage pkgs;
    inherit (generatedServeFixture)
      apiArtifact
      cdnRuntimeCurrentArtifact
      cdnRuntimeArtifact
      desiredState
      doltServiceArtifact
      previousApiArtifact
      previousCdnRuntimeArtifact
      previousCdnRuntimeCurrentArtifact
      previousDoltServiceArtifact
      previousSiteArtifact
      siteArtifact
      ;
  };

  gitops-served-symlink-transition-vm = import ./served-symlink-transition.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-served-caddy-handoff-vm = import ./served-caddy-handoff.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-served-rollback-transition-vm = import ./served-rollback-transition.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-failed-candidate-vm = import ./failed-candidate.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-failed-served-candidate-refusal = import ./failed-served-candidate-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-local-apply-without-optin-refusal = import ./local-apply-without-optin-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-missing-active-artifact-refusal = import ./missing-active-artifact-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-missing-retained-artifact-refusal = import ./missing-retained-artifact-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-missing-retained-release-refusal = import ./missing-retained-release-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-no-retained-release-refusal = import ./no-retained-release-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-active-retained-release-refusal = import ./active-retained-release-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-rollback-transition-retention-refusal = import ./rollback-transition-retention-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-raw-cdn-serve-refusal = import ./raw-cdn-serve-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-missing-cdn-runtime-file-refusal = import ./missing-cdn-runtime-file-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-missing-cdn-serving-manifest-entry-refusal = import ./missing-cdn-serving-manifest-entry-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-missing-cdn-retained-root-refusal = import ./missing-cdn-retained-root-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };

  gitops-wrong-cdn-retained-root-refusal = import ./wrong-cdn-retained-root-refusal.nix {
    inherit gitopsSrc mgmtPackage pkgs;
  };
}
