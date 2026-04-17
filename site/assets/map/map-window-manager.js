import {
  dispatchShellSignalPatch,
  FISHYMAP_SIGNAL_PATCHED_EVENT,
} from "./map-signal-patch.js";
import { normalizeWindowUiState } from "./map-signal-contract.js";

const DRAG_THRESHOLD_PX = 4;
const WINDOW_TITLEBAR_FALLBACK_HEIGHT_PX = 56;
const WINDOW_DRAG_IGNORE_SELECTOR =
  "input, textarea, select, button, a, label, summary, [data-window-drag-ignore='true']";

function isFiniteCoordinate(value) {
  return Number.isFinite(value);
}

export function clampManagedWindowPosition(shellRect, rootRect, titlebarHeight, x, y) {
  return {
    x: Math.max(0, Math.min(Math.round(x), Math.max(0, Math.round(shellRect.width - Math.min(rootRect.width, shellRect.width))))),
    y: Math.max(0, Math.min(Math.round(y), Math.max(0, Math.round(shellRect.height - titlebarHeight)))),
  };
}

export function buildWindowUiEntryPatch(windowUiState, windowId, entryPatch) {
  const nextWindowUi = normalizeWindowUiState({
    ...(windowUiState && typeof windowUiState === "object" ? windowUiState : {}),
    [windowId]: {
      ...(windowUiState?.[windowId] && typeof windowUiState[windowId] === "object"
        ? windowUiState[windowId]
        : {}),
      ...(entryPatch && typeof entryPatch === "object" ? entryPatch : {}),
    },
  });
  return {
    _map_ui: {
      windowUi: {
        [windowId]: nextWindowUi[windowId],
      },
    },
  };
}

function shouldIgnoreWindowDragTarget(target) {
  return Boolean(target?.closest?.(WINDOW_DRAG_IGNORE_SELECTOR));
}

function currentWindowUi(getSignals) {
  return normalizeWindowUiState(getSignals?.()?._map_ui?.windowUi);
}

function currentManagedWindowPosition(shell, part, entry) {
  if (isFiniteCoordinate(entry?.x) && isFiniteCoordinate(entry?.y)) {
    return { x: entry.x, y: entry.y };
  }
  const shellRect = shell.getBoundingClientRect();
  const rootRect = part.root.getBoundingClientRect();
  return {
    x: Math.round(rootRect.left - shellRect.left),
    y: Math.round(rootRect.top - shellRect.top),
  };
}

export function patchTouchesWindowUi(patch) {
  return Boolean(
    patch &&
      typeof patch === "object" &&
      patch._map_ui &&
      typeof patch._map_ui === "object" &&
      patch._map_ui.windowUi &&
      typeof patch._map_ui.windowUi === "object",
  );
}

function applyManagedWindowPosition(root, entry) {
  if (isFiniteCoordinate(entry?.x) && isFiniteCoordinate(entry?.y)) {
    root.style.left = `${entry.x}px`;
    root.style.top = `${entry.y}px`;
    root.style.right = "auto";
    root.style.bottom = "auto";
    if (root.id === "fishymap-search-window") {
      root.style.transform = "none";
    }
    return;
  }
  root.style.removeProperty("left");
  root.style.removeProperty("top");
  root.style.removeProperty("right");
  root.style.removeProperty("bottom");
  root.style.removeProperty("transform");
}

