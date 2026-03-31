import { DATASTAR_SIGNAL_PATCH_EVENT } from "../js/datastar-signals.js";
import { renderBookmarkManager } from "./map-bookmark-panel.js";
import { dispatchShellSignalPatch } from "./map-signal-patch.js";
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
  return `
    <div class="fishymap-overview-row">
      <span class="fishymap-overview-row-icon" aria-hidden="true">${spriteIcon(icon, "size-4")}</span>
      <span class="fishymap-overview-row-label">${escapeHtml(label)}</span>
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

export function createMapBookmarkPanelController({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  documentRef = globalThis.document,
  requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis),
  promptImpl = globalThis.prompt?.bind(globalThis),
  confirmImpl = globalThis.confirm?.bind(globalThis),
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
  };

  function signals() {
    return getSignals() || null;
  }

  function bundle() {
    return buildBookmarkPanelStateBundle(signals());
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
    setBooleanProperty(elements.bookmarkCopySelected, "disabled", true);
    setBooleanProperty(elements.bookmarkExport, "disabled", true);
    setBooleanProperty(elements.bookmarkImportTrigger, "disabled", true);
    elements.bookmarksList
      .querySelectorAll("button[data-bookmark-copy]")
      .forEach((button) => {
        button.disabled = true;
      });
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
    const nextPlacementKey = selectionBookmarkKey(current.state.selection);
    if (!nextPlacementKey || nextPlacementKey === state.lastPlacementKey) {
      return false;
    }
    state.lastPlacementKey = nextPlacementKey;
    const bookmark = createBookmarkFromSelection(current.state.selection, current.bookmarks);
    if (!bookmark) {
      return false;
    }
    writeBookmarkState((bookmarks, bookmarkUi) => {
      bookmarks.push(bookmark);
      bookmarkUi.placing = false;
      bookmarkUi.selectedIds = [bookmark.id];
    });
    return true;
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

  documentRef?.addEventListener?.(DATASTAR_SIGNAL_PATCH_EVENT, handleSignalPatch);

  return Object.freeze({
    render,
    scheduleRender,
  });
}
