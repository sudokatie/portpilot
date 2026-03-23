//! Port scanning functionality.

pub mod types;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

pub use types::{PortEntry, Protocol, ScanOptions, SocketState, format_bytes};

use thiserror::Error;

/// Scanner errors.
#[derive(Debug, Error)]
pub enum ScanError {
    #[error("Failed to scan ports: {0}")]
    ScanFailed(String),
    
    #[error("Permission denied")]
    PermissionDenied,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Port scanner trait.
pub trait PortScanner {
    /// List all listening ports.
    fn list_ports(&self, opts: &ScanOptions) -> Result<Vec<PortEntry>, ScanError>;
    
    /// Get detailed info for a specific port.
    fn get_port_detail(&self, port: u16, protocol: Protocol) -> Result<Option<PortEntry>, ScanError>;
}

/// Get the platform-specific scanner.
pub fn get_scanner() -> Box<dyn PortScanner> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsScanner::new())
    }
    
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxScanner::new())
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Box::new(FallbackScanner::new())
    }
}

/// Fallback scanner using sysinfo crate (cross-platform but less detailed).
#[derive(Default)]
pub struct FallbackScanner {
    #[allow(dead_code)]
    system: sysinfo::System,
}

impl FallbackScanner {
    pub fn new() -> Self {
        Self {
            system: sysinfo::System::new_all(),
        }
    }
}

impl PortScanner for FallbackScanner {
    fn list_ports(&self, opts: &ScanOptions) -> Result<Vec<PortEntry>, ScanError> {
        // Use netstat-like approach via command
        list_ports_via_command(opts)
    }
    
    fn get_port_detail(&self, port: u16, protocol: Protocol) -> Result<Option<PortEntry>, ScanError> {
        let opts = ScanOptions::default();
        let ports = self.list_ports(&opts)?;
        Ok(ports.into_iter().find(|p| p.port == port && p.protocol == protocol))
    }
}

/// List ports using lsof or ss command.
fn list_ports_via_command(opts: &ScanOptions) -> Result<Vec<PortEntry>, ScanError> {
    use std::process::Command;
    
    let mut entries = Vec::new();
    
    // Try lsof first (works on macOS and Linux)
    let output = Command::new("lsof")
        .args(["-i", "-n", "-P", "-sTCP:LISTEN"])
        .output();
    
    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                if let Some(entry) = parse_lsof_line(line) {
                    // Apply filters
                    if let Some(external) = opts.filter_external {
                        if external && !entry.is_external() {
                            continue;
                        }
                        if !external && !entry.is_localhost() {
                            continue;
                        }
                    }
                    entries.push(entry);
                }
            }
        }
    }
    
    // Include UDP if requested
    if opts.include_udp {
        let output = Command::new("lsof")
            .args(["-i", "UDP", "-n", "-P"])
            .output();
        
        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines().skip(1) {
                    if let Some(mut entry) = parse_lsof_line(line) {
                        entry.protocol = Protocol::Udp;
                        if let Some(external) = opts.filter_external {
                            if external && !entry.is_external() {
                                continue;
                            }
                            if !external && !entry.is_localhost() {
                                continue;
                            }
                        }
                        entries.push(entry);
                    }
                }
            }
        }
    }
    
    // Sort by port
    entries.sort_by_key(|e| e.port);
    
    // Deduplicate
    entries.dedup_by(|a, b| a.port == b.port && a.protocol == b.protocol);
    
    Ok(entries)
}

/// Parse a line from lsof output.
fn parse_lsof_line(line: &str) -> Option<PortEntry> {
    use std::net::{IpAddr, Ipv4Addr};
    
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }
    
    let process_name = parts[0].to_string();
    let pid: u32 = parts[1].parse().ok()?;
    let user = parts[2].to_string();
    
    // Find the NAME column (usually last or second to last)
    // Format: *:PORT or IP:PORT
    let name_part = parts.iter().rev().find(|p| p.contains(':'))?;
    
    // Parse address:port
    let (addr_str, port_str) = if name_part.starts_with('[') {
        // IPv6: [::1]:port
        let bracket_end = name_part.find(']')?;
        let addr = &name_part[1..bracket_end];
        let port = &name_part[bracket_end + 2..];
        (addr, port)
    } else {
        // IPv4 or *
        let colon = name_part.rfind(':')?;
        (&name_part[..colon], &name_part[colon + 1..])
    };
    
    let port: u16 = port_str.parse().ok()?;
    
    let address: IpAddr = if addr_str == "*" {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        addr_str.parse().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
    };
    
    let mut entry = PortEntry::new(port, Protocol::Tcp, address);
    entry.pid = Some(pid);
    entry.process_name = Some(process_name);
    entry.user = Some(user);
    
    Some(entry)
}

