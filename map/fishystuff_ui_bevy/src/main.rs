#[cfg(target_arch = "wasm32")]
fn main() {
    fishystuff_ui_bevy::run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("fishystuff_ui_bevy is wasm32-only. Build with --target wasm32-unknown-unknown.");
}
