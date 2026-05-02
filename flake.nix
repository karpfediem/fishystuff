{
  description = "FishyStuff - Fishing Guides and Tools for Black Desert";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    nix2container.url = "github:nlewo/nix2container";
    nix2container.inputs.nixpkgs.follows = "nixpkgs";
    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin";
    mgmt-fishystuff-beta.url = "git+file:///home/carp/code/mgmt-fishystuff-beta?rev=8ff41165c88368b84828ea2e37c24414be3f9532";
    mgmt-fishystuff-beta.inputs.flake-parts.follows = "flake-parts";
    mgmt-fishystuff-beta.inputs.nixpkgs.follows = "nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs = { nixpkgs.follows = "nixpkgs"; };

    waypoints.url = "github:flockenberger/bdo-fish-waypoints";
    waypoints.flake = false;
  };

  outputs = inputs@{ self, flake-parts, crane, mgmt-fishystuff-beta, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } ({ withSystem, ... }: {
      systems = [ "x86_64-linux" ];

      perSystem = { config, pkgs, system, waypoints, ... }:
        let
          operatorRepoRoot =
            let
              root = builtins.getEnv "FISHYSTUFF_OPERATOR_ROOT";
            in
            if root != "" then
              root
            else
              throw "FISHYSTUFF_OPERATOR_ROOT must be set for operator-local CDN data packages";
          filteredWaypointsSrc = pkgs.lib.cleanSourceWith {
            name = "waypoints-no-webp";
            src = inputs.waypoints;
            filter = path: type:
              let lower = pkgs.lib.toLower path; in
                !(pkgs.lib.hasSuffix ".webp" lower);
          };
          craneLib = crane.mkLib pkgs;
          apiWorkspaceCargoToml = pkgs.callPackage ./nix/packages/api-workspace-cargo-toml.nix { };
          tilegenWorkspaceCargoToml = pkgs.callPackage ./nix/packages/tilegen-workspace-cargo-toml.nix { };
          apiWorkspaceSrc = pkgs.callPackage ./nix/packages/api-workspace-src.nix {
            inherit apiWorkspaceCargoToml;
            apiWorkspaceCargoLock = ./nix/locks/api/Cargo.lock;
          };
          apiCargoSrc = pkgs.lib.cleanSourceWith {
            name = "fishystuff-api-cargo-src";
            src = apiWorkspaceSrc;
            filter = path: type:
              let
                root = "${toString apiWorkspaceSrc}/";
                rel = pkgs.lib.removePrefix root (toString path);
              in
                craneLib.filterCargoSources path type
                || rel == "site"
                || rel == "site/i18n"
                || pkgs.lib.hasPrefix "site/i18n/" rel;
          };
          siteSrc = pkgs.callPackage ./nix/packages/site-src.nix { };
          siteMapRuntimeCacheKey = import ./nix/packages/site-map-runtime-cache-key.nix;
          tilegenWorkspaceSrc = pkgs.callPackage ./nix/packages/tilegen-workspace-src.nix {
            inherit tilegenWorkspaceCargoToml;
          };
          minimapDisplayTiles = pkgs.callPackage ./nix/packages/minimap-display-tiles.nix {
            inherit craneLib tilegenWorkspaceSrc;
          };
          minimapSourceTiles = pkgs.callPackage ./nix/packages/minimap-source-tiles.nix {
            repoRoot = operatorRepoRoot;
          };
          cdnBaseContent = pkgs.callPackage ./nix/packages/cdn-base-content.nix {
            repoRoot = operatorRepoRoot;
          };
          cdnMinimapVisual = pkgs.callPackage ./nix/packages/cdn-minimap-visual.nix {
            inherit minimapDisplayTiles minimapSourceTiles;
          };
          botWaypoints = pkgs.callPackage ./nix/packages/bot-waypoints.nix {
            inherit filteredWaypointsSrc;
          };
          zineCli = pkgs.callPackage ./nix/packages/zine-prebuilt.nix { };
          botSrc = pkgs.callPackage ./nix/packages/bot-src.nix {
            inherit botWaypoints;
          };
          botCargoSrc = craneLib.cleanCargoSource botSrc;
          bot = craneLib.buildPackage { src = botCargoSrc; };
          bot-container = pkgs.dockerTools.buildLayeredImage {
            name = "crio";
            tag = "latest";
            contents = [ botWaypoints "${bot}/bin" ];
            config.Entrypoint = [ "bot" ];
            config.Env = [ "PATH=${bot}/bin" ];
          };

          api = craneLib.buildPackage {
            pname = "fishystuff_server";
            version = "0.1.0";
            src = apiCargoSrc;
            cargoExtraArgs = "-p fishystuff_server";
          };

          apiConfig = pkgs.callPackage ./nix/packages/api-config.nix { };
          apiEntrypoint = pkgs.callPackage ./nix/packages/api-entrypoint.nix {
            inherit api;
          };
          defaultDeploymentEnvironment = "beta";
          shortenFrontendSourceRevision =
            revision:
            let
              cleanRevision = pkgs.lib.removeSuffix "-dirty" revision;
              dirtySuffix = pkgs.lib.optionalString (pkgs.lib.hasSuffix "-dirty" revision) "-dirty";
            in
            if cleanRevision == "unknown" then
              "unknown"
            else
              "${builtins.substring 0 12 cleanRevision}${dirtySuffix}";
          frontendSourceRevision =
            if self ? rev then
              self.rev
            else if self ? dirtyRev then
              self.dirtyRev
            else
              "unknown";
          frontendSourceShortRevision =
            if self ? shortRev then
              self.shortRev
            else if self ? dirtyRev then
              shortenFrontendSourceRevision self.dirtyRev
            else if self ? dirtyShortRev then
              self.dirtyShortRev
            else if frontendSourceRevision == "unknown" then
              "unknown"
            else
              shortenFrontendSourceRevision frontendSourceRevision;
          frontendSourceDirty = !(self ? rev) && ((self ? dirtyRev) || (self ? dirtyShortRev));
          deploymentBaseHost =
            deploymentEnvironment:
            if deploymentEnvironment == "production" then
              "fishystuff.fish"
            else
              "${deploymentEnvironment}.fishystuff.fish";
          deploymentBaseUrl =
            subdomain: deploymentEnvironment:
            let
              baseHost = deploymentBaseHost deploymentEnvironment;
            in
            if subdomain == "" then
              "https://${baseHost}"
            else
              "https://${subdomain}.${baseHost}";
          cdnContent = pkgs.callPackage ./nix/packages/cdn-content.nix {
            inherit cdnBaseContent cdnMinimapVisual;
          };
          retainedCdnRootEnv = builtins.getEnv "FISHYSTUFF_RETAINED_CDN_ROOTS";
          retainedCdnRootStrings =
            pkgs.lib.filter (root: root != "") (pkgs.lib.splitString ":" retainedCdnRootEnv);
          retainedCdnRoots = map builtins.storePath retainedCdnRootStrings;
          operatorRootConfigured = (builtins.getEnv "FISHYSTUFF_OPERATOR_ROOT") != "";
          cdnServingRoot = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
            currentRoot = cdnContent;
            previousRoots = retainedCdnRoots;
          };
          gitopsDoltCommitEnv = builtins.getEnv "FISHYSTUFF_GITOPS_DOLT_COMMIT";
          gitopsDoltCommit =
            if gitopsDoltCommitEnv != "" then gitopsDoltCommitEnv else "validation-placeholder";
          siteContentFor =
            deploymentEnvironment: mapAssetCacheKey:
            pkgs.callPackage ./nix/packages/site-content.nix {
              inherit
                deploymentEnvironment
                frontendSourceDirty
                frontendSourceRevision
                frontendSourceShortRevision
                siteSrc
                ;
              frontendSourceRef = "";
              zine = zineCli;
              inherit mapAssetCacheKey;
              publicSiteBaseUrl = deploymentBaseUrl "" deploymentEnvironment;
              publicApiBaseUrl = deploymentBaseUrl "api" deploymentEnvironment;
              publicCdnBaseUrl = deploymentBaseUrl "cdn" deploymentEnvironment;
              publicTelemetryBaseUrl = deploymentBaseUrl "telemetry" deploymentEnvironment;
            };
          siteContent = siteContentFor "production" siteMapRuntimeCacheKey;
          siteContentBeta = siteContentFor defaultDeploymentEnvironment siteMapRuntimeCacheKey;
          siteContentStableMapRuntime = siteContentFor "production" "";
          siteContentBetaStableMapRuntime = siteContentFor defaultDeploymentEnvironment "";
          apiServiceBaseConfig = pkgs.callPackage ./nix/packages/api-service-base-config.nix { };
          serviceModules = import ./nix/services {
            inherit pkgs;
            lib = pkgs.lib;
          };
          evalService = pkgs.callPackage ./nix/services/eval-service.nix { };
          mkServiceBundle = pkgs.callPackage ./nix/services/mk-service-bundle.nix {
            inherit evalService;
          };
          apiServiceBundle = mkServiceBundle {
            name = "fishystuff-api";
            serviceModule = serviceModules.api;
            configuration.fishystuff.api = {
              package = api;
              baseConfigSource = apiServiceBaseConfig;
              requestTimeoutSecs = 90;
              runtimeEnvFile = "/run/fishystuff/api/env";
              environment.FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = defaultDeploymentEnvironment;
              environment.FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT = defaultDeploymentEnvironment;
            };
          };
          doltServiceBundle = mkServiceBundle {
            name = "fishystuff-dolt";
            serviceModule = serviceModules.dolt;
            configuration.fishystuff.dolt = {
              dynamicUser = false;
              runtimeEnvFile = "/run/fishystuff/api/env";
              environment.FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = defaultDeploymentEnvironment;
            };
          };
          gitopsDesiredStateBetaValidate = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "beta";
            environment = "beta";
            hostKey = "beta-single-host";
            generation = 1;
            releaseGeneration = 1;
            gitRev = frontendSourceRevision;
            doltCommit = gitopsDoltCommit;
            doltBranchContext = "beta";
            apiClosure = apiServiceBundle;
            siteClosure = siteContentBeta;
            cdnRuntimeClosure = if operatorRootConfigured then cdnServingRoot else null;
            doltServiceClosure = doltServiceBundle;
            mode = "validate";
            serve = false;
          };
          gitopsDesiredStateServeFixtureApi = pkgs.writeText "gitops-desired-state-serve-api-fixture" "api fixture\n";
          gitopsDesiredStateServeFixtureDoltService =
            pkgs.writeText "gitops-desired-state-serve-dolt-service-fixture" "dolt service fixture\n";
          gitopsDesiredStateServeFixtureSite = pkgs.runCommand "gitops-desired-state-serve-site-fixture" { } ''
            mkdir -p "$out"
            printf 'served fixture site\n' > "$out/index.html"
          '';
          gitopsDesiredStateServeFixtureCdnCurrent =
            pkgs.runCommand "gitops-desired-state-serve-cdn-current-fixture" { } ''
              mkdir -p "$out/map"
              printf '{"module":"fishystuff_ui_bevy.fixture.js","wasm":"fishystuff_ui_bevy_bg.fixture.wasm"}\n' > "$out/map/runtime-manifest.json"
              printf 'fixture module\n' > "$out/map/fishystuff_ui_bevy.fixture.js"
              printf 'fixture wasm\n' > "$out/map/fishystuff_ui_bevy_bg.fixture.wasm"
            '';
          gitopsDesiredStateServeFixtureCdn = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
            currentRoot = gitopsDesiredStateServeFixtureCdnCurrent;
          };
          gitopsDesiredStateVmServeFixture = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "local-test";
            environment = "local-test";
            hostKey = "vm-single-host";
            generation = 7;
            releaseGeneration = 7;
            gitRev = "serve-fixture";
            doltCommit = "serve-fixture";
            doltBranchContext = "local-test";
            apiClosure = gitopsDesiredStateServeFixtureApi;
            siteClosure = gitopsDesiredStateServeFixtureSite;
            cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
            doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
            mode = "vm-test";
            serve = true;
          };
          edgeServiceBundleFor =
            deploymentEnvironment:
            mkServiceBundle {
              name = "fishystuff-edge";
              serviceModule = serviceModules.edge;
              configuration.fishystuff.edge = {
                tlsEnable = true;
                siteAddress = deploymentBaseUrl "" deploymentEnvironment;
                apiAddress = deploymentBaseUrl "api" deploymentEnvironment;
                cdnAddress = deploymentBaseUrl "cdn" deploymentEnvironment;
                telemetryAddress = deploymentBaseUrl "telemetry" deploymentEnvironment;
              };
            };
          edgeServiceBundle = edgeServiceBundleFor defaultDeploymentEnvironment;
          edgeServiceBundleProduction = edgeServiceBundleFor "production";
          lokiServiceBundle = mkServiceBundle {
            name = "fishystuff-loki";
            serviceModule = serviceModules.loki;
          };
          otelCollectorServiceBundle = mkServiceBundle {
            name = "fishystuff-otel-collector";
            serviceModule = serviceModules."otel-collector";
          };
          vectorServiceBundle = mkServiceBundle {
            name = "fishystuff-vector";
            serviceModule = serviceModules.vector;
          };
          vectorAgentServiceBundleFor =
            deploymentEnvironment:
            mkServiceBundle {
              name = "fishystuff-vector-agent";
              serviceModule = serviceModules.vector;
              configuration.fishystuff.vector = {
                role = "agent";
                deploymentEnvironment = deploymentEnvironment;
                vectorSinkAddress = "10.0.0.4:6000";
                lokiAddress = "10.0.0.4";
                otelCollectorAddress = "10.0.0.4";
              };
            };
          vectorAgentServiceBundle = vectorAgentServiceBundleFor defaultDeploymentEnvironment;
          vectorAgentServiceBundleProduction = vectorAgentServiceBundleFor "production";
          prometheusServiceBundle = mkServiceBundle {
            name = "fishystuff-prometheus";
            serviceModule = serviceModules.prometheus;
          };
          jaegerServiceBundle = mkServiceBundle {
            name = "fishystuff-jaeger";
            serviceModule = serviceModules.jaeger;
          };
          grafanaServiceBundle = mkServiceBundle {
            name = "fishystuff-grafana";
            serviceModule = serviceModules.grafana;
          };
          serviceBundleChecks = import ./nix/tests/service-bundle-checks.nix {
            inherit
              apiServiceBundle
              doltServiceBundle
              edgeServiceBundle
              pkgs
              ;
          };
          modularServiceRuntime = pkgs.callPackage ./nix/tests/modular-service-runtime.nix {
            inherit serviceModules;
          };
          gitopsTests = import ./gitops/tests/nixos {
            inherit pkgs;
            gitopsSrc = ./gitops;
            mgmtPackage = mgmt-fishystuff-beta.packages.${system}.minimal;
            generatedServeFixture = {
              desiredState = gitopsDesiredStateVmServeFixture;
              apiArtifact = gitopsDesiredStateServeFixtureApi;
              siteArtifact = gitopsDesiredStateServeFixtureSite;
              cdnRuntimeArtifact = gitopsDesiredStateServeFixtureCdn;
              cdnRuntimeCurrentArtifact = gitopsDesiredStateServeFixtureCdnCurrent;
              doltServiceArtifact = gitopsDesiredStateServeFixtureDoltService;
            };
          };
          cdnServingRootRetentionCheck =
            let
              currentFixture = pkgs.runCommand "cdn-serving-current-fixture" { } ''
                mkdir -p "$out/map"
                printf 'current-manifest' > "$out/map/runtime-manifest.json"
                printf 'current-metadata' > "$out/.cdn-metadata.json"
                printf 'new-runtime' > "$out/map/fishystuff_ui_bevy.new.js"
                printf 'new-source-map' > "$out/map/fishystuff_ui_bevy.new.js.map"
              '';
              previousFixture = pkgs.runCommand "cdn-serving-previous-fixture" { } ''
                mkdir -p "$out/map"
                printf 'previous-manifest' > "$out/map/runtime-manifest.json"
                printf 'previous-metadata' > "$out/.cdn-metadata.json"
                printf 'old-runtime' > "$out/map/fishystuff_ui_bevy.old.js"
                printf 'old-source-map' > "$out/map/fishystuff_ui_bevy.old.js.map"
              '';
              servingRoot = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
                currentRoot = currentFixture;
                previousRoots = [ previousFixture ];
              };
            in
            pkgs.runCommand "cdn-serving-root-retention-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
              set -euo pipefail

              test "$(cat ${servingRoot}/map/runtime-manifest.json)" = "current-manifest"
              test "$(cat ${servingRoot}/.cdn-metadata.json)" = "current-metadata"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.new.js)" = "new-runtime"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.new.js.map)" = "new-source-map"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.old.js)" = "old-runtime"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.old.js.map)" = "old-source-map"

              test "$(jq -r '.retained_root_count' ${servingRoot}/cdn-serving-manifest.json)" = "1"
              test "$(jq -r '[.assets[] | select(.source == "retained")] | length' ${servingRoot}/cdn-serving-manifest.json)" = "2"
              touch "$out"
            '';
          siteAssetFinalizerCheck = pkgs.runCommand "site-asset-finalizer-check" {
            nativeBuildInputs = [
              pkgs.bun
              pkgs.esbuild
              pkgs.lightningcss
              pkgs.nodejs
              pkgs.writableTmpDirAsHomeHook
            ];
          } ''
            set -euo pipefail

            cp -R ${siteSrc}/. .
            chmod -R u+w .
            bun test scripts/write-runtime-config.test.mjs scripts/finalize-assets.test.mjs
            touch "$out"
          '';
          cdnRequiredFilesCheck = pkgs.runCommand "cdn-required-files-check" {
            nativeBuildInputs = [ pkgs.python3 ];
          } ''
            set -euo pipefail

            cp -R ${./tools/scripts}/. .
            python3 compute_required_cdn_filenames_test.py
            touch "$out"
          '';
          gitopsDesiredStateBetaValidateCheck = pkgs.runCommand "gitops-desired-state-beta-validate-check" {
            nativeBuildInputs = [
              mgmt-fishystuff-beta.packages.${system}.minimal
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            release_id="$(jq -r '.environments.beta.active_release' ${gitopsDesiredStateBetaValidate})"
            test "$release_id" != example-release
            test "$release_id" != beta-validation-release
            jq -e --arg release_id "$release_id" '.releases[$release_id].generation == 1' ${gitopsDesiredStateBetaValidate}

            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateBetaValidate}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateVmServeFixtureCheck = pkgs.runCommand "gitops-desired-state-vm-serve-fixture-check" {
            nativeBuildInputs = [
              mgmt-fishystuff-beta.packages.${system}.minimal
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            release_id="$(jq -r '.environments."local-test".active_release' ${gitopsDesiredStateVmServeFixture})"
            test "$release_id" != example-release
            jq -e --arg release_id "$release_id" '
              .mode == "vm-test"
              and .environments."local-test".serve == true
              and .releases[$release_id].generation == 7
              and ([.releases[$release_id].closures[] | .enabled] | all)
            ' ${gitopsDesiredStateVmServeFixture}

            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateVmServeFixture}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';

          api-container = pkgs.dockerTools.buildLayeredImage {
            name = "api-fishystuff-fish";
            tag = "latest";
            contents = [
              apiEntrypoint
              apiConfig
              pkgs.cacert
              pkgs.dockerTools.fakeNss
            ];
            config.Entrypoint = [ "${apiEntrypoint}/bin/fishystuff-api-entrypoint" ];
            config.Env = [
              "API_CONFIG_PATH=${apiConfig}/etc/fishystuff/config.toml"
              "FISHYSTUFF_SECRETSPEC_PATH=${apiConfig}/etc/fishystuff/secretspec.toml"
              "NIX_SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ];
          };
        in
        {
          packages = {
            inherit api api-container bot bot-container;
            default = api;
            api-service-base-config = apiServiceBaseConfig;
            api-service-bundle = apiServiceBundle;
            cdn-base-content = cdnBaseContent;
            cdn-content = cdnContent;
            cdn-serving-root = cdnServingRoot;
            dolt-service-bundle = doltServiceBundle;
            edge-service-bundle = edgeServiceBundle;
            edge-service-bundle-production = edgeServiceBundleProduction;
            gitops-desired-state-beta-validate = gitopsDesiredStateBetaValidate;
            gitops-desired-state-vm-serve-fixture = gitopsDesiredStateVmServeFixture;
            grafana-service-bundle = grafanaServiceBundle;
            jaeger-service-bundle = jaegerServiceBundle;
            loki-service-bundle = lokiServiceBundle;
            otel-collector-service-bundle = otelCollectorServiceBundle;
            minimap-display-tiles = minimapDisplayTiles;
            minimap-source-tiles = minimapSourceTiles;
            prometheus-service-bundle = prometheusServiceBundle;
            site-content = siteContent;
            site-content-beta = siteContentBeta;
            site-content-beta-stable-map-runtime = siteContentBetaStableMapRuntime;
            site-content-stable-map-runtime = siteContentStableMapRuntime;
            vector-agent-service-bundle = vectorAgentServiceBundle;
            vector-agent-service-bundle-production = vectorAgentServiceBundleProduction;
            vector-service-bundle = vectorServiceBundle;
            zine = zineCli;
          };
          checks =
            serviceBundleChecks
            // gitopsTests
            // {
              cdn-serving-root-retention = cdnServingRootRetentionCheck;
              cdn-required-files = cdnRequiredFilesCheck;
              gitops-desired-state-beta-validate = gitopsDesiredStateBetaValidateCheck;
              gitops-desired-state-vm-serve-fixture = gitopsDesiredStateVmServeFixtureCheck;
              modular-service-runtime = modularServiceRuntime;
              site-asset-finalizer = siteAssetFinalizerCheck;
            };
        };
      flake = {
        lib = {
          services = { pkgs }: import ./nix/services {
            inherit pkgs;
            lib = pkgs.lib;
          };
          evalService = { pkgs }: pkgs.callPackage ./nix/services/eval-service.nix { };
          mkServiceBundle =
            { pkgs }:
            pkgs.callPackage ./nix/services/mk-service-bundle.nix {
              evalService = pkgs.callPackage ./nix/services/eval-service.nix { };
            };
        };
      };
    });
}
