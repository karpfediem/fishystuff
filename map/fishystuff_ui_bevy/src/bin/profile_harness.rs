#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    fishystuff_ui_bevy::profiling::harness::run(
        fishystuff_ui_bevy::profiling::harness::HarnessCli::parse(),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
