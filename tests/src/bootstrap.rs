#![cfg(feature = "std")]

use std::path::Path;
use std::path::PathBuf;

macro_rules! assert_eq_bootstrapped_file {
    ($expected_path:expr, $actual_path:expr) => {{
        let expected = std::fs::read_to_string($expected_path).unwrap();
        let actual = std::fs::read_to_string($actual_path).unwrap();

        // Normalizes windows and Linux-style EOL
        let expected = expected.replace("\r\n", "\n");
        let actual = actual.replace("\r\n", "\n");

        if expected != actual {
            std::fs::write($expected_path, &actual).unwrap();
        }

        assert_eq!(expected, actual);
    }};
}

/// Test which bootstraps protobuf.rs and compiler.rs from the .proto definitions in the Protobuf
/// repo. Ensures that the checked-in compiled versions are up-to-date.
#[test]
fn bootstrap() {
    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("src")
        .join("include");
    let protobuf = include.join("google").join("protobuf");

    let tempdir = tempfile::Builder::new()
        .prefix("prost-types-bootstrap")
        .tempdir()
        .unwrap();

    prost_build::Config::new()
        .compile_well_known_types()
        .btree_map(["."])
        .type_attribute(
            ".",
            r#"#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]"#,
        )
        .out_dir(tempdir.path())
        .compile_protos(
            &[
                // Protobuf Plugins.
                protobuf.join("descriptor.proto"),
                protobuf.join("compiler").join("plugin.proto"),
                // Well-known Types (except wrapper.proto, whose types are substituted
                // for the corresponding standard library types).
                protobuf.join("any.proto"),
                protobuf.join("api.proto"),
                protobuf.join("duration.proto"),
                protobuf.join("field_mask.proto"),
                protobuf.join("source_context.proto"),
                protobuf.join("struct.proto"),
                protobuf.join("timestamp.proto"),
                protobuf.join("type.proto"),
            ],
            &[include],
        )
        .unwrap();

    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("no parent")
        .join("prost-types")
        .join("src");

    assert_eq_bootstrapped_file!(
        src.join("protobuf.rs"),
        tempdir.path().join("google.protobuf.rs")
    );

    assert_eq_bootstrapped_file!(
        src.join("compiler.rs"),
        tempdir.path().join("google.protobuf.compiler.rs")
    );
}
