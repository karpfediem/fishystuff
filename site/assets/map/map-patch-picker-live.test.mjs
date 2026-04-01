import test from "node:test";
import assert from "node:assert/strict";

import {
  buildPatchPickerStateBundle,
  normalizePatchCatalog,
  patchTouchesPatchPickerSignals,
} from "./map-patch-picker-live.js";

test("normalizePatchCatalog orders patches by newest first and falls back to patch id labels", () => {
  assert.deepEqual(
    normalizePatchCatalog([
      { patchId: "2026-02-26", startTsUtc: 100, patchName: "Valencia Ballad" },
      { patchId: "2026-03-12", startTsUtc: 200 },
      { patchId: "2026-01-01", startTsUtc: 50, patchName: "Calpheon Overture" },
    ]),
    [
      { patchId: "2026-03-12", label: "2026-03-12", startTsUtc: 200 },
      { patchId: "2026-02-26", label: "Valencia Ballad", startTsUtc: 100 },
      { patchId: "2026-01-01", label: "Calpheon Overture", startTsUtc: 50 },
    ],
  );
});

test("buildPatchPickerStateBundle keeps only runtime patch catalog and bridged patch selections", () => {
  assert.deepEqual(
    buildPatchPickerStateBundle({
      _map_runtime: {
        ready: true,
        catalog: {
          patches: [{ patchId: "2026-03-12", patchName: "New Era", startTsUtc: 200 }],
          fish: [{ fishId: 1 }],
        },
      },
      _map_bridged: {
        filters: {
          patchId: "legacy-patch",
          fromPatchId: "2026-02-26",
          toPatchId: "2026-03-12",
          fishIds: [42],
        },
      },
    }),
    {
      state: {
        ready: true,
        catalog: {
          patches: [{ patchId: "2026-03-12", label: "New Era", startTsUtc: 200 }],
        },
      },
      inputState: {
        filters: {
          patchId: "legacy-patch",
          fromPatchId: "2026-02-26",
          toPatchId: "2026-03-12",
        },
      },
    },
  );
});

test("patchTouchesPatchPickerSignals only reacts to patch-relevant branches", () => {
  assert.equal(
    patchTouchesPatchPickerSignals({
      _map_runtime: {
        catalog: {
          patches: [{ patchId: "2026-03-12" }],
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesPatchPickerSignals({
      _map_bridged: {
        filters: {
          fromPatchId: "2026-02-26",
        },
      },
    }),
    true,
  );
  assert.equal(
    patchTouchesPatchPickerSignals({
      _map_runtime: {
        selection: {
          pointKind: "clicked",
        },
      },
    }),
    false,
  );
  assert.equal(
    patchTouchesPatchPickerSignals({
      _map_ui: {
        windowUi: {
          settings: { open: false },
        },
      },
    }),
    false,
  );
});
