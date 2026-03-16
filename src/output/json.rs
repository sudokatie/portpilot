//! JSON output formatting.

use crate::scanner::PortEntry;
use super::PortSummary;
use serde::Serialize;

/// JSON output structure.
#[derive(Serialize)]
struct JsonOutput<'a> {
    ports: &'a [PortEntry],
    summary: PortSummary,
}

/// Format ports as JSON.
pub fn format_ports(entries: &[PortEntry]) -> String {
    let output = JsonOutput {
        ports: entries,
        summary: PortSummary::from_entries(entries),
    };
    
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
}

/// Format a single port as JSON.
pub fn format_port(entry: &PortEntry) -> String {
    serde_json::to_string_pretty(entry).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::scanner::Protocol;
    
    #[test]
    fn test_format_ports_json() {
        let mut entry = PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        entry.process_name = Some("node".to_string());
        entry.parent_name = Some("bash".to_string());
        let entries = vec![entry];
        
        let json = format_ports(&entries);
        
        assert!(json.contains("\"port\": 3000"));
        assert!(json.contains("\"protocol\": \"tcp\""));
        assert!(json.contains("\"summary\""));
        // Verify spec field names
        assert!(json.contains("\"process\": \"node\""), "should use 'process' not 'process_name'");
        assert!(json.contains("\"parent_process\": \"bash\""), "should use 'parent_process' not 'parent_name'");
        assert!(json.contains("\"external\": false"), "should include 'external' field");
    }
    
    #[test]
    fn test_format_port_json() {
        let entry = PortEntry::new(8080, Protocol::Tcp, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        
        let json = format_port(&entry);
        
        assert!(json.contains("\"port\": 8080"));
        assert!(json.contains("\"external\": true"), "0.0.0.0 should be external=true");
    }
}
