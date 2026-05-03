(function (globalRef) {
  const CHANGE_EVENT = "fishystuff:page-health-change";
  const SETTINGS_KEY = "fishystuff.ui.settings.v1";
  const SETTINGS_PATH = "app.health.hideNavMenu";
  const AUTO_REPORT_SETTING_PATH = "app.health.autoReportWarnings";
  const TELEMETRY_STATUS_EVENT = "fishystuff:telemetry-status-change";
  const DEFAULT_SOURCE = "page";
  const REPORT_CATEGORY = "page-health";
  const MAX_DETAIL_CHARS = 240;
  const runtimeConfig = isPlainObject(globalRef.__fishystuffPageHealthConfig)
    ? globalRef.__fishystuffPageHealthConfig
    : {};
  const AUTO_REPORT_DELAY_MS = Math.max(
    0,
    Number.parseInt(runtimeConfig.autoReportDelayMs, 10) || 1500,
  );

  const issues = new Map();
  const subscribers = new Set();
  let autoReportTimer = 0;
  let autoReportInFlight = false;
  let lastAutoReportAt = "";
  let lastAutoReportFailureReason = "";

  function isPlainObject(value) {
    return Boolean(value) && Object.prototype.toString.call(value) === "[object Object]";
  }

  function cloneJson(value) {
    try {
      return JSON.parse(JSON.stringify(value));
    } catch (_error) {
      return {};
    }
  }

  function trimString(value) {
    return String(value ?? "").trim();
  }

  function nowIso() {
    return new Date().toISOString();
  }

  function randomId(prefix) {
    const normalizedPrefix = trimString(prefix) || "issue";
    if (globalRef.crypto && typeof globalRef.crypto.randomUUID === "function") {
      return `${normalizedPrefix}-${globalRef.crypto.randomUUID()}`;
    }
    return `${normalizedPrefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  }

  function compactText(value, maxChars = MAX_DETAIL_CHARS) {
    const singleLine = trimString(value).replace(/\s+/g, " ");
    if (singleLine.length <= maxChars) {
      return singleLine;
    }
    return `${singleLine.slice(0, Math.max(0, maxChars - 3))}...`;
  }

  function normalizeSeverity(value) {
    const normalized = trimString(value).toLowerCase();
    if (normalized === "error" || normalized === "warning" || normalized === "info") {
      return normalized;
    }
    return "warning";
  }

  function issueIsWarning(issue) {
    return issue?.severity === "warning" || issue?.severity === "error";
  }

  function normalizeAutoReportStatus(value) {
    const normalized = trimString(value).toLowerCase();
    if (normalized === "pending" || normalized === "sent" || normalized === "failed") {
      return normalized;
    }
    return "";
  }

  function stableIssueId(source) {
    const explicitId = trimString(source.id);
    if (explicitId) {
      return explicitId;
    }
    const sourceName = trimString(source.source) || DEFAULT_SOURCE;
    const category = trimString(source.category) || "general";
    const title = trimString(source.title) || trimString(source.summary) || "Issue";
    return `${sourceName}:${category}:${title}`.toLowerCase();
  }

  function normalizeIssue(value) {
    const source = isPlainObject(value) ? value : {};
    const title = compactText(source.title || source.summary || "Page issue", 96);
    const detail = compactText(source.detail || source.message || "", MAX_DETAIL_CHARS);
    const severity = normalizeSeverity(source.severity);
    const now = nowIso();
    return {
      id: stableIssueId(source),
      severity,
      source: trimString(source.source) || DEFAULT_SOURCE,
      category: trimString(source.category) || "general",
      title,
      detail,
      actionLabel: compactText(source.actionLabel || "", 80),
      createdAt: trimString(source.createdAt) || now,
      updatedAt: trimString(source.updatedAt) || now,
      count: Math.max(1, Number.parseInt(source.count, 10) || 1),
      dismissed: source.dismissed === true,
      reportable: source.reportable !== false,
      autoReport: source.autoReport !== false,
      autoReportStatus: normalizeAutoReportStatus(source.autoReportStatus),
      autoReportAttemptedAt: trimString(source.autoReportAttemptedAt),
      autoReportedAt: trimString(source.autoReportedAt),
      autoReportReason: compactText(source.autoReportReason || "", 120),
      context: isPlainObject(source.context) ? cloneJson(source.context) : {},
    };
  }

  function hiddenByUserSetting() {
    const settings = globalRef.__fishystuffUiSettings;
    if (!settings || typeof settings.get !== "function") {
      return getStoredSetting(SETTINGS_PATH, false) === true;
    }
    return settings.get(SETTINGS_PATH, false) === true;
  }

  function setMenuHidden(hidden) {
    const settings = globalRef.__fishystuffUiSettings;
    if (settings && typeof settings.set === "function") {
      settings.set(SETTINGS_PATH, hidden === true);
    } else {
      setStoredSetting(SETTINGS_PATH, hidden === true);
    }
    emitChange("settings");
    return current();
  }

  function autoReportEnabledByUserSetting() {
    const settings = globalRef.__fishystuffUiSettings;
    if (!settings || typeof settings.get !== "function") {
      return getStoredSetting(AUTO_REPORT_SETTING_PATH, true) !== false;
    }
    return settings.get(AUTO_REPORT_SETTING_PATH, true) !== false;
  }

  function setAutoReportEnabled(enabled) {
    const settings = globalRef.__fishystuffUiSettings;
    if (settings && typeof settings.set === "function") {
      settings.set(AUTO_REPORT_SETTING_PATH, enabled === true);
    } else {
      setStoredSetting(AUTO_REPORT_SETTING_PATH, enabled === true);
    }
    emitChange("auto-report-settings");
    return current();
  }

  function pathParts(path) {
    return String(path || "")
      .split(".")
      .map((part) => part.trim())
      .filter(Boolean);
  }

  function readStoredSettings() {
    try {
      const parsed = JSON.parse(globalRef.localStorage?.getItem(SETTINGS_KEY) || "{}");
      return isPlainObject(parsed) ? parsed : {};
    } catch (_error) {
      return {};
    }
  }

  function getStoredSetting(path, fallback) {
    let current = readStoredSettings();
    for (const part of pathParts(path)) {
      if (!isPlainObject(current) || !(part in current)) {
        return fallback;
      }
      current = current[part];
    }
    return current === undefined ? fallback : current;
  }

  function setAtPath(root, parts, value) {
    if (!parts.length) {
      return isPlainObject(value) ? value : {};
    }
    const nextRoot = { ...(isPlainObject(root) ? root : {}) };
    let cursor = nextRoot;
    for (const part of parts.slice(0, -1)) {
      cursor[part] = { ...(isPlainObject(cursor[part]) ? cursor[part] : {}) };
      cursor = cursor[part];
    }
    cursor[parts[parts.length - 1]] = value;
    return nextRoot;
  }

  function setStoredSetting(path, value) {
    try {
      const nextSettings = setAtPath(readStoredSettings(), pathParts(path), value);
      globalRef.localStorage?.setItem(SETTINGS_KEY, JSON.stringify(nextSettings));
    } catch (_error) {
    }
  }

  function sortedIssues() {
    const severityRank = { error: 0, warning: 1, info: 2 };
    return Array.from(issues.values()).sort((left, right) => {
      const severityDelta = (severityRank[left.severity] ?? 9) - (severityRank[right.severity] ?? 9);
      if (severityDelta) {
        return severityDelta;
      }
      return String(right.updatedAt).localeCompare(String(left.updatedAt));
    });
  }

  function activeWarnings() {
    return sortedIssues().filter((issue) => issueIsWarning(issue) && !issue.dismissed);
  }

  function reportableWarnings(values) {
    return (Array.isArray(values) ? values : [])
      .filter((issue) => issueIsWarning(issue) && !issue.dismissed && issue.reportable !== false);
  }

  function autoReportCandidateIssues(values) {
    return reportableWarnings(values).filter((issue) =>
      issue.autoReport !== false
      && issue.autoReportStatus !== "pending"
      && issue.autoReportStatus !== "sent"
      && issue.autoReportStatus !== "failed",
    );
  }

  function telemetryStatusLabel(reason, status) {
    if (status.lastLogExport?.ok === false) {
      return "Export failed";
    }
    if (status.diagnosticReportsEnabled) {
      return "Ready";
    }
    if (reason === "opt-in-required" || reason === "disabled-by-user" || reason === "disabled-by-query") {
      return "Off";
    }
    if (reason === "logs-disabled" || reason === "missing-logs-exporter" || reason === "logs-unavailable") {
      return "Logs off";
    }
    if (reason === "disabled-by-runtime-policy") {
      return "Disabled";
    }
    return "Unavailable";
  }

  function telemetryStatusDetail(reason, status) {
    if (status.lastLogExport?.ok === false) {
      return "Last report delivery failed; telemetry logs are not being accepted.";
    }
    if (status.diagnosticReportsEnabled) {
      return "Automatic telemetry is active and reports use the same browser logs pipeline.";
    }
    if (reason === "opt-in-required") {
      return "Automatic telemetry is off until the current profile opts in.";
    }
    if (reason === "disabled-by-user") {
      return "Automatic telemetry is disabled for the current profile.";
    }
    if (reason === "disabled-by-query") {
      return "Automatic telemetry is disabled by the current URL.";
    }
    if (reason === "missing-logs-exporter") {
      return "Telemetry is active, but no browser logs exporter is configured.";
    }
    if (reason === "logs-disabled" || reason === "logs-unavailable") {
      return "Telemetry is active, but browser logs are not available for reports.";
    }
    if (reason === "disabled-by-runtime-policy") {
      return "Telemetry reports are disabled by the runtime policy.";
    }
    return "Telemetry has not initialized for this page.";
  }

  function telemetryStatusTone(status) {
    if (status.lastLogExport?.ok === false) {
      return "error";
    }
    if (status.diagnosticReportsEnabled) {
      return "success";
    }
    if (status.automaticTelemetryActive) {
      return "warning";
    }
    return "neutral";
  }

  function normalizeTelemetryStatus(value) {
    const source = isPlainObject(value) ? value : {};
    const diagnosticReportsEnabled =
      source.diagnosticReportsEnabled === true
      || source.manualDiagnosticReportsEnabled === true;
    const automaticTelemetryActive =
      source.automaticTelemetryActive === true
      || source.telemetryEffectiveEnabled === true;
    const lastLogExport = isPlainObject(source.lastLogExport) ? cloneJson(source.lastLogExport) : null;
    const reason = trimString(source.reason || source.telemetryReason)
      || (diagnosticReportsEnabled ? "ready" : "telemetry-unavailable");
    const status = {
      initialized: source.initialized === true,
      automaticTelemetryActive,
      tracingEnabled: source.tracingEnabled === true,
      metricsEnabled: source.metricsEnabled === true,
      loggingEnabled: source.loggingEnabled === true,
      diagnosticReportsEnabled,
      reason,
      telemetryDefaultMode: trimString(source.telemetryDefaultMode),
      telemetryPreference: trimString(source.telemetryPreference),
      telemetryReason: trimString(source.telemetryReason),
      telemetrySource: trimString(source.telemetrySource),
      logsExporterEndpoint: trimString(source.logsExporterEndpoint),
      lastLogExport,
    };
    return {
      ...status,
      label: telemetryStatusLabel(reason, status),
      detail: telemetryStatusDetail(reason, status),
      tone: telemetryStatusTone(status),
    };
  }

  function resolveTelemetryStatus() {
    const otel = globalRef.__fishystuffOtel;
    if (otel && typeof otel.status === "function") {
      try {
        return normalizeTelemetryStatus(otel.status());
      } catch (_error) {
        return normalizeTelemetryStatus({ reason: "telemetry-status-error" });
      }
    }
    return normalizeTelemetryStatus({ reason: "telemetry-unavailable" });
  }

  function autoReportSnapshot(active, telemetry) {
    const reportable = reportableWarnings(active);
    const candidates = autoReportCandidateIssues(active);
    const enabledByUser = autoReportEnabledByUserSetting();
    const enabled = enabledByUser && telemetry.diagnosticReportsEnabled === true;
    const failed = reportable.filter((issue) => issue.autoReportStatus === "failed").length;
    const sent = reportable.filter((issue) => issue.autoReportStatus === "sent").length;
    let label = "Auto off";
    let detail = "Automatic reports are disabled.";
    let tone = "neutral";
    if (!enabledByUser) {
      label = "Auto off";
      detail = "Automatic reports are disabled in page settings.";
    } else if (!telemetry.diagnosticReportsEnabled) {
      label = "Auto waiting";
      detail = "Automatic reports will use telemetry when browser logs are ready.";
      tone = telemetry.tone === "error" ? "error" : "warning";
    } else if (autoReportInFlight) {
      label = "Auto sending";
      detail = "Sending a diagnostic report while keeping warnings visible.";
      tone = "info";
    } else if (failed > 0) {
      label = "Auto failed";
      detail = "An automatic report failed; use Send report to retry.";
      tone = "error";
    } else if (candidates.length > 0) {
      label = "Auto queued";
      detail = "A diagnostic report will be sent automatically.";
      tone = "warning";
    } else if (sent > 0) {
      label = "Auto sent";
      detail = "Warnings remain visible after automatic reporting.";
      tone = "success";
    } else {
      label = "Auto ready";
      detail = "New warnings will be reported automatically.";
      tone = "success";
    }
    return {
      enabledByUser,
      enabled,
      inFlight: autoReportInFlight,
      pendingIssueCount: candidates.length,
      reportableIssueCount: reportable.length,
      sentIssueCount: sent,
      failedIssueCount: failed,
      lastSentAt: lastAutoReportAt,
      lastFailureReason: lastAutoReportFailureReason,
      label,
      detail,
      tone,
    };
  }

  function current() {
    const active = activeWarnings();
    const hidden = hiddenByUserSetting();
    const telemetry = resolveTelemetryStatus();
    return {
      version: 1,
      issueCount: issues.size,
      activeWarningCount: active.length,
      hiddenByUserSetting: hidden,
      navMenuVisible: active.length > 0 && !hidden,
      issues: sortedIssues(),
      activeWarnings: active,
      telemetry,
      autoReport: autoReportSnapshot(active, telemetry),
    };
  }

  function dispatchDomChange(snapshot, reason) {
    globalRef.dispatchEvent?.(
      new CustomEvent(CHANGE_EVENT, {
        detail: {
          reason: trimString(reason) || "update",
          snapshot,
        },
      }),
    );
  }

  function clearAutoReportTimer() {
    if (!autoReportTimer) {
      return;
    }
    globalRef.clearTimeout?.(autoReportTimer);
    autoReportTimer = 0;
  }

  function maybeScheduleAutoReport(snapshot, reason) {
    const normalizedReason = trimString(reason);
    if (
      normalizedReason === "auto-report-pending"
      || normalizedReason === "auto-report-sent"
      || normalizedReason === "auto-report-failed"
    ) {
      return;
    }
    if (
      !snapshot.autoReport?.enabled
      || snapshot.autoReport.pendingIssueCount <= 0
      || autoReportInFlight
    ) {
      clearAutoReportTimer();
      return;
    }
    if (autoReportTimer) {
      return;
    }
    autoReportTimer = globalRef.setTimeout?.(() => {
      autoReportTimer = 0;
      Promise.resolve(sendAutomaticReport("scheduled")).catch(() => {});
    }, AUTO_REPORT_DELAY_MS) || 0;
  }

  function emitChange(reason) {
    const snapshot = current();
    for (const listener of subscribers) {
      try {
        listener(snapshot, reason);
      } catch (_error) {
      }
    }
    dispatchDomChange(snapshot, reason);
    renderAll(snapshot);
    maybeScheduleAutoReport(snapshot, reason);
    return snapshot;
  }

  function upsertIssue(value, options = {}) {
    const nextIssue = normalizeIssue(value);
    const previous = issues.get(nextIssue.id);
    const shouldPreserveDismissed = options.preserveDismissed !== false;
    const merged = previous
      ? {
          ...previous,
          ...nextIssue,
          createdAt: previous.createdAt || nextIssue.createdAt,
          updatedAt: nextIssue.updatedAt || nowIso(),
          count: Math.max(1, previous.count + 1),
          dismissed: shouldPreserveDismissed ? previous.dismissed : nextIssue.dismissed,
          autoReportStatus: nextIssue.autoReportStatus || previous.autoReportStatus || "",
          autoReportAttemptedAt: nextIssue.autoReportAttemptedAt || previous.autoReportAttemptedAt || "",
          autoReportedAt: nextIssue.autoReportedAt || previous.autoReportedAt || "",
          autoReportReason: nextIssue.autoReportReason || previous.autoReportReason || "",
        }
      : nextIssue;
    issues.set(merged.id, merged);
    return emitChange("issue");
  }

  function dismissIssue(id) {
    const normalizedId = trimString(id);
    const issue = issues.get(normalizedId);
    if (!issue) {
      return current();
    }
    issues.set(normalizedId, { ...issue, dismissed: true, updatedAt: nowIso() });
    return emitChange("dismiss");
  }

  function dismissAll() {
    let changed = false;
    for (const [id, issue] of issues.entries()) {
      if (issueIsWarning(issue) && !issue.dismissed) {
        issues.set(id, { ...issue, dismissed: true, updatedAt: nowIso() });
        changed = true;
      }
    }
    return changed ? emitChange("dismiss-all") : current();
  }

  function dismissIssues(issueIds, reason = "dismiss-selected") {
    const ids = new Set((Array.isArray(issueIds) ? issueIds : [])
      .map((id) => trimString(id))
      .filter(Boolean));
    let changed = false;
    for (const id of ids) {
      const issue = issues.get(id);
      if (issue && issueIsWarning(issue) && !issue.dismissed) {
        issues.set(id, { ...issue, dismissed: true, updatedAt: nowIso() });
        changed = true;
      }
    }
    return changed ? emitChange(reason) : current();
  }

  function clearIssue(id) {
    const normalizedId = trimString(id);
    if (!issues.delete(normalizedId)) {
      return current();
    }
    return emitChange("clear");
  }

  function clearAll(reason) {
    clearAutoReportTimer();
    autoReportInFlight = false;
    lastAutoReportAt = "";
    lastAutoReportFailureReason = "";
    issues.clear();
    return emitChange(reason || "clear-all");
  }

  function syncSourceIssues(source, nextIssues) {
    const normalizedSource = trimString(source) || DEFAULT_SOURCE;
    const incoming = Array.isArray(nextIssues) ? nextIssues : [];
    const nextIds = new Set();
    for (const issue of incoming) {
      const normalizedIssue = normalizeIssue({
        ...issue,
        source: trimString(issue?.source) || normalizedSource,
      });
      nextIds.add(normalizedIssue.id);
      const previous = issues.get(normalizedIssue.id);
      issues.set(normalizedIssue.id, {
        ...(previous || normalizedIssue),
        ...normalizedIssue,
        createdAt: previous?.createdAt || normalizedIssue.createdAt,
        count: previous ? previous.count : normalizedIssue.count,
        dismissed: previous?.dismissed === true,
        autoReportStatus: normalizedIssue.autoReportStatus || previous?.autoReportStatus || "",
        autoReportAttemptedAt: normalizedIssue.autoReportAttemptedAt || previous?.autoReportAttemptedAt || "",
        autoReportedAt: normalizedIssue.autoReportedAt || previous?.autoReportedAt || "",
        autoReportReason: normalizedIssue.autoReportReason || previous?.autoReportReason || "",
      });
    }
    for (const [id, issue] of issues.entries()) {
      if (issue.source === normalizedSource && !nextIds.has(id)) {
        issues.delete(id);
      }
    }
    return emitChange("sync-source");
  }

  function clearSourceIssues(source) {
    const normalizedSource = trimString(source) || DEFAULT_SOURCE;
    let changed = false;
    for (const [id, issue] of issues.entries()) {
      if (issue.source === normalizedSource) {
        issues.delete(id);
        changed = true;
      }
    }
    return changed ? emitChange("clear-source") : current();
  }

  function registerSource(source, defaults = {}) {
    const normalizedSource = trimString(source) || DEFAULT_SOURCE;
    const baseIssue = isPlainObject(defaults) ? cloneJson(defaults) : {};
    function withSource(issue) {
      return {
        ...baseIssue,
        ...(isPlainObject(issue) ? issue : {}),
        source: trimString(issue?.source) || normalizedSource,
      };
    }
    return Object.freeze({
      source: normalizedSource,
      report(issue, options) {
        return upsertIssue(withSource(issue), options);
      },
      sync(nextIssues) {
        return syncSourceIssues(
          normalizedSource,
          (Array.isArray(nextIssues) ? nextIssues : []).map(withSource),
        );
      },
      clear(id) {
        return clearIssue(id);
      },
      clearAll() {
        return clearSourceIssues(normalizedSource);
      },
    });
  }

  function normalizeReportMode(value) {
    return trimString(value).toLowerCase() === "automatic" ? "automatic" : "manual";
  }

  function buildDiagnosticReport(selectedIssues, options = {}) {
    const reportId = randomId("health-report");
    const active = Array.isArray(selectedIssues) && selectedIssues.length
      ? selectedIssues
      : activeWarnings();
    const summary = active.length === 1
      ? active[0].title
      : `${active.length} page health warnings`;
    const mode = normalizeReportMode(options.mode);
    const trigger = compactText(options.trigger || "", 80);
    const clientSession = globalRef.__fishystuffClientSession;
    const draft = clientSession && typeof clientSession.createDiagnosticReportDraft === "function"
      ? clientSession.createDiagnosticReportDraft({ summary, category: REPORT_CATEGORY })
      : {
          createdAt: nowIso(),
          page: {
            href: trimString(globalRef.location?.href),
            path: trimString(globalRef.location?.pathname),
          },
          report: { summary, category: REPORT_CATEGORY },
        };
    return {
      ...draft,
      report: {
        ...(draft.report || {}),
        id: reportId,
        summary,
        category: REPORT_CATEGORY,
        issueCount: active.length,
        mode,
        automatic: mode === "automatic",
        ...(trigger ? { trigger } : {}),
      },
      health: {
        hiddenByUserSetting: hiddenByUserSetting(),
        autoReportEnabled: autoReportEnabledByUserSetting(),
        issues: active.map((issue) => ({
          id: issue.id,
          severity: issue.severity,
          source: issue.source,
          category: issue.category,
          title: issue.title,
          detail: issue.detail,
          count: issue.count,
          autoReportStatus: issue.autoReportStatus,
          createdAt: issue.createdAt,
          updatedAt: issue.updatedAt,
          context: issue.context,
        })),
      },
    };
  }

  function reportFailureMessage(reason, telemetry) {
    if (reason === "telemetry-export-failed" || telemetry?.lastLogExport?.ok === false) {
      return "Telemetry report could not be delivered.";
    }
    if (reason === "opt-in-required" || reason === "disabled-by-user" || reason === "disabled-by-query") {
      return "Telemetry is off for this page.";
    }
    if (reason === "logs-disabled" || reason === "missing-logs-exporter" || reason === "logs-unavailable") {
      return "Telemetry logs are unavailable right now.";
    }
    return "Telemetry is unavailable right now.";
  }

  function markIssueReportState(selected, patch, reason) {
    const ids = new Set((Array.isArray(selected) ? selected : [])
      .map((issue) => trimString(issue?.id))
      .filter(Boolean));
    let changed = false;
    for (const id of ids) {
      const issue = issues.get(id);
      if (!issue) {
        continue;
      }
      issues.set(id, {
        ...issue,
        ...patch,
        updatedAt: nowIso(),
      });
      changed = true;
    }
    return changed ? emitChange(reason) : current();
  }

  async function sendDiagnosticReport(selectedIssues, options = {}) {
    const selected = reportableWarnings(selectedIssues);
    const mode = normalizeReportMode(options.mode);
    const notify = options.notify !== false;
    const dismissOnSuccess = options.dismissOnSuccess !== false;
    if (!selected.length) {
      return { sent: false, reason: "no-active-warnings", report: null, telemetry: resolveTelemetryStatus() };
    }
    const report = buildDiagnosticReport(selected, {
      mode,
      trigger: options.trigger,
    });
    const clientSession = globalRef.__fishystuffClientSession;
    clientSession?.markDiagnosticReportPrepared?.();
    const otel = globalRef.__fishystuffOtel;
    if (!otel || typeof otel.emitDiagnosticReport !== "function") {
      const telemetry = resolveTelemetryStatus();
      if (notify) {
        showToast("error", reportFailureMessage("telemetry-unavailable", telemetry));
      }
      emitChange("report-failed");
      return { sent: false, reason: "telemetry-unavailable", report, telemetry };
    }
    let result;
    try {
      result = await otel.emitDiagnosticReport(report, {
        mode,
        trigger: trimString(options.trigger),
      });
    } catch (_error) {
      result = {
        sent: false,
        reason: "telemetry-export-failed",
        status: {
          ...resolveTelemetryStatus(),
          lastLogExport: {
            ok: false,
            error: "diagnostic report export failed",
          },
        },
      };
    }
    const telemetry = normalizeTelemetryStatus(result?.status || resolveTelemetryStatus());
    if (result?.sent !== true) {
      const reason = trimString(result?.reason) || "telemetry-unavailable";
      if (notify) {
        showToast("error", reportFailureMessage(reason, telemetry));
      }
      emitChange("report-failed");
      return { sent: false, reason, report, telemetry };
    }
    if (dismissOnSuccess) {
      dismissIssues(selected.map((issue) => issue.id), "report-sent");
    } else {
      emitChange("report-sent");
    }
    clientSession?.markDiagnosticReportSubmitted?.();
    if (notify) {
      showToast("success", "Diagnostic report sent.");
    }
    return { sent: true, reason: "sent", report, telemetry };
  }

  async function sendManualReport() {
    return sendDiagnosticReport(activeWarnings(), {
      mode: "manual",
      dismissOnSuccess: true,
      notify: true,
      trigger: "manual",
    });
  }

  async function sendAutomaticReport(trigger = "automatic") {
    clearAutoReportTimer();
    const telemetry = resolveTelemetryStatus();
    if (!autoReportEnabledByUserSetting()) {
      return { sent: false, reason: "auto-report-disabled", report: null, telemetry };
    }
    if (!telemetry.diagnosticReportsEnabled) {
      return { sent: false, reason: telemetry.reason || "telemetry-unavailable", report: null, telemetry };
    }
    if (autoReportInFlight) {
      return { sent: false, reason: "auto-report-in-flight", report: null, telemetry };
    }
    const selected = autoReportCandidateIssues(activeWarnings());
    if (!selected.length) {
      return { sent: false, reason: "no-active-warnings", report: null, telemetry };
    }
    autoReportInFlight = true;
    const attemptedAt = nowIso();
    markIssueReportState(selected, {
      autoReportStatus: "pending",
      autoReportAttemptedAt: attemptedAt,
      autoReportReason: "sending",
    }, "auto-report-pending");
    const result = await sendDiagnosticReport(selected, {
      mode: "automatic",
      dismissOnSuccess: false,
      notify: false,
      trigger,
    });
    autoReportInFlight = false;
    if (result.sent) {
      lastAutoReportAt = nowIso();
      lastAutoReportFailureReason = "";
      markIssueReportState(selected, {
        autoReportStatus: "sent",
        autoReportedAt: lastAutoReportAt,
        autoReportReason: "sent",
      }, "auto-report-sent");
      return result;
    }
    lastAutoReportFailureReason = trimString(result.reason) || "telemetry-unavailable";
    markIssueReportState(selected, {
      autoReportStatus: "failed",
      autoReportReason: lastAutoReportFailureReason,
    }, "auto-report-failed");
    return result;
  }

  function showToast(tone, message) {
    const toast = globalRef.__fishystuffToast;
    if (!toast || !message) {
      return;
    }
    if (typeof toast[tone] === "function") {
      toast[tone](message);
      return;
    }
    if (typeof toast.show === "function") {
      toast.show({ tone, message });
    }
  }

  const navRoots = new Set();

  function text(node, value) {
    if (node) {
      node.textContent = value;
    }
  }

  function severityBadgeClass(severity) {
    if (severity === "error") {
      return "badge badge-error badge-sm";
    }
    if (severity === "warning") {
      return "badge badge-warning badge-sm";
    }
    return "badge badge-info badge-sm";
  }

  function telemetryBadgeClass(tone) {
    if (tone === "success") {
      return "badge badge-success badge-sm";
    }
    if (tone === "info") {
      return "badge badge-info badge-sm";
    }
    if (tone === "warning") {
      return "badge badge-warning badge-sm";
    }
    if (tone === "error") {
      return "badge badge-error badge-sm";
    }
    return "badge badge-neutral badge-sm";
  }

  function autoReportIssueBadge(issue) {
    if (issue.autoReportStatus === "sent") {
      return { label: "Auto reported", className: "badge badge-success badge-soft badge-sm" };
    }
    if (issue.autoReportStatus === "pending") {
      return { label: "Report queued", className: "badge badge-info badge-soft badge-sm" };
    }
    if (issue.autoReportStatus === "failed") {
      return { label: "Report failed", className: "badge badge-error badge-soft badge-sm" };
    }
    return null;
  }

  function renderIssueList(root, snapshot) {
    const list = root.querySelector("[data-health-issue-list]");
    if (!list) {
      return;
    }
    list.replaceChildren();
    const ownerDocument = root.ownerDocument || globalRef.document;
    for (const issue of snapshot.activeWarnings.slice(0, 5)) {
      const row = ownerDocument.createElement("div");
      row.className = "rounded-box border border-base-300 bg-base-200/45 p-3";

      const header = ownerDocument.createElement("div");
      header.className = "flex items-start justify-between gap-3";
      row.appendChild(header);

      const body = ownerDocument.createElement("div");
      body.className = "min-w-0";
      header.appendChild(body);

      const title = ownerDocument.createElement("div");
      title.className = "break-words text-sm font-semibold text-base-content";
      title.textContent = issue.title;
      body.appendChild(title);

      if (issue.detail) {
        const detail = ownerDocument.createElement("p");
        detail.className = "mt-1 break-words text-xs leading-5 text-base-content/65";
        detail.textContent = issue.detail;
        body.appendChild(detail);
      }

      const meta = ownerDocument.createElement("div");
      meta.className = "mt-2 flex min-w-0 flex-wrap items-center gap-2";
      body.appendChild(meta);

      const severity = ownerDocument.createElement("span");
      severity.className = severityBadgeClass(issue.severity);
      severity.textContent = issue.severity;
      meta.appendChild(severity);

      const source = ownerDocument.createElement("span");
      source.className = "badge badge-outline badge-sm";
      source.textContent = issue.source;
      meta.appendChild(source);

      if (issue.count > 1) {
        const count = ownerDocument.createElement("span");
        count.className = "badge badge-ghost badge-sm";
        count.textContent = `${issue.count}x`;
        meta.appendChild(count);
      }

      const reportBadge = autoReportIssueBadge(issue);
      if (reportBadge) {
        const badge = ownerDocument.createElement("span");
        badge.className = reportBadge.className;
        badge.textContent = reportBadge.label;
        if (issue.autoReportReason) {
          badge.title = issue.autoReportReason;
        }
        meta.appendChild(badge);
      }

      const dismissButton = ownerDocument.createElement("button");
      dismissButton.type = "button";
      dismissButton.className = "btn btn-ghost btn-xs shrink-0";
      dismissButton.textContent = "Dismiss";
      dismissButton.setAttribute("data-health-dismiss-id", issue.id);
      header.appendChild(dismissButton);

      list.appendChild(row);
    }
  }

  function renderTelemetryStatus(root, snapshot) {
    const telemetry = snapshot.telemetry || resolveTelemetryStatus();
    const autoReport = snapshot.autoReport || autoReportSnapshot(snapshot.activeWarnings || [], telemetry);
    const row = root.querySelector("[data-health-telemetry-row]");
    if (row) {
      row.classList.toggle("border-error/35", telemetry.tone === "error");
      row.classList.toggle("border-success/25", telemetry.tone === "success");
      row.classList.toggle("border-warning/30", telemetry.tone === "warning");
    }
    const label = root.querySelector("[data-health-telemetry-label]");
    if (label) {
      label.className = telemetryBadgeClass(telemetry.tone);
      label.textContent = telemetry.label;
    }
    text(root.querySelector("[data-health-telemetry-detail]"), telemetry.detail);
    const autoLabel = root.querySelector("[data-health-auto-report-state]");
    if (autoLabel) {
      autoLabel.className = telemetryBadgeClass(autoReport.tone);
      autoLabel.textContent = autoReport.label;
      autoLabel.title = autoReport.detail;
    }
    text(root.querySelector("[data-health-auto-report-detail]"), autoReport.detail);

    root.querySelectorAll("[data-health-send-report]").forEach((button) => {
      const canSend = telemetry.diagnosticReportsEnabled === true
        && snapshot.activeWarnings.some((issue) => issue.reportable !== false);
      button.disabled = canSend !== true;
      button.title = canSend
        ? "Send active warnings through telemetry"
        : telemetry.detail;
    });
    root.querySelectorAll("[data-health-auto-report-toggle]").forEach((toggle) => {
      toggle.checked = autoReport.enabledByUser === true;
    });
  }

  function renderRoot(root, snapshot) {
    const navVisible = snapshot.navMenuVisible;
    root.classList.toggle("hidden", !navVisible);
    root.hidden = !navVisible;
    root.querySelectorAll("[data-health-count]").forEach((node) => {
      text(node, String(snapshot.activeWarningCount));
    });
    text(root.querySelector("[data-health-summary]"), snapshot.activeWarningCount === 1
      ? "1 active warning"
      : `${snapshot.activeWarningCount} active warnings`);
    text(root.querySelector("[data-health-hidden-label]"), snapshot.hiddenByUserSetting
      ? "Hidden in settings"
      : "Visible when warnings are active");
    renderIssueList(root, snapshot);
    renderTelemetryStatus(root, snapshot);

    root.querySelectorAll("[data-health-menu-toggle]").forEach((toggle) => {
      toggle.checked = snapshot.hiddenByUserSetting !== true;
    });
    root.querySelectorAll("[data-health-auto-report-toggle]").forEach((toggle) => {
      toggle.checked = snapshot.autoReport?.enabledByUser === true;
    });
  }

  function renderGlobalToggles(snapshot) {
    globalRef.document?.querySelectorAll?.("[data-health-menu-toggle]").forEach((toggle) => {
      toggle.checked = snapshot.hiddenByUserSetting !== true;
    });
    globalRef.document?.querySelectorAll?.("[data-health-auto-report-toggle]").forEach((toggle) => {
      toggle.checked = snapshot.autoReport?.enabledByUser === true;
    });
  }

  function renderAll(snapshot = current()) {
    for (const root of navRoots) {
      renderRoot(root, snapshot);
    }
    renderGlobalToggles(snapshot);
  }

  function handleNavClick(event) {
    const dismissButton = event.target.closest?.("[data-health-dismiss-id]");
    if (dismissButton) {
      dismissIssue(dismissButton.getAttribute("data-health-dismiss-id"));
      return;
    }
    if (event.target.closest?.("[data-health-dismiss-all]")) {
      dismissAll();
      return;
    }
    const sendButton = event.target.closest?.("[data-health-send-report]");
    if (sendButton) {
      const root = event.currentTarget;
      sendButton.disabled = true;
      Promise.resolve(sendManualReport()).finally(() => {
        renderRoot(root, current());
      });
    }
  }

  function handleToggleChange(event) {
    const autoReportToggle = event.target.closest?.("[data-health-auto-report-toggle]");
    if (autoReportToggle) {
      setAutoReportEnabled(autoReportToggle.checked === true);
      return;
    }
    const toggle = event.target.closest?.("[data-health-menu-toggle]");
    if (!toggle) {
      return;
    }
    setMenuHidden(toggle.checked !== true);
  }

  function bindNav(root) {
    if (!root || typeof root.querySelector !== "function") {
      return null;
    }
    navRoots.add(root);
    root.addEventListener("click", handleNavClick);
    renderRoot(root, current());
    return root;
  }

  function bindDocument(rootDocument = globalRef.document) {
    if (!rootDocument || typeof rootDocument.querySelectorAll !== "function") {
      return;
    }
    rootDocument.querySelectorAll("[data-page-health-menu]").forEach(bindNav);
    rootDocument.addEventListener("change", handleToggleChange);
    renderAll();
  }

  function subscribe(listener) {
    if (typeof listener !== "function") {
      return function () {};
    }
    subscribers.add(listener);
    listener(current(), "subscribe");
    return function () {
      subscribers.delete(listener);
    };
  }

  function issueFromErrorEvent(event) {
    const error = event?.error;
    const message = trimString(error?.message || event?.message || "Unhandled page error");
    const location = [event?.filename, event?.lineno, event?.colno]
      .map((part) => trimString(part))
      .filter(Boolean)
      .join(":");
    return {
      id: `browser:error:${message}:${location}`.toLowerCase(),
      severity: "error",
      source: "browser",
      category: "runtime",
      title: "Page script error",
      detail: location ? `${message} (${location})` : message,
      context: {
        filename: trimString(event?.filename),
        lineno: event?.lineno || null,
        colno: event?.colno || null,
        stack: compactText(error?.stack || "", 2000),
      },
    };
  }

  function issueFromRejection(event) {
    const reason = event?.reason;
    const message = trimString(reason?.message || reason || "Unhandled promise rejection");
    return {
      id: `browser:rejection:${message}`.toLowerCase(),
      severity: "error",
      source: "browser",
      category: "runtime",
      title: "Unhandled page error",
      detail: message,
      context: {
        stack: compactText(reason?.stack || "", 2000),
      },
    };
  }

  globalRef.addEventListener?.("error", (event) => {
    upsertIssue(issueFromErrorEvent(event));
  });
  globalRef.addEventListener?.("unhandledrejection", (event) => {
    upsertIssue(issueFromRejection(event));
  });
  globalRef.addEventListener?.("storage", (event) => {
    if (event.key === SETTINGS_KEY) {
      emitChange("settings-storage");
    }
  });
  globalRef.addEventListener?.("fishystuff:uisettingschange", (event) => {
    if (
      !event?.detail?.path
      || event.detail.path === SETTINGS_PATH
      || event.detail.path === AUTO_REPORT_SETTING_PATH
    ) {
      emitChange("settings-change");
    }
  });
  globalRef.addEventListener?.(TELEMETRY_STATUS_EVENT, () => {
    emitChange("telemetry-status");
  });

  if (globalRef.document?.readyState === "loading") {
    globalRef.document.addEventListener("DOMContentLoaded", () => bindDocument(), { once: true });
  } else {
    bindDocument();
  }

  globalRef.__fishystuffPageHealth = Object.freeze({
    CHANGE_EVENT,
    AUTO_REPORT_SETTING_PATH,
    SETTINGS_KEY,
    SETTINGS_PATH,
    TELEMETRY_STATUS_EVENT,
    bindDocument,
    bindNav,
    buildDiagnosticReport,
    clearAll,
    clearIssue,
    current,
    dismissAll,
    dismissIssues,
    dismissIssue,
    registerSource,
    reportIssue: upsertIssue,
    sendAutomaticReport,
    sendDiagnosticReport,
    sendManualReport,
    setAutoReportEnabled,
    setMenuHidden,
    subscribe,
    syncSourceIssues,
  });
})(typeof window !== "undefined" ? window : globalThis);
