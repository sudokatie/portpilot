//! Output formatting tests.

use portpilot::scanner::{PortEntry, Protocol};
use portpilot::output::{format_ports, OutputFormat, OutputOptions};
use std::net::{IpAddr, Ipv4Addr};

fn make_entry(port: u16, process: &str, external: bool) -> PortEntry {
    let addr = if external {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    };
    let mut e = PortEntry::new(port, Protocol::Tcp, addr);
    e.process_name = Some(process.to_string());
    e.pid = Some(1234);
    e.memory_bytes = Some(1024 * 1024); // 1MB
    e
}

#[test]
fn test_human_format_contains_port() {
    let entries = vec![make_entry(3000, "node", false)];
    let opts = OutputOptions {
        format: OutputFormat::Human,
        no_color: true,
        mask_values: false,
    };
    
    let output = format_ports(&entries, &opts);
    assert!(output.contains("3000"));
    assert!(output.contains("node"));
}

#[test]
fn test_json_format_valid() {
    let entries = vec![make_entry(8080, "java", true)];
    let opts = OutputOptions {
        format: OutputFormat::Json,
        no_color: true,
        mask_values: false,
    };
    
    let output = format_ports(&entries, &opts);
    
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON");
    assert!(parsed.get("ports").is_some());
    assert!(parsed.get("summary").is_some());
}

#[test]
fn test_oneline_format_tabs() {
    let entries = vec![make_entry(5432, "postgres", false)];
    let opts = OutputOptions {
        format: OutputFormat::Oneline,
        no_color: true,
        mask_values: false,
    };
    
    let output = format_ports(&entries, &opts);
    
    // Should be tab-separated
    assert!(output.contains('\t'));
    assert!(output.contains("5432"));
    assert!(output.contains("postgres"));
}

#[test]
fn test_quiet_format_empty() {
    let entries = vec![make_entry(80, "nginx", true)];
    let opts = OutputOptions {
        format: OutputFormat::Quiet,
        no_color: true,
        mask_values: false,
    };
    
    let output = format_ports(&entries, &opts);
    assert!(output.is_empty());
}

#[test]
fn test_summary_counts() {
    let entries = vec![
        make_entry(80, "nginx", true),     // external
        make_entry(3000, "node", false),   // local
        make_entry(443, "nginx", true),    // external
    ];
    let opts = OutputOptions {
        format: OutputFormat::Human,
        no_color: true,
        mask_values: false,
    };
    
    let output = format_ports(&entries, &opts);
    assert!(output.contains("3 ports listening"));
    assert!(output.contains("2 external"));
    assert!(output.contains("1 localhost"));
}
