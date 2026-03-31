import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";
import { renderBookmarkManager } from "./map-bookmark-panel.js";
import { dispatchShellSignalPatch } from "./map-signal-patch.js";
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

const ICON_SPRITE_URL = "/img/icons.svg";

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function escapeHtml(value) {
  return String(value ?? "").replace(
    /[&<>"']/g,
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
  return `<svg class="fishy-icon ${sizeClass}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="${ICON_SPRITE_URL}#fishy-${name}"></use></svg>`;
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

function buildFocusBookmarkPatch(bookmark, currentActions) {
  const currentToken = Number(currentActions?.focusWorldPointToken || 0);
  return {
    _map_actions: {
      focusWorldPointToken: currentToken + 1,
      focusWorldPoint: {
        worldX: bookmark.worldX,
        worldZ: bookmark.worldZ,
        pointKind: "bookmark",
        pointLabel: bookmarkDisplayLabel(bookmark),
      },
    },
  };
}

export function buildBookmarkPlacementSelectionResult({
  selection,
  bookmarks = [],
  placing = false,
  lastPlacementKey = "",
  allowSameSelection = false,
  requireClickedPoint = false,
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
  const bookmark = createBookmarkFromSelection(selection, bookmarks);
  if (!bookmark) {
    return null;
  }
  return {
    bookmark,
    placementKey: nextPlacementKey,
  };
}

export function createMapBookmarkPanelController({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  documentRef = globalThis.document,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  promptImpl = globalThis.prompt?.bind(globalThis),
  confirmImpl = globalThis.confirm?.bind(globalThis),
  listenToSignalPatches = true,
} = {}) {
  if (!shell || typeof shell.querySelector !== "function") {
    throw new Error("createMapBookmarkPanelController requires a shell element");
  }
  if (typeof getSignals !== "function") {
    throw new Error("createMapBookmarkPanelController requires getSignals()");
  }

  const elements = {
    shell,
    bookmarkPlace: shell.querySelector("#fishymap-bookmark-place"),
    bookmarkPlaceLabel: shell.querySelector("#fishymap-bookmark-place-label"),
    bookmarkCopySelected: shell.querySelector("#fishymap-bookmark-copy-selected"),
    bookmarkExport: shell.querySelector("#fishymap-bookmark-export"),
    bookmarkImportTrigger: shell.querySelector("#fishymap-bookmark-import-trigger"),
    bookmarkImportInput: shell.querySelector("#fishymap-bookmark-import-input"),
    bookmarkSelectAll: shell.querySelector("#fishymap-bookmark-select-all"),
    bookmarkDeleteSelected: shell.querySelector("#fishymap-bookmark-delete-selected"),
    bookmarkClearSelection: shell.querySelector("#fishymap-bookmark-clear-selection"),
    bookmarkClearSelectionLabel: shell.querySelector("#fishymap-bookmark-clear-selection-label"),
    bookmarkCancel: shell.querySelector("#fishymap-bookmark-cancel"),
    bookmarksList: shell.querySelector("#fishymap-bookmarks-list"),
  };
  if (!(elements.bookmarksList instanceof HTMLElement)) {
    throw new Error("createMapBookmarkPanelController requires bookmark list elements");
  }

  const state = {
    frameId: 0,
    lastPlacementKey: "",
    draggingBookmarkId: "",
    dropBookmarkId: "",
    dropPosition: "",
    zoneCatalog: [],
  };

  function showSiteToast(tone, message, options = {}) {
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

  function signals() {
    return getSignals() || null;
  }

  function bundle() {
    return {
      ...buildBookmarkPanelStateBundle(signals()),
      zoneCatalog: cloneJson(state.zoneCatalog),
    };
  }

  function writeBookmarkState(mutator) {
    const current = bundle();
    const nextBookmarks = cloneJson(current.bookmarks);
    const nextBookmarkUi = cloneJson(current.bookmarkUi);
    mutator(nextBookmarks, nextBookmarkUi, current);
    dispatchPatch(shell, {
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
    scheduleRender();
  }

  function render() {
    state.frameId = 0;
    const current = bundle();
    renderBookmarkManager(
      elements,
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
        overviewRowMarkup,
        escapeHtml,
        dragHandleIcon,
        spriteIcon,
      },
    );
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

  function maybePlaceBookmarkFromSelection() {
    const current = bundle();
    if (!current.bookmarkUi.placing) {
      state.lastPlacementKey = "";
      return false;
    }
    const result = buildBookmarkPlacementSelectionResult({
      selection: current.state.selection,
      bookmarks: current.bookmarks,
      placing: current.bookmarkUi.placing,
      lastPlacementKey: state.lastPlacementKey,
    });
    if (!result) {
      return false;
    }
    state.lastPlacementKey = result.placementKey;
    writeBookmarkState((bookmarks, bookmarkUi) => {
      bookmarks.push(result.bookmark);
      bookmarkUi.placing = false;
      bookmarkUi.selectedIds = [result.bookmark.id];
    });
    return true;
  }

  function handleSelectionChanged(event) {
    const current = bundle();
    const result = buildBookmarkPlacementSelectionResult({
      selection: event?.detail?.state?.selection,
      bookmarks: current.bookmarks,
      placing: current.bookmarkUi.placing,
      lastPlacementKey: state.lastPlacementKey,
      allowSameSelection: true,
      requireClickedPoint: true,
    });
    if (!result) {
      return;
    }
    state.lastPlacementKey = result.placementKey;
    writeBookmarkState((bookmarks, bookmarkUi) => {
      bookmarks.push(result.bookmark);
      bookmarkUi.placing = false;
      bookmarkUi.selectedIds = [result.bookmark.id];
    });
  }

  function handleSignalPatch(event) {
    if (!patchTouchesBookmarkSignals(event?.detail)) {
      return;
    }
    if (maybePlaceBookmarkFromSelection()) {
      return;
    }
    scheduleRender();
  }

  elements.bookmarkPlace?.addEventListener("click", () => {
    const current = bundle();
    const nextPlacing = current.bookmarkUi.placing !== true;
    state.lastPlacementKey = nextPlacing ? selectionBookmarkKey(current.state.selection) : "";
    writeBookmarkState((_bookmarks, bookmarkUi) => {
      bookmarkUi.placing = nextPlacing;
    });
  });

  elements.bookmarkCancel?.addEventListener("click", () => {
    state.lastPlacementKey = "";
    writeBookmarkState((_bookmarks, bookmarkUi) => {
      bookmarkUi.placing = false;
    });
  });

  elements.bookmarkSelectAll?.addEventListener("click", () => {
    writeBookmarkState((bookmarks, bookmarkUi) => {
      bookmarkUi.selectedIds = normalizeBookmarks(bookmarks).map((bookmark) => bookmark.id);
    });
  });

  elements.bookmarkClearSelection?.addEventListener("click", () => {
    writeBookmarkState((_bookmarks, bookmarkUi) => {
      bookmarkUi.selectedIds = [];
    });
  });

  elements.bookmarkDeleteSelected?.addEventListener("click", () => {
    const current = bundle();
    if (!current.bookmarkUi.selectedIds.length) {
      return;
    }
    if (typeof confirmImpl === "function" && !confirmImpl(`Delete ${current.bookmarkUi.selectedIds.length} selected bookmarks?`)) {
      return;
    }
    const selectedIds = new Set(current.bookmarkUi.selectedIds);
    writeBookmarkState((bookmarks, bookmarkUi) => {
      const next = normalizeBookmarks(bookmarks).filter((bookmark) => !selectedIds.has(bookmark.id));
      bookmarks.splice(0, bookmarks.length, ...next);
      bookmarkUi.selectedIds = [];
    });
  });

  elements.bookmarkCopySelected?.addEventListener("click", async () => {
    const current = bundle();
    const selectedBookmarks = current.bookmarks.filter((bookmark) =>
      current.bookmarkUi.selectedIds.includes(bookmark.id),
    );
    if (!selectedBookmarks.length) {
      showSiteToast("warning", "Select one or more bookmarks to copy.");
      return;
    }
    try {
      await copyTextToClipboard(serializeBookmarksForExport(selectedBookmarks));
      showSiteToast("success", buildBookmarkSelectionCopyMessage(selectedBookmarks.length));
    } catch (_error) {
      showSiteToast("error", "Clipboard access is unavailable in this browser.");
    }
  });

  elements.bookmarksList.addEventListener("change", (event) => {
    const checkbox = event.target.closest("input[data-bookmark-select]");
    if (!checkbox) {
      return;
    }
    const bookmarkId = String(checkbox.getAttribute("data-bookmark-select") || "").trim();
    if (!bookmarkId) {
      return;
    }
    writeBookmarkState((bookmarks, bookmarkUi) => {
      const currentSelectedIds = new Set(normalizeSelectedBookmarkIds(bookmarks, bookmarkUi.selectedIds));
      if (checkbox.checked) {
        currentSelectedIds.add(bookmarkId);
      } else {
        currentSelectedIds.delete(bookmarkId);
      }
      bookmarkUi.selectedIds = Array.from(currentSelectedIds);
    });
  });

  elements.bookmarksList.addEventListener("click", (event) => {
    const deleteButton = event.target.closest("button[data-bookmark-delete]");
    if (deleteButton) {
      const bookmarkId = String(deleteButton.getAttribute("data-bookmark-delete") || "").trim();
      if (!bookmarkId) {
        return;
      }
      const current = bundle();
      const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
      if (!bookmark) {
        return;
      }
      if (typeof confirmImpl === "function" && !confirmImpl(`Delete bookmark "${bookmarkDisplayLabel(bookmark)}"?`)) {
        return;
      }
      writeBookmarkState((bookmarks, bookmarkUi) => {
        const next = normalizeBookmarks(bookmarks).filter((candidate) => candidate.id !== bookmarkId);
        bookmarks.splice(0, bookmarks.length, ...next);
        bookmarkUi.selectedIds = normalizeSelectedBookmarkIds(next, bookmarkUi.selectedIds);
      });
      return;
    }

    const renameButton = event.target.closest("button[data-bookmark-rename]");
    if (renameButton) {
      const bookmarkId = String(renameButton.getAttribute("data-bookmark-rename") || "").trim();
      const current = bundle();
      const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
      if (!bookmark || typeof promptImpl !== "function") {
        return;
      }
      const nextLabel = promptImpl("Rename bookmark", bookmarkDisplayLabel(bookmark));
      if (nextLabel == null) {
        return;
      }
      writeBookmarkState((bookmarks) => {
        const next = renameBookmark(bookmarks, bookmarkId, nextLabel);
        bookmarks.splice(0, bookmarks.length, ...next);
      });
      return;
    }

    const activateButton = event.target.closest("button[data-bookmark-activate]");
    if (activateButton) {
      const bookmarkId = String(activateButton.getAttribute("data-bookmark-activate") || "").trim();
      const current = bundle();
      const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
      if (!bookmark) {
        return;
      }
      dispatchPatch(shell, buildFocusBookmarkPatch(bookmark, signals()?._map_actions));
      return;
    }

    const copyButton = event.target.closest("button[data-bookmark-copy]");
    if (copyButton) {
      const bookmarkId = String(copyButton.getAttribute("data-bookmark-copy") || "").trim();
      const current = bundle();
      const bookmark = current.bookmarks.find((candidate) => candidate.id === bookmarkId);
      if (!bookmark) {
        return;
      }
      void copyTextToClipboard(serializeBookmarksForExport([bookmark]))
        .then(() => {
          showSiteToast("success", "Copied bookmark XML.");
        })
        .catch(() => {
          showSiteToast("error", "Clipboard access is unavailable in this browser.");
        });
      return;
    }
  });

  elements.bookmarkExport?.addEventListener("click", () => {
    const current = bundle();
    const selectedBookmarks = current.bookmarks.filter((bookmark) =>
      current.bookmarkUi.selectedIds.includes(bookmark.id),
    );
    const exportBookmarks = selectedBookmarks.length ? selectedBookmarks : current.bookmarks;
    if (!exportBookmarks.length) {
      showSiteToast("warning", "There are no bookmarks to export yet.");
      return;
    }
    try {
      downloadBookmarkExport(exportBookmarks);
      showSiteToast(
        "info",
        buildBookmarkExportMessage(exportBookmarks.length, selectedBookmarks.length),
      );
    } catch (_error) {
      showSiteToast("error", "Bookmark export is unavailable in this browser.");
    }
  });

  elements.bookmarkImportTrigger?.addEventListener("click", () => {
    if (!elements.bookmarkImportInput) {
      showSiteToast("error", "Bookmark import is unavailable in this browser.");
      return;
    }
    elements.bookmarkImportInput.value = "";
    elements.bookmarkImportInput.click();
  });

  elements.bookmarkImportInput?.addEventListener("change", async () => {
    const file = elements.bookmarkImportInput?.files?.[0];
    if (!file) {
      return;
    }
    try {
      const importedBookmarks = parseImportedBookmarks(await readBookmarkImportFile(file));
      if (!importedBookmarks.length) {
        showSiteToast("warning", "The selected file did not contain any bookmark XML.");
        return;
      }
      const current = bundle();
      const merged = mergeImportedBookmarks(current.bookmarks, importedBookmarks);
      const importedCount = merged.length - current.bookmarks.length;
      const skippedCount = importedBookmarks.length - importedCount;
      const importedIds = merged.slice(current.bookmarks.length).map((bookmark) => bookmark.id);
      writeBookmarkState((bookmarks, bookmarkUi) => {
        bookmarks.splice(0, bookmarks.length, ...merged);
        bookmarkUi.placing = false;
        if (importedIds.length) {
          bookmarkUi.selectedIds = normalizeSelectedBookmarkIds(
            merged,
            bookmarkUi.selectedIds.concat(importedIds),
          );
        }
      });
      showSiteToast(
        importedCount ? "success" : "info",
        buildBookmarkImportMessage(importedCount, skippedCount),
      );
    } catch (error) {
      console.warn("Failed to import map bookmarks", error);
      showSiteToast("error", "Bookmark import failed. Choose a valid WorldmapBookMark XML file.");
    } finally {
      elements.bookmarkImportInput.value = "";
    }
  });

  elements.bookmarksList.addEventListener("dragstart", (event) => {
    const handle = event.target.closest("button[data-bookmark-drag][draggable='true']");
    const card = handle?.closest(".fishymap-bookmark-card");
    if (!handle || !card) {
      return;
    }
    const bookmarkId = String(card.getAttribute("data-bookmark-id") || "").trim();
    if (!bookmarkId) {
      return;
    }
    state.draggingBookmarkId = bookmarkId;
    card.dataset.dragging = "true";
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", bookmarkId);
    }
  });

  elements.bookmarksList.addEventListener("dragover", (event) => {
    if (!state.draggingBookmarkId) {
      return;
    }
    event.preventDefault();
    const card = event.target.closest(".fishymap-bookmark-card");
    if (!card) {
      return;
    }
    const bookmarkId = String(card.getAttribute("data-bookmark-id") || "").trim();
    if (!bookmarkId || bookmarkId === state.draggingBookmarkId) {
      return;
    }
    const rect = card.getBoundingClientRect();
    const offsetY = event.clientY - rect.top;
    state.dropBookmarkId = bookmarkId;
    state.dropPosition = offsetY >= rect.height / 2 ? "after" : "before";
    elements.bookmarksList
      .querySelectorAll(".fishymap-bookmark-card")
      .forEach((candidate) => {
        if (candidate.getAttribute("data-bookmark-id") === state.dropBookmarkId) {
          candidate.dataset.dropPosition = state.dropPosition;
          return;
        }
        delete candidate.dataset.dropPosition;
      });
  });

  elements.bookmarksList.addEventListener("drop", (event) => {
    if (!state.draggingBookmarkId || !state.dropBookmarkId || !state.dropPosition) {
      return;
    }
    event.preventDefault();
    writeBookmarkState((bookmarks) => {
      const next = moveBookmarkBefore(
        bookmarks,
        state.draggingBookmarkId,
        state.dropBookmarkId,
        state.dropPosition,
      );
      bookmarks.splice(0, bookmarks.length, ...next);
    });
    state.draggingBookmarkId = "";
    state.dropBookmarkId = "";
    state.dropPosition = "";
    elements.bookmarksList
      .querySelectorAll(".fishymap-bookmark-card")
      .forEach((candidate) => {
        delete candidate.dataset.dragging;
        delete candidate.dataset.dropPosition;
      });
  });

  elements.bookmarksList.addEventListener("dragend", () => {
    state.draggingBookmarkId = "";
    state.dropBookmarkId = "";
    state.dropPosition = "";
    elements.bookmarksList
      .querySelectorAll(".fishymap-bookmark-card")
      .forEach((candidate) => {
        delete candidate.dataset.dragging;
        delete candidate.dataset.dropPosition;
      });
  });

  if (listenToSignalPatches) {
    documentRef?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);
  }
  shell.addEventListener("fishymap:selection-changed", handleSelectionChanged);

  return Object.freeze({
    render,
    scheduleRender,
    setZoneCatalog(nextZoneCatalog) {
      state.zoneCatalog = Array.isArray(nextZoneCatalog) ? cloneJson(nextZoneCatalog) : [];
      scheduleRender();
    },
  });
}
