//! Watch mode for live updates.

use crate::scanner::{get_scanner, PortEntry, ScanOptions};
use crate::filter::{filter_entries, FilterOptions};
use crate::sort::{sort_entries, SortOptions};
use crate::output::{OutputFormat, OutputOptions, PortSummary};
use colored::Colorize;
use crossterm::event::{self, Event, KeyCode};
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Run watch mode.
pub fn run_watch(
    scan_opts: ScanOptions,
    filter_opts: FilterOptions,
    sort_opts: SortOptions,
    output_opts: OutputOptions,
    interval_ms: u64,
) -> io::Result<()> {
    let scanner = get_scanner();
    let mut previous_ports: HashSet<u16> = HashSet::new();
    let use_colors = !output_opts.no_color && output_opts.format == OutputFormat::Human;
    
    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl+C handler");
    
    // Enable raw mode for keyboard input
    crossterm::terminal::enable_raw_mode()?;
    
    while running.load(Ordering::SeqCst) {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush()?;
        
        // Scan ports
        match scanner.list_ports(&scan_opts) {
            Ok(entries) => {
                let entries = filter_entries(entries, &filter_opts);
                let entries = sort_entries(entries, &sort_opts);
                
                // Track changes
                let current_ports: HashSet<u16> = entries.iter().map(|e| e.port).collect();
                let added: HashSet<_> = current_ports.difference(&previous_ports).copied().collect();
                let removed: HashSet<_> = previous_ports.difference(&current_ports).copied().collect();
                
                // Output with highlighting
                let output = format_watch_output(&entries, &added, &removed, use_colors, &output_opts);
                print!("{}", output);
                io::stdout().flush()?;
                
                // Update for next iteration
                previous_ports = current_ports;
            }
            Err(e) => {
                eprintln!("Error scanning ports: {}", e);
            }
        }
        
        // Poll for 'q' key during interval
        let poll_interval = Duration::from_millis(50);
        let total_wait = Duration::from_millis(interval_ms);
        let mut elapsed = Duration::ZERO;
        
        while elapsed < total_wait && running.load(Ordering::SeqCst) {
            if event::poll(poll_interval)? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') {
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
            elapsed += poll_interval;
        }
    }
    
    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    
    Ok(())
}

/// Format watch output with change highlighting.
fn format_watch_output(
    entries: &[PortEntry],
    added: &HashSet<u16>,
    removed: &HashSet<u16>,
    use_colors: bool,
    opts: &OutputOptions,
) -> String {
    let mut output = String::new();
    
    // Header
    let header = format!(
        "  {:>5}   {:5}  {:16} {:>6}   {:>6}   {:10} {}",
        "PORT", "PROTO", "PROCESS", "PID", "MEM", "STATE", "ADDRESS"
    );
    output.push_str(&if use_colors { header.bold().to_string() } else { header });
    output.push('\n');
    
    // Entries with highlighting
    for entry in entries {
        let is_new = added.contains(&entry.port);
        
        let addr_str = if entry.is_external() {
            "0.0.0.0".to_string()
        } else {
            entry.address.to_string()
        };
        
        let process = entry.process_display();
        let pid = entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
        let mem = entry.memory_display();
        
        let line = format!(
            "  {:>5}   {:5}  {:16} {:>6}   {:>6}   {:10} {}",
            entry.port,
            entry.protocol,
            truncate(process, 16),
            pid,
            mem,
            entry.state,
            addr_str
        );
        
        // Highlight new entries in green
        let styled_line = if use_colors && is_new {
            format!("{}", line.green())
        } else {
            line
        };
        
        output.push_str(&styled_line);
        output.push('\n');
    }
    
    // Show removed ports in red
    if use_colors && !removed.is_empty() {
        for port in removed {
            let line = format!(
                "  {:>5}   {:5}  {:16} {:>6}   {:>6}   {:10} {}",
                port, "-", "(removed)", "-", "-", "-", "-"
            );
            output.push_str(&format!("{}\n", line.red()));
        }
    }
    
    // Summary
    let summary = PortSummary::from_entries(entries);
    output.push('\n');
    
    let mut summary_parts = vec![format!("{} ports listening", summary.total)];
    if !added.is_empty() {
        summary_parts.push(if use_colors {
            format!("+{}", added.len()).green().to_string()
        } else {
            format!("+{}", added.len())
        });
    }
    if !removed.is_empty() {
        summary_parts.push(if use_colors {
            format!("-{}", removed.len()).red().to_string()
        } else {
            format!("-{}", removed.len())
        });
    }
    
    let summary_line = format!(
        "  {} ({} external, {} localhost-only)  [q to quit]",
        summary_parts.join(" "),
        summary.external,
        summary.localhost
    );
    output.push_str(&if use_colors && !opts.no_color {
        summary_line.dimmed().to_string()
    } else {
        summary_line
    });
    output.push('\n');
    
    output
}

/// Truncate a string.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
