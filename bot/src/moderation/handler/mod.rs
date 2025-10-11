pub(crate) mod burst_guard;
mod trap;

pub use burst_guard::burst_event_handler;
pub use trap::trap_event_handler;