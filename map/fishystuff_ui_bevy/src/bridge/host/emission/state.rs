use super::super::snapshot::hover_layer_samples_snapshot;
use super::super::*;

pub(in crate::bridge::host) fn emit_ready_event() {
    crate::perf_scope!("bridge.emit.ready");
    CURRENT_SNAPSHOT.with(|snapshot| {
        let snapshot = snapshot.borrow();
        READY_EMITTED.with(|emitted| {
            let mut emitted = emitted.borrow_mut();
            if *emitted || !snapshot.ready {
                return;
            }
            crate::perf_counter_add!("bridge.emit.ready.count", 1);
            super::super::emit_event(&FishyMapOutputEvent::Ready {
                version: snapshot.version,
                capabilities: snapshot.catalog.capabilities.clone(),
            });
            *emitted = true;
        });
    });
}

pub(in crate::bridge::host) fn emit_selection_changed_event(selection: Res<SelectionState>) {
    crate::perf_scope!("bridge.emit.selection");
    if !selection.is_changed() {
        return;
    }

    crate::perf_counter_add!("bridge.emit.selection.count", 1);
    let payload = FishyMapOutputEvent::SelectionChanged {
        version: 1,
        zone_rgb: selection.info.as_ref().and_then(|info| info.rgb_u32),
    };
    super::super::emit_event(&payload);
}

pub(in crate::bridge::host) fn emit_hover_changed_event(hover: Res<HoverState>) {
    crate::perf_scope!("bridge.emit.hover");
    if !hover.is_changed() {
        return;
    }

    let payload = FishyMapOutputEvent::HoverChanged {
        version: 1,
        world_x: hover.info.as_ref().map(|info| info.world_x),
        world_z: hover.info.as_ref().map(|info| info.world_z),
        zone_rgb: hover.info.as_ref().and_then(|info| info.rgb_u32),
        layer_samples: hover
            .info
            .as_ref()
            .map(|info| hover_layer_samples_snapshot(&info.layer_samples))
            .unwrap_or_default(),
    };
    let Ok(serialized) = serde_json::to_string(&payload) else {
        return;
    };
    LAST_HOVER_PAYLOAD.with(|last_payload| {
        let mut last_payload = last_payload.borrow_mut();
        if last_payload.as_deref() == Some(serialized.as_str()) {
            return;
        }
        crate::perf_counter_add!("bridge.emit.hover.count", 1);
        super::super::emit_event(&payload);
        *last_payload = Some(serialized);
    });
}
