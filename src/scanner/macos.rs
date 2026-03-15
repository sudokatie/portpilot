//! macOS-specific port scanning.

use super::{PortEntry, PortScanner, Protocol, ScanError, ScanOptions, enrich_with_sysinfo};
use sysinfo::System;

/// macOS port scanner.
pub struct MacOsScanner {
    system: System,
}

impl MacOsScanner {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }
}

impl PortScanner for MacOsScanner {
    fn list_ports(&self, opts: &ScanOptions) -> Result<Vec<PortEntry>, ScanError> {
        // Use lsof on macOS
        let mut entries = super::list_ports_via_command(opts)?;
        
        // Enrich with sysinfo
        for entry in &mut entries {
            enrich_with_sysinfo(entry, &self.system);
        }
        
        Ok(entries)
    }
    
    fn get_port_detail(&self, port: u16, protocol: Protocol) -> Result<Option<PortEntry>, ScanError> {
        let opts = ScanOptions::default();
        let ports = self.list_ports(&opts)?;
        Ok(ports.into_iter().find(|p| p.port == port && p.protocol == protocol))
    }
}

impl Default for MacOsScanner {
    fn default() -> Self {
        Self::new()
    }
}
