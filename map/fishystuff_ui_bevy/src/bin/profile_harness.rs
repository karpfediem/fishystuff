use clap::Parser;

fn main() -> anyhow::Result<()> {
    fishystuff_ui_bevy::profiling::harness::run(
        fishystuff_ui_bevy::profiling::harness::HarnessCli::parse(),
    )
}
