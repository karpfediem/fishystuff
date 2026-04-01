import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";

function cloneNodeList(nodes) {
  return Array.from(nodes, (node) => node.cloneNode(true));
}

function escapeHtml(value) {
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

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizePatchId(value) {
  const normalized = String(value ?? "").trim();
  return normalized || "";
}

function normalizePatchLabel(patchId, patchName) {
  const normalizedName = String(patchName ?? "").trim();
  return normalizedName || patchId;
}

function normalizePatchStartTs(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
}

function buildPatchSummarySearchText(patch) {
  return [patch.label, patch.patchId].filter(Boolean).join(" ");
}

function createPatchTemplate(documentRef, patch) {
  const template = documentRef.createElement("template");
  template.dataset.role = "selected-content";
  template.dataset.value = patch.patchId;
  template.dataset.label = patch.label;
  template.dataset.searchText = buildPatchSummarySearchText(patch);
  template.innerHTML = `
    <span class="flex min-w-0 flex-1 items-center gap-3 text-sm">
      <span class="truncate font-medium">${escapeHtml(patch.label)}</span>
      ${
        patch.label !== patch.patchId
          ? `<span class="badge badge-ghost badge-xs shrink-0">${escapeHtml(patch.patchId)}</span>`
          : ""
      }
    </span>
  `;
  return template;
}

function createSimpleTemplate(documentRef, { value, label, searchText = "" }) {
  const template = documentRef.createElement("template");
  template.dataset.role = "selected-content";
  template.dataset.value = String(value ?? "");
  template.dataset.label = String(label ?? "").trim();
  template.dataset.searchText = String(searchText || label || value || "").trim();
  template.innerHTML = `
    <span class="flex min-w-0 flex-1 items-center gap-3 text-sm">
      <span class="truncate font-medium">${escapeHtml(label)}</span>
    </span>
  `;
  return template;
}

function placeholderMarkup(label, { loading = false } = {}) {
  if (loading) {
    return `
      <span class="inline-flex items-center gap-2 text-base-content/70">
        <span class="loading loading-spinner loading-xs" aria-hidden="true"></span>
        <span class="truncate font-medium">${escapeHtml(label)}</span>
      </span>
    `;
  }
  return `<span class="truncate font-medium text-base-content/70">${escapeHtml(label)}</span>`;
}

function normalizePatchSummary(patch) {
  const patchId = normalizePatchId(patch?.patchId ?? patch?.patch_id);
  if (!patchId) {
    return null;
  }
  return {
    patchId,
    label: normalizePatchLabel(patchId, patch?.patchName ?? patch?.patch_name),
    startTsUtc: normalizePatchStartTs(patch?.startTsUtc ?? patch?.start_ts_utc),
  };
}

function comparePatchSummariesDescending(left, right) {
  if (right.startTsUtc !== left.startTsUtc) {
    return right.startTsUtc - left.startTsUtc;
  }
  return right.patchId.localeCompare(left.patchId);
}

function selectedPatchId(filters, bound) {
  const patchId = normalizePatchId(filters?.patchId);
  if (bound === "from") {
    return normalizePatchId(filters?.fromPatchId) || patchId;
  }
  if (bound === "to") {
    return normalizePatchId(filters?.toPatchId) || patchId;
  }
  return patchId;
}

function syncSelectedContent(dropdown, selectedTemplate, placeholderLabel, { loading = false } = {}) {
  if (!(dropdown instanceof HTMLElement)) {
    return;
  }
  const container = dropdown.querySelector('[data-role="selected-content"]');
  if (!(container instanceof HTMLElement)) {
    return;
  }
  if (selectedTemplate instanceof HTMLTemplateElement) {
    container.replaceChildren(...cloneNodeList(selectedTemplate.content.childNodes));
    return;
  }
  container.innerHTML = placeholderMarkup(placeholderLabel, { loading });
}

function renderPickerCatalog(documentRef, catalog, patches, extraTemplates = []) {
  if (!(catalog instanceof HTMLElement)) {
    return [];
  }
  const templates = [
    ...extraTemplates,
    ...patches.map((patch) => createPatchTemplate(documentRef, patch)),
  ];
  catalog.replaceChildren(...templates);
  return templates;
}

function maybeRefreshDropdown(dropdown) {
  if (!dropdown || typeof dropdown.refreshResults !== "function") {
    return;
  }
  dropdown.refreshResults();
}

export function normalizePatchCatalog(patches) {
  return (Array.isArray(patches) ? patches : [])
    .map(normalizePatchSummary)
    .filter(Boolean)
    .sort(comparePatchSummariesDescending);
}

export function buildPatchPickerStateBundle(signals) {
  const runtime = isPlainObject(signals?._map_runtime) ? signals._map_runtime : {};
  const bridgedFilters = isPlainObject(signals?._map_bridged?.filters) ? signals._map_bridged.filters : {};
  return {
    state: {
      ready: runtime.ready === true,
      catalog: {
        patches: normalizePatchCatalog(runtime.catalog?.patches),
      },
    },
    inputState: {
      filters: {
        patchId: normalizePatchId(bridgedFilters.patchId) || null,
        fromPatchId: normalizePatchId(bridgedFilters.fromPatchId) || null,
        toPatchId: normalizePatchId(bridgedFilters.toPatchId) || null,
      },
    },
  };
}

export function patchTouchesPatchPickerSignals(patch) {
  if (!isPlainObject(patch)) {
    return false;
  }
  if (patch._map_runtime?.ready != null) {
    return true;
  }
  if (patch._map_runtime?.catalog?.patches != null) {
    return true;
  }
  const filters = patch._map_bridged?.filters;
  return Boolean(
    filters
      && (
        filters.patchId != null
        || filters.fromPatchId != null
        || filters.toPatchId != null
      ),
  );
}

export function createMapPatchPickerController({
  shell,
  getSignals,
  documentRef = globalThis.document,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  listenToSignalPatches = true,
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapPatchPickerController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapPatchPickerController requires getSignals()");
  }

  const pickers = {
    from: {
      dropdown: shell.querySelector("#fishymap-patch-from-picker"),
      catalog: shell.querySelector("#fishymap-patch-from-picker [data-role='selected-content-catalog']"),
    },
    to: {
      dropdown: shell.querySelector("#fishymap-patch-to-picker"),
      catalog: shell.querySelector("#fishymap-patch-to-picker [data-role='selected-content-catalog']"),
    },
  };
  if (!(pickers.from.dropdown instanceof HTMLElement) || !(pickers.to.dropdown instanceof HTMLElement)) {
    throw new Error("createMapPatchPickerController requires live patch picker elements");
  }

  const state = {
    frameId: 0,
  };

  function signals() {
    return getSignals() || null;
  }

  function renderPicker(bound, picker, bundle) {
    const patches = bundle.state.catalog.patches;
    const selectedId = selectedPatchId(bundle.inputState.filters, bound);
    const extraTemplates =
      bound === "to"
        ? [
            createSimpleTemplate(documentRef, {
              value: "",
              label: "Now",
              searchText: "now latest current",
            }),
          ]
        : [];
    const templates = renderPickerCatalog(documentRef, picker.catalog, patches, extraTemplates);
    const selectedTemplate =
      templates.find((template) => template.dataset.value === selectedId)
      || (bound === "to" && !selectedId ? extraTemplates[0] : null);
    const dropdown = picker.dropdown;
    const placeholderLabel = bundle.state.ready
      ? patches.length
        ? "Select patch"
        : "No patches available"
      : "Loading patches…";
    syncSelectedContent(dropdown, selectedTemplate, placeholderLabel, {
      loading: !bundle.state.ready,
    });
    if (selectedTemplate instanceof HTMLTemplateElement) {
      dropdown.setAttribute("label", selectedTemplate.dataset.label || selectedId || "");
      dropdown.setAttribute("value", selectedId);
    } else {
      dropdown.setAttribute("label", placeholderLabel);
      dropdown.setAttribute("value", "");
    }
    maybeRefreshDropdown(dropdown);
  }

  function render() {
    state.frameId = 0;
    const bundle = buildPatchPickerStateBundle(signals());
    renderPicker("from", pickers.from, bundle);
    renderPicker("to", pickers.to, bundle);
  }

  function scheduleRender() {
    if (state.frameId) {
      return;
    }
    if (typeof requestAnimationFrameImpl === "function") {
      state.frameId = requestAnimationFrameImpl(() => {
        render();
      });
      return;
    }
    render();
  }

  function handleSignalPatch(event) {
    if (patchTouchesPatchPickerSignals(event?.detail || null)) {
      scheduleRender();
    }
  }

  if (listenToSignalPatches) {
    documentRef?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
  }

  return Object.freeze({
    render,
    scheduleRender,
    disconnect() {
      documentRef?.removeEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
    },
  });
}
