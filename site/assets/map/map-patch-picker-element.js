import {
  buildPatchPickerDefaultSignalPatch,
  buildPatchPickerStateBundle,
  patchTouchesPatchPickerSignals,
} from "./map-patch-picker-live.js";
import { FISHYMAP_LIVE_INIT_EVENT, readMapShellSignals } from "./map-shell-signals.js";
import { dispatchShellSignalPatch, FISHYMAP_SIGNAL_PATCHED_EVENT } from "./map-signal-patch.js";

const PATCH_PICKER_TAG_NAME = "fishymap-patch-picker";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

let nextPatchPickerId = 1;

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>\"']/g,
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

export function readMapPatchPickerShellSignals(shell) {
  return readMapShellSignals(shell);
}

function selectedPatchId(filters, bound) {
  const patchId = String(filters?.patchId ?? "").trim();
  if (bound === "from") {
    return String(filters?.fromPatchId ?? "").trim() || patchId;
  }
  if (bound === "to") {
    return String(filters?.toPatchId ?? "").trim() || patchId;
  }
  return patchId;
}

function cloneNodeList(nodes) {
  return Array.from(nodes, (node) => node.cloneNode(true));
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

function createTemplate(documentRef, { value, label, badge = "", searchText = "" }) {
  const template = documentRef.createElement("template");
  template.dataset.role = "selected-content";
  template.dataset.value = String(value ?? "");
  template.dataset.label = String(label ?? "").trim();
  template.dataset.searchText = String(searchText || label || value || "").trim();
  template.innerHTML = `
    <span class="flex min-w-0 flex-1 items-center gap-3 text-sm">
      <span class="truncate font-medium">${escapeHtml(label)}</span>
      ${badge ? `<span class="badge badge-ghost badge-xs shrink-0">${escapeHtml(badge)}</span>` : ""}
    </span>
  `;
  return template;
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
    dropdown.setAttribute("label", selectedTemplate.dataset.label || "");
    dropdown.setAttribute("value", selectedTemplate.dataset.value || "");
    return;
  }
  container.innerHTML = placeholderMarkup(placeholderLabel, { loading });
  dropdown.setAttribute("label", placeholderLabel);
  dropdown.setAttribute("value", "");
}

function renderPickerCatalog(documentRef, catalog, patches, extraTemplates = []) {
  if (!(catalog instanceof HTMLElement)) {
    return [];
  }
  const templates = [
    ...extraTemplates,
    ...patches.map((patch) =>
      createTemplate(documentRef, {
        value: patch.patchId,
        label: patch.label,
        badge: patch.label !== patch.patchId ? patch.patchId : "",
        searchText: [patch.label, patch.patchId].filter(Boolean).join(" "),
      }),
    ),
  ];
  catalog.replaceChildren(...templates);
  return templates;
}

