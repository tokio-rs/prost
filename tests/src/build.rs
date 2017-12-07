extern crate env_logger;
extern crate prost_build;
extern crate protobuf;

fn main() {
    let _ = env_logger::init();

    let proto_includes = protobuf::include().join("google").join("protobuf");

    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    let mut prost_build = prost_build::Config::new();
    prost_build.btree_map(&["."]);
    prost_build.container_attribute(".Foo.Bar_Baz.Foo_barBaz", "#[derive(Eq, PartialOrd, Ord)]");
    prost_build.container_attribute(".Foo.Bar_Baz.Foo_barBaz.fuzz_buster",
                                    "#[derive(Eq, PartialOrd, Ord)]");
    // No idea what better to place on that field :-(. We don't wont to depend or eg. Serde to be
    // able to place arbitrary attributes on fields. We'll have to check in nasty way, by reading
    // the text file.
    prost_build.field_attribute(".Foo.Bar_Baz.Foo_barBaz.fooBar_baz", "// Testing comment");

    prost_build.compile_protos(&[proto_includes.join("test_messages_proto2.proto")],
                               &[protobuf::include()]).unwrap();

    prost_build.compile_protos(&[proto_includes.join("test_messages_proto3.proto")],
                               &[protobuf::include()]).unwrap();

    prost_build.compile_protos(&[proto_includes.join("unittest.proto")],
                               &[protobuf::include()]).unwrap();

    prost_build.compile_protos(&["src/packages/widget_factory.proto"],
                               &["src/packages"]).unwrap();

    prost_build.compile_protos(&["src/ident_conversion.proto"],
                               &["src"]).unwrap();

    prost_build.compile_protos(&["src/nesting.proto"],
                               &["src"]).unwrap();

    prost_build.compile_protos(&["src/recursive_oneof.proto"],
                               &["src"]).unwrap();
}
