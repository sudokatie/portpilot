//! Integration tests for portpilot.

use portpilot::scanner::{get_scanner, Protocol, ScanOptions};
use portpilot::filter::{filter_entries, FilterOptions};
use portpilot::sort::{sort_entries, SortField, SortOptions};
use portpilot::output::{format_ports, format_port_detail, OutputFormat, OutputOptions};

#[test]
fn test_scanner_returns_vec() {
    let scanner = get_scanner();
    let opts = ScanOptions::default();
    
    let result = scanner.list_ports(&opts);
    assert!(result.is_ok());
}

#[test]
fn test_scan_with_udp() {
    let scanner = get_scanner();
    let opts = ScanOptions {
        include_udp: true,
        ..Default::default()
    };
    
    let result = scanner.list_ports(&opts);
    assert!(result.is_ok());
}

#[test]
fn test_scan_external_only() {
    let scanner = get_scanner();
    let opts = ScanOptions {
        filter_external: Some(true),
        ..Default::default()
    };
    
    let result = scanner.list_ports(&opts);
    assert!(result.is_ok());
    
    // All results should be external
    if let Ok(entries) = result {
        for entry in &entries {
            assert!(entry.is_external(), "Entry {} should be external", entry.port);
        }
    }
}

#[test]
fn test_scan_localhost_only() {
    let scanner = get_scanner();
    let opts = ScanOptions {
        filter_external: Some(false),
        ..Default::default()
    };
    
    let result = scanner.list_ports(&opts);
    assert!(result.is_ok());
    
    // All results should be localhost
    if let Ok(entries) = result {
        for entry in &entries {
            assert!(entry.is_localhost(), "Entry {} should be localhost", entry.port);
        }
    }
}

#[test]
fn test_full_pipeline() {
    // Scan -> Filter -> Sort -> Format
    let scanner = get_scanner();
    let scan_opts = ScanOptions::default();
    
    let entries = scanner.list_ports(&scan_opts).unwrap();
    
    let filter_opts = FilterOptions::default();
    let filtered = filter_entries(entries, &filter_opts);
    
    let sort_opts = SortOptions::new(SortField::Port);
    let sorted = sort_entries(filtered, &sort_opts);
    
    let output_opts = OutputOptions {
        format: OutputFormat::Human,
        no_color: true,
        ..Default::default()
    };
    
    let output = format_ports(&sorted, &output_opts);
    
    // Should produce some output (header + summary at minimum)
    assert!(output.contains("PORT") || output.contains("ports listening"));
}

#[test]
fn test_json_output_valid() {
    let scanner = get_scanner();
    let opts = ScanOptions::default();
    
    let entries = scanner.list_ports(&opts).unwrap();
    
    let output_opts = OutputOptions {
        format: OutputFormat::Json,
        ..Default::default()
    };
    
    let json = format_ports(&entries, &output_opts);
    
    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(parsed.is_ok(), "JSON output should be valid: {}", json);
    
    // Should have ports array and summary
    let value = parsed.unwrap();
    assert!(value.get("ports").is_some(), "Should have 'ports' field");
    assert!(value.get("summary").is_some(), "Should have 'summary' field");
}

#[test]
fn test_port_detail_nonexistent() {
    let scanner = get_scanner();
    
    // Port 65432 is unlikely to be in use
    let result = scanner.get_port_detail(65432, Protocol::Tcp);
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_none(), "Port 65432 should not be in use");
}

#[test]
fn test_format_port_detail_json() {
    use std::net::{IpAddr, Ipv4Addr};
    use portpilot::scanner::PortEntry;
    
    let mut entry = PortEntry::new(3000, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST));
    entry.process_name = Some("test".to_string());
    entry.pid = Some(1234);
    
    let opts = OutputOptions {
        format: OutputFormat::Json,
        ..Default::default()
    };
    
    let json = format_port_detail(&entry, &opts);
    
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(parsed.is_ok());
    
    let value = parsed.unwrap();
    assert_eq!(value.get("port").and_then(|v| v.as_u64()), Some(3000));
}

#[test]
fn test_sort_reverse() {
    use std::net::{IpAddr, Ipv4Addr};
    use portpilot::scanner::PortEntry;
    
    let entries = vec![
        PortEntry::new(80, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
        PortEntry::new(443, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
        PortEntry::new(8080, Protocol::Tcp, IpAddr::V4(Ipv4Addr::LOCALHOST)),
    ];
    
    let sorted = sort_entries(entries, &SortOptions::new(SortField::Port).reverse(true));
    
    assert_eq!(sorted[0].port, 8080);
    assert_eq!(sorted[1].port, 443);
    assert_eq!(sorted[2].port, 80);
}
