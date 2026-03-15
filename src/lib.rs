//! portpilot - Cross-platform CLI tool for inspecting and managing ports.
//!
//! This crate provides:
//! - Port scanning to discover listening ports
//! - Process information for each port
//! - Filtering and sorting capabilities
//! - Kill functionality for freeing ports
//! - Interactive TUI mode

pub mod scanner;
pub mod process;
pub mod output;
pub mod filter;
pub mod sort;
pub mod watch;
pub mod tui;
pub mod cli;

pub use scanner::{get_scanner, PortEntry, Protocol, ScanOptions, SocketState};
pub use process::{kill_process, KillOptions};
pub use filter::{filter_entries, FilterOptions};
pub use sort::{sort_entries, SortField, SortOptions};
pub use output::{format_ports, format_port_detail, OutputFormat, OutputOptions};
