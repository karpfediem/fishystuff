{
  pkgs,
  mgmtPackage ? pkgs.mgmt,
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

  gitops-closure-roots-vm = import ./closure-roots.nix {
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

  gitops-missing-retained-release-refusal = import ./missing-retained-release-refusal.nix {
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
}