function ensurePatchPickerMarkup(host, ids) {
  if (host.querySelector("#fishymap-patch-picker-fieldset")) {
    return;
  }
  host.innerHTML = `
    <fieldset id="fishymap-patch-picker-fieldset" class="fieldset rounded-box border border-base-300/70 bg-base-200 p-4">
      <legend class="fieldset-legend px-1 text-sm font-semibold">Patch window</legend>
      <span class="label px-1 pt-0 text-[11px] uppercase tracking-[0.18em] text-base-content/45">Inclusive</span>
      <div class="flex flex-col gap-3">
        <label class="form-control gap-2">
          <span class="label-text text-xs font-semibold uppercase tracking-[0.18em] text-base-content/60">From</span>
          <input id="${escapeHtml(ids.fromInputId)}" type="hidden" value="">
          <fishy-searchable-dropdown
            id="${escapeHtml(ids.fromPickerId)}"
            class="relative block w-full"
            input-id="${escapeHtml(ids.fromInputId)}"
            label="Loading patches…"
            value=""
            placeholder="Search patches"
          >
            <button
              type="button"
              data-role="trigger"
              class="flex min-h-10 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-left text-sm shadow-sm"
              aria-haspopup="listbox"
              aria-expanded="false"
              aria-controls="${escapeHtml(ids.fromPanelId)}"
            >
              <span data-role="selected-content" class="flex min-w-0 flex-1 items-center gap-3 text-sm"><span class="inline-flex items-center gap-2 text-base-content/70"><span class="loading loading-spinner loading-xs" aria-hidden="true"></span><span class="truncate font-medium">Loading patches…</span></span></span>
              <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-caret-down"></use></svg>
            </button>
            <div id="${escapeHtml(ids.fromPanelId)}" data-role="panel" class="absolute left-0 top-0 z-50 w-full min-w-full max-w-full" hidden>
              <div class="grid w-full min-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg">
                <label class="flex min-h-10 w-full min-w-full items-center gap-3 bg-base-100 px-3 py-2 text-sm">
                  <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-search-field"></use></svg>
                  <input
                    id="${escapeHtml(ids.fromSearchId)}"
                    data-role="search-input"
                    type="search"
                    class="w-full border-0 bg-transparent p-0 shadow-none outline-none"
                    style="outline: none; box-shadow: none;"
                    placeholder="Search patches"
                    autocomplete="off"
                    spellcheck="false"
                  >
                </label>
                <div class="px-1 pb-1">
                  <ul id="${escapeHtml(ids.fromResultsId)}" tabindex="-1" data-role="results" class="menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1">
                    <li class="menu-disabled"><span class="inline-flex items-center gap-2 text-base-content/70"><span class="loading loading-spinner loading-xs" aria-hidden="true"></span><span>Loading patches…</span></span></li>
                  </ul>
                </div>
              </div>
            </div>
            <div data-role="selected-content-catalog" hidden></div>
          </fishy-searchable-dropdown>
        </label>
        <label class="form-control gap-2">
          <span class="label-text text-xs font-semibold uppercase tracking-[0.18em] text-base-content/60">Until (incl.)</span>
          <input id="${escapeHtml(ids.toInputId)}" type="hidden" value="">
          <fishy-searchable-dropdown
            id="${escapeHtml(ids.toPickerId)}"
            class="relative block w-full"
            input-id="${escapeHtml(ids.toInputId)}"
            label="Loading patches…"
            value=""
            placeholder="Search patches"
          >
            <button
              type="button"
              data-role="trigger"
              class="flex min-h-10 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-left text-sm shadow-sm"
              aria-haspopup="listbox"
              aria-expanded="false"
              aria-controls="${escapeHtml(ids.toPanelId)}"
            >
              <span data-role="selected-content" class="flex min-w-0 flex-1 items-center gap-3 text-sm"><span class="inline-flex items-center gap-2 text-base-content/70"><span class="loading loading-spinner loading-xs" aria-hidden="true"></span><span class="truncate font-medium">Loading patches…</span></span></span>
              <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-caret-down"></use></svg>
            </button>
            <div id="${escapeHtml(ids.toPanelId)}" data-role="panel" class="absolute left-0 top-0 z-50 w-full min-w-full max-w-full" hidden>
              <div class="grid w-full min-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg">
                <label class="flex min-h-10 w-full min-w-full items-center gap-3 bg-base-100 px-3 py-2 text-sm">
                  <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-search-field"></use></svg>
                  <input
                    id="${escapeHtml(ids.toSearchId)}"
                    data-role="search-input"
                    type="search"
                    class="w-full border-0 bg-transparent p-0 shadow-none outline-none"
                    style="outline: none; box-shadow: none;"
                    placeholder="Search patches"
                    autocomplete="off"
                    spellcheck="false"
                  >
                </label>
                <div class="px-1 pb-1">
                  <ul id="${escapeHtml(ids.toResultsId)}" tabindex="-1" data-role="results" class="menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1">
                    <li class="menu-disabled"><span class="inline-flex items-center gap-2 text-base-content/70"><span class="loading loading-spinner loading-xs" aria-hidden="true"></span><span>Loading patches…</span></span></li>
                  </ul>
                </div>
              </div>
            </div>
            <div data-role="selected-content-catalog" hidden></div>
          </fishy-searchable-dropdown>
        </label>
      </div>
    </fieldset>
  `;
}

export class FishyMapPatchPickerElement extends HTMLElementBase {
  constructor() {
    super();
    this._instanceId = nextPatchPickerId++;
    this._shell = null;
    this._rafId = 0;
    this._elements = null;
    this._handleSignalPatched = (event) => {
      if (patchTouchesPatchPickerSignals(event?.detail || null)) {
        this.scheduleRender();
      }
    };
    this._handleLiveInit = () => {
      this.scheduleRender();
    };
    this._handleFromInput = () => {
      this.dispatchPatch({
        _map_bridged: {
          filters: {
            fromPatchId: String(this._elements?.fromInput?.value ?? "").trim() || null,
          },
        },
      });
    };
    this._handleToInput = () => {
      this.dispatchPatch({
        _map_bridged: {
          filters: {
            toPatchId: String(this._elements?.toInput?.value ?? "").trim() || null,
          },
        },
      });
    };
  }

