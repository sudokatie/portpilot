//! Linux-specific port scanning.

use super::{PortEntry, PortScanner, Protocol, ScanError, ScanOptions, SocketState, enrich_with_sysinfo};
use sysinfo::System;
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

/// Linux port scanner using /proc filesystem.
pub struct LinuxScanner {
    system: System,
}

impl LinuxScanner {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }
    
    /// Parse /proc/net/tcp or /proc/net/tcp6.
    fn parse_proc_net(&self, path: &str, is_ipv6: bool) -> Result<Vec<PortEntry>, ScanError> {
        let content = fs::read_to_string(path)?;
        let mut entries = Vec::new();
        
        for line in content.lines().skip(1) {
            if let Some(entry) = self.parse_proc_net_line(line, is_ipv6) {
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }
    
    /// Parse a single line from /proc/net/tcp.
    fn parse_proc_net_line(&self, line: &str, is_ipv6: bool) -> Option<PortEntry> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            return None;
        }
        
        // Parse local address (format: ADDR:PORT in hex)
        let local = parts[1];
        let (addr_hex, port_hex) = local.rsplit_once(':')?;
        
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        let address = parse_hex_addr(addr_hex, is_ipv6)?;
        
        // Parse state
        let state_num = u8::from_str_radix(parts[3], 16).ok()?;
        let state = match state_num {
            0x0A => SocketState::Listen,
            0x01 => SocketState::Established,
            0x06 => SocketState::TimeWait,
            0x08 => SocketState::CloseWait,
            _ => SocketState::Other(format!("{:02X}", state_num)),
        };
        
        // Only include listening sockets by default
        if state != SocketState::Listen {
            return None;
        }
        
        // Get inode
        let inode: u64 = parts[9].parse().ok()?;
        
        let mut entry = PortEntry::new(port, Protocol::Tcp, address);
        entry.state = state;
        
        // Map inode to PID
        if let Some(pid) = self.find_pid_for_inode(inode) {
            entry.pid = Some(pid);
            enrich_with_sysinfo(&mut entry, &self.system);
        }
        
        Some(entry)
    }
    
    /// Parse /proc/net/unix for Unix sockets.
    fn parse_unix_sockets(&self) -> Result<Vec<PortEntry>, ScanError> {
        let content = fs::read_to_string("/proc/net/unix")?;
        let mut entries = Vec::new();
        
        for line in content.lines().skip(1) {
            if let Some(entry) = self.parse_unix_socket_line(line) {
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }
    
    /// Parse a single line from /proc/net/unix.
    fn parse_unix_socket_line(&self, line: &str) -> Option<PortEntry> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 {
            return None;
        }
        
        // Format: Num RefCount Protocol Flags Type St Inode Path
        // Only include listening sockets (State 01 = LISTENING for stream sockets)
        let state_num = u8::from_str_radix(parts[5], 16).ok()?;
        
        // 01 = UNCONNECTED (listening for STREAM), 03 = CONNECTED
        // For DGRAM sockets we want state 07 (listening)
        let socket_type = u8::from_str_radix(parts[4], 16).ok()?;
        
        // Type 1 = STREAM, Type 2 = DGRAM
        let is_listening = match socket_type {
            1 => state_num == 1,  // STREAM: 01 = unconnected/listening
            2 => true,            // DGRAM: always "listening" 
            _ => false,
        };
        
        if !is_listening {
            return None;
        }
        
        let inode: u64 = parts[6].parse().ok()?;
        
        // Path is optional (abstract sockets may not have one)
        let path = if parts.len() > 7 {
            parts[7..].join(" ")
        } else {
            String::new()
        };
        
        // Skip sockets without a path (anonymous)
        if path.is_empty() {
            return None;
        }
        
        // Use port 0 for Unix sockets, store path in command field
        let mut entry = PortEntry::new(0, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        entry.state = SocketState::Listen;
        entry.command = Some(path);
        
        // Map inode to PID
        if let Some(pid) = self.find_pid_for_inode(inode) {
            entry.pid = Some(pid);
            enrich_with_sysinfo(&mut entry, &self.system);
        }
        
        Some(entry)
    }
    
    /// Find PID that owns an inode by scanning /proc/[pid]/fd/.
    fn find_pid_for_inode(&self, target_inode: u64) -> Option<u32> {
        let proc = Path::new("/proc");
        
        for entry in fs::read_dir(proc).ok()? {
            let entry = entry.ok()?;
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            
            let fd_dir = proc.join(format!("{}/fd", pid));
            if let Ok(fds) = fs::read_dir(&fd_dir) {
                for fd in fds.filter_map(|f| f.ok()) {
                    if let Ok(link) = fs::read_link(fd.path()) {
                        let link_str = link.to_string_lossy();
                        if link_str.starts_with("socket:[") {
                            if let Some(inode_str) = link_str.strip_prefix("socket:[").and_then(|s| s.strip_suffix(']')) {
                                if let Ok(inode) = inode_str.parse::<u64>() {
                                    if inode == target_inode {
                                        return Some(pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
}

impl PortScanner for LinuxScanner {
    fn list_ports(&self, opts: &ScanOptions) -> Result<Vec<PortEntry>, ScanError> {
        let mut entries = Vec::new();
        
        // Parse TCP
        if let Ok(tcp) = self.parse_proc_net("/proc/net/tcp", false) {
            entries.extend(tcp);
        }
        if let Ok(tcp6) = self.parse_proc_net("/proc/net/tcp6", true) {
            entries.extend(tcp6);
        }
        
        // Parse UDP if requested
        if opts.include_udp {
            if let Ok(mut udp) = self.parse_proc_net("/proc/net/udp", false) {
                for e in &mut udp {
                    e.protocol = Protocol::Udp;
                }
                entries.extend(udp);
            }
            if let Ok(mut udp6) = self.parse_proc_net("/proc/net/udp6", true) {
                for e in &mut udp6 {
                    e.protocol = Protocol::Udp;
                }
                entries.extend(udp6);
            }
        }
        
        // Parse Unix sockets if requested
        if opts.include_sockets {
            if let Ok(unix_sockets) = self.parse_unix_sockets() {
                entries.extend(unix_sockets);
            }
        }
        
        // Apply filters
        if let Some(external) = opts.filter_external {
            entries.retain(|e| {
                if external {
                    e.is_external()
                } else {
                    e.is_localhost()
                }
            });
        }
        
        // Sort by port
        entries.sort_by_key(|e| e.port);
        
        Ok(entries)
    }
    
    fn get_port_detail(&self, port: u16, protocol: Protocol) -> Result<Option<PortEntry>, ScanError> {
        let opts = ScanOptions {
            include_udp: protocol == Protocol::Udp,
            ..Default::default()
        };
        let ports = self.list_ports(&opts)?;
        Ok(ports.into_iter().find(|p| p.port == port && p.protocol == protocol))
    }
}

impl Default for LinuxScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse hex-encoded IP address.
fn parse_hex_addr(hex: &str, is_ipv6: bool) -> Option<IpAddr> {
    if is_ipv6 {
        // IPv6 is 32 hex chars
        if hex.len() != 32 {
            return None;
        }
        let mut octets = [0u8; 16];
        for i in 0..16 {
            octets[i] = u8::from_str_radix(&hex[i*2..i*2+2], 16).ok()?;
        }
        // Linux stores in network byte order, need to reverse each 4-byte group
        let mut addr = [0u8; 16];
        for i in 0..4 {
            addr[i*4] = octets[i*4 + 3];
            addr[i*4 + 1] = octets[i*4 + 2];
            addr[i*4 + 2] = octets[i*4 + 1];
            addr[i*4 + 3] = octets[i*4];
        }
        Some(IpAddr::V6(Ipv6Addr::from(addr)))
    } else {
        // IPv4 is 8 hex chars in reversed byte order
        if hex.len() != 8 {
            return None;
        }
        let bytes = u32::from_str_radix(hex, 16).ok()?;
        Some(IpAddr::V4(Ipv4Addr::from(bytes.swap_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_hex_addr_ipv4() {
        // 0100007F = 127.0.0.1 (reversed)
        let addr = parse_hex_addr("0100007F", false);
        assert_eq!(addr, Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        
        // 00000000 = 0.0.0.0
        let addr = parse_hex_addr("00000000", false);
        assert_eq!(addr, Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    }
}
