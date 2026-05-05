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
          deploymentToolsCargoSrc = craneLib.cleanCargoSource ./.;
          fishystuffDeployCargoArtifacts = craneLib.buildDepsOnly {
            pname = "fishystuff_deploy";
            version = "0.1.0";
            src = deploymentToolsCargoSrc;
            cargoExtraArgs = "-p fishystuff_deploy";
          };
          fishystuffDeploy = craneLib.buildPackage {
            pname = "fishystuff_deploy";
            version = "0.1.0";
            src = deploymentToolsCargoSrc;
            cargoArtifacts = fishystuffDeployCargoArtifacts;
            cargoExtraArgs = "-p fishystuff_deploy";
          };
          fishystuffDeployTests = craneLib.cargoTest {
            pname = "fishystuff_deploy-tests";
            version = "0.1.0";
            src = deploymentToolsCargoSrc;
            cargoArtifacts = fishystuffDeployCargoArtifacts;
            cargoExtraArgs = "-p fishystuff_deploy";
            nativeBuildInputs = [ pkgs.dolt ];
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
          cdnServingRoot = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
            currentRoot = cdnContent;
          };
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
          apiServiceBundleFor =
            deploymentEnvironment:
            mkServiceBundle {
              name = "fishystuff-api";
              serviceModule = serviceModules.api;
              configuration.fishystuff.api = {
                package = api;
                baseConfigSource = apiServiceBaseConfig;
                requestTimeoutSecs = 90;
                runtimeEnvFile = "/run/fishystuff/api/env";
                environment.FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = deploymentEnvironment;
                environment.FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT = deploymentEnvironment;
              };
            };
          apiServiceBundle = apiServiceBundleFor defaultDeploymentEnvironment;
          apiServiceBundleProduction = apiServiceBundleFor "production";
          doltServiceBundleFor =
            deploymentEnvironment:
            mkServiceBundle {
              name = "fishystuff-dolt";
              serviceModule = serviceModules.dolt;
              configuration.fishystuff.dolt = {
                dynamicUser = false;
                runtimeEnvFile = "/run/fishystuff/api/env";
                environment.FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = deploymentEnvironment;
              };
            };
          doltServiceBundle = doltServiceBundleFor defaultDeploymentEnvironment;
          doltServiceBundleProduction = doltServiceBundleFor "production";
          gitopsDesiredStateBetaValidate = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "beta";
            environment = "beta";
            hostKey = "beta-single-host";
            generation = 1;
            releaseGeneration = 1;
            gitRev = frontendSourceRevision;
            doltCommit = "validation-placeholder";
            doltBranchContext = "beta";
            apiClosure = apiServiceBundle;
            siteClosure = siteContentBeta;
            cdnRuntimeClosure = null;
            doltServiceClosure = doltServiceBundle;
            mode = "validate";
            serve = false;
          };
          gitopsDesiredStateProductionValidate = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "production";
            environment = "production";
            hostKey = "production-single-host";
            generation = 1;
            releaseGeneration = 1;
            gitRev = frontendSourceRevision;
            doltCommit = "validation-placeholder";
            doltBranchContext = "main";
            apiClosure = apiServiceBundleProduction;
            siteClosure = siteContent;
            cdnRuntimeClosure = null;
            doltServiceClosure = doltServiceBundleProduction;
            mode = "validate";
            serve = false;
          };
          gitopsDesiredStateServeFixtureApi = pkgs.writeText "gitops-desired-state-serve-api-fixture" "api fixture\n";
          gitopsDesiredStateServeFixtureDoltService =
            pkgs.writeText "gitops-desired-state-serve-dolt-service-fixture" "dolt service fixture\n";
          gitopsDesiredStateServeFixturePreviousApi =
            pkgs.writeText "gitops-desired-state-serve-previous-api-fixture" "previous api fixture\n";
          gitopsDesiredStateServeFixturePreviousDoltService =
            pkgs.writeText "gitops-desired-state-serve-previous-dolt-service-fixture" "previous dolt service fixture\n";
          gitopsDesiredStateServeFixtureSite = pkgs.runCommand "gitops-desired-state-serve-site-fixture" { } ''
            mkdir -p "$out"
            printf 'served fixture site\n' > "$out/index.html"
          '';
          gitopsDesiredStateServeFixturePreviousSite =
            pkgs.runCommand "gitops-desired-state-serve-previous-site-fixture" { } ''
              mkdir -p "$out"
              printf 'previous served fixture site\n' > "$out/index.html"
            '';
          gitopsDesiredStateServeFixtureCdnCurrent =
            pkgs.runCommand "gitops-desired-state-serve-cdn-current-fixture" { } ''
              mkdir -p "$out/map"
              printf '{"module":"fishystuff_ui_bevy.fixture.js","wasm":"fishystuff_ui_bevy_bg.fixture.wasm"}\n' > "$out/map/runtime-manifest.json"
              printf 'fixture module\n' > "$out/map/fishystuff_ui_bevy.fixture.js"
              printf 'fixture wasm\n' > "$out/map/fishystuff_ui_bevy_bg.fixture.wasm"
            '';
          gitopsDesiredStateServeFixturePreviousCdnCurrent =
            pkgs.runCommand "gitops-desired-state-serve-previous-cdn-current-fixture" { } ''
              mkdir -p "$out/map"
              printf '{"module":"fishystuff_ui_bevy.previous-fixture.js","wasm":"fishystuff_ui_bevy_bg.previous-fixture.wasm"}\n' > "$out/map/runtime-manifest.json"
              printf 'previous fixture module\n' > "$out/map/fishystuff_ui_bevy.previous-fixture.js"
              printf 'previous fixture wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous-fixture.wasm"
            '';
          gitopsDesiredStateServeFixturePreviousCdn = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
            currentRoot = gitopsDesiredStateServeFixturePreviousCdnCurrent;
          };
          gitopsDesiredStateServeFixtureCdn = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
            currentRoot = gitopsDesiredStateServeFixtureCdnCurrent;
            previousRoots = [ gitopsDesiredStateServeFixturePreviousCdnCurrent ];
          };
          gitopsDesiredStateServeFixtureRollbackCdn = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
            currentRoot = gitopsDesiredStateServeFixturePreviousCdnCurrent;
            previousRoots = [ gitopsDesiredStateServeFixtureCdnCurrent ];
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
            retainedReleaseObjects = [
              {
                releaseId = "previous-release";
                generation = 6;
                gitRev = "previous-serve-fixture";
                doltCommit = "previous-serve-fixture";
                doltBranchContext = "local-test";
                apiClosure = gitopsDesiredStateServeFixturePreviousApi;
                siteClosure = gitopsDesiredStateServeFixturePreviousSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixturePreviousCdn;
                doltServiceClosure = gitopsDesiredStateServeFixturePreviousDoltService;
              }
            ];
            mode = "vm-test";
            serve = true;
          };
          gitopsDesiredStateRollbackTransitionFixture = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "local-test";
            environment = "local-test";
            hostKey = "vm-single-host";
            activeRelease = "previous-release";
            generation = 10;
            releaseGeneration = 10;
            gitRev = "previous-serve-fixture";
            doltCommit = "previous-serve-fixture";
            doltBranchContext = "local-test";
            apiClosure = gitopsDesiredStateServeFixturePreviousApi;
            siteClosure = gitopsDesiredStateServeFixturePreviousSite;
            cdnRuntimeClosure = gitopsDesiredStateServeFixtureRollbackCdn;
            doltServiceClosure = gitopsDesiredStateServeFixturePreviousDoltService;
            retainedReleaseObjects = [
              {
                releaseId = "candidate-release";
                generation = 9;
                gitRev = "serve-fixture";
                doltCommit = "serve-fixture";
                doltBranchContext = "local-test";
                apiClosure = gitopsDesiredStateServeFixtureApi;
                siteClosure = gitopsDesiredStateServeFixtureSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
                doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
              }
            ];
            transition = {
              kind = "rollback";
              from_release = "candidate-release";
              reason = "generated rollback fixture";
            };
            mode = "vm-test";
            serve = true;
          };
          gitopsDesiredStateAdmissionProbeFixture = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "local-test";
            environment = "local-test";
            hostKey = "vm-single-host";
            generation = 9;
            releaseGeneration = 9;
            gitRev = "admission-probe-fixture";
            doltCommit = "admission-probe-fixture";
            doltBranchContext = "main";
            doltMaterialization = "fetch_pin";
            doltRemoteUrl = "file:///tmp/fishystuff-gitops-admission-probe-remote";
            admissionProbe = {
              kind = "dolt_sql_scalar";
              query = "select 'ok'";
              expected_scalar = "ok";
            };
            mode = "vm-test";
            serve = false;
          };
          gitopsDesiredStateHttpAdmissionProbeFixture = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "local-test";
            environment = "local-test";
            hostKey = "vm-single-host";
            generation = 11;
            releaseGeneration = 11;
            gitRev = "http-admission-probe-fixture";
            doltCommit = "http-admission-probe-fixture";
            doltBranchContext = "local-test";
            apiService = "fishystuff-gitops-candidate-api-local-test";
            apiUpstream = "http://127.0.0.1:18082";
            admissionProbe = {
              kind = "api_meta";
              probe_name = "api-meta";
              url = "http://127.0.0.1:18082/api/v1/meta";
              expected_status = 200;
              timeout_ms = 2000;
            };
            mode = "local-apply";
            serve = false;
          };
          gitopsDesiredStateLocalApplyRollbackFixture = pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
            cluster = "local-test";
            environment = "local-test";
            hostKey = "vm-single-host";
            activeRelease = "previous-release";
            generation = 12;
            releaseGeneration = 12;
            gitRev = "previous-serve-fixture";
            doltCommit = "previous-serve-fixture";
            doltBranchContext = "local-test";
            apiClosure = gitopsDesiredStateServeFixturePreviousApi;
            siteClosure = gitopsDesiredStateServeFixturePreviousSite;
            cdnRuntimeClosure = gitopsDesiredStateServeFixtureRollbackCdn;
            doltServiceClosure = gitopsDesiredStateServeFixturePreviousDoltService;
            apiService = "fishystuff-gitops-candidate-api-local-test";
            apiUpstream = "http://127.0.0.1:18082";
            admissionProbe = {
              kind = "api_meta";
              probe_name = "api-meta";
              url = "http://127.0.0.1:18082/api/v1/meta";
              expected_status = 200;
              timeout_ms = 2000;
            };
            retainedReleaseObjects = [
              {
                releaseId = "candidate-release";
                generation = 11;
                gitRev = "serve-fixture";
                doltCommit = "serve-fixture";
                doltBranchContext = "local-test";
                apiClosure = gitopsDesiredStateServeFixtureApi;
                siteClosure = gitopsDesiredStateServeFixtureSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
                doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
              }
            ];
            transition = {
              kind = "rollback";
              from_release = "candidate-release";
              reason = "generated local apply rollback fixture";
            };
            mode = "local-apply";
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
              apiServiceBundleProduction
              doltServiceBundle
              doltServiceBundleProduction
              edgeServiceBundle
              edgeServiceBundleProduction
              pkgs
              ;
          };
          modularServiceRuntime = pkgs.callPackage ./nix/tests/modular-service-runtime.nix {
            inherit serviceModules;
          };
          mgmtGitopsPackage = mgmt-fishystuff-beta.packages.${system}.minimal.overrideAttrs (old: {
            patches = (old.patches or [ ]) ++ [
              ./nix/patches/mgmt-recwatch-bound-watch-path-index.patch
            ];
          });
          gitopsTests = import ./gitops/tests/nixos {
            inherit pkgs;
            gitopsSrc = ./gitops;
            fishystuffServerPackage = api;
            fishystuffDeployPackage = fishystuffDeploy;
            mgmtPackage = mgmtGitopsPackage;
            generatedServeFixture = {
              desiredState = gitopsDesiredStateVmServeFixture;
              apiArtifact = gitopsDesiredStateServeFixtureApi;
              siteArtifact = gitopsDesiredStateServeFixtureSite;
              cdnRuntimeArtifact = gitopsDesiredStateServeFixtureCdn;
              cdnRuntimeCurrentArtifact = gitopsDesiredStateServeFixtureCdnCurrent;
              doltServiceArtifact = gitopsDesiredStateServeFixtureDoltService;
              previousApiArtifact = gitopsDesiredStateServeFixturePreviousApi;
              previousCdnRuntimeArtifact = gitopsDesiredStateServeFixturePreviousCdn;
              previousCdnRuntimeCurrentArtifact = gitopsDesiredStateServeFixturePreviousCdnCurrent;
              previousDoltServiceArtifact = gitopsDesiredStateServeFixturePreviousDoltService;
              previousSiteArtifact = gitopsDesiredStateServeFixturePreviousSite;
            };
          };
          cdnServingRootRetentionCheck =
            let
              currentFixture = pkgs.runCommand "cdn-serving-current-fixture" { } ''
                mkdir -p "$out/map"
                printf '{"module":"fishystuff_ui_bevy.new.js","wasm":"fishystuff_ui_bevy_bg.new.wasm"}\n' > "$out/map/runtime-manifest.json"
                printf 'current-metadata' > "$out/.cdn-metadata.json"
                printf 'new-runtime' > "$out/map/fishystuff_ui_bevy.new.js"
                printf 'new-wasm' > "$out/map/fishystuff_ui_bevy_bg.new.wasm"
                printf 'new-source-map' > "$out/map/fishystuff_ui_bevy.new.js.map"
                printf 'shared-runtime' > "$out/map/fishystuff_ui_bevy.shared.js"
              '';
              previousFixture = pkgs.runCommand "cdn-serving-previous-fixture" { } ''
                mkdir -p "$out/map"
                printf 'previous-manifest' > "$out/map/runtime-manifest.json"
                printf 'previous-metadata' > "$out/.cdn-metadata.json"
                printf 'old-runtime' > "$out/map/fishystuff_ui_bevy.old.js"
                printf 'old-source-map' > "$out/map/fishystuff_ui_bevy.old.js.map"
                printf 'shared-runtime' > "$out/map/fishystuff_ui_bevy.shared.js"
              '';
              servingRoot = pkgs.callPackage ./nix/packages/cdn-serving-root.nix {
                currentRoot = currentFixture;
                previousRoots = [ previousFixture ];
              };
            in
            pkgs.runCommand "cdn-serving-root-retention-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
              set -euo pipefail

              test "$(jq -r '.module' ${servingRoot}/map/runtime-manifest.json)" = "fishystuff_ui_bevy.new.js"
              test "$(jq -r '.wasm' ${servingRoot}/map/runtime-manifest.json)" = "fishystuff_ui_bevy_bg.new.wasm"
              test "$(cat ${servingRoot}/.cdn-metadata.json)" = "current-metadata"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.new.js)" = "new-runtime"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy_bg.new.wasm)" = "new-wasm"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.new.js.map)" = "new-source-map"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.shared.js)" = "shared-runtime"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.old.js)" = "old-runtime"
              test "$(cat ${servingRoot}/map/fishystuff_ui_bevy.old.js.map)" = "old-source-map"

              test "$(jq -r '.retained_root_count' ${servingRoot}/cdn-serving-manifest.json)" = "1"
              test "$(jq -r '[.assets[] | select(.source == "retained")] | length' ${servingRoot}/cdn-serving-manifest.json)" = "2"
              test "$(jq -r '[.assets[] | select(.path == "/map/fishystuff_ui_bevy.shared.js")] | length' ${servingRoot}/cdn-serving-manifest.json)" = "1"
              test "$(jq -r '.assets[] | select(.path == "/map/fishystuff_ui_bevy.shared.js") | .source' ${servingRoot}/cdn-serving-manifest.json)" = "current"
              touch "$out"
            '';
          siteAssetFinalizerCheck = pkgs.runCommand "site-asset-finalizer-check" {
            nativeBuildInputs = [
              pkgs.bun
              pkgs.esbuild
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
              mgmtGitopsPackage
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
          gitopsDesiredStateProductionValidateCheck = pkgs.runCommand "gitops-desired-state-production-validate-check" {
            nativeBuildInputs = [
              mgmtGitopsPackage
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            release_id="$(jq -r '.environments.production.active_release' ${gitopsDesiredStateProductionValidate})"
            test "$release_id" != example-release
            test "$release_id" != production-validation-release
            jq -e --arg release_id "$release_id" '
              .cluster == "production"
              and .mode == "validate"
              and .environments.production.serve == false
              and .environments.production.host == "production-single-host"
              and .releases[$release_id].generation == 1
              and .releases[$release_id].dolt.branch_context == "main"
            ' ${gitopsDesiredStateProductionValidate}

            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateProductionValidate}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateVmServeFixtureCheck = pkgs.runCommand "gitops-desired-state-vm-serve-fixture-check" {
            nativeBuildInputs = [
              mgmtGitopsPackage
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            release_id="$(jq -r '.environments."local-test".active_release' ${gitopsDesiredStateVmServeFixture})"
            test "$release_id" != example-release
            jq -e '
              .mode == "vm-test"
              and .environments."local-test".serve == true
              and .releases[$release_id].generation == 7
              and ([.releases[$release_id].closures[] | .enabled] | all)
            ' ${gitopsDesiredStateVmServeFixture}

            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateVmServeFixture}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateRollbackTransitionCheck = pkgs.runCommand "gitops-desired-state-rollback-transition-check" {
            nativeBuildInputs = [
              mgmtGitopsPackage
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            jq -e '
              .mode == "vm-test"
              and .generation == 10
              and .environments."local-test".serve == true
              and .environments."local-test".active_release == "previous-release"
              and .environments."local-test".retained_releases == ["candidate-release"]
              and .environments."local-test".transition.kind == "rollback"
              and .environments."local-test".transition.from_release == "candidate-release"
              and .environments."local-test".transition.reason == "generated rollback fixture"
              and .releases."previous-release".generation == 10
              and .releases."candidate-release".generation == 9
              and ([.releases."previous-release".closures[] | .enabled] | all)
              and ([.releases."candidate-release".closures[] | .enabled] | all)
            ' ${gitopsDesiredStateRollbackTransitionFixture}

            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateRollbackTransitionFixture}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateAdmissionProbeCheck = pkgs.runCommand "gitops-desired-state-admission-probe-check" {
            nativeBuildInputs = [
              mgmtGitopsPackage
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            release_id="$(jq -r '.environments."local-test".active_release' ${gitopsDesiredStateAdmissionProbeFixture})"
            jq -e --arg release_id "$release_id" --arg expected_query "select 'ok'" '
              .mode == "vm-test"
              and .environments."local-test".serve == false
              and .environments."local-test".admission_probe.kind == "dolt_sql_scalar"
              and .environments."local-test".admission_probe.query == $expected_query
              and .environments."local-test".admission_probe.expected_scalar == "ok"
              and .releases[$release_id].dolt.materialization == "fetch_pin"
              and .releases[$release_id].dolt.remote_url == "file:///tmp/fishystuff-gitops-admission-probe-remote"
              and .releases[$release_id].dolt.cache_dir == ""
              and .releases[$release_id].dolt.release_ref == ""
            ' ${gitopsDesiredStateAdmissionProbeFixture}

            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateAdmissionProbeFixture}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateHttpAdmissionProbeCheck = pkgs.runCommand "gitops-desired-state-http-admission-probe-check" {
            nativeBuildInputs = [
              mgmtGitopsPackage
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            release_id="$(jq -r '.environments."local-test".active_release' ${gitopsDesiredStateHttpAdmissionProbeFixture})"
            jq -e --arg release_id "$release_id" '
              .mode == "local-apply"
              and .environments."local-test".serve == false
              and .environments."local-test".api_upstream == "http://127.0.0.1:18082"
              and .environments."local-test".api_service == "fishystuff-gitops-candidate-api-local-test"
              and .environments."local-test".admission_probe.kind == "api_meta"
              and .environments."local-test".admission_probe.probe_name == "api-meta"
              and .environments."local-test".admission_probe.url == "http://127.0.0.1:18082/api/v1/meta"
              and .environments."local-test".admission_probe.expected_status == 200
              and .environments."local-test".admission_probe.timeout_ms == 2000
              and .releases[$release_id].dolt.materialization == "metadata_only"
            ' ${gitopsDesiredStateHttpAdmissionProbeFixture}

            export FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1
            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateHttpAdmissionProbeFixture}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateLocalApplyRollbackCheck = pkgs.runCommand "gitops-desired-state-local-apply-rollback-check" {
            nativeBuildInputs = [
              mgmtGitopsPackage
              pkgs.jq
            ];
          } ''
            set -euo pipefail

            jq -e '
              .mode == "local-apply"
              and .generation == 12
              and .environments."local-test".serve == true
              and .environments."local-test".active_release == "previous-release"
              and .environments."local-test".retained_releases == ["candidate-release"]
              and .environments."local-test".api_upstream == "http://127.0.0.1:18082"
              and .environments."local-test".api_service == "fishystuff-gitops-candidate-api-local-test"
              and .environments."local-test".admission_probe.kind == "api_meta"
              and .environments."local-test".admission_probe.url == "http://127.0.0.1:18082/api/v1/meta"
              and .environments."local-test".transition.kind == "rollback"
              and .environments."local-test".transition.from_release == "candidate-release"
              and .environments."local-test".transition.reason == "generated local apply rollback fixture"
              and .releases."previous-release".generation == 12
              and .releases."candidate-release".generation == 11
              and ([.releases."previous-release".closures[] | .enabled] | all)
              and ([.releases."candidate-release".closures[] | .enabled] | all)
            ' ${gitopsDesiredStateLocalApplyRollbackFixture}

            export FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1
            export FISHYSTUFF_GITOPS_STATE_FILE=${gitopsDesiredStateLocalApplyRollbackFixture}
            mgmt run --tmp-prefix --no-network --no-pgp lang --only-unify ${./gitops}/main.mcl
            touch "$out"
          '';
          gitopsDesiredStateServeWithoutRetainedCheck =
            let
              attempted = builtins.tryEval (builtins.deepSeq (pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
                cluster = "local-test";
                environment = "local-test";
                hostKey = "vm-single-host";
                generation = 8;
                releaseGeneration = 8;
                gitRev = "serve-without-retained-fixture";
                doltCommit = "serve-without-retained-fixture";
                doltBranchContext = "local-test";
                apiClosure = gitopsDesiredStateServeFixtureApi;
                siteClosure = gitopsDesiredStateServeFixtureSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
                doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
                mode = "vm-test";
                serve = true;
              }) true);
            in
            pkgs.runCommand "gitops-desired-state-serve-without-retained-check" { } ''
              set -euo pipefail

              test "${if attempted.success then "success" else "failure"}" = "failure"
              touch "$out"
            '';
          gitopsDesiredStateActiveRetainedCheck =
            let
              attempted = builtins.tryEval (builtins.deepSeq (pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
                cluster = "local-test";
                environment = "local-test";
                hostKey = "vm-single-host";
                activeRelease = "candidate-release";
                generation = 9;
                releaseGeneration = 9;
                gitRev = "active-retained-fixture";
                doltCommit = "active-retained-fixture";
                doltBranchContext = "local-test";
                apiClosure = gitopsDesiredStateServeFixtureApi;
                siteClosure = gitopsDesiredStateServeFixtureSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
                doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
                retainedReleaseObjects = [
                  {
                    releaseId = "candidate-release";
                    generation = 8;
                    gitRev = "active-retained-fixture-previous";
                    doltCommit = "active-retained-fixture-previous";
                    apiClosure = gitopsDesiredStateServeFixturePreviousApi;
                    siteClosure = gitopsDesiredStateServeFixturePreviousSite;
                    cdnRuntimeClosure = gitopsDesiredStateServeFixturePreviousCdn;
                    doltServiceClosure = gitopsDesiredStateServeFixturePreviousDoltService;
                  }
                ];
                mode = "vm-test";
                serve = true;
              }) true);
            in
            pkgs.runCommand "gitops-desired-state-active-retained-check" { } ''
              set -euo pipefail

              test "${if attempted.success then "success" else "failure"}" = "failure"
              touch "$out"
            '';
          gitopsDesiredStateRollbackTransitionRetainedCheck =
            let
              attempted = builtins.tryEval (builtins.deepSeq (pkgs.callPackage ./nix/packages/gitops-desired-state.nix {
                cluster = "local-test";
                environment = "local-test";
                hostKey = "vm-single-host";
                activeRelease = "previous-release";
                generation = 11;
                releaseGeneration = 11;
                gitRev = "previous-rollback-retention-fixture";
                doltCommit = "previous-rollback-retention-fixture";
                doltBranchContext = "local-test";
                apiClosure = gitopsDesiredStateServeFixturePreviousApi;
                siteClosure = gitopsDesiredStateServeFixturePreviousSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixtureRollbackCdn;
                doltServiceClosure = gitopsDesiredStateServeFixturePreviousDoltService;
                retainedReleaseObjects = [
                  {
                    releaseId = "older-release";
                    generation = 8;
                    gitRev = "older-rollback-retention-fixture";
                    doltCommit = "older-rollback-retention-fixture";
                    apiClosure = gitopsDesiredStateServeFixtureApi;
                    siteClosure = gitopsDesiredStateServeFixtureSite;
                    cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
                    doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
                  }
                ];
                transition = {
                  kind = "rollback";
                  from_release = "candidate-release";
                  reason = "unsafe generated rollback fixture";
                };
                mode = "vm-test";
                serve = true;
              }) true);
            in
            pkgs.runCommand "gitops-desired-state-rollback-transition-retention-check" { } ''
              set -euo pipefail

              test "${if attempted.success then "success" else "failure"}" = "failure"
              touch "$out"
            '';
          gitopsDesiredStateTransitionShapeCheck =
            let
              attemptDesiredState =
                args:
                builtins.tryEval (builtins.deepSeq (pkgs.callPackage ./nix/packages/gitops-desired-state.nix args) true);
              base = {
                cluster = "local-test";
                environment = "local-test";
                hostKey = "vm-single-host";
                doltBranchContext = "local-test";
                mode = "vm-test";
              };
              retainedPreviousRelease = {
                releaseId = "previous-release";
                generation = 12;
                gitRev = "transition-shape-previous-fixture";
                doltCommit = "transition-shape-previous-fixture";
                apiClosure = gitopsDesiredStateServeFixturePreviousApi;
                siteClosure = gitopsDesiredStateServeFixturePreviousSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixturePreviousCdn;
                doltServiceClosure = gitopsDesiredStateServeFixturePreviousDoltService;
              };
              servingBase = base // {
                apiClosure = gitopsDesiredStateServeFixtureApi;
                siteClosure = gitopsDesiredStateServeFixtureSite;
                cdnRuntimeClosure = gitopsDesiredStateServeFixtureCdn;
                doltServiceClosure = gitopsDesiredStateServeFixtureDoltService;
                retainedReleaseObjects = [ retainedPreviousRelease ];
                serve = true;
              };
              candidateWhileServing = attemptDesiredState (servingBase // {
                generation = 13;
                releaseGeneration = 13;
                gitRev = "candidate-while-serving-fixture";
                doltCommit = "candidate-while-serving-fixture";
                transition = {
                  kind = "candidate";
                  from_release = "";
                  reason = "contradictory generated candidate fixture";
                };
              });
              activateWithoutServing = attemptDesiredState (base // {
                generation = 14;
                releaseGeneration = 14;
                gitRev = "activate-without-serving-fixture";
                doltCommit = "activate-without-serving-fixture";
                transition = {
                  kind = "activate";
                  from_release = "";
                  reason = "contradictory generated activate fixture";
                };
                serve = false;
              });
              nonRollbackFromRelease = attemptDesiredState (servingBase // {
                generation = 15;
                releaseGeneration = 15;
                gitRev = "activate-with-from-release-fixture";
                doltCommit = "activate-with-from-release-fixture";
                transition = {
                  kind = "activate";
                  from_release = "previous-release";
                  reason = "from_release belongs only to rollback";
                };
              });
            in
            pkgs.runCommand "gitops-desired-state-transition-shape-check" { } ''
              set -euo pipefail

              test "${if candidateWhileServing.success then "success" else "failure"}" = "failure"
              test "${if activateWithoutServing.success then "success" else "failure"}" = "failure"
              test "${if nonRollbackFromRelease.success then "success" else "failure"}" = "failure"
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
            api-service-bundle-production = apiServiceBundleProduction;
            cdn-base-content = cdnBaseContent;
            cdn-content = cdnContent;
            cdn-serving-root = cdnServingRoot;
            dolt-service-bundle = doltServiceBundle;
            dolt-service-bundle-production = doltServiceBundleProduction;
            edge-service-bundle = edgeServiceBundle;
            edge-service-bundle-production = edgeServiceBundleProduction;
            fishystuff-deploy = fishystuffDeploy;
            gitops-desired-state-beta-validate = gitopsDesiredStateBetaValidate;
            gitops-desired-state-http-admission-probe-fixture = gitopsDesiredStateHttpAdmissionProbeFixture;
            gitops-desired-state-local-apply-rollback-fixture = gitopsDesiredStateLocalApplyRollbackFixture;
            gitops-desired-state-production-validate = gitopsDesiredStateProductionValidate;
            gitops-desired-state-rollback-transition-fixture = gitopsDesiredStateRollbackTransitionFixture;
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
              fishystuff-deploy-tests = fishystuffDeployTests;
              gitops-desired-state-admission-probe = gitopsDesiredStateAdmissionProbeCheck;
              gitops-desired-state-http-admission-probe = gitopsDesiredStateHttpAdmissionProbeCheck;
              gitops-desired-state-local-apply-rollback = gitopsDesiredStateLocalApplyRollbackCheck;
              gitops-desired-state-active-retained-refusal = gitopsDesiredStateActiveRetainedCheck;
              gitops-desired-state-beta-validate = gitopsDesiredStateBetaValidateCheck;
              gitops-desired-state-production-validate = gitopsDesiredStateProductionValidateCheck;
              gitops-desired-state-rollback-transition = gitopsDesiredStateRollbackTransitionCheck;
              gitops-desired-state-rollback-transition-retention-refusal =
                gitopsDesiredStateRollbackTransitionRetainedCheck;
              gitops-desired-state-serve-without-retained-refusal = gitopsDesiredStateServeWithoutRetainedCheck;
              gitops-desired-state-transition-shape-refusal = gitopsDesiredStateTransitionShapeCheck;
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
