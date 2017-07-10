extern crate prost_build;
extern crate protobuf;

fn main() {
    let proto_includes = protobuf::include().join("google").join("protobuf");

    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    let mut prost_build = prost_build::Config::new();
    prost_build.btree_map(&["."]);

    prost_build.compile_protos(&[proto_includes.join("test_messages_proto3.proto")],
                               &[protobuf::include()]).unwrap();

    prost_build.compile_protos(&[proto_includes.join("unittest.proto")],
                               &[protobuf::include()]).unwrap();

    prost_build.compile_protos(&["src/packages/widget_factory.proto"],
                               &["src/packages"]).unwrap();
}
