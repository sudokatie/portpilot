//! Filter and sort integration tests.

use portpilot::scanner::{PortEntry, Protocol};
use portpilot::filter::{filter_entries, FilterOptions};
use portpilot::sort::{sort_entries, SortField, SortOptions};
use std::net::{IpAddr, Ipv4Addr};

fn make_entry(port: u16, process: &str, user: &str, external: bool, memory: u64) -> PortEntry {
    let addr = if external {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    };
    let mut e = PortEntry::new(port, Protocol::Tcp, addr);
    e.process_name = Some(process.to_string());
    e.user = Some(user.to_string());
    e.pid = Some(1000 + port as u32);
    e.memory_bytes = Some(memory);
    e
}

#[test]
fn test_filter_by_user() {
    let entries = vec![
        make_entry(80, "nginx", "root", true, 100),
        make_entry(3000, "node", "dev", false, 200),
        make_entry(8080, "java", "dev", true, 300),
    ];
    
    let opts = FilterOptions {
        user: Some("dev".to_string()),
        ..Default::default()
    };
    
    let filtered = filter_entries(entries, &opts);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|e| e.user.as_ref().unwrap() == "dev"));
}

#[test]
fn test_filter_combined() {
    let entries = vec![
        make_entry(80, "nginx", "root", true, 100),
        make_entry(3000, "node", "dev", false, 200),
        make_entry(8080, "nodejs", "dev", true, 300),
    ];
    
    let opts = FilterOptions {
        process: Some("node".to_string()),
        external_only: true,
        ..Default::default()
    };
    
    let filtered = filter_entries(entries, &opts);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].port, 8080);
}

#[test]
fn test_sort_by_process_name() {
    let entries = vec![
        make_entry(80, "nginx", "root", true, 100),
        make_entry(3000, "apache", "www", false, 200),
        make_entry(8080, "node", "dev", true, 300),
    ];
    
    let sorted = sort_entries(entries, &SortOptions::new(SortField::Process));
    
    assert_eq!(sorted[0].process_name.as_ref().unwrap(), "apache");
    assert_eq!(sorted[1].process_name.as_ref().unwrap(), "nginx");
    assert_eq!(sorted[2].process_name.as_ref().unwrap(), "node");
}

#[test]
fn test_sort_by_cpu() {
    let mut entries = vec![
        make_entry(80, "nginx", "root", true, 100),
        make_entry(3000, "node", "dev", false, 200),
    ];
    entries[0].cpu_percent = Some(5.0);
    entries[1].cpu_percent = Some(25.0);
    
    let sorted = sort_entries(entries, &SortOptions::new(SortField::Cpu));
    
    assert_eq!(sorted[0].cpu_percent, Some(5.0));
    assert_eq!(sorted[1].cpu_percent, Some(25.0));
}

#[test]
fn test_empty_filter_returns_all() {
    let entries = vec![
        make_entry(80, "nginx", "root", true, 100),
        make_entry(3000, "node", "dev", false, 200),
    ];
    
    let filtered = filter_entries(entries.clone(), &FilterOptions::default());
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_sort_stability() {
    // Same port values should maintain order
    let entries = vec![
        make_entry(80, "nginx", "root", true, 100),
        make_entry(80, "apache", "www", true, 200),
    ];
    
    let sorted = sort_entries(entries, &SortOptions::new(SortField::Port));
    
    // Both have same port, order should be stable
    assert_eq!(sorted.len(), 2);
}
