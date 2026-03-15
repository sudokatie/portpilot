//! Human-readable output formatting.

use crate::scanner::PortEntry;
use super::{OutputOptions, PortSummary};
use colored::Colorize;
use chrono::Utc;

/// Format overview of all ports.
pub fn format_overview(entries: &[PortEntry], opts: &OutputOptions) -> String {
    let mut output = String::new();
    let use_colors = !opts.no_color && should_use_colors();
    
    // Header
    let header = format!(
        "  {:>5}   {:5}  {:16} {:>6}   {:>6}   {:10} {}",
        "PORT", "PROTO", "PROCESS", "PID", "MEM", "STATE", "ADDRESS"
    );
    output.push_str(&if use_colors { header.bold().to_string() } else { header });
    output.push('\n');
    
    // Entries
    for entry in entries {
        let addr_str = if entry.is_external() {
            if use_colors {
                "0.0.0.0".yellow().to_string()
            } else {
                "0.0.0.0".to_string()
            }
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
        
        output.push_str(&line);
        output.push('\n');
    }
    
    // Summary
    let summary = PortSummary::from_entries(entries);
    output.push('\n');
    let summary_line = format!(
        "  {} ports listening ({} external, {} localhost-only)",
        summary.total, summary.external, summary.localhost
    );
    output.push_str(&if use_colors { summary_line.dimmed().to_string() } else { summary_line });
    output.push('\n');
    
    output
}

/// Format detailed view of a single port.
pub fn format_detail(entry: &PortEntry, opts: &OutputOptions) -> String {
    let mut output = String::new();
    let use_colors = !opts.no_color && should_use_colors();
    
    // Header
    let header = format!("  PORT {} is in use", entry.port);
    output.push_str(&if use_colors { header.bold().to_string() } else { header });
    output.push_str("\n\n");
    
    // Process info
    let process = entry.process_name.as_deref().unwrap_or("unknown");
    output.push_str(&format!("  Process:    {}\n", process));
    
    if let Some(pid) = entry.pid {
        output.push_str(&format!("  PID:        {}\n", pid));
    }
    
    if let Some(ref cmd) = entry.command {
        output.push_str(&format!("  Command:    {}\n", cmd));
    }
    
    if let Some(ref started) = entry.started_at {
        let ago = format_time_ago(*started);
        let time_str = started.format("%H:%M:%S").to_string();
        output.push_str(&format!("  Started:    {} ({})\n", ago, time_str));
    }
    
    output.push_str(&format!("  Memory:     {}\n", entry.memory_display()));
    
    if let Some(cpu) = entry.cpu_percent {
        output.push_str(&format!("  CPU:        {:.1}%\n", cpu));
    }
    
    if let Some(ref user) = entry.user {
        output.push_str(&format!("  User:       {}\n", user));
    }
    
    // Address info
    let exposure = if entry.is_external() {
        "externally accessible"
    } else if entry.is_localhost() {
        "localhost only"
    } else {
        ""
    };
    output.push_str(&format!("  Listening:  {}:{} ({})\n", entry.address, entry.port, exposure));
    
    // Parent process
    if let (Some(ppid), Some(ref pname)) = (entry.parent_pid, &entry.parent_name) {
        output.push_str(&format!("\n  Parent:     {} (PID {})\n", pname, ppid));
        if let Some(ref cmd) = entry.command {
            let short_cmd = cmd.split('/').last().unwrap_or(cmd);
            output.push_str(&format!("              └─ {}\n", short_cmd));
        }
    }
    
    output
}

/// Format a port range view.
pub fn format_range(entries: &[PortEntry], start: u16, end: u16, opts: &OutputOptions) -> String {
    let mut output = String::new();
    let use_colors = !opts.no_color && should_use_colors();
    
    // Header
    let header = format!("  {:>5}   {:16} {:>6}   {}", "PORT", "PROCESS", "PID", "COMMAND");
    output.push_str(&if use_colors { header.bold().to_string() } else { header });
    output.push('\n');
    
    let mut in_use = 0;
    let mut available = 0;
    
    for port in start..=end {
        if let Some(entry) = entries.iter().find(|e| e.port == port) {
            in_use += 1;
            let process = entry.process_display();
            let pid = entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
            let cmd = entry.command.as_deref().unwrap_or("-");
            
            let line = format!(
                "  {:>5}   {:16} {:>6}   {}",
                port,
                truncate(process, 16),
                pid,
                truncate(cmd, 40)
            );
            output.push_str(&line);
        } else {
            available += 1;
            let line = format!("  {:>5}   {:16} {:>6}   {}", port, "-", "-", "(available)");
            output.push_str(&if use_colors { line.dimmed().to_string() } else { line });
        }
        output.push('\n');
    }
    
    output.push('\n');
    let summary = format!("  {} port(s) in use ({} available)", in_use, available);
    output.push_str(&if use_colors { summary.dimmed().to_string() } else { summary });
    output.push('\n');
    
    output
}

/// Check if colors should be used.
fn should_use_colors() -> bool {
    // Check NO_COLOR env var
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    
    // Check TERM=dumb
    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return false;
    }
    
    // Check if stdout is a TTY
    colored::control::SHOULD_COLORIZE.should_colorize()
}

/// Format time ago.
fn format_time_ago(time: chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(time);
    
    if duration.num_days() > 0 {
        format!("{} day(s) ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hour(s) ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minute(s) ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

/// Truncate a string.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::scanner::Protocol;
    
    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is...");
    }
    
    #[test]
    fn test_format_overview() {
        let entries = vec![
            PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
        ];
        let opts = OutputOptions { no_color: true, ..Default::default() };
        let output = format_overview(&entries, &opts);
        
        assert!(output.contains("3000"));
        assert!(output.contains("tcp"));
    }
}
