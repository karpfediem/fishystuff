use bevy::prelude::*;

use crate::profiling;

pub struct BrowserProfilingPlugin;

impl Plugin for BrowserProfilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, begin_browser_frame)
            .add_systems(Last, end_browser_frame);
    }
}

fn begin_browser_frame() {
    profiling::begin_frame(profiling::current_frame());
}

fn end_browser_frame() {
    let frame = profiling::current_frame();
    profiling::end_frame(frame);
    profiling::set_current_frame(frame.saturating_add(1));
}
