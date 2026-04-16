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

function normalizeOperator(value) {
  return String(value ?? "").trim().toLowerCase() === "and" ? "and" : "or";
}

function normalizeNegated(value) {
  return value === true;
}

function normalizeAllowNegation(value) {
  return value !== false;
}

function normalizeExpressionNode(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  const type = String(value.type || "").trim().toLowerCase();
  if (type === "term") {
    return {
      type: "term",
      key: String(value.key || value.label || "").trim(),
      path: String(value.path || "").trim(),
      label: String(value.label || "").trim(),
      kindLabel: String(value.kindLabel || "").trim(),
      description: String(value.description || "").trim(),
      contentMarkup: String(value.contentMarkup || "").trim(),
      grade: String(value.grade || "").trim(),
      allowNegation: normalizeAllowNegation(value.allowNegation ?? value.showNegationToggle),
      negated: normalizeNegated(value.negated ?? value.not ?? value.inverted),
      removeLabel: String(value.removeLabel || "").trim(),
      removeAttributes: normalizeAttributes(value.removeAttributes),
    };
  }

  if (type !== "group") {
    return null;
  }

  return {
    type: "group",
    key: String(value.key || value.path || "group").trim(),
    path: String(value.path || "").trim(),
    label: String(value.label || "").trim(),
    description: String(value.description || "").trim(),
    operator: normalizeOperator(value.operator),
    negated: normalizeNegated(value.negated ?? value.not ?? value.inverted),
    children: (Array.isArray(value.children) ? value.children : [])
      .map((child) => normalizeExpressionNode(child))
      .filter((child) => child && !(child.type === "group" && child.children.length === 0)),
  };
}

function countLeafTerms(node) {
  if (!node) {
    return 0;
  }
  if (node.type === "term") {
    return 1;
  }
  return node.children.reduce((total, child) => total + countLeafTerms(child), 0);
}

function buildRenderKey(node) {
  if (!node) {
    return null;
  }
  if (node.type === "term") {
    return [
      "term",
      node.key,
      node.path,
      node.label,
      node.kindLabel,
      node.description,
      node.contentMarkup,
      node.grade,
      node.allowNegation,
      node.negated,
      node.removeLabel,
      Object.entries(node.removeAttributes),
    ];
  }
  return [
    "group",
    node.key,
    node.path,
    node.label,
    node.description,
    node.operator,
    node.negated,
    node.children.map((child) => buildRenderKey(child)),
  ];
}

function renderOperatorBadge(operator, escapeHtml, options = {}) {
  const compact = options.compact === true;
  const toneClass = operator === "and" ? "badge-soft" : "badge-ghost";
  const sizeClass = compact ? "badge-xs" : "badge-sm";
  const groupPath = String(options.groupPath || "").trim();
  const boundaryIndex = Number.parseInt(options.boundaryIndex, 10);
  const nextOperator = operator === "and" ? "or" : "and";
  const baseClass = `fishy-applied-expression-operator badge ${toneClass} ${sizeClass} uppercase tracking-[0.24em]`;
  if (!groupPath || !Number.isInteger(boundaryIndex)) {
    return `<span class="${baseClass}">${escapeHtml(operator)}</span>`;
  }
  return `
    <button
      class="${baseClass} fishy-applied-expression-operator-toggle cursor-pointer"
      type="button"
      data-expression-group-path="${escapeHtml(groupPath)}"
      data-expression-boundary-index="${escapeHtml(boundaryIndex)}"
      data-expression-drop-slot-group-path="${escapeHtml(groupPath)}"
      data-expression-drop-slot-index="${escapeHtml(boundaryIndex)}"
      data-expression-next-operator="${escapeHtml(nextOperator)}"
      aria-label="${escapeHtml(`Change group operator to ${nextOperator.toUpperCase()}`)}"
      title="${escapeHtml(`Change group operator to ${nextOperator.toUpperCase()}`)}"
    >
      ${escapeHtml(operator)}
    </button>
  `;
}

function renderNegationToggle(path, negated, escapeHtml, options = {}) {
  const normalizedPath = String(path || "").trim();
  if (!normalizedPath) {
    return "";
  }
  const label = String(options.label || "condition").trim() || "condition";
  const extraClass = String(options.className || "").trim();
  const baseClass = [
    "fishy-applied-expression-negate-toggle",
    "badge",
    negated ? "badge-neutral" : "badge-ghost",
    "badge-xs",
    "cursor-pointer",
    "shrink-0",
    "uppercase",
    "tracking-[0.24em]",
    extraClass,
  ]
    .filter(Boolean)
    .join(" ");
  const title = negated ? `Remove NOT from ${label}` : `Negate ${label}`;
  return `
    <button
      class="${baseClass}"
      type="button"
      data-expression-negate-path="${escapeHtml(normalizedPath)}"
      aria-pressed="${negated ? "true" : "false"}"
      aria-label="${escapeHtml(title)}"
      title="${escapeHtml(title)}"
    >
      ${escapeHtml("not")}
    </button>
  `;
}

