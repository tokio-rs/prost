fn main() {
    prost_build::Config::new()
        .compile_protos(&["hello.proto"], &["."])
        .unwrap();
}
