//! Sorting functionality.

use crate::scanner::PortEntry;

/// Sort field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    Port,
    Process,
    Memory,
    Cpu,
    Time,
}

impl std::str::FromStr for SortField {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "port" => Ok(SortField::Port),
            "process" | "name" => Ok(SortField::Process),
            "memory" | "mem" => Ok(SortField::Memory),
            "cpu" => Ok(SortField::Cpu),
            "time" | "started" => Ok(SortField::Time),
            _ => Err(format!("Unknown sort field: {}. Use: port, process, memory, cpu, time", s)),
        }
    }
}

/// Sort options.
#[derive(Debug, Clone, Default)]
pub struct SortOptions {
    pub field: SortField,
    pub reverse: bool,
}

impl SortOptions {
    pub fn new(field: SortField) -> Self {
        Self { field, reverse: false }
    }
    
    pub fn reverse(mut self, rev: bool) -> Self {
        self.reverse = rev;
        self
    }
}

/// Sort entries.
pub fn sort_entries(mut entries: Vec<PortEntry>, opts: &SortOptions) -> Vec<PortEntry> {
    entries.sort_by(|a, b| {
        let cmp = match opts.field {
            SortField::Port => a.port.cmp(&b.port),
            SortField::Process => {
                let a_name = a.process_name.as_deref().unwrap_or("");
                let b_name = b.process_name.as_deref().unwrap_or("");
                a_name.to_lowercase().cmp(&b_name.to_lowercase())
            }
            SortField::Memory => {
                let a_mem = a.memory_bytes.unwrap_or(0);
                let b_mem = b.memory_bytes.unwrap_or(0);
                a_mem.cmp(&b_mem)
            }
            SortField::Cpu => {
                let a_cpu = a.cpu_percent.unwrap_or(0.0);
                let b_cpu = b.cpu_percent.unwrap_or(0.0);
                a_cpu.partial_cmp(&b_cpu).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortField::Time => {
                let a_time = a.started_at.map(|t| t.timestamp()).unwrap_or(0);
                let b_time = b.started_at.map(|t| t.timestamp()).unwrap_or(0);
                a_time.cmp(&b_time)
            }
        };
        
        if opts.reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });
    
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::scanner::Protocol;
    
    fn make_entry(port: u16, process: &str, memory: u64) -> PortEntry {
        let mut e = PortEntry::new(port, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
        e.process_name = Some(process.to_string());
        e.memory_bytes = Some(memory);
        e
    }
    
    #[test]
    fn test_sort_by_port() {
        let entries = vec![
            make_entry(8080, "java", 100),
            make_entry(3000, "node", 200),
            make_entry(80, "nginx", 50),
        ];
        
        let sorted = sort_entries(entries, &SortOptions::new(SortField::Port));
        
        assert_eq!(sorted[0].port, 80);
        assert_eq!(sorted[1].port, 3000);
        assert_eq!(sorted[2].port, 8080);
    }
    
    #[test]
    fn test_sort_by_memory() {
        let entries = vec![
            make_entry(3000, "node", 100),
            make_entry(8080, "java", 500),
            make_entry(80, "nginx", 50),
        ];
        
        let sorted = sort_entries(entries, &SortOptions::new(SortField::Memory));
        
        assert_eq!(sorted[0].memory_bytes, Some(50));
        assert_eq!(sorted[2].memory_bytes, Some(500));
    }
    
    #[test]
    fn test_sort_reverse() {
        let entries = vec![
            make_entry(80, "nginx", 50),
            make_entry(3000, "node", 100),
            make_entry(8080, "java", 500),
        ];
        
        let sorted = sort_entries(entries, &SortOptions::new(SortField::Port).reverse(true));
        
        assert_eq!(sorted[0].port, 8080);
        assert_eq!(sorted[2].port, 80);
    }
    
    #[test]
    fn test_sort_field_from_str() {
        assert_eq!("port".parse::<SortField>().unwrap(), SortField::Port);
        assert_eq!("memory".parse::<SortField>().unwrap(), SortField::Memory);
        assert_eq!("mem".parse::<SortField>().unwrap(), SortField::Memory);
        assert!("invalid".parse::<SortField>().is_err());
    }
}
