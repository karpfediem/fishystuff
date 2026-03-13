# map

Bevy WASM runtime only.

This component should own:

- the `fishystuff_ui_bevy` crate
- browser bridge interop
- rendering, camera, terrain, raster, vector, and interaction runtime code

This component should depend on shared crates under `lib/`, not on API-server internals.
