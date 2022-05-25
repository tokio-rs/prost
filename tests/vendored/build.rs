fn main() {
    prost_build::Config::new()
        .protoc_path(protobuf_src::protoc())
        .protoc_include_path(protobuf_src::include())
        .compile_protos(&["proto/foo.proto"], &["proto"])
        .unwrap()
}
