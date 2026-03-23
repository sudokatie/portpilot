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
            let short_cmd = cmd.rsplit('/').next().unwrap_or(cmd);
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
    let port_word = if in_use == 1 { "port" } else { "ports" };
    let summary = format!("  {} {} in use ({} available)", in_use, port_word, available);
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
        let days = duration.num_days();
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else if duration.num_hours() > 0 {
        let hours = duration.num_hours();
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if duration.num_minutes() > 0 {
        let minutes = duration.num_minutes();
        if minutes == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", minutes)
        }
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
    use chrono::Duration;
    
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
    
    #[test]
    fn test_format_time_ago_pluralization() {
        let now = Utc::now();
        
        // 1 hour ago - singular
        let one_hour = now - Duration::hours(1);
        assert_eq!(format_time_ago(one_hour), "1 hour ago");
        
        // 2 hours ago - plural
        let two_hours = now - Duration::hours(2);
        assert_eq!(format_time_ago(two_hours), "2 hours ago");
        
        // 1 day ago - singular
        let one_day = now - Duration::days(1);
        assert_eq!(format_time_ago(one_day), "1 day ago");
        
        // 3 days ago - plural
        let three_days = now - Duration::days(3);
        assert_eq!(format_time_ago(three_days), "3 days ago");
        
        // 1 minute ago - singular
        let one_minute = now - Duration::minutes(1);
        assert_eq!(format_time_ago(one_minute), "1 minute ago");
        
        // 5 minutes ago - plural
        let five_minutes = now - Duration::minutes(5);
        assert_eq!(format_time_ago(five_minutes), "5 minutes ago");
        
        // Just now
        let just_now = now - Duration::seconds(30);
        assert_eq!(format_time_ago(just_now), "just now");
    }
    
    #[test]
    fn test_format_range_summary_pluralization() {
        let entries = vec![
            PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
        ];
        let opts = OutputOptions { no_color: true, ..Default::default() };
        
        // Single port in use
        let output = format_range(&entries, 3000, 3001, &opts);
        assert!(output.contains("1 port in use"), "Should say '1 port' not '1 ports': {}", output);
        
        // Multiple ports in use
        let entries2 = vec![
            PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
            PortEntry::new(3001, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
        ];
        let output2 = format_range(&entries2, 3000, 3002, &opts);
        assert!(output2.contains("2 ports in use"), "Should say '2 ports' not '2 port': {}", output2);
    }
}
