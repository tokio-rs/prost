fn main() {
    prost_build::compile_protos(&["proto/foo.proto"], &["proto"]).unwrap()
}
