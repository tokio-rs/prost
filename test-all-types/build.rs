extern crate prost_build;
extern crate prost_codegen;

use prost_codegen::CodeGeneratorConfig;

fn main() {
    // Generate BTreeMap fields for all messages. This forces encoded output to be consistent, so
    // that encode/decode roundtrips can use encoded output for comparison. Otherwise trying to
    // compare based on the Rust PartialEq implementations is difficult, due to presence of NaN
    // values.
    prost_build::compile_protos_with_config(CodeGeneratorConfig::new().btree_map(&["."]),
                                            &["src/test_messages_proto3.proto"],
                                            &["src"]).unwrap();
}
