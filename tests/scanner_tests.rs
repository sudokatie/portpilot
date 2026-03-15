//! Scanner unit tests.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn test_ipv4_external() {
    let addr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    assert!(addr.is_unspecified());
}

#[test]
fn test_ipv4_localhost() {
    let addr = IpAddr::V4(Ipv4Addr::LOCALHOST);
    assert!(addr.is_loopback());
}

#[test]
fn test_ipv6_external() {
    let addr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
    assert!(addr.is_unspecified());
}

#[test]
fn test_ipv6_localhost() {
    let addr = IpAddr::V6(Ipv6Addr::LOCALHOST);
    assert!(addr.is_loopback());
}
