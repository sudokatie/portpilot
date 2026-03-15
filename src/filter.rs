//! Filtering functionality.

use crate::scanner::PortEntry;

/// Filter options.
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    /// Filter by process name (substring match, case-insensitive).
    pub process: Option<String>,
    /// Filter by username.
    pub user: Option<String>,
    /// Only externally accessible ports.
    pub external_only: bool,
    /// Only localhost-bound ports.
    pub localhost_only: bool,
}

impl FilterOptions {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn process(mut self, pattern: impl Into<String>) -> Self {
        self.process = Some(pattern.into());
        self
    }
    
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
    
    pub fn external_only(mut self, only: bool) -> Self {
        self.external_only = only;
        self
    }
    
    pub fn localhost_only(mut self, only: bool) -> Self {
        self.localhost_only = only;
        self
    }
}

/// Filter entries based on options.
pub fn filter_entries(entries: Vec<PortEntry>, opts: &FilterOptions) -> Vec<PortEntry> {
    entries.into_iter()
        .filter(|e| {
            // Process name filter
            if let Some(ref pattern) = opts.process {
                let pattern_lower = pattern.to_lowercase();
                let matches = e.process_name.as_ref()
                    .map(|n| n.to_lowercase().contains(&pattern_lower))
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            
            // User filter
            if let Some(ref user) = opts.user {
                let matches = e.user.as_ref().map(|u| u == user).unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            
            // External only
            if opts.external_only && !e.is_external() {
                return false;
            }
            
            // Localhost only
            if opts.localhost_only && !e.is_localhost() {
                return false;
            }
            
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::scanner::Protocol;
    
    fn make_entry(port: u16, process: &str, external: bool) -> PortEntry {
        let addr = if external {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        } else {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        };
        let mut e = PortEntry::new(port, Protocol::Tcp, addr);
        e.process_name = Some(process.to_string());
        e
    }
    
    #[test]
    fn test_filter_by_process() {
        let entries = vec![
            make_entry(3000, "node", false),
            make_entry(5432, "postgres", false),
            make_entry(8080, "nodejs", false),
        ];
        
        let opts = FilterOptions::new().process("node");
        let filtered = filter_entries(entries, &opts);
        
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.process_name.as_ref().unwrap().contains("node")));
    }
    
    #[test]
    fn test_filter_external_only() {
        let entries = vec![
            make_entry(80, "nginx", true),
            make_entry(3000, "node", false),
        ];
        
        let opts = FilterOptions::new().external_only(true);
        let filtered = filter_entries(entries, &opts);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].port, 80);
    }
    
    #[test]
    fn test_filter_localhost_only() {
        let entries = vec![
            make_entry(80, "nginx", true),
            make_entry(3000, "node", false),
        ];
        
        let opts = FilterOptions::new().localhost_only(true);
        let filtered = filter_entries(entries, &opts);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].port, 3000);
    }
    
    #[test]
    fn test_filter_case_insensitive() {
        let entries = vec![
            make_entry(3000, "Node", false),
            make_entry(8080, "NODE", false),
        ];
        
        let opts = FilterOptions::new().process("node");
        let filtered = filter_entries(entries, &opts);
        
        assert_eq!(filtered.len(), 2);
    }
}
