export const DEFAULT_DOCUMENT_TITLE_SUFFIX = "FishyStuff";

function normalizeString(value) {
  return String(value ?? "").trim();
}

function normalizeHostname(value) {
  const normalized = normalizeString(value);
  if (!normalized) {
    return "";
  }
  try {
    return new URL(normalized).hostname.toLowerCase();
  } catch {
    return "";
  }
}

export function isBetaDeploymentSite(baseUrl) {
  const hostname = normalizeHostname(baseUrl);
  return hostname === "beta.fishystuff.fish" || hostname.startsWith("beta.");
}

export function formatDeploymentTitleLabel(value) {
  const normalized = normalizeString(value);
  if (!normalized) {
    return "";
  }

  const lower = normalized.toLowerCase();
  if (lower === "production" || lower === "prod") {
    return "";
  }

  return normalized
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .split(" ")
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

export function resolveDocumentTitleSuffix(config = {}) {
  const configuredTitleSuffix = normalizeString(
    config.documentTitleSuffix
      ?? config.FISHYSTUFF_RUNTIME_DOCUMENT_TITLE_SUFFIX
      ?? config.FISHYSTUFF_PUBLIC_DOCUMENT_TITLE_SUFFIX,
  );
  if (configuredTitleSuffix) {
    return configuredTitleSuffix;
  }

  const deploymentLabel = formatDeploymentTitleLabel(
    config.deploymentName
      ?? config.deploymentEnvironment
      ?? config.FISHYSTUFF_DEPLOYMENT_ENVIRONMENT
      ?? config.FISHYSTUFF_RUNTIME_DEPLOYMENT_NAME
      ?? config.FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT,
  );
  if (deploymentLabel) {
    return `${DEFAULT_DOCUMENT_TITLE_SUFFIX} (${deploymentLabel})`;
  }

  return DEFAULT_DOCUMENT_TITLE_SUFFIX;
}
