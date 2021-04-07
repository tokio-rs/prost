fn main() {
    prost_build::compile_protos(&["proto/prosit.proto"], &["proto/", ".."]).unwrap();
}
