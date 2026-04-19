{ pkgs }:
{
  config,
  lib,
  ...
}:
let
  helpers = import ./helpers.nix { inherit lib; };
  systemdBackend = import ./systemd-backend.nix { inherit lib pkgs; };
  inherit (lib) mkOption optional types;
  cfg = config.fishystuff.vector;
  journalUnitsYaml =
    lib.concatMapStringsSep "\n" (unit: "      - ${unit}") cfg.journalUnits;
  configSource = pkgs.writeText "fishystuff-vector.yaml" ''
    data_dir: "${cfg.dataDir}/state"

    api:
      enabled: true
      address: "${cfg.apiListenAddress}:${toString cfg.apiPort}"

    sources:
      systemd_journal:
        type: journald
        current_boot_only: false
        include_units:
    ${journalUnitsYaml}
        data_dir: "${cfg.dataDir}/journal"
      telemetry_logs_ingress:
        type: http_server
        address: "${cfg.telemetryLogsListenAddress}:${toString cfg.telemetryLogsPort}"
        method: POST
        path: "/v1/logs"
        strict_path: true
        response_code: 200
        decoding:
          codec: otlp
          signal_types:
            - logs
      telemetry_otlp_ingress:
        type: http_server
        address: "${cfg.telemetryOtlpListenAddress}:${toString cfg.telemetryOtlpPort}"
        method: POST
        path: "/v1"
        strict_path: false
        response_code: 200
        decoding:
          codec: otlp
          signal_types:
            - metrics
            - traces

    transforms:
      telemetry_logs_only:
        type: filter
        inputs:
          - telemetry_logs_ingress
        condition:
          type: is_log
      telemetry_metrics_only:
        type: filter
        inputs:
          - telemetry_otlp_ingress
        condition:
          type: is_metric
      telemetry_traces_only:
        type: filter
        inputs:
          - telemetry_otlp_ingress
        condition:
          type: is_trace
      normalized_process_logs:
        type: remap
        inputs:
          - systemd_journal
        source: |
          .app = "fishystuff"
          .deployment_environment = "${cfg.deploymentEnvironment}"
          .observability_kind = "log"
          .log_schema = "fishystuff.journal.v1"
          .correlation = {}
          .http = {}
          .browser = {}
          .process = string!(._SYSTEMD_UNIT ?? "unknown.service")
          .service = .process
          .level = "info"

          if !exists(.message) || is_null(.message) {
            .message = "journal entry"
          } else {
            .message = string!(.message)
          }
      normalized_telemetry_logs:
        type: remap
        inputs:
          - telemetry_logs_only
        source: |
          .app = "fishystuff"
          .deployment_environment = "${cfg.deploymentEnvironment}"
          .observability_kind = "log"
          .log_schema = "fishystuff.browser-otel.v1"
          .process = "browser"
          .correlation = {}
          .http = {}
          .browser = {}
          first_resource_log = get(., ["resourceLogs", 0]) ?? {}
          first_scope_log = get(first_resource_log, ["scopeLogs", 0]) ?? {}
          first_log_record = get(first_scope_log, ["logRecords", 0]) ?? {}

          event_timestamp = null
          if exists(.timestamp) && !is_null(.timestamp) {
            event_timestamp = parse_timestamp(string!(.timestamp), format: "%+") ?? null
          }
          if event_timestamp == null && exists(.observed_timestamp) && !is_null(.observed_timestamp) {
            event_timestamp = parse_timestamp(string!(.observed_timestamp), format: "%+") ?? null
          }
          if event_timestamp != null {
            .timestamp = event_timestamp
          }

          .service = "fishystuff-site-beta"
          service_name = get(.resources, ["service.name"]) ?? null
          if service_name != null && string!(service_name) != "" {
            .service = string!(service_name)
          }

          deployment_environment = get(.resources, ["deployment.environment"]) ?? null
          if deployment_environment != null && string!(deployment_environment) != "" {
            .deployment_environment = string!(deployment_environment)
          }

          severity_text = get(., ["severity_text"]) ?? null
          if severity_text == null {
            severity_text = get(first_log_record, ["severityText"]) ?? null
          }
          if severity_text != null && string!(severity_text) != "" {
            .level = downcase(string!(severity_text))
          } else {
            .level = "info"
          }

          logger_name = get(.scope, ["name"]) ?? null
          if logger_name == null {
            logger_name = get(first_scope_log, ["scope", "name"]) ?? null
          }
          if logger_name != null && string!(logger_name) != "" {
            .logger = string!(logger_name)
          }

          message_value = get(., ["message"]) ?? null
          if message_value == null {
            message_value = get(first_log_record, ["body", "stringValue"]) ?? null
          }
          if message_value == null || string!(message_value) == "" {
            .message = "browser log"
          } else {
            .message = string!(message_value)
          }

          request_id = get(.attributes, ["request.id"]) ?? null
          if request_id != null && string!(request_id) != "" {
            .correlation.request_id = string!(request_id)
            .request_id = .correlation.request_id
          }

          trace_id = get(.attributes, ["trace.id"]) ?? null
          if trace_id != null && string!(trace_id) != "" {
            .correlation.trace_id = string!(trace_id)
            .trace_id = .correlation.trace_id
          }

          span_id = get(.attributes, ["span.id"]) ?? null
          if span_id != null && string!(span_id) != "" {
            .correlation.span_id = string!(span_id)
            .span_id = .correlation.span_id
          }

          http_status_code = get(.attributes, ["http.response.status_code"]) ?? null
          if http_status_code != null {
            .http.status_code = http_status_code
          }

          url_full = get(.attributes, ["url.full"]) ?? null
          if url_full != null && string!(url_full) != "" {
            .browser.url_full = string!(url_full)
          }

          url_path = get(.attributes, ["url.path"]) ?? null
          if url_path != null && string!(url_path) != "" {
            .browser.url_path = string!(url_path)
          }

          url_query = get(.attributes, ["url.query"]) ?? null
          if url_query != null && string!(url_query) != "" {
            .browser.url_query = string!(url_query)
          }

          log_source = get(.attributes, ["fishystuff.log.source"]) ?? null
          if log_source != null && string!(log_source) != "" {
            .browser.source = string!(log_source)
          }

          error_type = get(.attributes, ["error.type"]) ?? null
          if error_type != null && string!(error_type) != "" {
            .browser.error_type = string!(error_type)
          }

          error_message = get(.attributes, ["error.message"]) ?? null
          if error_message != null && string!(error_message) != "" {
            .browser.error_message = string!(error_message)
          }

          error_stack = get(.attributes, ["error.stack"]) ?? null
          if error_stack != null && string!(error_stack) != "" {
            .browser.error_stack = string!(error_stack)
          }

          if !exists(.browser.source) {
            .browser.source = "browser"
          }

    sinks:
      logs_archive:
        type: file
        inputs:
          - normalized_process_logs
        path: "${cfg.dataDir}/archive/logs/%Y-%m-%d.ndjson"
        encoding:
          codec: json

      logs_loki:
        type: loki
        inputs:
          - normalized_process_logs
          - normalized_telemetry_logs
        endpoint: "http://${cfg.lokiAddress}:${toString cfg.lokiPort}"
        encoding:
          codec: json
        labels:
          app: "{{ app }}"
          env: "{{ deployment_environment }}"
          process: "{{ process }}"
          service: "{{ service }}"
          level: "{{ level }}"
        structured_metadata:
          log_schema: "{{ log_schema }}"
          '"correlation_*"': "{{ correlation }}"
          '"http_*"': "{{ http }}"
          '"browser_*"': "{{ browser }}"
        healthcheck:
          enabled: false

      traces_archive:
        type: file
        inputs:
          - telemetry_traces_only
        path: "${cfg.dataDir}/archive/traces/%Y-%m-%d.ndjson"
        encoding:
          codec: json

      telemetry_ingress_traces_to_collector:
        type: opentelemetry
        inputs:
          - telemetry_traces_only
        protocol:
          type: http
          uri: "http://${cfg.otelCollectorAddress}:${toString cfg.otelCollectorPort}/v1/traces"
          encoding:
            codec: otlp
        healthcheck:
          enabled: false

      telemetry_ingress_metrics_to_collector:
        type: opentelemetry
        inputs:
          - telemetry_metrics_only
        protocol:
          type: http
          uri: "http://${cfg.otelCollectorAddress}:${toString cfg.otelCollectorPort}/v1/metrics"
          encoding:
            codec: otlp
        healthcheck:
          enabled: false

      telemetry_ingress_logs_archive:
        type: file
        inputs:
          - telemetry_logs_only
        path: "${cfg.dataDir}/archive/otel-logs/%Y-%m-%d.ndjson"
        encoding:
          codec: json
  '';
  serviceArgv = [
    (lib.getExe' cfg.package "vector")
    "--config-yaml"
    configSource
  ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-vector.service";
    description = "Fishystuff Vector service";
    argv = serviceArgv;
    environment = { };
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    workingDirectory = cfg.dataDir;
    after = [
      "network-online.target"
      "fishystuff-loki.service"
      "fishystuff-otel-collector.service"
    ];
    wants = [
      "network-online.target"
      "fishystuff-loki.service"
      "fishystuff-otel-collector.service"
    ];
    restartPolicy = "on-failure";
    restartDelaySeconds = 5;
    serviceLines = [
      "StateDirectory=${cfg.stateDirectoryName}"
      "StateDirectoryMode=0750"
      "PrivateTmp=true"
      "PrivateDevices=true"
      "ProtectSystem=strict"
      "ProtectHome=true"
      "ProtectKernelTunables=true"
      "ProtectKernelModules=true"
      "ProtectControlGroups=true"
      "LockPersonality=true"
      "NoNewPrivileges=true"
      "RestrictRealtime=true"
      "RestrictSUIDSGID=true"
      "SystemCallArchitectures=native"
      "UMask=0077"
    ];
  };
in
{
  _class = "service";
  imports = [ ./bundle-module.nix ];

  options.fishystuff.vector = {
    package = mkOption {
      type = types.package;
      default = pkgs.vector;
      defaultText = lib.literalExpression "pkgs.vector";
      description = "Package containing the `vector` executable.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "vector.yaml";
      description = "Bundle-relative name for the Vector config artifact.";
    };

    stateDirectoryName = mkOption {
      type = types.str;
      default = "fishystuff/vector";
      description = "systemd StateDirectory name used for Vector state.";
    };

    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/${cfg.stateDirectoryName}";
      description = "Persistent Vector data directory.";
    };

    deploymentEnvironment = mkOption {
      type = types.str;
      default = "beta";
      description = "Deployment environment label written into normalized events.";
    };

    apiListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Vector admin API.";
    };

    apiPort = mkOption {
      type = types.port;
      default = 8686;
      description = "TCP port for the Vector admin API.";
    };

    telemetryLogsListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for browser OTLP log ingestion.";
    };

    telemetryLogsPort = mkOption {
      type = types.port;
      default = 4820;
      description = "TCP port for browser OTLP log ingestion.";
    };

    telemetryOtlpListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for browser OTLP metrics and traces.";
    };

    telemetryOtlpPort = mkOption {
      type = types.port;
      default = 4821;
      description = "TCP port for browser OTLP metrics and traces.";
    };

    lokiAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Loki upstream address.";
    };

    lokiPort = mkOption {
      type = types.port;
      default = 3100;
      description = "Loki upstream HTTP port.";
    };

    otelCollectorAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "OTEL collector upstream address.";
    };

    otelCollectorPort = mkOption {
      type = types.port;
      default = 4818;
      description = "OTEL collector upstream HTTP port.";
    };

    journalUnits = mkOption {
      type = types.listOf types.str;
      default = [
        "fishystuff-api.service"
        "fishystuff-dolt.service"
        "fishystuff-edge.service"
        "fishystuff-vector.service"
        "fishystuff-loki.service"
        "fishystuff-otel-collector.service"
        "fishystuff-jaeger.service"
        "fishystuff-prometheus.service"
        "fishystuff-grafana.service"
      ];
      description = "systemd units collected from journald.";
    };

    dynamicUser = mkOption {
      type = types.bool;
      default = true;
      description = "Whether a backend may allocate an ephemeral user.";
    };

    supplementaryGroups = mkOption {
      type = types.listOf types.str;
      default = [ "systemd-journal" ];
      description = "Supplementary runtime groups.";
    };
  };

  config = {
    configData.${cfg.configFileName}.source = configSource;
    process.argv = serviceArgv;

    bundle = {
      id = "fishystuff-vector";

      roots.store = [
        cfg.package
        configSource
        systemdUnit.file
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = lib.getExe' cfg.package "vector";
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = configSource;
          destination = cfg.configFileName;
        };

        "systemd/unit" = systemdUnit.artifact;
      };

      activation = {
        directories = [ ];
        users = [ ];
        groups = [ ];
        writablePaths = [ cfg.dataDir ];
        requiredPaths = [ ];
      };

      supervision = {
        environment = { };
        environmentFiles = [ ];
        workingDirectory = cfg.dataDir;
        identity = {
          user = null;
          group = null;
          dynamicUser = cfg.dynamicUser;
          supplementaryGroups = cfg.supplementaryGroups;
        };
        restart = {
          policy = "on-failure";
          delaySeconds = 5;
        };
        reload = {
          mode = "restart";
          signal = null;
          argv = [ ];
        };
        stop = {
          mode = "signal";
          signal = "TERM";
          argv = [ ];
          timeoutSeconds = 30;
        };
        readiness = {
          mode = "simple";
        };
      };

      runtimeOverlays = [ ];
      requiredCapabilities = optional cfg.dynamicUser "dynamic-user";
      backends.systemd = systemdUnit.backend;
    };
  };
}
