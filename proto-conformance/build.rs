extern crate proto_build;

fn main() {

    proto_build::compile_protos(&["unittest.proto"], &["/Users/dan/src/cpp/protobuf/src"]).unwrap();
}
