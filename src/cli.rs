//! CLI argument parsing.

use clap::Parser;

/// portpilot - Inspect and manage ports.
#[derive(Parser, Debug)]
#[command(name = "portpilot")]
#[command(version, about = "Cross-platform CLI tool for inspecting and managing ports")]
pub struct Cli {
    /// Port number or range to query (e.g., 3000 or 3000-3010).
    #[arg(value_name = "PORT")]
    pub port: Option<String>,
    
    /// Output as JSON.
    #[arg(short = 'j', long)]
    pub json: bool,
    
    /// Exit code only, no output.
    #[arg(short = 'q', long)]
    pub quiet: bool,
    
    /// Minimal single-line output per port.
    #[arg(short = '1', long)]
    pub oneline: bool,
    
    /// Disable colored output.
    #[arg(long)]
    pub no_color: bool,
    
    /// Include UDP ports (default TCP only).
    #[arg(short = 'u', long)]
    pub udp: bool,
    
    /// Include Unix sockets.
    #[arg(short = 's', long)]
    pub sockets: bool,
    
    /// Filter by process name (substring match).
    #[arg(short = 'f', long)]
    pub filter: Option<String>,
    
    /// Filter by username.
    #[arg(long)]
    pub user: Option<String>,
    
    /// Only externally-accessible ports (0.0.0.0).
    #[arg(short = 'e', long)]
    pub external: bool,
    
    /// Only localhost-bound ports.
    #[arg(short = 'l', long)]
    pub local: bool,
    
    /// Sort by: port, process, memory, cpu, time.
    #[arg(long, default_value = "port")]
    pub sort: String,
    
    /// Reverse sort order.
    #[arg(short = 'r', long)]
    pub reverse: bool,
    
    /// Live-updating display.
    #[arg(short = 'w', long)]
    pub watch: bool,
    
    /// Watch refresh interval in milliseconds.
    #[arg(long, default_value = "1000")]
    pub interval: u64,
    
    /// Kill the process using the port.
    #[arg(short = 'k', long)]
    pub kill: bool,
    
    /// Use SIGKILL immediately (with --kill).
    #[arg(long)]
    pub force: bool,
    
    /// Block until port is free.
    #[arg(long)]
    pub wait: bool,
    
    /// Timeout for --wait in seconds.
    #[arg(long, default_value = "30")]
    pub timeout: u64,
    
    /// Launch interactive TUI.
    #[arg(long)]
    pub tui: bool,
}

impl Cli {
    /// Parse port argument into single port or range.
    pub fn parse_port(&self) -> Option<PortSpec> {
        self.port.as_ref().map(|s| {
            if s.contains('-') {
                let parts: Vec<&str> = s.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end)) = (parts[0].parse(), parts[1].parse()) {
                        return PortSpec::Range(start, end);
                    }
                }
                PortSpec::Invalid(s.clone())
            } else if let Ok(port) = s.parse() {
                PortSpec::Single(port)
            } else {
                PortSpec::Invalid(s.clone())
            }
        })
    }
}

/// Port specification.
#[derive(Debug, Clone)]
pub enum PortSpec {
    Single(u16),
    Range(u16, u16),
    Invalid(String),
}

impl PortSpec {
    /// Validate the port spec.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            PortSpec::Single(p) if *p == 0 => Err("Port must be between 1 and 65535".to_string()),
            PortSpec::Range(start, end) if start > end => {
                Err(format!("Invalid range: {} > {}", start, end))
            }
            PortSpec::Range(start, _) if *start == 0 => {
                Err("Port must be between 1 and 65535".to_string())
            }
            PortSpec::Invalid(s) => Err(format!("Invalid port: {}", s)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_single_port() {
        let cli = Cli::parse_from(["portpilot", "3000"]);
        match cli.parse_port() {
            Some(PortSpec::Single(3000)) => {}
            _ => panic!("Expected Single(3000)"),
        }
    }
    
    #[test]
    fn test_parse_port_range() {
        let cli = Cli::parse_from(["portpilot", "3000-3010"]);
        match cli.parse_port() {
            Some(PortSpec::Range(3000, 3010)) => {}
            _ => panic!("Expected Range(3000, 3010)"),
        }
    }
    
    #[test]
    fn test_parse_invalid_port() {
        let cli = Cli::parse_from(["portpilot", "abc"]);
        match cli.parse_port() {
            Some(PortSpec::Invalid(_)) => {}
            _ => panic!("Expected Invalid"),
        }
    }
    
    #[test]
    fn test_validate_range() {
        assert!(PortSpec::Range(3010, 3000).validate().is_err());
        assert!(PortSpec::Range(3000, 3010).validate().is_ok());
    }
    
    #[test]
    fn test_flags() {
        let cli = Cli::parse_from(["portpilot", "--json", "--udp", "-e"]);
        assert!(cli.json);
        assert!(cli.udp);
        assert!(cli.external);
    }
}
