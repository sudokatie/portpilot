//! Oneline output formatting.

use crate::scanner::PortEntry;

/// Format ports as one line per port.
pub fn format_ports(entries: &[PortEntry]) -> String {
    entries.iter().map(format_port).collect::<Vec<_>>().join("\n")
}

/// Format a single port as one line.
pub fn format_port(entry: &PortEntry) -> String {
    let process = entry.process_name.as_deref().unwrap_or("-");
    let pid = entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
    let cmd = entry.command.as_deref().unwrap_or("-");
    
    // Tab-separated: port, process, pid, command (truncated to 40 chars)
    format!("{}\t{}\t{}\t{}", entry.port, process, pid, truncate(cmd, 40))
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
    fn test_format_port_oneline() {
        let mut entry = PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        entry.process_name = Some("node".to_string());
        entry.pid = Some(1234);
        entry.command = Some("node server.js".to_string());
        
        let line = format_port(&entry);
        
        assert!(line.contains("3000"));
        assert!(line.contains("node"));
        assert!(line.contains("1234"));
        assert!(line.contains("\t"));
    }
    
    #[test]
    fn test_format_ports_oneline() {
        let entries = vec![
            PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
            PortEntry::new(8080, Protocol::Tcp, IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        ];
        
        let output = format_ports(&entries);
        let lines: Vec<_> = output.lines().collect();
        
        assert_eq!(lines.len(), 2);
    }
}
