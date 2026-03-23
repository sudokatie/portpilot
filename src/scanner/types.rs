//! Port information types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Protocol type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
        }
    }
}

/// Socket state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SocketState {
    #[default]
    Listen,
    Established,
    TimeWait,
    CloseWait,
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for SocketState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketState::Listen => write!(f, "LISTEN"),
            SocketState::Established => write!(f, "ESTABLISHED"),
            SocketState::TimeWait => write!(f, "TIME_WAIT"),
            SocketState::CloseWait => write!(f, "CLOSE_WAIT"),
            SocketState::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Information about a port and its associated process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortEntry {
    /// Port number.
    pub port: u16,
    /// Protocol (TCP or UDP).
    pub protocol: Protocol,
    /// Bind address.
    pub address: IpAddr,
    /// Process ID.
    pub pid: Option<u32>,
    /// Process name.
    #[serde(rename = "process", skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    /// Full command line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Username running process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// RSS memory usage in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// CPU usage percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    /// Process start time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Parent process ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_pid: Option<u32>,
    /// Parent process name.
    #[serde(rename = "parent_process", skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
    /// Socket state.
    pub state: SocketState,
    /// Container name if in Docker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    /// Whether port is externally accessible (computed field for JSON).
    #[serde(skip_deserializing)]
    pub external: bool,
    /// Whether process info was denied due to permissions.
    #[serde(skip)]
    pub access_denied: bool,
}

impl PortEntry {
    /// Create a new port entry.
    pub fn new(port: u16, protocol: Protocol, address: IpAddr) -> Self {
        let external = match address {
            IpAddr::V4(addr) => addr.is_unspecified(),
            IpAddr::V6(addr) => addr.is_unspecified(),
        };
        Self {
            port,
            protocol,
            address,
            pid: None,
            process_name: None,
            command: None,
            user: None,
            memory_bytes: None,
            cpu_percent: None,
            started_at: None,
            parent_pid: None,
            parent_name: None,
            state: SocketState::Listen,
            container: None,
            external,
            access_denied: false,
        }
    }
    
    /// Check if this port is externally accessible.
    pub fn is_external(&self) -> bool {
        match self.address {
            IpAddr::V4(addr) => addr.is_unspecified(),
            IpAddr::V6(addr) => addr.is_unspecified(),
        }
    }
    
    /// Check if this port is localhost only.
    pub fn is_localhost(&self) -> bool {
        match self.address {
            IpAddr::V4(addr) => addr.is_loopback(),
            IpAddr::V6(addr) => addr.is_loopback(),
        }
    }
    
    /// Get formatted memory string.
    pub fn memory_display(&self) -> String {
        match self.memory_bytes {
            Some(bytes) => format_bytes(bytes),
            None => "-".to_string(),
        }
    }
    
    /// Get process name or placeholder.
    /// Returns "(access denied)" if process info unavailable due to permissions.
    pub fn process_display(&self) -> &str {
        if self.access_denied {
            "(access denied)"
        } else {
            self.process_name.as_deref().unwrap_or("-")
        }
    }
}

/// Format bytes to human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{}MB", bytes / MB)
    } else if bytes >= KB {
        format!("{}KB", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

/// Scan options.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Include UDP ports.
    pub include_udp: bool,
    /// Include Unix sockets.
    pub include_sockets: bool,
    /// Filter by exposure: Some(true) = external only, Some(false) = local only.
    pub filter_external: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    
    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Tcp.to_string(), "tcp");
        assert_eq!(Protocol::Udp.to_string(), "udp");
    }
    
    #[test]
    fn test_socket_state_display() {
        assert_eq!(SocketState::Listen.to_string(), "LISTEN");
        assert_eq!(SocketState::Other("CUSTOM".into()).to_string(), "CUSTOM");
    }
    
    #[test]
    fn test_port_entry_external() {
        let entry = PortEntry::new(80, Protocol::Tcp, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert!(entry.is_external());
        assert!(!entry.is_localhost());
    }
    
    #[test]
    fn test_port_entry_localhost() {
        let entry = PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert!(!entry.is_external());
        assert!(entry.is_localhost());
    }
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(2048), "2KB");
        assert_eq!(format_bytes(10 * 1024 * 1024), "10MB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.0GB");
    }
    
    #[test]
    fn test_process_display_normal() {
        let mut entry = PortEntry::new(80, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        entry.process_name = Some("nginx".to_string());
        assert_eq!(entry.process_display(), "nginx");
    }
    
    #[test]
    fn test_process_display_no_name() {
        let entry = PortEntry::new(80, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(entry.process_display(), "-");
    }
    
    #[test]
    fn test_process_display_access_denied() {
        let mut entry = PortEntry::new(80, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        entry.pid = Some(1234);
        entry.access_denied = true;
        assert_eq!(entry.process_display(), "(access denied)");
    }
}
