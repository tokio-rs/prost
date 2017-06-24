extern crate proto_build;

fn main() {
    proto_build::compile_protos(&["src/test_messages_proto3.proto"],
                                &["src"],
                                None).unwrap();
}