function renderTermNode(node, escapeHtml, buttonClass) {
  const label = node.label || "Applied term";
  const removeLabel = node.removeLabel || `Remove ${label}`;
  const negateToggleMarkup = node.allowNegation
    ? renderNegationToggle(node.path, node.negated, escapeHtml, {
      label,
      className: "join-item",
    })
    : "";
  return `
    <div
      class="fishy-applied-term join items-stretch max-w-full cursor-grab"
      role="listitem"
      draggable="true"
      data-expression-node-kind="term"
      data-expression-path="${escapeHtml(node.path)}"
      data-expression-drag-path="${escapeHtml(node.path)}"
      data-expression-drop-node-path="${escapeHtml(node.path)}"
      data-expression-drop-term-path="${escapeHtml(node.path)}"
      data-expression-key="${escapeHtml(node.key || label)}"${
        node.grade ? ` data-grade="${escapeHtml(node.grade)}"` : ""
      }${node.negated && node.allowNegation ? ' data-expression-negated="true"' : ""}
    >
      ${negateToggleMarkup}
      <div class="fishy-applied-term-main join-item">
        ${
          node.kindLabel
            ? `<span class="badge badge-soft badge-xs fishy-applied-term-kind">${escapeHtml(node.kindLabel)}</span>`
            : ""
        }
        <div class="fishy-applied-term-body">
          ${node.contentMarkup || `<span class="truncate">${escapeHtml(label)}</span>`}
        </div>
        ${
          node.description
            ? `<span class="fishy-applied-term-description text-xs text-base-content/70" title="${escapeHtml(node.description)}">${escapeHtml(node.description)}</span>`
            : ""
        }
      </div>
      <button
        class="${escapeHtml(buttonClass)} join-item"
        type="button"
        aria-label="${escapeHtml(removeLabel)}"
        data-expression-remove-path="${escapeHtml(node.path)}"${renderAttributes(node.removeAttributes, escapeHtml)}
      >
        ×
      </button>
    </div>
  `;
}

function renderInsertionSlot(groupPath, childIndex, escapeHtml) {
  return `
    <span
      class="fishy-applied-expression-slot"
      aria-hidden="true"
      data-expression-drop-slot-group-path="${escapeHtml(groupPath)}"
      data-expression-drop-slot-index="${escapeHtml(childIndex)}"
      title="${escapeHtml(`Insert at position ${Number(childIndex) + 1}`)}"
    ></span>
  `;
}

function renderGroupNode(node, escapeHtml, buttonClass, options = {}) {
  const isRoot = options.isRoot === true;
  const children = node.children.filter(Boolean);
  const operator = normalizeOperator(node.operator);
  const negationMarkup = renderNegationToggle(node.path, node.negated, escapeHtml, {
    label: isRoot ? "expression" : node.label || "group",
  });
  const leadingMarkup = isRoot
    ? ""
    : `
      <span
        class="fishy-applied-expression-group-handle cursor-grab text-base-content/40"
        draggable="true"
        data-expression-node-kind="group"
        data-expression-path="${escapeHtml(node.path)}"
        data-expression-drag-path="${escapeHtml(node.path)}"
        data-expression-drop-node-path="${escapeHtml(node.path)}"
        title="${escapeHtml("Drag group")}"
      >(</span>
    `;
  const trailingMarkup = isRoot ? "" : '<span class="fishy-applied-expression-bracket text-base-content/40" aria-hidden="true">)</span>';

  const childMarkup = children
    .map((child, index) => {
      const renderedChild =
        child.type === "group"
          ? renderGroupNode(child, escapeHtml, buttonClass)
          : renderTermNode(child, escapeHtml, buttonClass);
      if (index === 0) {
        return `${renderInsertionSlot(node.path, 0, escapeHtml)}${renderedChild}`;
      }
      return `${renderOperatorBadge(operator, escapeHtml, {
        compact: true,
        groupPath: node.path,
        boundaryIndex: index,
      })}${renderedChild}`;
    })
    .join("") + renderInsertionSlot(node.path, children.length, escapeHtml);

  return `
    <div
      class="fishy-applied-expression-group inline-flex max-w-full flex-wrap items-center gap-2"
      data-expression-node-kind="group"
      data-expression-path="${escapeHtml(node.path)}"
      data-expression-drop-group-path="${escapeHtml(node.path)}"
      data-expression-operator="${escapeHtml(operator)}"
      ${node.negated ? 'data-expression-negated="true"' : ""}
    >
      ${negationMarkup}
      ${leadingMarkup}
      ${childMarkup}
      ${trailingMarkup}
    </div>
  `;
}

export function buildAppliedSearchTermsView(expression, options = {}) {
  const escapeHtml =
    typeof options.escapeHtml === "function" ? options.escapeHtml : defaultEscapeHtml;
  const removeButtonClass = String(options.removeButtonClass || "").trim();
  const normalizedExpression = normalizeExpressionNode(expression);
  const rootNode =
    normalizedExpression?.type === "group"
      ? normalizedExpression
      : normalizedExpression
        ? {
            type: "group",
            key: "root",
            path: "root",
            label: "",
            description: "",
            operator: "or",
            children: [normalizedExpression],
          }
        : null;
  const leafCount = countLeafTerms(rootNode);
  const renderKey = JSON.stringify(buildRenderKey(rootNode));

  if (!rootNode || !leafCount) {
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
    <section
      class="fishy-applied-expression max-w-full"
      data-expression-node-kind="group"
      data-expression-path="${escapeHtml(rootNode.path || "root")}"
      data-expression-operator="${escapeHtml(rootNode.operator)}"
    >
      ${renderGroupNode(rootNode, escapeHtml, buttonClass, { isRoot: true })}
    </section>
  `;

  return {
    hasContent: true,
    html,
    renderKey,
  };
}
