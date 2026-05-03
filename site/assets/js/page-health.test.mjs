import { beforeEach, test } from "bun:test";
import assert from "node:assert/strict";

if (typeof globalThis.CustomEvent !== "function") {
  globalThis.CustomEvent = class CustomEvent extends Event {
    constructor(type, options = {}) {
      super(type);
      this.detail = options.detail;
    }
  };
}

const settingsState = {};

function getSetting(path, fallback) {
  const parts = String(path || "").split(".").filter(Boolean);
  let current = settingsState;
  for (const part of parts) {
    if (!current || typeof current !== "object" || !(part in current)) {
      return fallback;
    }
    current = current[part];
  }
  return current === undefined ? fallback : current;
}

function setSetting(path, value) {
  const parts = String(path || "").split(".").filter(Boolean);
  let current = settingsState;
  for (const part of parts.slice(0, -1)) {
    current[part] = current[part] && typeof current[part] === "object" ? current[part] : {};
    current = current[part];
  }
  current[parts[parts.length - 1]] = value;
  return settingsState;
}

globalThis.__fishystuffUiSettings = {
  key: "fishystuff.ui.settings.v1",
  get: getSetting,
  set: setSetting,
};
globalThis.__fishystuffPageHealthConfig = {
  autoReportDelayMs: 5,
};

await import(`./page-health.js?test=${Date.now()}`);

const health = globalThis.__fishystuffPageHealth;

beforeEach(() => {
  for (const key of Object.keys(settingsState)) {
    delete settingsState[key];
  }
  delete globalThis.__fishystuffOtel;
  delete globalThis.__fishystuffClientSession;
  health.clearAll("test");
});

test("page health menu visibility follows active warning dismissal and user setting", () => {
  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    title: "Points API request failed",
    detail: "request failed; retrying in 4s",
  });

  assert.equal(health.current().activeWarningCount, 1);
  assert.equal(health.current().navMenuVisible, true);

  health.dismissIssue("map-api:points");
  assert.equal(health.current().activeWarningCount, 0);
  assert.equal(health.current().navMenuVisible, false);

  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    title: "Points API request failed",
    detail: "request failed; retrying in 8s",
  });
  assert.equal(health.current().activeWarningCount, 0);

  health.clearIssue("map-api:points");
  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    title: "Points API request failed",
  });
  health.setMenuHidden(true);
  assert.equal(health.current().activeWarningCount, 1);
  assert.equal(health.current().hiddenByUserSetting, true);
  assert.equal(health.current().navMenuVisible, false);
});

test("syncSourceIssues clears stale source warnings", () => {
  health.syncSourceIssues("map-api", [
    {
      id: "map-api:meta",
      severity: "warning",
      title: "Meta API request failed",
    },
  ]);
  assert.equal(health.current().activeWarningCount, 1);

  health.syncSourceIssues("map-api", []);
  assert.equal(health.current().activeWarningCount, 0);
  assert.equal(health.current().issues.length, 0);
});

test("syncSourceIssues keeps continuous source warnings as one active issue", () => {
  health.syncSourceIssues("map-api", [
    {
      id: "map-api:points",
      severity: "warning",
      title: "Points API request failed",
      detail: "request failed; retrying in 4s",
    },
  ]);
  health.syncSourceIssues("map-api", [
    {
      id: "map-api:points",
      severity: "warning",
      title: "Points API request failed",
      detail: "request failed; retrying in 3s",
    },
  ]);

  assert.equal(health.current().activeWarningCount, 1);
  assert.equal(health.current().activeWarnings[0].count, 1);
});

test("registerSource creates reusable scoped issue reporters", () => {
  const source = health.registerSource("calculator", {
    category: "ui",
    severity: "warning",
  });

  source.report({
    id: "calculator:layout",
    title: "Calculator layout failed",
  });

  assert.equal(health.current().activeWarningCount, 1);
  assert.equal(health.current().activeWarnings[0].source, "calculator");
  assert.equal(health.current().activeWarnings[0].category, "ui");

  source.sync([]);
  assert.equal(health.current().activeWarningCount, 0);
});

