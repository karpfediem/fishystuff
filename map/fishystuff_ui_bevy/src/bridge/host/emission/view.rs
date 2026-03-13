use super::super::persistence::contract_view_mode;
use super::super::snapshot::effective_view_snapshot;
use super::super::*;

pub(in crate::bridge::host) fn emit_view_changed_event(
    time: Res<Time>,
    view_mode: Res<ViewModeState>,
    map_view: Res<Map2dViewState>,
    terrain_view: Res<Terrain3dViewState>,
) {
    let payload = FishyMapOutputEvent::ViewChanged {
        version: 1,
        view_mode: contract_view_mode(view_mode.mode),
        camera: effective_view_snapshot(&view_mode, &map_view, &terrain_view).camera,
    };
    let serialized = match serde_json::to_string(&payload) {
        Ok(value) => value,
        Err(_) => return,
    };
    let now = time.elapsed_secs_f64();

    LAST_VIEW_PAYLOAD.with(|last_payload| {
        LAST_VIEW_EMIT_SECS.with(|last_emit| {
            let mut last_payload = last_payload.borrow_mut();
            let mut last_emit = last_emit.borrow_mut();
            let changed = last_payload.as_deref() != Some(serialized.as_str());
            let interval_elapsed = now - *last_emit >= 0.12;
            let mode_changed = view_mode.is_changed();
            if !(mode_changed || (changed && interval_elapsed)) {
                return;
            }
            super::super::emit_event(&payload);
            *last_payload = Some(serialized);
            *last_emit = now;
        });
    });
}
