use crate::prelude::*;

use super::super::{BrowserBridgeState, PENDING_PATCHES};

pub(in crate::bridge::host) fn ingest_pending_browser_patches(
    mut bridge: ResMut<BrowserBridgeState>,
) {
    crate::perf_scope!("bridge.patch_ingest");
    let drained = PENDING_PATCHES.with(|pending| pending.take());
    crate::perf_gauge!("bridge.pending_patches", drained.len());
    crate::perf_counter_add!("bridge.patches.ingested", drained.len());
    for patch in drained {
        let commands = bridge.input.apply_patch(patch);
        if !super::commands_is_empty(&commands) {
            bridge.pending_commands.push(commands);
        }
    }
    crate::perf_gauge!("bridge.pending_commands", bridge.pending_commands.len());
}
