//! portpilot CLI entry point.

use clap::Parser;
use portpilot::{
    cli::{Cli, PortSpec},
    filter::{filter_entries, FilterOptions},
    output::{format_port_detail, format_ports, OutputFormat, OutputOptions},
    process::{kill_process, wait_for_port_free, KillOptions},
    scanner::{get_scanner, Protocol, ScanOptions},
    sort::{sort_entries, SortField, SortOptions},
    tui,
    watch,
};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    
    // Build options from CLI args
    let scan_opts = ScanOptions {
        include_udp: cli.udp,
        include_sockets: cli.sockets,
        filter_external: if cli.external {
            Some(true)
        } else if cli.local {
            Some(false)
        } else {
            None
        },
    };
    
    let filter_opts = FilterOptions {
        process: cli.filter.clone(),
        user: cli.user.clone(),
        external_only: cli.external,
        localhost_only: cli.local,
    };
    
    let sort_field: SortField = cli.sort.parse().unwrap_or_default();
    let sort_opts = SortOptions {
        field: sort_field,
        reverse: cli.reverse,
    };
    
    let output_format = if cli.json {
        OutputFormat::Json
    } else if cli.quiet {
        OutputFormat::Quiet
    } else if cli.oneline {
        OutputFormat::Oneline
    } else {
        OutputFormat::Human
    };
    
    let output_opts = OutputOptions {
        format: output_format,
        no_color: cli.no_color,
        mask_values: false,
    };
    
    // TUI mode
    if cli.tui {
        if let Err(e) = tui::run_tui(scan_opts) {
            eprintln!("TUI error: {}", e);
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    
    // Watch mode
    if cli.watch {
        if let Err(e) = watch::run_watch(scan_opts, filter_opts, sort_opts, output_opts, cli.interval) {
            eprintln!("Watch error: {}", e);
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    
    // Get scanner
    let scanner = get_scanner();
    
    // Handle specific port or range
    if let Some(port_spec) = cli.parse_port() {
        if let Err(msg) = port_spec.validate() {
            eprintln!("Error: {}", msg);
            return ExitCode::FAILURE;
        }
        
        match port_spec {
            PortSpec::Single(port) => {
                return handle_single_port(port, &cli, &output_opts);
            }
            PortSpec::Range(start, end) => {
                return handle_port_range(start, end, &scan_opts, &filter_opts, &sort_opts, &output_opts);
            }
            PortSpec::Invalid(s) => {
                eprintln!("Invalid port: {}", s);
                return ExitCode::FAILURE;
            }
        }
    }
    
    // List all ports
    match scanner.list_ports(&scan_opts) {
        Ok(entries) => {
            let entries = filter_entries(entries, &filter_opts);
            let entries = sort_entries(entries, &sort_opts);
            
            if entries.is_empty() && output_format == OutputFormat::Human {
                if !cli.quiet {
                    println!("No listening ports found.");
                }
                return ExitCode::SUCCESS;
            }
            
            let output = format_ports(&entries, &output_opts);
            if !output.is_empty() {
                println!("{}", output);
            }
            
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error scanning ports: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn handle_single_port(port: u16, cli: &Cli, output_opts: &OutputOptions) -> ExitCode {
    let scanner = get_scanner();
    
    // Check if port is in use
    match scanner.get_port_detail(port, Protocol::Tcp) {
        Ok(Some(entry)) => {
            // Port is in use
            if cli.kill {
                // Kill the process
                if let Some(pid) = entry.pid {
                    let kill_opts = KillOptions::new()
                        .force(cli.force)
                        .timeout(cli.timeout);
                    
                    match kill_process(pid, &kill_opts) {
                        Ok(()) => {
                            if !cli.quiet {
                                println!("Killed process {} (PID {})", 
                                    entry.process_name.as_deref().unwrap_or("unknown"),
                                    pid);
                            }
                            
                            // Wait for port to be free if requested
                            if cli.wait {
                                if let Err(e) = wait_for_port_free(port, cli.timeout) {
                                    eprintln!("Timeout waiting for port {}: {}", port, e);
                                    return ExitCode::FAILURE;
                                }
                            }
                            
                            return ExitCode::SUCCESS;
                        }
                        Err(e) => {
                            eprintln!("Failed to kill process: {}", e);
                            return ExitCode::FAILURE;
                        }
                    }
                } else {
                    eprintln!("Cannot determine process ID for port {}", port);
                    return ExitCode::FAILURE;
                }
            }
            
            // Just show port info
            let output = format_port_detail(&entry, output_opts);
            if !output.is_empty() {
                print!("{}", output);
            }
            
            ExitCode::SUCCESS
        }
        Ok(None) => {
            // Port is not in use
            if cli.wait {
                // Already free
                if !cli.quiet {
                    println!("Port {} is available.", port);
                }
                return ExitCode::SUCCESS;
            }
            
            if cli.quiet {
                // Exit 1 for "not in use" in quiet mode
                return ExitCode::FAILURE;
            }
            
            println!("Port {} is not in use.", port);
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Error checking port {}: {}", port, e);
            ExitCode::FAILURE
        }
    }
}

fn handle_port_range(
    start: u16,
    end: u16,
    scan_opts: &ScanOptions,
    filter_opts: &FilterOptions,
    sort_opts: &SortOptions,
    output_opts: &OutputOptions,
) -> ExitCode {
    let scanner = get_scanner();
    
    match scanner.list_ports(scan_opts) {
        Ok(entries) => {
            // Filter to range
            let entries: Vec<_> = entries
                .into_iter()
                .filter(|e| e.port >= start && e.port <= end)
                .collect();
            
            let entries = filter_entries(entries, filter_opts);
            let entries = sort_entries(entries, sort_opts);
            
            // Show range view
            let output = portpilot::output::human::format_range(&entries, start, end, output_opts);
            print!("{}", output);
            
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error scanning ports: {}", e);
            ExitCode::FAILURE
        }
    }
}
