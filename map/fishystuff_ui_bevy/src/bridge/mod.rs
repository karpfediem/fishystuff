pub mod contract;
#[cfg(target_arch = "wasm32")]
pub mod host;
pub mod theme;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct BrowserInputStateSet;