  connectedCallback() {
    this._shell = this.closest?.("#map-page-shell") || null;
    const ids = {
      fromInputId: `fishymap-patch-from-${this._instanceId}`,
      fromPickerId: `fishymap-patch-from-picker-${this._instanceId}`,
      fromPanelId: `fishymap-patch-from-panel-${this._instanceId}`,
      fromSearchId: `fishymap-patch-from-search-input-${this._instanceId}`,
      fromResultsId: `fishymap-patch-from-results-${this._instanceId}`,
      toInputId: `fishymap-patch-to-${this._instanceId}`,
      toPickerId: `fishymap-patch-to-picker-${this._instanceId}`,
      toPanelId: `fishymap-patch-to-panel-${this._instanceId}`,
      toSearchId: `fishymap-patch-to-search-input-${this._instanceId}`,
      toResultsId: `fishymap-patch-to-results-${this._instanceId}`,
    };
    ensurePatchPickerMarkup(this, ids);
    this._elements = {
      fromInput: this.querySelector(`#${ids.fromInputId}`),
      fromPicker: this.querySelector(`#${ids.fromPickerId}`),
      fromCatalog: this.querySelector(`#${ids.fromPickerId} [data-role='selected-content-catalog']`),
      toInput: this.querySelector(`#${ids.toInputId}`),
      toPicker: this.querySelector(`#${ids.toPickerId}`),
      toCatalog: this.querySelector(`#${ids.toPickerId} [data-role='selected-content-catalog']`),
    };
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this._elements.fromInput?.addEventListener?.("input", this._handleFromInput);
    this._elements.toInput?.addEventListener?.("input", this._handleToInput);
    this.render();
  }

  disconnectedCallback() {
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this._elements?.fromInput?.removeEventListener?.("input", this._handleFromInput);
    this._elements?.toInput?.removeEventListener?.("input", this._handleToInput);
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    this._rafId = 0;
    this._shell = null;
    this._elements = null;
  }

  signals() {
    return readMapShellSignals(this._shell);
  }

  dispatchPatch(patch) {
    dispatchShellSignalPatch(this._shell, patch);
  }

  renderPicker(bound, picker, bundle) {
    const patches = bundle.state.catalog.patches;
    const selectedId = selectedPatchId(bundle.inputState.filters, bound);
    const extraTemplates =
      bound === "to"
        ? [
            createTemplate(globalThis.document, {
              value: "",
              label: "Now",
              searchText: "now latest current",
            }),
          ]
        : [];
    const templates = renderPickerCatalog(globalThis.document, picker.catalog, patches, extraTemplates);
    const selectedTemplate =
      templates.find((template) => template.dataset.value === selectedId)
      || (bound === "to" && !selectedId ? extraTemplates[0] : null);
    const placeholderLabel = bundle.state.ready
      ? patches.length
        ? "Select patch"
        : "No patches available"
      : "Loading patches…";
    syncSelectedContent(picker.dropdown, selectedTemplate, placeholderLabel, {
      loading: !bundle.state.ready,
    });
    const nextValue = selectedTemplate?.dataset?.value ?? "";
    if (picker.input instanceof HTMLInputElement && picker.input.value !== nextValue) {
      picker.input.value = nextValue;
    }
    if (typeof picker.dropdown?.refreshResults === "function") {
      picker.dropdown.refreshResults();
    }
  }

  render() {
    this._rafId = 0;
    const signals = this.signals();
    if (!signals || !this._elements?.fromPicker || !this._elements?.toPicker) {
      return;
    }
    const defaultPatch = buildPatchPickerDefaultSignalPatch(signals);
    if (defaultPatch) {
      this.dispatchPatch(defaultPatch);
      return;
    }
    const bundle = buildPatchPickerStateBundle(signals);
    this.renderPicker(
      "from",
      {
        dropdown: this._elements.fromPicker,
        catalog: this._elements.fromCatalog,
        input: this._elements.fromInput,
      },
      bundle,
    );
    this.renderPicker(
      "to",
      {
        dropdown: this._elements.toPicker,
        catalog: this._elements.toCatalog,
        input: this._elements.toInput,
      },
      bundle,
    );
  }

  scheduleRender() {
    if (this._rafId && typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(this._rafId);
    }
    if (typeof globalThis.requestAnimationFrame === "function") {
      this._rafId = globalThis.requestAnimationFrame(() => {
        this.render();
      }) || 0;
      if (this._rafId) {
        return;
      }
    }
    this.render();
  }
}

export function registerFishyMapPatchPickerElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (registry.get(PATCH_PICKER_TAG_NAME)) {
    return true;
  }
  registry.define(PATCH_PICKER_TAG_NAME, FishyMapPatchPickerElement);
  return true;
}

registerFishyMapPatchPickerElement();
