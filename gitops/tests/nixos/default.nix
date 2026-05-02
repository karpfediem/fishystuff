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
      siteArtifact
      ;
  };
}
