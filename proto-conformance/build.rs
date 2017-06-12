extern crate proto_build;

fn main() {
    proto_build::compile_protos(&["src/google/protobuf/unittest_proto3.proto",
                                  "src/conformance.proto"],
                                &["src"]).unwrap();

    proto_build::compile_protos(&["src/google/protobuf/unittest.proto"],
                                &["src"]).unwrap();
}
