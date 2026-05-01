{
  pkgs,
  mgmtPackage ? pkgs.mgmt,
  gitopsSrc,
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
}
