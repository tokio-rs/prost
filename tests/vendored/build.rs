fn main() {
    std::env::set_var("PROTOC", protobuf_src::protoc());

    prost_build::Config::new()
        .include_file("includes1.rs")
        .compile_protos(&["proto/foo.proto"], &["proto"])
        .unwrap();

    prost_build::Config::new()
        .compile_well_known_types()
        .include_file("includes2.rs")
        .compile_protos(&["proto/foo.proto"], &["proto"])
        .unwrap();
}
