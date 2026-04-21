const DEFAULT_DOCUMENT_TITLE_SUFFIX = "FishyStuff";

function trimString(value) {
  return String(value ?? "").trim();
}

export function resolveDeploymentName(env = process.env) {
  return trimString(
    env.FISHYSTUFF_DEPLOYMENT_ENVIRONMENT
      ?? env.FISHYSTUFF_RUNTIME_DEPLOYMENT_NAME
      ?? env.FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT,
  ) || "production";
}

export function formatDeploymentTitleLabel(value) {
  const normalized = trimString(value);
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

export function resolveDocumentTitleSuffix(env = process.env) {
  const deploymentLabel = formatDeploymentTitleLabel(resolveDeploymentName(env));
  if (deploymentLabel) {
    return `${DEFAULT_DOCUMENT_TITLE_SUFFIX} (${deploymentLabel})`;
  }
  return DEFAULT_DOCUMENT_TITLE_SUFFIX;
}

export function stripDocumentTitleSuffix(value, suffix = DEFAULT_DOCUMENT_TITLE_SUFFIX) {
  const normalized = trimString(value);
  const knownSuffixes = [trimString(suffix), DEFAULT_DOCUMENT_TITLE_SUFFIX];
  for (const candidate of knownSuffixes) {
    if (!candidate) {
      continue;
    }
    const brandTail = ` | ${candidate}`;
    if (normalized.endsWith(brandTail)) {
      return trimString(normalized.slice(0, -brandTail.length));
    }
  }
  return normalized;
}

export function buildDocumentTitle(pageTitle, suffix = DEFAULT_DOCUMENT_TITLE_SUFFIX) {
  const normalizedSuffix = trimString(suffix) || DEFAULT_DOCUMENT_TITLE_SUFFIX;
  const normalizedTitle = stripDocumentTitleSuffix(pageTitle, normalizedSuffix);
  if (!normalizedTitle || normalizedTitle === normalizedSuffix) {
    return normalizedSuffix;
  }
  return `${normalizedTitle} | ${normalizedSuffix}`;
}
