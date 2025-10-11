mod debug_perms;
pub(crate) mod index;
pub(crate) mod handler;
mod types;
pub(crate) mod actions;

use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
