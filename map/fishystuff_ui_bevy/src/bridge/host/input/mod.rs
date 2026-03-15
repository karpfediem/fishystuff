mod commands;
mod queue;
mod state;

use crate::bridge::contract::FishyMapCommands;

pub(super) use commands::apply_browser_commands;
pub(super) use queue::ingest_pending_browser_patches;
pub(super) use state::apply_browser_input_state;

fn commands_is_empty(commands: &FishyMapCommands) -> bool {
    !commands.reset_view.unwrap_or(false)
        && commands.set_view_mode.is_none()
        && commands.select_zone_rgb.is_none()
        && commands.restore_view.is_none()
}
