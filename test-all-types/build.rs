extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/test_messages_proto3.proto"],
                                &["src"]).unwrap();
}
