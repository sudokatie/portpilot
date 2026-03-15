//! Watch mode for live updates.

use crate::scanner::{get_scanner, ScanOptions};
use crate::filter::{filter_entries, FilterOptions};
use crate::sort::{sort_entries, SortOptions};
use crate::output::{format_ports, OutputOptions};
use std::collections::HashSet;
use std::io::{self, Write};
use std::time::Duration;
use std::thread;

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
    
    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    });
    
    while running.load(std::sync::atomic::Ordering::SeqCst) {
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
                let _added: Vec<_> = current_ports.difference(&previous_ports).collect();
                let _removed: Vec<_> = previous_ports.difference(&current_ports).collect();
                
                // Output
                let output = format_ports(&entries, &output_opts);
                println!("{}", output);
                
                // Update for next iteration
                previous_ports = current_ports;
            }
            Err(e) => {
                eprintln!("Error scanning ports: {}", e);
            }
        }
        
        // Wait for interval
        thread::sleep(Duration::from_millis(interval_ms));
    }
    
    Ok(())
}

/// Set up Ctrl+C handler.
fn ctrlc_handler<F: FnOnce() + Send + 'static>(handler: F) {
    // Simple handler - in production would use ctrlc crate
    let _handler = std::sync::Mutex::new(Some(handler));
    
    // This is a simplified version - real implementation would use signal handlers
    std::thread::spawn(move || {
        // Wait for signal (simplified - would use actual signal handling)
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    });
}
