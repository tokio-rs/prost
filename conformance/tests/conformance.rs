#![cfg(not(target_os = "windows"))]

use std::env;
use std::process::Command;

use protobuf::conformance;

/// Runs the protobuf conformance test. This must be done in an integration test
/// so that Cargo will build the proto-conformance binary.
#[test]
fn test_conformance() {
    let proto_conformance =
        env::var("CARGO_BIN_EXE_conformance").expect("Cargo must provide path to build binaries");

    let status = Command::new(conformance::test_runner())
        .arg("--enforce_recommended")
        .arg(proto_conformance)
        .status()
        .expect("failed to execute conformance-test-runner");

    assert!(status.success(), "proto conformance test failed");
}
