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

test("buildPatchPickerStateBundle keeps only runtime patch catalog and derived patch selections", () => {
  assert.deepEqual(
    buildPatchPickerStateBundle({
      _map_runtime: {
        ready: true,
        catalog: {
          patches: [{ patchId: "2026-03-12", patchName: "New Era", startTsUtc: 200 }],
          fish: [{ fishId: 1 }],
        },
      },
      _map_ui: {
        search: {
          expression: {
            type: "group",
            operator: "or",
            children: [
              { type: "term", term: { kind: "patch-bound", bound: "from", patchId: "2026-02-26" } },
              { type: "term", term: { kind: "patch-bound", bound: "to", patchId: "2026-03-12" } },
            ],
          },
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
          patchId: null,
          fromPatchId: "2026-02-26",
          toPatchId: "2026-03-12",
        },
      },
    },
  );
});

test("buildPatchPickerStateBundle preserves an open-ended until selection as null", () => {
  assert.deepEqual(
    buildPatchPickerStateBundle({
      _map_runtime: {
        ready: true,
        catalog: {
          patches: [{ patchId: "2026-03-12", patchName: "New Era", startTsUtc: 200 }],
        },
      },
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "patch-bound", bound: "from", patchId: "2026-02-26" }],
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
          patchId: null,
          fromPatchId: "2026-02-26",
          toPatchId: null,
        },
      },
    },
  );
});

test("buildPatchPickerStateBundle leaves the patch window unbounded by default", () => {
  assert.deepEqual(
    buildPatchPickerStateBundle({
      _map_runtime: {
        ready: true,
        catalog: {
          patches: [
            { patchId: "2026-03-12", patchName: "New Era", startTsUtc: 200 },
            { patchId: "2026-02-26", patchName: "Old Guard", startTsUtc: 100 },
          ],
        },
      },
      _map_ui: {
        search: {
          expression: { type: "group", operator: "or", children: [] },
        },
      },
    }),
    {
      state: {
        ready: true,
        catalog: {
          patches: [
            { patchId: "2026-03-12", label: "New Era", startTsUtc: 200 },
            { patchId: "2026-02-26", label: "Old Guard", startTsUtc: 100 },
          ],
        },
      },
      inputState: {
        filters: {
          patchId: null,
          fromPatchId: null,
          toPatchId: null,
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
      _map_ui: {
        search: {
          selectedTerms: [{ kind: "patch-bound", bound: "from", patchId: "2026-02-26" }],
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
