function defaultEscapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
    (char) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[char] || char,
  );
}

function normalizeAttributes(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return Object.fromEntries(
    Object.entries(value)
      .filter(([name, attributeValue]) => String(name || "").trim() && attributeValue != null)
      .sort(([left], [right]) => left.localeCompare(right)),
  );
}

function renderAttributes(attributes, escapeHtml) {
  return Object.entries(normalizeAttributes(attributes))
    .map(([name, value]) => ` ${escapeHtml(name)}="${escapeHtml(value)}"`)
    .join("");
}

function normalizeGroupItems(items) {
  if (!Array.isArray(items)) {
    return [];
  }
  return items.filter((item) => item && typeof item === "object");
}

export function buildAppliedSearchTermsView(groups, options = {}) {
  const escapeHtml =
    typeof options.escapeHtml === "function" ? options.escapeHtml : defaultEscapeHtml;
  const removeButtonClass = String(options.removeButtonClass || "").trim();
  const activeGroups = (Array.isArray(groups) ? groups : [])
    .filter((group) => group && typeof group === "object")
    .map((group) => ({
      key: String(group.key || group.label || "").trim(),
      label: String(group.label || "").trim(),
      description: String(group.description || "").trim(),
      iconMarkup: String(group.iconMarkup || "").trim(),
      items: normalizeGroupItems(group.items),
    }))
    .filter((group) => group.items.length > 0);

  const renderKey = JSON.stringify(
    activeGroups.map((group) => [
      group.key,
      group.label,
      group.description,
      group.iconMarkup,
      group.items.map((item) => [
        String(item.key || item.label || ""),
        String(item.label || ""),
        String(item.kindLabel || ""),
        String(item.description || ""),
        String(item.contentMarkup || ""),
        String(item.grade || ""),
        String(item.removeLabel || ""),
        Object.entries(normalizeAttributes(item.removeAttributes)),
      ]),
    ]),
  );

  if (!activeGroups.length) {
    return {
      hasContent: false,
      html: "",
      renderKey,
    };
  }

  const buttonClass = [
    "fishy-applied-term-remove",
    "btn",
    "btn-ghost",
    "btn-xs",
    "btn-circle",
    "h-7",
    "min-h-0",
    "w-7",
    "border-0",
    "text-base-content/70",
    removeButtonClass,
  ]
    .filter(Boolean)
    .join(" ");

  const html = `
    <div class="fishy-applied-terms">
      ${activeGroups
        .map((group) => {
          const groupLabel = group.label || "Applied";
          return `
            <section class="fishy-applied-terms-group" data-applied-group="${escapeHtml(group.key || groupLabel.toLowerCase())}">
              <header class="fishy-applied-terms-group-header">
                <span class="fishy-applied-terms-group-title">
                  ${group.iconMarkup}
                  <span>${escapeHtml(groupLabel)}</span>
                </span>
                <span class="badge badge-ghost badge-xs">${group.items.length}</span>
              </header>
              ${
                group.description
                  ? `<p class="fishy-applied-terms-group-description">${escapeHtml(group.description)}</p>`
                  : ""
              }
              <div class="fishy-applied-terms-group-list" role="list">
                ${group.items
                  .map((item) => {
                    const label = String(item.label || "").trim() || "Applied term";
                    const contentMarkup = String(item.contentMarkup || "").trim();
                    const grade = String(item.grade || "").trim();
                    const kindLabel = String(item.kindLabel || "").trim();
                    const description = String(item.description || "").trim();
                    const removeLabel =
                      String(item.removeLabel || "").trim() || `Remove ${label}`;
                    return `
                      <div class="fishy-applied-term" role="listitem"${
                        grade ? ` data-grade="${escapeHtml(grade)}"` : ""
                      }>
                        <span class="fishy-applied-term-main">
                          ${
                            kindLabel
                              ? `<span class="badge badge-outline badge-xs fishy-applied-term-kind">${escapeHtml(kindLabel)}</span>`
                              : ""
                          }
                          <span class="fishy-applied-term-body">
                            ${contentMarkup || `<span class="truncate">${escapeHtml(label)}</span>`}
                          </span>
                          ${
                            description
                              ? `<span class="fishy-applied-term-description" title="${escapeHtml(description)}">${escapeHtml(description)}</span>`
                              : ""
                          }
                        </span>
                        <button
                          class="${escapeHtml(buttonClass)}"
                          type="button"
                          aria-label="${escapeHtml(removeLabel)}"${renderAttributes(item.removeAttributes, escapeHtml)}
                        >
                          ×
                        </button>
                      </div>
                    `;
                  })
                  .join("")}
              </div>
            </section>
          `;
        })
        .join("")}
    </div>
  `;

  return {
    hasContent: true,
    html,
    renderKey,
  };
}
