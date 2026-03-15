//! Output formatting.

pub mod human;
pub mod json;
pub mod oneline;

use crate::scanner::PortEntry;

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Oneline,
    Quiet,
}

/// Output options.
#[derive(Debug, Clone, Default)]
pub struct OutputOptions {
    pub format: OutputFormat,
    pub no_color: bool,
    pub mask_values: bool,
}

/// Format port entries.
pub fn format_ports(entries: &[PortEntry], opts: &OutputOptions) -> String {
    match opts.format {
        OutputFormat::Human => human::format_overview(entries, opts),
        OutputFormat::Json => json::format_ports(entries),
        OutputFormat::Oneline => oneline::format_ports(entries),
        OutputFormat::Quiet => String::new(),
    }
}

/// Format a single port detail.
pub fn format_port_detail(entry: &PortEntry, opts: &OutputOptions) -> String {
    match opts.format {
        OutputFormat::Human => human::format_detail(entry, opts),
        OutputFormat::Json => json::format_port(entry),
        OutputFormat::Oneline => oneline::format_port(entry),
        OutputFormat::Quiet => String::new(),
    }
}

/// Summary of ports.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PortSummary {
    pub total: usize,
    pub external: usize,
    pub localhost: usize,
}

impl PortSummary {
    pub fn from_entries(entries: &[PortEntry]) -> Self {
        Self {
            total: entries.len(),
            external: entries.iter().filter(|e| e.is_external()).count(),
            localhost: entries.iter().filter(|e| e.is_localhost()).count(),
        }
    }
}
