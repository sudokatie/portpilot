//! macOS-specific port scanning.

use super::{PortEntry, PortScanner, Protocol, ScanError, ScanOptions, SocketState, enrich_with_sysinfo};
use sysinfo::System;
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;

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
    
    /// Parse Unix sockets via lsof.
    fn list_unix_sockets(&self) -> Result<Vec<PortEntry>, ScanError> {
        let output = Command::new("lsof")
            .args(["-U", "-n", "-P"])
            .output()?;
        
        if !output.status.success() {
            return Ok(Vec::new());
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        
        for line in stdout.lines().skip(1) {
            if let Some(entry) = self.parse_unix_socket_line(line) {
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }
    
    fn parse_unix_socket_line(&self, line: &str) -> Option<PortEntry> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return None;
        }
        
        let process_name = parts[0].to_string();
        let pid: u32 = parts[1].parse().ok()?;
        let user = parts[2].to_string();
        
        // Look for the socket path in the NAME column (usually last)
        let name_idx = parts.iter().position(|p| p.starts_with('/'))?;
        let path = parts[name_idx..].join(" ");
        
        // Use port 0 for Unix sockets
        let mut entry = PortEntry::new(0, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        entry.pid = Some(pid);
        entry.process_name = Some(process_name);
        entry.user = Some(user);
        entry.command = Some(path);
        entry.state = SocketState::Listen;
        
        Some(entry)
    }
}

impl PortScanner for MacOsScanner {
    fn list_ports(&self, opts: &ScanOptions) -> Result<Vec<PortEntry>, ScanError> {
        // Use lsof on macOS for TCP/UDP
        let mut entries = super::list_ports_via_command(opts)?;
        
        // Add Unix sockets if requested
        if opts.include_sockets {
            if let Ok(unix_sockets) = self.list_unix_sockets() {
                entries.extend(unix_sockets);
            }
        }
        
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
