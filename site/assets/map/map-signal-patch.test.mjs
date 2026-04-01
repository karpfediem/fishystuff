import test from "node:test";
import assert from "node:assert/strict";

import {
  combineSignalPatches,
  dispatchShellPatchedSignalEvent,
  dispatchShellSignalPatch,
  FISHYMAP_SIGNAL_PATCHED_EVENT,
  FISHYMAP_SIGNAL_PATCH_EVENT,
} from "./map-signal-patch.js";

test("combineSignalPatches deep-clones top-level branches", () => {
  const input = { _map_runtime: { ready: true } };
  const session = { _map_session: { view: { viewMode: "3d" } } };
  const combined = combineSignalPatches(input, session);

  assert.deepEqual(combined, {
    _map_runtime: { ready: true },
    _map_session: { view: { viewMode: "3d" } },
  });

  input._map_runtime.ready = false;
  session._map_session.view.viewMode = "2d";

  assert.deepEqual(combined, {
    _map_runtime: { ready: true },
    _map_session: { view: { viewMode: "3d" } },
  });
});

test("dispatchShellSignalPatch emits a cloned bubbling custom event", () => {
  let dispatchedEvent = null;
  class CustomEventStub {
    constructor(type, init = {}) {
      this.type = type;
      this.detail = init.detail;
      this.bubbles = init.bubbles;
    }
  }
  const shell = {
    dispatchEvent(event) {
      dispatchedEvent = event;
      return true;
    },
  };
  const patch = { _map_ui: { windowUi: { search: { open: true } } } };

  const result = dispatchShellSignalPatch(shell, patch, CustomEventStub);

  assert.equal(result, true);
  assert.equal(dispatchedEvent.type, FISHYMAP_SIGNAL_PATCH_EVENT);
  assert.equal(dispatchedEvent.bubbles, true);
  assert.deepEqual(dispatchedEvent.detail, patch);

  patch._map_ui.windowUi.search.open = false;

  assert.deepEqual(dispatchedEvent.detail, {
    _map_ui: { windowUi: { search: { open: true } } },
  });
});

test("dispatchShellPatchedSignalEvent emits the shell-local applied patch event", () => {
  let dispatchedEvent = null;
  class CustomEventStub {
    constructor(type, init = {}) {
      this.type = type;
      this.detail = init.detail;
      this.bubbles = init.bubbles;
    }
  }
  const shell = {
    dispatchEvent(event) {
      dispatchedEvent = event;
      return true;
    },
  };

  const result = dispatchShellPatchedSignalEvent(
    shell,
    { _map_runtime: { ready: true } },
    CustomEventStub,
  );

  assert.equal(result, true);
  assert.equal(dispatchedEvent.type, FISHYMAP_SIGNAL_PATCHED_EVENT);
  assert.equal(dispatchedEvent.bubbles, true);
  assert.deepEqual(dispatchedEvent.detail, { _map_runtime: { ready: true } });
});
