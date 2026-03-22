pub mod app;
pub mod bridge;
pub mod config;
pub mod map;
pub mod plugins;
pub mod prelude;
pub mod profiling;
pub mod public_assets;
pub mod runtime_io;

#[cfg(target_arch = "wasm32")]
pub fn run() {
    app::run_browser();
}
