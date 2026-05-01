{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
pkgs.runCommand "fishystuff-gitops-empty-unify"
  {
    nativeBuildInputs = [ mgmtPackage ];
  }
  ''
    set -euo pipefail

    export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsSrc}/fixtures/empty.desired.json
    mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${gitopsSrc}/main.mcl

    test ! -e /var/lib/fishystuff/gitops-test
    test ! -e /run/fishystuff/gitops-test

    touch "$out"
  ''
