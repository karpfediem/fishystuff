pub(crate) mod actions;
mod debug_perms;
pub(crate) mod handler;
pub(crate) mod index;
mod types;

use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
