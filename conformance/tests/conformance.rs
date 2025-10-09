#![cfg(not(target_os = "windows"))]

use std::env;
use std::process::Command;

use protobuf::conformance;

/// Runs the protobuf conformance test. This must be done in an integration test
/// so that Cargo will build the proto-conformance binary.
#[test]
fn test_conformance() {
    // Get the path to the proto-conformance binary. Adapted from
    // https://github.com/rust-lang/cargo/blob/19fdb308cdbb25faf4f1e25a71351d8d603fa447/tests/cargotest/support/mod.rs#L306.
    let proto_conformance = env::current_exe()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path.join("conformance")
        })
        .unwrap();

    let status = Command::new(conformance::test_runner())
        .arg("--enforce_recommended")
        .arg("--maximum_edition")
        .arg("2023")
        .arg("--failure_list")
        .arg("failing_tests.txt")
        .arg(proto_conformance)
        .status()
        .expect("failed to execute conformance-test-runner");

    assert!(status.success(), "proto conformance test failed");
}
