extern crate prost_build;
extern crate prost_codegen;
extern crate protobuf;

fn main() {
    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    let proto_includes = protobuf::include().join("google").join("protobuf");
    prost_build::compile_protos_with_config(prost_codegen::Config::new().btree_map(&["."]),
                                            &[proto_includes.join("test_messages_proto3.proto")],
                                            &[protobuf::include()]).unwrap();

    prost_build::compile_protos_with_config(prost_codegen::Config::new().btree_map(&["."]),
                                            &[proto_includes.join("unittest.proto")],
                                            &[protobuf::include()]).unwrap();

    prost_build::compile_protos_with_config(prost_codegen::Config::new().btree_map(&["."]),
                                            &["src/packages/widget_factory.proto"],
                                            &["src/packages"]).unwrap();
}
