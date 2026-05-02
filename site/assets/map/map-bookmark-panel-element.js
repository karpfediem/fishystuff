import { renderBookmarkManager } from "./map-bookmark-panel.js";
import { mapCountText, mapText } from "./map-i18n.js";
import {
  dispatchShellSignalPatch,
  FISHYMAP_SIGNAL_PATCHED_EVENT,
} from "./map-signal-patch.js";
import { FISHYMAP_ZONE_CATALOG_READY_EVENT } from "./map-zone-catalog-live.js";
import {
  buildBookmarkExportMessage,
  buildBookmarkImportMessage,
  buildBookmarkSelectionCopyMessage,
  copyTextToClipboard,
  downloadBookmarkExport,
  mergeImportedBookmarks,
  parseImportedBookmarks,
  readBookmarkImportFile,
  serializeBookmarksForExport,
} from "./map-bookmark-io.js";
import {
  bookmarkCurrentPointSubtitle,
  bookmarkDisplayLabel,
  buildBookmarkOverviewRows,
  buildBookmarkPanelStateBundle,
  createBookmarkFromSelection,
  moveBookmarkBefore,
  normalizeBookmarks,
  normalizeSelectedBookmarkIds,
  patchTouchesBookmarkSignals,
  renameBookmark,
  selectionBookmarkKey,
} from "./map-bookmark-state.js";
import { FISHYMAP_LIVE_INIT_EVENT, readMapShellSignals } from "./map-shell-signals.js";
import { buildFocusWorldPointSignalPatch } from "./map-selection-actions.js";

export { patchTouchesBookmarkSignals } from "./map-bookmark-state.js";

const BOOKMARK_PANEL_TAG_NAME = "fishymap-bookmark-panel";
const HTMLElementBase = globalThis.HTMLElement ?? class {};

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>\"']/g,
    (char) =>
      (
        {
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&#39;",
        }[char] || char
      ),
  );
}

export function readMapBookmarkPanelShellSignals(shell) {
  return readMapShellSignals(shell);
}

function setBooleanProperty(element, propertyName, value) {
  if (!element) {
    return;
  }
  element[propertyName] = Boolean(value);
}

function setTextContent(element, text) {
  if (!element) {
    return;
  }
  element.textContent = String(text ?? "");
}

function setMarkup(element, renderKey, markup) {
  if (!element) {
    return;
  }
  const normalizedKey = String(renderKey ?? "");
  if (element.dataset.renderKey === normalizedKey) {
    return;
  }
  element.dataset.renderKey = normalizedKey;
  element.innerHTML = String(markup ?? "");
}

function spriteIcon(name, sizeClass = "size-5") {
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-${name}"></use></svg>`;
}

function overviewRowMarkup(row) {
  const icon = String(row?.icon || "information-circle").trim();
  const label = String(row?.label || "").trim();
  const value = String(row?.value || "").trim();
  const hideLabel = row?.hideLabel === true;
  return `
    <div class="fishymap-overview-row${hideLabel ? " fishymap-overview-row--label-less" : ""}">
      <span class="fishymap-overview-row-icon" aria-hidden="true">${spriteIcon(icon, "size-4")}</span>
      ${hideLabel ? "" : `<span class="fishymap-overview-row-label">${escapeHtml(label)}</span>`}
      <span class="fishymap-overview-row-value">${escapeHtml(value)}</span>
    </div>
  `;
}

function dragHandleIcon() {
  return spriteIcon("drag-handle", "size-4");
}

export function buildFocusBookmarkPatch(bookmark, signals = {}) {
  const worldX = Number(bookmark?.worldX);
  const worldZ = Number(bookmark?.worldZ);
  if (!Number.isFinite(worldX) || !Number.isFinite(worldZ)) {
    return null;
  }
  return buildFocusWorldPointSignalPatch(
    {
      elementKind: "bookmark",
      worldX,
      worldZ,
      pointKind: "bookmark",
      pointLabel: bookmarkDisplayLabel(bookmark),
    },
    signals,
  );
}

