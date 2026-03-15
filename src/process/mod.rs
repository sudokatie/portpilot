//! Process management.

mod info;
mod kill;

pub use info::get_process_info;
pub use kill::{kill_process, wait_for_port_free, KillOptions};
