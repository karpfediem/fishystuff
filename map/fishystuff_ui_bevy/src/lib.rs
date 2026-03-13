pub mod bridge;

#[cfg(target_arch = "wasm32")]
pub mod app;
#[cfg(target_arch = "wasm32")]
pub mod config;
#[cfg(target_arch = "wasm32")]
pub mod map;
#[cfg(target_arch = "wasm32")]
pub mod plugins;
#[cfg(target_arch = "wasm32")]
pub mod prelude;

#[cfg(target_arch = "wasm32")]
pub fn run() {
    app::run();
}
