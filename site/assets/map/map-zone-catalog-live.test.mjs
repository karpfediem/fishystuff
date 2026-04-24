import { test } from "bun:test";
import assert from "node:assert/strict";

import {
  dispatchShellZoneCatalogReadyEvent,
  FISHYMAP_ZONE_CATALOG_READY_EVENT,
} from "./map-zone-catalog-live.js";

test("dispatchShellZoneCatalogReadyEvent emits a cloned bubbling catalog event", () => {
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
  const zoneCatalog = [{ zoneRgb: 123, name: "Alpha Sea" }];

  const result = dispatchShellZoneCatalogReadyEvent(shell, zoneCatalog, CustomEventStub);

  assert.equal(result, true);
  assert.equal(dispatchedEvent.type, FISHYMAP_ZONE_CATALOG_READY_EVENT);
  assert.equal(dispatchedEvent.bubbles, true);
  assert.deepEqual(dispatchedEvent.detail, {
    zoneCatalog: [{ zoneRgb: 123, name: "Alpha Sea" }],
  });

  zoneCatalog[0].name = "Beta Sea";
  assert.deepEqual(dispatchedEvent.detail, {
    zoneCatalog: [{ zoneRgb: 123, name: "Alpha Sea" }],
  });
});
