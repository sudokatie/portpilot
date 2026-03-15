//! CLI integration tests.

use assert_cmd::Command;
use predicates::prelude::*;

fn portpilot() -> Command {
    Command::cargo_bin("portpilot").unwrap()
}

#[test]
fn test_help() {
    portpilot()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("portpilot"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--kill"));
}

#[test]
fn test_version() {
    portpilot()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("portpilot"));
}

#[test]
fn test_list_ports() {
    // This will list whatever ports are currently in use
    portpilot()
        .assert()
        .success();
}

#[test]
fn test_list_ports_json() {
    // JSON mode should always output valid JSON even if no ports
    portpilot()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"));
}

#[test]
fn test_invalid_port() {
    portpilot()
        .arg("abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid port"));
}

#[test]
fn test_invalid_port_range() {
    portpilot()
        .arg("3010-3000")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid range"));
}

#[test]
fn test_port_not_in_use() {
    // Port 65432 is unlikely to be in use
    portpilot()
        .arg("65432")
        .assert()
        .failure()
        .stdout(predicate::str::contains("not in use"));
}

#[test]
fn test_port_not_in_use_quiet() {
    // Quiet mode returns exit code 1 for not in use
    portpilot()
        .args(["65432", "--quiet"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_oneline_format() {
    portpilot()
        .arg("--oneline")
        .assert()
        .success();
}

#[test]
fn test_filter_nonexistent_process() {
    portpilot()
        .args(["--filter", "NONEXISTENT_PROCESS_NAME_12345"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No listening ports"));
}

#[test]
fn test_external_filter() {
    portpilot()
        .arg("--external")
        .assert()
        .success();
}

#[test]
fn test_local_filter() {
    portpilot()
        .arg("--local")
        .assert()
        .success();
}

#[test]
fn test_sort_options() {
    for sort in &["port", "process", "memory", "cpu", "time"] {
        portpilot()
            .args(["--sort", sort])
            .assert()
            .success();
    }
}

#[test]
fn test_reverse_sort() {
    portpilot()
        .args(["--sort", "port", "--reverse"])
        .assert()
        .success();
}

#[test]
fn test_udp_flag() {
    portpilot()
        .arg("--udp")
        .assert()
        .success();
}

#[test]
fn test_no_color() {
    portpilot()
        .arg("--no-color")
        .assert()
        .success();
}

#[test]
fn test_combined_filters() {
    // JSON output with filters should still be valid JSON
    portpilot()
        .args(["--json", "--external", "--sort", "memory", "--reverse"])
        .assert()
        .success()
        .stdout(predicate::str::contains("{"));
}
