//! TUI mode for interactive port management.

mod app;
mod ui;
mod events;

pub use app::App;

use crate::scanner::ScanOptions;
use std::io;

/// Run the TUI.
pub fn run_tui(scan_opts: ScanOptions) -> io::Result<()> {
    let mut app = App::new(scan_opts);
    app.run()
}