export function createMapWindowManager({
  shell,
  getSignals,
  dispatchPatch = dispatchShellSignalPatch,
  listenToSignalPatches = true,
} = {}) {
  if (!shell || typeof shell.querySelectorAll !== "function") {
    throw new Error("createMapWindowManager requires a shell element");
  }

  const parts = Object.fromEntries(
    Array.from(shell.querySelectorAll("[data-window-id]")).map((root) => {
      const windowId = String(root.getAttribute("data-window-id") || "").trim();
      return [
        windowId,
        {
          root,
          titlebar: shell.querySelector(`[data-window-titlebar="${windowId}"]`),
        },
      ];
    }).filter(([windowId]) => windowId),
  );

  const state = {
    previousWindowUi: currentWindowUi(getSignals),
    nextWindowZIndex: 24,
    frameId: 0,
    drag: {
      windowId: null,
      pointerId: null,
      titlebar: null,
      startClientX: 0,
      startClientY: 0,
      originX: 0,
      originY: 0,
      moved: false,
    },
  };

  function bringToFront(windowId) {
    const root = parts[windowId]?.root;
    if (!root) {
      return;
    }
    state.nextWindowZIndex += 1;
    root.style.zIndex = String(state.nextWindowZIndex);
  }

  function shouldApplyWindowPositionFromPatch(windowId, patch) {
    const windowUiPatch = patch && patch._map_ui && typeof patch._map_ui === "object"
      ? patch._map_ui.windowUi
      : null;
    if (!windowUiPatch || typeof windowUiPatch !== "object") {
      return true;
    }
    const nextWindowUiPatch = windowUiPatch[windowId];
    if (!nextWindowUiPatch || typeof nextWindowUiPatch !== "object") {
      return false;
    }
    return (
      isFiniteCoordinate(nextWindowUiPatch.x)
      || isFiniteCoordinate(nextWindowUiPatch.y)
    );
  }

  function applyFromSignals(patch = null) {
    const nextWindowUi = currentWindowUi(getSignals);
    for (const [windowId, part] of Object.entries(parts)) {
      const nextEntry = nextWindowUi[windowId];
      const previousEntry = state.previousWindowUi?.[windowId];
      if (state.drag.windowId !== windowId && shouldApplyWindowPositionFromPatch(windowId, patch)) {
        applyManagedWindowPosition(part.root, nextEntry);
      }
      if (nextEntry?.open !== false && previousEntry?.open === false) {
        bringToFront(windowId);
      }
    }
    state.previousWindowUi = nextWindowUi;
  }

  function scheduleApplyFromSignals(patch = null) {
    if (state.frameId || typeof globalThis.requestAnimationFrame !== "function") {
      if (!state.frameId) {
        applyFromSignals(patch);
      }
      return;
    }
    state.frameId = globalThis.requestAnimationFrame(() => {
      state.frameId = 0;
      applyFromSignals(patch);
    });
  }

  function clearDrag() {
    if (state.drag.windowId) {
      delete parts[state.drag.windowId]?.root?.dataset?.dragging;
    }
    state.drag.windowId = null;
    state.drag.pointerId = null;
    state.drag.titlebar = null;
    state.drag.moved = false;
  }

  function finishDrag(toggleOnTap) {
    const windowId = state.drag.windowId;
    const pointerId = state.drag.pointerId;
    const titlebar = state.drag.titlebar;
    const moved = state.drag.moved;
    if (titlebar && pointerId != null && titlebar.hasPointerCapture?.(pointerId)) {
      titlebar.releasePointerCapture(pointerId);
    }
    clearDrag();
    if (!windowId) {
      return;
    }
    if (!moved && toggleOnTap && windowId !== "search") {
      const windowUiState = currentWindowUi(getSignals);
      const currentEntry = windowUiState[windowId];
      dispatchPatch(shell, buildWindowUiEntryPatch(windowUiState, windowId, {
        collapsed: !currentEntry?.collapsed,
      }));
      bringToFront(windowId);
      return;
    }
    scheduleApplyFromSignals();
  }

  function handlePointerMove(event) {
    if (state.drag.pointerId !== event.pointerId || !state.drag.windowId) {
      return;
    }
    const part = parts[state.drag.windowId];
    if (!part?.root) {
      finishDrag(false);
      return;
    }
    const deltaX = event.clientX - state.drag.startClientX;
    const deltaY = event.clientY - state.drag.startClientY;
    if (!state.drag.moved && Math.abs(deltaX) < DRAG_THRESHOLD_PX && Math.abs(deltaY) < DRAG_THRESHOLD_PX) {
      return;
    }
    state.drag.moved = true;
    part.root.dataset.dragging = "true";
    const shellRect = shell.getBoundingClientRect();
    const rootRect = part.root.getBoundingClientRect();
    const titlebarHeight = part.titlebar?.offsetHeight || WINDOW_TITLEBAR_FALLBACK_HEIGHT_PX;
    const clamped = clampManagedWindowPosition(
      shellRect,
      rootRect,
      titlebarHeight,
      state.drag.originX + deltaX,
      state.drag.originY + deltaY,
    );
    applyManagedWindowPosition(part.root, clamped);
  }

  function handlePointerUp(event) {
    if (state.drag.pointerId !== event.pointerId || !state.drag.windowId) {
      return;
    }
    const windowId = state.drag.windowId;
    const part = parts[windowId];
    if (state.drag.moved && part?.root) {
      const shellRect = shell.getBoundingClientRect();
      const rootRect = part.root.getBoundingClientRect();
      const titlebarHeight = part.titlebar?.offsetHeight || WINDOW_TITLEBAR_FALLBACK_HEIGHT_PX;
      const clamped = clampManagedWindowPosition(
        shellRect,
        rootRect,
        titlebarHeight,
        state.drag.originX + (event.clientX - state.drag.startClientX),
        state.drag.originY + (event.clientY - state.drag.startClientY),
      );
      dispatchPatch(shell, buildWindowUiEntryPatch(currentWindowUi(getSignals), windowId, clamped));
    }
    finishDrag(true);
  }

  function handlePointerCancel(event) {
    if (state.drag.pointerId !== event.pointerId) {
      return;
    }
    finishDrag(false);
  }

  for (const [windowId, part] of Object.entries(parts)) {
    if (!part.titlebar) {
      continue;
    }
    part.titlebar.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || shouldIgnoreWindowDragTarget(event.target)) {
        return;
      }
      const windowUiState = currentWindowUi(getSignals);
      const entry = windowUiState[windowId];
      state.drag.windowId = windowId;
      state.drag.pointerId = event.pointerId;
      state.drag.titlebar = part.titlebar;
      state.drag.startClientX = event.clientX;
      state.drag.startClientY = event.clientY;
      const position = currentManagedWindowPosition(shell, part, entry);
      state.drag.originX = position.x;
      state.drag.originY = position.y;
      state.drag.moved = false;
      part.titlebar.setPointerCapture?.(event.pointerId);
      bringToFront(windowId);
    });
  }

  globalThis.addEventListener?.("pointermove", handlePointerMove);
  globalThis.addEventListener?.("pointerup", handlePointerUp);
  globalThis.addEventListener?.("pointercancel", handlePointerCancel);
  globalThis.addEventListener?.("resize", scheduleApplyFromSignals);
  if (listenToSignalPatches) {
    shell.addEventListener(FISHYMAP_SIGNAL_PATCHED_EVENT, (event) => {
      if (patchTouchesWindowUi(event?.detail || null)) {
        scheduleApplyFromSignals(event?.detail || null);
      }
    });
  }

  applyFromSignals();

  return Object.freeze({
    applyFromSignals,
    scheduleApplyFromSignals,
  });
}
