use crate::prelude::*;

use super::super::{BrowserBridgeState, PENDING_PATCHES};

pub(in crate::bridge::host) fn ingest_pending_browser_patches(
    mut bridge: ResMut<BrowserBridgeState>,
) {
    let drained = PENDING_PATCHES.with(|pending| pending.take());
    for patch in drained {
        let commands = bridge.input.apply_patch(patch);
        if !super::commands_is_empty(&commands) {
            bridge.pending_commands.push(commands);
        }
    }
}