/// Enrich port entry with process info from sysinfo.
pub fn enrich_with_sysinfo(entry: &mut PortEntry, system: &sysinfo::System) {
    use chrono::DateTime;
    use sysinfo::Pid;
    
    if let Some(pid) = entry.pid {
        let pid_u32 = pid;
        let pid = Pid::from_u32(pid_u32);
        if let Some(process) = system.process(pid) {
            let cmd_parts: Vec<String> = process.cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            entry.command = Some(cmd_parts.join(" "));
            entry.memory_bytes = Some(process.memory());
            entry.cpu_percent = Some(process.cpu_usage());
            
            // Set started_at from process start time
            entry.started_at = DateTime::from_timestamp(process.start_time() as i64, 0);
            
            if let Some(parent_pid) = process.parent() {
                entry.parent_pid = Some(parent_pid.as_u32());
                if let Some(parent) = system.process(parent_pid) {
                    entry.parent_name = Some(parent.name().to_string_lossy().to_string());
                }
            }
        } else {
            // Process exists (we have PID) but can't access info - likely permission denied
            if entry.process_name.is_none() {
                entry.access_denied = true;
            }
        }
        
        // Detect container
        #[cfg(target_os = "linux")]
        {
            entry.container = detect_container_linux(pid_u32);
        }
        
        #[cfg(target_os = "macos")]
        {
            entry.container = detect_container_macos(pid_u32);
        }
    }
}

/// Detect if a process is running in a Docker container (Linux).
#[cfg(target_os = "linux")]
fn detect_container_linux(pid: u32) -> Option<String> {
    use std::fs;
    
    // Read cgroup info
    let cgroup_path = format!("/proc/{}/cgroup", pid);
    let content = fs::read_to_string(&cgroup_path).ok()?;
    
    for line in content.lines() {
        // Format: hierarchy-ID:controller-list:cgroup-path
        // Docker containers have paths like /docker/<container_id>
        if let Some(path) = line.split(':').nth(2) {
            if path.contains("/docker/") {
                // Extract container ID (first 12 chars)
                if let Some(id_start) = path.find("/docker/") {
                    let id = &path[id_start + 8..];
                    let short_id = if id.len() > 12 { &id[..12] } else { id };
                    
                    // Try to get container name from Docker
                    if let Some(name) = get_docker_container_name(short_id) {
                        return Some(name);
                    }
                    return Some(short_id.to_string());
                }
            }
            // Also check for containerd/kubernetes
            if path.contains("/kubepods/") || path.contains("/containerd/") {
                if let Some(id_pos) = path.rfind('/') {
                    let id = &path[id_pos + 1..];
                    let short_id = if id.len() > 12 { &id[..12] } else { id };
                    return Some(short_id.to_string());
                }
            }
        }
    }
    
    None
}

/// Try to get Docker container name via docker CLI.
#[cfg(target_os = "linux")]
fn get_docker_container_name(container_id: &str) -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.Name}}", container_id])
        .output()
        .ok()?;
    
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout);
        let name = name.trim().trim_start_matches('/');
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    
    None
}

/// Detect if a process is running in a Docker container (macOS).
/// On macOS, Docker runs in a VM, so detection is different.
#[cfg(target_os = "macos")]
fn detect_container_macos(pid: u32) -> Option<String> {
    use std::process::Command;
    
    // On macOS, Docker Desktop uses com.docker.backend to proxy ports.
    // We can check if this port is mapped to a container via docker ps.
    
    // First, try to find containers with port mappings
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.ID}}\t{{.Names}}\t{{.Ports}}"])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    // Get the port this process is listening on
    // We need to correlate via the port, not the PID on macOS
    // For now, return None - full implementation would require
    // correlating the port to container port mappings
    
    // Check if the process is com.docker.backend (Docker's port proxy)
    let ps_output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    
    if ps_output.status.success() {
        let comm = String::from_utf8_lossy(&ps_output.stdout);
        let comm = comm.trim();
        if comm.contains("docker") || comm.contains("com.docker") {
            // This is Docker's port proxy, but we'd need to map to container
            // For now, indicate it's Docker-related
            return Some("docker".to_string());
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_lsof_line() {
        let line = "node      12345 user   21u  IPv4 0x12345      0t0  TCP *:3000 (LISTEN)";
        let entry = parse_lsof_line(line);
        
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.port, 3000);
        assert_eq!(entry.process_name, Some("node".to_string()));
        assert_eq!(entry.pid, Some(12345));
    }
    
    #[test]
    fn test_parse_lsof_line_localhost() {
        let line = "postgres  1234 user   10u  IPv4 0xabcd      0t0  TCP 127.0.0.1:5432 (LISTEN)";
        let entry = parse_lsof_line(line);
        
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.port, 5432);
        assert!(entry.is_localhost());
    }
}