test("sendManualReport emits telemetry and marks diagnostic report state", async () => {
  const otelCalls = [];
  const sessionCalls = [];
  globalThis.__fishystuffOtel = {
    status() {
      return {
        initialized: true,
        automaticTelemetryActive: true,
        loggingEnabled: true,
        diagnosticReportsEnabled: true,
        reason: "ready",
      };
    },
    async emitDiagnosticReport(report) {
      otelCalls.push({ report });
      return { sent: true, reason: "sent", status: this.status() };
    },
  };
  globalThis.__fishystuffClientSession = {
    createDiagnosticReportDraft(context) {
      return {
        createdAt: "2026-05-03T00:00:00.000Z",
        page: { href: "https://fishystuff.fish/map/", path: "/map/" },
        report: context,
      };
    },
    markDiagnosticReportPrepared() {
      sessionCalls.push("prepared");
    },
    markDiagnosticReportSubmitted() {
      sessionCalls.push("submitted");
    },
  };

  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    category: "api",
    title: "Points API request failed",
    detail: "request failed; retrying in 4s",
  });

  const result = await health.sendManualReport();

  assert.equal(result.sent, true);
  assert.deepEqual(sessionCalls, ["prepared", "submitted"]);
  assert.equal(health.current().activeWarningCount, 0);
  assert.equal(otelCalls.length, 1);
  assert.equal(otelCalls[0].report.report.mode, "manual");
  assert.equal(otelCalls[0].report.health.issues.length, 1);
  assert.equal(otelCalls[0].report.health.issues[0].title, "Points API request failed");
  assert.equal(result.telemetry.diagnosticReportsEnabled, true);
});

test("sendManualReport reports telemetry availability through the shared status", async () => {
  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    category: "api",
    title: "Points API request failed",
  });

  const result = await health.sendManualReport();

  assert.equal(result.sent, false);
  assert.equal(result.reason, "telemetry-unavailable");
  assert.equal(result.telemetry.diagnosticReportsEnabled, false);
  assert.equal(health.current().activeWarningCount, 1);
});

test("sendManualReport does not dismiss warnings when telemetry export fails", async () => {
  globalThis.__fishystuffOtel = {
    status() {
      return {
        initialized: true,
        automaticTelemetryActive: true,
        loggingEnabled: true,
        diagnosticReportsEnabled: true,
        reason: "ready",
        lastLogExport: {
          ok: false,
          error: "OTLP logs export failed with HTTP 502",
        },
      };
    },
    async emitDiagnosticReport() {
      return {
        sent: false,
        reason: "telemetry-export-failed",
        status: this.status(),
      };
    },
  };

  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    category: "api",
    title: "Points API request failed",
  });

  const result = await health.sendManualReport();

  assert.equal(result.sent, false);
  assert.equal(result.reason, "telemetry-export-failed");
  assert.equal(result.telemetry.label, "Export failed");
  assert.equal(health.current().activeWarningCount, 1);
});

test("current exposes telemetry status from the shared bridge", () => {
  globalThis.__fishystuffOtel = {
    status() {
      return {
        initialized: true,
        automaticTelemetryActive: true,
        loggingEnabled: true,
        diagnosticReportsEnabled: true,
        reason: "ready",
      };
    },
  };

  const snapshot = health.current();

  assert.equal(snapshot.telemetry.label, "Ready");
  assert.equal(snapshot.telemetry.diagnosticReportsEnabled, true);
});

test("sendAutomaticReport sends telemetry without dismissing warnings", async () => {
  const otelCalls = [];
  globalThis.__fishystuffOtel = {
    status() {
      return {
        initialized: true,
        automaticTelemetryActive: true,
        loggingEnabled: true,
        diagnosticReportsEnabled: true,
        reason: "ready",
      };
    },
    async emitDiagnosticReport(report, options) {
      otelCalls.push({ report, options });
      return { sent: true, reason: "sent", status: this.status() };
    },
  };

  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    category: "api",
    title: "Points API request failed",
  });

  const result = await health.sendAutomaticReport("test");
  const snapshot = health.current();

  assert.equal(result.sent, true);
  assert.equal(otelCalls.length, 1);
  assert.equal(otelCalls[0].report.report.mode, "automatic");
  assert.equal(otelCalls[0].options.mode, "automatic");
  assert.equal(snapshot.activeWarningCount, 1);
  assert.equal(snapshot.activeWarnings[0].autoReportStatus, "sent");
  assert.equal(snapshot.autoReport.label, "Auto sent");
});

test("automatic reports are scheduled when telemetry is ready", async () => {
  const otelCalls = [];
  globalThis.__fishystuffOtel = {
    status() {
      return {
        initialized: true,
        automaticTelemetryActive: true,
        loggingEnabled: true,
        diagnosticReportsEnabled: true,
        reason: "ready",
      };
    },
    async emitDiagnosticReport(report, options) {
      otelCalls.push({ report, options });
      return { sent: true, reason: "sent", status: this.status() };
    },
  };

  health.reportIssue({
    id: "map-api:points",
    severity: "warning",
    source: "map-api",
    category: "api",
    title: "Points API request failed",
  });

  await new Promise((resolve) => setTimeout(resolve, 30));

  assert.equal(otelCalls.length, 1);
  assert.equal(otelCalls[0].options.mode, "automatic");
  assert.equal(health.current().activeWarningCount, 1);
  assert.equal(health.current().activeWarnings[0].autoReportStatus, "sent");
});
