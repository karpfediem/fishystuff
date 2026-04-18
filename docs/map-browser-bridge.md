# Map Browser Bridge

This document defines the current browser-facing contract for `/map`.

## Ownership split

- HTML/DaisyUI page code owns:
  - layout
  - search, selectors, toggles, legends, diagnostics chrome
  - theme selection and resolved theme tokens
  - URL/session/local restore
- `window.FishyMapBridge` owns:
  - WASM lifecycle
  - DOM-event translation
  - debounced state patch forwarding
  - persistence helpers
- Bevy WASM owns:
  - rendering
  - camera movement
  - hit testing
  - hover/selection state
  - terrain and layer interaction

## Public DOM events

Dispatch these on the map root/container:

- `fishymap:set-state`
- `fishymap:command`
- `fishymap:request-state`

The bridge emits these from the same container:

- `fishymap:ready`
- `fishymap:view-changed`
- `fishymap:selection-changed`
- `fishymap:hover-changed`
- `fishymap:diagnostic`

`fishymap:request-state` is synchronous. The bridge fills `event.detail.state` and `event.detail.inputState` during dispatch.

## Lifecycle bridge

Only page bootstrap/lifecycle code should call the bridge directly:

```js
window.FishyMapBridge = {
  mount(container, options) {},
  destroy() {},
  setState(patch) {},
  sendCommand(command) {},
  getCurrentState() {},
  on(type, handler) {},
  off(type, handler) {},
};
```

Normal page UI should prefer DOM events over direct bridge calls.

## State contract

All browser→WASM payloads are versioned JSON with `version: 1`.

Primary patch shape:

```json
{
  "version": 1,
  "theme": {
    "name": "fishy",
    "colors": {
      "base100": "rgb(16 24 32 / 1)",
      "primary": "rgb(240 120 60 / 1)",
      "primaryContent": "rgb(255 255 255 / 1)"
    }
  },
  "filters": {
    "fishIds": [821015],
    "searchText": "coel",
    "prizeOnly": false,
    "fromPatchId": "2026-02-26",
    "toPatchId": "2026-03-12",
    "layerIdsVisible": ["terrain", "zones"],
    "layerIdsOrdered": ["zones", "terrain", "minimap"],
    "layerOpacities": {
      "zones": 0.7
    },
    "layerClipMasks": {
      "terrain": "zones"
    }
  },
  "ui": {
    "diagnosticsOpen": false,
    "legendOpen": true,
    "leftPanelOpen": true,
    "showPoints": true,
    "showPointIcons": true,
    "pointIconScale": 1.5
  },
  "commands": {
    "resetView": false,
    "setViewMode": "3d",
    "selectZoneRgb": 1193046,
    "restoreView": {
      "viewMode": "3d",
      "camera": {
        "pivotWorldX": 1000,
        "pivotWorldZ": 2000,
        "yaw": 0.4,
        "pitch": -0.7,
        "distance": 5200
      }
    }
  }
}
```

WASM→browser events currently emit:

- `ready`
- `view-changed`
- `selection-changed`
- `hover-changed`
- `diagnostic`

The bridge refreshes the full snapshot for `ready`, `view-changed`, `selection-changed`, and `diagnostic` events and includes it in the DOM event detail as `detail.state`.

`hover-changed` is intentionally lighter-weight and carries only the hover payload needed for the cursor tooltip:

```json
{
  "type": "hover-changed",
  "version": 1,
  "worldX": 123.4,
  "worldZ": 567.8,
  "zoneRgb": 1193046,
  "zoneName": "Coastal Shelf"
}
```

## Theme sync

- The webpage is authoritative for theme choice.
- JS resolves concrete DaisyUI colors from the active theme.
- The bridge sends resolved color tokens to WASM via `fishymap:set-state`.
- Rust should consume actual colors, not DaisyUI theme names or utility classes.

## Persistence

Current storage keys:

- `fishystuff.map.session.v1`
- `fishystuff.map.prefs.v1`
- `fishystuff.pokedex.caught.v1`

Restore order:

1. URL/query params
2. `sessionStorage`
3. `localStorage`
4. server/API defaults

Current usage:

- URL/query params:
  - `fish`
  - `focusFish`
  - `patch`
  - `fromPatch`
  - `patchFrom`
  - `toPatch`
  - `untilPatch`
  - `patchTo`
  - `view`
  - `mode`
  - `zone`
  - `layers`
  - `layerSet`
  - `search`
  - `prizeOnly`
  - `diagnostics`
  - `legend`
- `sessionStorage`:
  - current camera/view
  - selection
  - transient filters, including patch ranges
  - transient layer visibility/order/opacity/clip-mask overrides
  - open panel state
- `localStorage`:
  - preferred visible layers
  - preferred layer order
  - preferred layer opacity overrides
  - preferred layer clip-mask overrides
  - long-lived filter defaults, including patch ranges
  - panel defaults

## Deep links

Other pages should link into `/map` with query params rather than custom page-specific hooks.

Examples:

- `/map?fish=821015`
- `/map?fromPatch=2026-02-26&toPatch=2026-03-12&layers=terrain,zones`
- `/map?focusFish=821015&view=3d`

`patch` remains supported as a legacy exact-patch alias and expands to the same `fromPatchId` / `toPatchId`.

## Build / sync notes

- Canonical browser host sources now live directly in `site/assets/map/`.
- Run `tools/scripts/build_map.sh` after changing Bevy runtime code or the copied Bevy UI stylesheet.
- The script writes the generated wasm/js bundle into `data/cdn/public/map/` with hashed filenames plus a stable `runtime-manifest.json`.
- `site/assets/map/loader.js` and `site/assets/map/map-host.js` are hand-edited site-owned source files.
- `site/assets/map/ui/fishystuff.css` is a copied build output.
- `data/cdn/public/map/fishystuff_ui_bevy.<hash>.js` and `data/cdn/public/map/fishystuff_ui_bevy_bg.<hash>.wasm` are CDN-served build outputs.

## Future Performance Follow-up

- Fish icons currently work by resolving `fish_id -> item_id -> CDN image URL` and then fetching/decoding individual PNGs inside Bevy WASM.

## Browser Request Tracing

- Local distributed tracing is request-scoped, not frame-scoped.
- The browser host uses the JS OpenTelemetry Web SDK to instrument `fetch` and export to a direct OTLP/HTTP traces endpoint.
- The API emits request and store spans directly from Rust and propagates W3C trace context headers.
- `site/.out/runtime-config.js` is authoritative for whether browser tracing is enabled and which local endpoints it uses.
- The site is static-hostable, so browser tracing must use real API and OTLP collector URLs plus explicit CORS rather than relying on a site-side proxy.
- Bevy WASM does not stream `tracing` events into JS/OTEL continuously. If we need continuous runtime telemetry later, it must be a separate Rust-side bounded batch path designed around the browser bridge as a scarce bus.
- This is correct but not ideal for web performance because WASM image decode work still runs on the single browser/WASM thread.
- If icon load cost becomes a problem again, replace the per-icon remote image pipeline with one CDN-served fish icon atlas plus a small manifest keyed by `item_id`.
- Target shape:
  - API still returns fish/item IDs only.
  - CDN serves one atlas texture and one manifest.
  - Bevy loads the atlas once and does rect lookup by `item_id`.
  - No per-icon fetch/decode path in Bevy.
