use env_logger;
use prost_build;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let _ = env_logger::init();

    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    let mut prost_build = prost_build::Config::new();
    prost_build.btree_map(&["."]);
    // Tests for custom attributes
    prost_build.type_attribute("Foo.Bar_Baz.Foo_barBaz", "#[derive(Eq, PartialOrd, Ord)]");
    prost_build.type_attribute(
        "Foo.Bar_Baz.Foo_barBaz.fuzz_buster",
        "#[derive(Eq, PartialOrd, Ord)]",
    );
    prost_build.type_attribute("Foo.Custom.Attrs.Msg", "#[allow(missing_docs)]");
    prost_build.type_attribute("Foo.Custom.Attrs.Msg.field", "/// Oneof docs");
    prost_build.type_attribute("Foo.Custom.Attrs.AnEnum", "#[allow(missing_docs)]");
    prost_build.type_attribute("Foo.Custom.Attrs.AnotherEnum", "/// Oneof docs");
    prost_build.type_attribute(
        "Foo.Custom.OneOfAttrs.Msg.field",
        "#[derive(Eq, PartialOrd, Ord)]",
    );
    prost_build.field_attribute("Foo.Custom.Attrs.AnotherEnum.C", "/// The C docs");
    prost_build.field_attribute("Foo.Custom.Attrs.AnotherEnum.D", "/// The D docs");
    prost_build.field_attribute("Foo.Custom.Attrs.Msg.field.a", "/// Oneof A docs");
    prost_build.field_attribute("Foo.Custom.Attrs.Msg.field.b", "/// Oneof B docs");

    prost_build
        .compile_protos(&["src/ident_conversion.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/nesting.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/recursive_oneof.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/custom_attributes.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/oneof_attributes.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/no_unused_results.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/default_enum_value.proto"], &["src"])
        .unwrap();

    prost_build
        .compile_protos(&["src/packages/widget_factory.proto"], &["src/packages"])
        .unwrap();

    // Compile some of the modules examples as an extern_path.
    let extern_path =
        &PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"))
            .join("extern_paths");
    fs::create_dir_all(extern_path).expect("failed to create prefix directory");

    prost_build
        .out_dir(extern_path)
        .extern_path(".packages.gizmo", "crate::packages::gizmo")
        .compile_protos(&["src/packages/widget_factory.proto"], &["src/packages"])
        .unwrap();
}