function ensureBookmarkPanelMarkup(host) {
  if (host.querySelector("#fishymap-bookmarks-list")) {
    return;
  }
  host.innerHTML = `
    <div id="fishymap-bookmarks-controls">
      <div class="fishymap-bookmarks-controls-row fishymap-bookmarks-controls-row--primary">
        <button id="fishymap-bookmark-copy-selected" class="btn btn-primary btn-sm" type="button"><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-copy"></use></svg>${escapeHtml(mapText("bookmarks.copy"))}</button>
        <button id="fishymap-bookmark-export" class="btn btn-soft btn-sm" type="button"><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-export"></use></svg>${escapeHtml(mapText("bookmarks.export"))}</button>
        <button id="fishymap-bookmark-import-trigger" class="btn btn-soft btn-sm" type="button"><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-import"></use></svg>${escapeHtml(mapText("bookmarks.import"))}</button>
        <button id="fishymap-bookmark-cancel" class="btn btn-ghost btn-sm" type="button" hidden><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-clear"></use></svg>${escapeHtml(mapText("bookmarks.cancel"))}</button>
      </div>
      <div class="fishymap-bookmarks-controls-row fishymap-bookmarks-controls-row--secondary">
        <button id="fishymap-bookmark-select-all" class="btn btn-ghost btn-sm" type="button"><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-select-all"></use></svg>${escapeHtml(mapText("bookmarks.select_all"))}</button>
        <button id="fishymap-bookmark-clear-selection" class="btn btn-ghost btn-sm" type="button"><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-clear"></use></svg><span id="fishymap-bookmark-clear-selection-label">${escapeHtml(mapText("bookmarks.clear"))}</span></button>
        <button id="fishymap-bookmark-delete-selected" class="btn btn-ghost btn-error btn-sm" type="button"><svg class="fishy-icon size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="#fishy-trash"></use></svg>${escapeHtml(mapText("bookmarks.delete"))}</button>
      </div>
      <input id="fishymap-bookmark-import-input" class="hidden" type="file" accept=".xml,.txt,text/xml,application/xml">
    </div>
    <div id="fishymap-bookmarks-list" class="rounded-box border border-base-300/70 bg-base-200 p-1">
      <div class="fishymap-bookmark-empty text-sm text-base-content/65">${escapeHtml(mapText("bookmarks.empty"))}</div>
    </div>
  `;
}

export function buildBookmarkPlacementSelectionResult({
  selection,
  bookmarks = [],
  placing = false,
  lastPlacementKey = "",
  allowSameSelection = false,
  requireClickedPoint = false,
  zoneCatalog = [],
  runtimeLayers = [],
} = {}) {
  if (placing !== true) {
    return null;
  }
  const nextPlacementKey = selectionBookmarkKey(selection);
  if (!nextPlacementKey) {
    return null;
  }
  const pointKind = String(selection?.pointKind || "").trim();
  if (requireClickedPoint && pointKind !== "clicked") {
    return null;
  }
  if (!allowSameSelection && nextPlacementKey === String(lastPlacementKey || "")) {
    return null;
  }
  const bookmark = createBookmarkFromSelection(selection, bookmarks, {
    zoneCatalog,
    runtimeLayers,
  });
  if (!bookmark) {
    return null;
  }
  return {
    bookmark,
    placementKey: nextPlacementKey,
  };
}

