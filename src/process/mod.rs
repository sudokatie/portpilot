//! Process management.

mod info;
mod kill;

pub use info::get_process_info;
pub use kill::{kill_process, send_sigterm, wait_for_port_free, KillOptions};
