//! TUI mode for interactive port management.

mod app;
mod ui;
mod events;

pub use app::App;

use crate::scanner::ScanOptions;
use std::io;

/// Run the TUI with default refresh interval.
pub fn run_tui(scan_opts: ScanOptions) -> io::Result<()> {
    run_tui_with_interval(scan_opts, 2000)
}

/// Run the TUI with custom refresh interval.
pub fn run_tui_with_interval(scan_opts: ScanOptions, interval_ms: u64) -> io::Result<()> {
    let mut app = App::new(scan_opts, interval_ms);
    app.run()
}