export class FishyMapBookmarkPanelElement extends HTMLElementBase {
  constructor() {
    super();
    this._shell = null;
    this._rafId = 0;
    this._state = {
      lastPlacementKey: "",
      draggingBookmarkId: "",
      dropBookmarkId: "",
      dropPosition: "",
      zoneCatalog: [],
    };
    this._elements = null;
    this._handleSignalPatched = (event) => {
      if (patchTouchesBookmarkSignals(event?.detail || null)) {
        this.scheduleRender();
      }
    };
    this._handleZoneCatalogReady = (event) => {
      this._state.zoneCatalog = Array.isArray(event?.detail?.zoneCatalog)
        ? cloneJson(event.detail.zoneCatalog)
        : [];
      this.scheduleRender();
    };
    this._handleLiveInit = () => {
      this.scheduleRender();
    };
    this._handleSelectionChanged = (event) => {
      const current = this.bundle();
      const result = buildBookmarkPlacementSelectionResult({
        selection: event?.detail?.state?.selection,
        bookmarks: current.bookmarks,
        placing: current.bookmarkUi.placing,
        lastPlacementKey: this._state.lastPlacementKey,
        allowSameSelection: true,
        requireClickedPoint: true,
        zoneCatalog: current.zoneCatalog,
        runtimeLayers: current.state.catalog.layers,
      });
      if (!result) {
        return;
      }
      this._state.lastPlacementKey = result.placementKey;
      this.writeBookmarkState((bookmarks, bookmarkUi) => {
        bookmarks.push(result.bookmark);
        bookmarkUi.placing = false;
        bookmarkUi.selectedIds = [result.bookmark.id];
      });
    };
    this._handlePlaceClick = () => {
      const current = this.bundle();
      const nextPlacing = current.bookmarkUi.placing !== true;
      this._state.lastPlacementKey = nextPlacing ? selectionBookmarkKey(current.state.selection) : "";
      this.writeBookmarkState((_bookmarks, bookmarkUi) => {
        bookmarkUi.placing = nextPlacing;
      });
    };
    this._handleCancelClick = () => {
      this._state.lastPlacementKey = "";
      this.writeBookmarkState((_bookmarks, bookmarkUi) => {
        bookmarkUi.placing = false;
      });
    };
    this._handleSelectAllClick = () => {
      this.writeBookmarkState((bookmarks, bookmarkUi) => {
        bookmarkUi.selectedIds = normalizeBookmarks(bookmarks).map((bookmark) => bookmark.id);
      });
    };
    this._handleClearSelectionClick = () => {
      this.writeBookmarkState((_bookmarks, bookmarkUi) => {
        bookmarkUi.selectedIds = [];
      });
    };
    this._handleDeleteSelectedClick = () => {
      const current = this.bundle();
      if (!current.bookmarkUi.selectedIds.length) {
        return;
      }
      const confirmImpl = globalThis.confirm?.bind(globalThis);
      if (typeof confirmImpl === "function" && !confirmImpl(mapCountText("bookmarks.confirm.delete_selected", current.bookmarkUi.selectedIds.length))) {
        return;
      }
      const selectedIds = new Set(current.bookmarkUi.selectedIds);
      this.writeBookmarkState((bookmarks, bookmarkUi) => {
        const next = normalizeBookmarks(bookmarks).filter((bookmark) => !selectedIds.has(bookmark.id));
        bookmarks.splice(0, bookmarks.length, ...next);
        bookmarkUi.selectedIds = [];
      });
    };
    this._handleCopySelectedClick = async () => {
      const current = this.bundle();
      const selectedBookmarks = current.bookmarks.filter((bookmark) =>
        current.bookmarkUi.selectedIds.includes(bookmark.id),
      );
      if (!selectedBookmarks.length) {
        this.showSiteToast("warning", mapText("bookmarks.toast.select_to_copy"));
        return;
      }
      try {
        await copyTextToClipboard(serializeBookmarksForExport(selectedBookmarks));
        this.showSiteToast("success", buildBookmarkSelectionCopyMessage(selectedBookmarks.length));
      } catch (_error) {
        this.showSiteToast("error", mapText("bookmarks.toast.clipboard_unavailable"));
      }
    };
    this._handleExportClick = () => {
      const current = this.bundle();
      const selectedBookmarks = current.bookmarks.filter((bookmark) =>
        current.bookmarkUi.selectedIds.includes(bookmark.id),
      );
      const exportBookmarks = selectedBookmarks.length ? selectedBookmarks : current.bookmarks;
      if (!exportBookmarks.length) {
        this.showSiteToast("warning", mapText("bookmarks.toast.no_export"));
        return;
      }
      try {
        downloadBookmarkExport(exportBookmarks);
        this.showSiteToast(
          "info",
          buildBookmarkExportMessage(exportBookmarks.length, selectedBookmarks.length),
        );
      } catch (_error) {
        this.showSiteToast("error", mapText("bookmarks.toast.export_unavailable"));
      }
    };
    this._handleImportTriggerClick = () => {
      if (!this._elements?.bookmarkImportInput) {
        this.showSiteToast("error", mapText("bookmarks.toast.import_unavailable"));
        return;
      }
      this._elements.bookmarkImportInput.value = "";
      this._elements.bookmarkImportInput.click();
    };
    this._handleImportInputChange = async () => {
      const file = this._elements?.bookmarkImportInput?.files?.[0];
      if (!file) {
        return;
      }
      try {
        const importedBookmarks = parseImportedBookmarks(await readBookmarkImportFile(file));
        if (!importedBookmarks.length) {
          this.showSiteToast("warning", mapText("bookmarks.toast.import_no_xml"));
          return;
        }
        const current = this.bundle();
        const merged = mergeImportedBookmarks(current.bookmarks, importedBookmarks);
        const importedCount = merged.length - current.bookmarks.length;
        const skippedCount = importedBookmarks.length - importedCount;
        const importedIds = merged.slice(current.bookmarks.length).map((bookmark) => bookmark.id);
        this.writeBookmarkState((bookmarks, bookmarkUi) => {
          bookmarks.splice(0, bookmarks.length, ...merged);
          bookmarkUi.placing = false;
          if (importedIds.length) {
            bookmarkUi.selectedIds = normalizeSelectedBookmarkIds(
              merged,
              bookmarkUi.selectedIds.concat(importedIds),
            );
          }
        });
        this.showSiteToast(
          importedCount ? "success" : "info",
          buildBookmarkImportMessage(importedCount, skippedCount),
        );
      } catch (error) {
        console.warn("Failed to import map bookmarks", error);
        this.showSiteToast("error", mapText("bookmarks.toast.import_failed"));
      } finally {
        if (this._elements?.bookmarkImportInput) {
          this._elements.bookmarkImportInput.value = "";
        }
      }
    };
    this._handleListChange = (event) => {
      const checkbox = event.target.closest("input[data-bookmark-select]");
      if (!checkbox) {
        return;
      }
      const bookmarkId = String(checkbox.getAttribute("data-bookmark-select") || "").trim();
      if (!bookmarkId) {
        return;
      }
      this.writeBookmarkState((bookmarks, bookmarkUi) => {
        const currentSelectedIds = new Set(normalizeSelectedBookmarkIds(bookmarks, bookmarkUi.selectedIds));
        if (checkbox.checked) {
          currentSelectedIds.add(bookmarkId);
        } else {
          currentSelectedIds.delete(bookmarkId);
        }
        bookmarkUi.selectedIds = Array.from(currentSelectedIds);
      });
    };
    this._handleListClick = (event) => {
      const deleteButton = event.target.closest("button[data-bookmark-delete]");
      if (deleteButton) {
        const bookmarkId = String(deleteButton.getAttribute("data-bookmark-delete") || "").trim();
        if (!bookmarkId) {
          return;
        }
        const current = this.bundle();
        const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
        if (!bookmark) {
          return;
        }
        const confirmImpl = globalThis.confirm?.bind(globalThis);
        if (typeof confirmImpl === "function" && !confirmImpl(mapText("bookmarks.confirm.delete_single", {
          label: bookmarkDisplayLabel(bookmark),
        }))) {
          return;
        }
        this.writeBookmarkState((bookmarks, bookmarkUi) => {
          const next = normalizeBookmarks(bookmarks).filter((candidate) => candidate.id !== bookmarkId);
          bookmarks.splice(0, bookmarks.length, ...next);
          bookmarkUi.selectedIds = normalizeSelectedBookmarkIds(next, bookmarkUi.selectedIds);
        });
        return;
      }

      const renameButton = event.target.closest("button[data-bookmark-rename]");
      if (renameButton) {
        const bookmarkId = String(renameButton.getAttribute("data-bookmark-rename") || "").trim();
        const current = this.bundle();
        const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
        const promptImpl = globalThis.prompt?.bind(globalThis);
        if (!bookmark || typeof promptImpl !== "function") {
          return;
        }
        const nextLabel = promptImpl(
          mapText("bookmarks.prompt.rename"),
          bookmarkDisplayLabel(bookmark),
        );
        if (nextLabel == null) {
          return;
        }
        this.writeBookmarkState((bookmarks) => {
          const next = renameBookmark(bookmarks, bookmarkId, nextLabel);
          bookmarks.splice(0, bookmarks.length, ...next);
        });
        return;
      }

      const activateButton = event.target.closest("button[data-bookmark-activate]");
      if (activateButton) {
        const bookmarkId = String(activateButton.getAttribute("data-bookmark-activate") || "").trim();
        const current = this.bundle();
        const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
        if (!bookmark) {
          return;
        }
        dispatchShellSignalPatch(this._shell, buildFocusBookmarkPatch(bookmark, this.signals()));
        return;
      }

      const copyButton = event.target.closest("button[data-bookmark-copy]");
      if (copyButton) {
        const bookmarkId = String(copyButton.getAttribute("data-bookmark-copy") || "").trim();
        const current = this.bundle();
        const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
        if (!bookmark) {
          return;
        }
        void copyTextToClipboard(serializeBookmarksForExport([bookmark]))
          .then(() => {
            this.showSiteToast("success", mapText("bookmarks.toast.copied_single"));
          })
          .catch(() => {
            this.showSiteToast("error", mapText("bookmarks.toast.clipboard_unavailable"));
          });
      }
    };
    this._handleListDragStart = (event) => {
      const handle = event.target.closest("button[data-bookmark-drag][draggable='true']");
      const card = handle?.closest(".fishymap-bookmark-card");
      if (!handle || !card) {
        return;
      }
      const bookmarkId = String(card.getAttribute("data-bookmark-id") || "").trim();
      if (!bookmarkId) {
        return;
      }
      this._state.draggingBookmarkId = bookmarkId;
      card.dataset.dragging = "true";
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", bookmarkId);
      }
    };
    this._handleListDragOver = (event) => {
      if (!this._state.draggingBookmarkId) {
        return;
      }
      event.preventDefault();
      const card = event.target.closest(".fishymap-bookmark-card");
      if (!card) {
        return;
      }
      const bookmarkId = String(card.getAttribute("data-bookmark-id") || "").trim();
      if (!bookmarkId || bookmarkId === this._state.draggingBookmarkId) {
        return;
      }
      const rect = card.getBoundingClientRect();
      const offsetY = event.clientY - rect.top;
      this._state.dropBookmarkId = bookmarkId;
      this._state.dropPosition = offsetY >= rect.height / 2 ? "after" : "before";
      this._elements.bookmarksList
        ?.querySelectorAll(".fishymap-bookmark-card")
        .forEach((candidate) => {
          if (candidate.getAttribute("data-bookmark-id") === this._state.dropBookmarkId) {
            candidate.dataset.dropPosition = this._state.dropPosition;
            return;
          }
          delete candidate.dataset.dropPosition;
        });
    };
    this._handleListDrop = (event) => {
      if (!this._state.draggingBookmarkId || !this._state.dropBookmarkId || !this._state.dropPosition) {
        return;
      }
      event.preventDefault();
      this.writeBookmarkState((bookmarks) => {
        const next = moveBookmarkBefore(
          bookmarks,
          this._state.draggingBookmarkId,
          this._state.dropBookmarkId,
          this._state.dropPosition,
        );
        bookmarks.splice(0, bookmarks.length, ...next);
      });
      this.clearDropState();
    };
    this._handleListDragEnd = () => {
      this.clearDropState();
    };
  }

  connectedCallback() {
    this._shell = this.closest?.("#map-page-shell") || null;
    ensureBookmarkPanelMarkup(this);
    this._elements = {
      shell: this._shell,
      bookmarkPlace: this._shell?.querySelector?.("#fishymap-bookmark-place") || null,
      bookmarkPlaceLabel: this._shell?.querySelector?.("#fishymap-bookmark-place-label") || null,
      bookmarkCopySelected: this.querySelector("#fishymap-bookmark-copy-selected"),
      bookmarkExport: this.querySelector("#fishymap-bookmark-export"),
      bookmarkImportTrigger: this.querySelector("#fishymap-bookmark-import-trigger"),
      bookmarkImportInput: this.querySelector("#fishymap-bookmark-import-input"),
      bookmarkSelectAll: this.querySelector("#fishymap-bookmark-select-all"),
      bookmarkDeleteSelected: this.querySelector("#fishymap-bookmark-delete-selected"),
      bookmarkClearSelection: this.querySelector("#fishymap-bookmark-clear-selection"),
      bookmarkClearSelectionLabel: this.querySelector("#fishymap-bookmark-clear-selection-label"),
      bookmarkCancel: this.querySelector("#fishymap-bookmark-cancel"),
      bookmarksList: this.querySelector("#fishymap-bookmarks-list"),
    };
    this._shell?.addEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.addEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.addEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this._shell?.addEventListener?.("fishymap:selection-changed", this._handleSelectionChanged);
    this._elements.bookmarkPlace?.addEventListener?.("click", this._handlePlaceClick);
    this._elements.bookmarkCancel?.addEventListener?.("click", this._handleCancelClick);
    this._elements.bookmarkSelectAll?.addEventListener?.("click", this._handleSelectAllClick);
    this._elements.bookmarkClearSelection?.addEventListener?.("click", this._handleClearSelectionClick);
    this._elements.bookmarkDeleteSelected?.addEventListener?.("click", this._handleDeleteSelectedClick);
    this._elements.bookmarkCopySelected?.addEventListener?.("click", this._handleCopySelectedClick);
    this._elements.bookmarkExport?.addEventListener?.("click", this._handleExportClick);
    this._elements.bookmarkImportTrigger?.addEventListener?.("click", this._handleImportTriggerClick);
    this._elements.bookmarkImportInput?.addEventListener?.("change", this._handleImportInputChange);
    this._elements.bookmarksList?.addEventListener?.("change", this._handleListChange);
    this._elements.bookmarksList?.addEventListener?.("click", this._handleListClick);
    this._elements.bookmarksList?.addEventListener?.("dragstart", this._handleListDragStart);
    this._elements.bookmarksList?.addEventListener?.("dragover", this._handleListDragOver);
    this._elements.bookmarksList?.addEventListener?.("drop", this._handleListDrop);
    this._elements.bookmarksList?.addEventListener?.("dragend", this._handleListDragEnd);
    this.render();
  }

  disconnectedCallback() {
    this._shell?.removeEventListener?.(FISHYMAP_SIGNAL_PATCHED_EVENT, this._handleSignalPatched);
    this._shell?.removeEventListener?.(FISHYMAP_ZONE_CATALOG_READY_EVENT, this._handleZoneCatalogReady);
    this._shell?.removeEventListener?.(FISHYMAP_LIVE_INIT_EVENT, this._handleLiveInit);
    this._shell?.removeEventListener?.("fishymap:selection-changed", this._handleSelectionChanged);
    this._elements?.bookmarkPlace?.removeEventListener?.("click", this._handlePlaceClick);
    this._elements?.bookmarkCancel?.removeEventListener?.("click", this._handleCancelClick);
    this._elements?.bookmarkSelectAll?.removeEventListener?.("click", this._handleSelectAllClick);
    this._elements?.bookmarkClearSelection?.removeEventListener?.("click", this._handleClearSelectionClick);
    this._elements?.bookmarkDeleteSelected?.removeEventListener?.("click", this._handleDeleteSelectedClick);
    this._elements?.bookmarkCopySelected?.removeEventListener?.("click", this._handleCopySelectedClick);
    this._elements?.bookmarkExport?.removeEventListener?.("click", this._handleExportClick);
    this._elements?.bookmarkImportTrigger?.removeEventListener?.("click", this._handleImportTriggerClick);
    this._elements?.bookmarkImportInput?.removeEventListener?.("change", this._handleImportInputChange);
    this._elements?.bookmarksList?.removeEventListener?.("change", this._handleListChange);
    this._elements?.bookmarksList?.removeEventListener?.("click", this._handleListClick);
    this._elements?.bookmarksList?.removeEventListener?.("dragstart", this._handleListDragStart);
    this._elements?.bookmarksList?.removeEventListener?.("dragover", this._handleListDragOver);
    this._elements?.bookmarksList?.removeEventListener?.("drop", this._handleListDrop);
    this._elements?.bookmarksList?.removeEventListener?.("dragend", this._handleListDragEnd);
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

  bundle() {
    return {
      ...buildBookmarkPanelStateBundle(this.signals()),
      zoneCatalog: cloneJson(this._state.zoneCatalog),
    };
  }

  showSiteToast(tone, message, options = {}) {
    const text = String(message || "").trim();
    if (!text) {
      return;
    }
    const toast = globalThis.__fishystuffToast;
    if (!toast) {
      return;
    }
    const handler =
      typeof toast[tone] === "function"
        ? toast[tone]
        : typeof toast.show === "function"
          ? (value, extra) => toast.show({ tone, message: value, ...(extra || {}) })
          : null;
    handler?.(text, options);
  }

  writeBookmarkState(mutator) {
    const current = this.bundle();
    const nextBookmarks = cloneJson(current.bookmarks);
    const nextBookmarkUi = cloneJson(current.bookmarkUi);
    mutator(nextBookmarks, nextBookmarkUi, current);
    dispatchShellSignalPatch(this._shell, {
      _map_bookmarks: {
        entries: normalizeBookmarks(nextBookmarks),
      },
      _map_ui: {
        bookmarks: {
          placing: nextBookmarkUi.placing === true,
          selectedIds: normalizeSelectedBookmarkIds(nextBookmarks, nextBookmarkUi.selectedIds),
        },
      },
    });
    this.scheduleRender();
  }

  clearDropState() {
    this._state.draggingBookmarkId = "";
    this._state.dropBookmarkId = "";
    this._state.dropPosition = "";
    this._elements?.bookmarksList
      ?.querySelectorAll(".fishymap-bookmark-card")
      .forEach((candidate) => {
        delete candidate.dataset.dragging;
        delete candidate.dataset.dropPosition;
      });
  }

  render() {
    this._rafId = 0;
    const current = this.bundle();
    renderBookmarkManager(
      this._elements,
      { state: current.state },
      current.bookmarks,
      current.bookmarkUi,
      {
        resolveDisplayBookmarks: (_stateBundle, bookmarks) => normalizeBookmarks(bookmarks),
        normalizeSelectedBookmarkIds,
        setBooleanProperty,
        setTextContent,
        setMarkup,
        buildBookmarkOverviewRows,
        bookmarkDisplayLabel,
        bookmarkCurrentPointSubtitle,
        overviewRowMarkup,
        escapeHtml,
        dragHandleIcon,
        spriteIcon,
      },
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

export function registerFishyMapBookmarkPanelElement(registry = globalThis.customElements) {
  if (!registry || typeof registry.get !== "function" || typeof registry.define !== "function") {
    return false;
  }
  if (registry.get(BOOKMARK_PANEL_TAG_NAME)) {
    return true;
  }
  registry.define(BOOKMARK_PANEL_TAG_NAME, FishyMapBookmarkPanelElement);
  return true;
}

registerFishyMapBookmarkPanelElement();
