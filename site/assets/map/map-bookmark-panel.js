export function bookmarkClearSelectionLabel(selectedCount) {
  return selectedCount > 0 ? `Clear (${selectedCount})` : "Clear";
}

export function renderBookmarkManager(elements, stateBundle, bookmarks, bookmarkUi, options = {}) {
  if (
    !elements.bookmarksList ||
    !elements.bookmarkPlace ||
    !elements.bookmarkPlaceLabel
  ) {
    return;
  }

  const resolveDisplayBookmarks =
    typeof options.resolveDisplayBookmarks === "function"
      ? options.resolveDisplayBookmarks
      : (_stateBundle, value) => (Array.isArray(value) ? value : []);
  const normalizeSelectedBookmarkIds =
    typeof options.normalizeSelectedBookmarkIds === "function"
      ? options.normalizeSelectedBookmarkIds
      : (_bookmarks, selectedIds) => (Array.isArray(selectedIds) ? selectedIds : []);
  const setBooleanProperty =
    typeof options.setBooleanProperty === "function" ? options.setBooleanProperty : () => {};
  const setTextContent =
    typeof options.setTextContent === "function" ? options.setTextContent : () => {};
  const setMarkup = typeof options.setMarkup === "function" ? options.setMarkup : () => {};
  const buildBookmarkOverviewRows =
    typeof options.buildBookmarkOverviewRows === "function"
      ? options.buildBookmarkOverviewRows
      : () => [];
  const bookmarkDisplayLabel =
    typeof options.bookmarkDisplayLabel === "function"
      ? options.bookmarkDisplayLabel
      : (bookmark, fallbackIndex = 0) =>
          String(bookmark?.label || "").trim() || `Bookmark ${fallbackIndex + 1}`;
  const overviewRowMarkup =
    typeof options.overviewRowMarkup === "function" ? options.overviewRowMarkup : () => "";
  const escapeHtml = typeof options.escapeHtml === "function" ? options.escapeHtml : (value) => String(value || "");
  const dragHandleIcon =
    typeof options.dragHandleIcon === "function" ? options.dragHandleIcon : () => "";
  const spriteIcon = typeof options.spriteIcon === "function" ? options.spriteIcon : () => "";

  const state = stateBundle?.state || {};
  const canPlace = state.ready === true && state.view?.viewMode !== "3d";

  if (elements.shell) {
    if (bookmarkUi?.placing) {
      elements.shell.dataset.bookmarkPlacing = "true";
    } else {
      delete elements.shell.dataset.bookmarkPlacing;
    }
  }

  const normalizedBookmarks = resolveDisplayBookmarks(stateBundle, bookmarks);
  const selectedIds = normalizeSelectedBookmarkIds(normalizedBookmarks, bookmarkUi?.selectedIds);
  const selectedIdSet = new Set(selectedIds);

  setBooleanProperty(elements.bookmarkPlace, "disabled", !canPlace && !bookmarkUi?.placing);
  setTextContent(
    elements.bookmarkPlaceLabel,
    bookmarkUi?.placing ? "Click map to place" : "New bookmark",
  );
  setBooleanProperty(elements.bookmarkCopySelected, "disabled", selectedIds.length === 0);
  setBooleanProperty(elements.bookmarkExport, "disabled", normalizedBookmarks.length === 0);
  setBooleanProperty(
    elements.bookmarkSelectAll,
    "disabled",
    normalizedBookmarks.length === 0 || selectedIds.length === normalizedBookmarks.length,
  );
  setBooleanProperty(elements.bookmarkDeleteSelected, "disabled", selectedIds.length === 0);
  setBooleanProperty(elements.bookmarkClearSelection, "disabled", selectedIds.length === 0);
  setTextContent(
    elements.bookmarkClearSelectionLabel,
    bookmarkClearSelectionLabel(selectedIds.length),
  );
  setBooleanProperty(elements.bookmarkCancel, "hidden", !bookmarkUi?.placing);

  setMarkup(
    elements.bookmarksList,
    JSON.stringify({
      bookmarks: normalizedBookmarks,
      selectedIds,
    }),
    normalizedBookmarks.length
      ? normalizedBookmarks
          .map((bookmark, index) => {
            const overviewRows = buildBookmarkOverviewRows(bookmark, index, stateBundle);
            const [titleRow, ...detailRows] = overviewRows;
            const displayLabel = bookmarkDisplayLabel(bookmark, index, stateBundle);
            return `
              <div class="fishymap-bookmark-card rounded-box border border-base-300/70 bg-base-100" data-bookmark-id="${escapeHtml(bookmark.id)}">
                <div class="fishymap-bookmark-rail">
                  <button
                    class="fishymap-bookmark-drag btn btn-xs btn-circle btn-ghost"
                    data-bookmark-drag="${escapeHtml(bookmark.id)}"
                    type="button"
                    aria-label="Drag ${escapeHtml(displayLabel)}"
                    draggable="true"
                    tabindex="-1"
                  >
                    ${dragHandleIcon()}
                  </button>
                  <span class="fishymap-bookmark-order badge badge-soft badge-sm">${index + 1}</span>
                  <label class="fishymap-bookmark-toggle" aria-label="Select ${escapeHtml(displayLabel)}">
                    <input
                      class="checkbox checkbox-sm"
                      type="checkbox"
                      data-bookmark-select="${escapeHtml(bookmark.id)}"
                      ${selectedIdSet.has(bookmark.id) ? "checked" : ""}
                    >
                  </label>
                </div>
                <div class="fishymap-bookmark-main">
                  <div class="fishymap-bookmark-titlebar">
                    <div class="fishymap-bookmark-title">
                      ${titleRow ? overviewRowMarkup(titleRow) : ""}
                    </div>
                    <button
                      class="fishymap-bookmark-rename btn btn-soft btn-sm btn-square"
                      type="button"
                      data-bookmark-rename="${escapeHtml(bookmark.id)}"
                      aria-label="Rename bookmark"
                      title="Rename bookmark"
                    >
                      ${spriteIcon("bookmark-edit", "size-5")}
                    </button>
                  </div>
                  ${
                    detailRows.length
                      ? `
                    <div class="fishymap-overview-list fishymap-overview-list--bookmark">
                      ${detailRows.map((row) => overviewRowMarkup(row)).join("")}
                    </div>
                  `
                      : ""
                  }
                </div>
                <div class="fishymap-bookmark-actions-rail">
                  <button
                    class="fishymap-bookmark-activate btn btn-soft btn-sm btn-square"
                    type="button"
                    data-bookmark-activate="${escapeHtml(bookmark.id)}"
                    aria-label="Inspect bookmark"
                    title="Inspect bookmark"
                  >
                    ${spriteIcon("map-view", "size-5")}
                  </button>
                  <button
                    class="fishymap-bookmark-copy btn btn-soft btn-primary btn-sm btn-square"
                    type="button"
                    data-bookmark-copy="${escapeHtml(bookmark.id)}"
                    aria-label="Copy bookmark XML"
                    title="Copy bookmark XML"
                  >
                    ${spriteIcon("copy", "size-5")}
                  </button>
                  <button
                    class="fishymap-bookmark-delete btn btn-ghost btn-error btn-xs btn-square"
                    type="button"
                    data-bookmark-delete="${escapeHtml(bookmark.id)}"
                    aria-label="Delete bookmark"
                    title="Delete bookmark"
                  >
                    ${spriteIcon("trash", "size-4")}
                  </button>
                </div>
              </div>
            `;
          })
          .join("")
      : `
        <div class="fishymap-bookmark-empty text-sm text-base-content/65">
          No bookmarks yet.
        </div>
      `,
  );
}
