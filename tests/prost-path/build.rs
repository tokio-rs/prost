fn main() {
    prost_build::Config::new()
        .prost_path("::reexported_prost::prost")
        .prost_types_path("::reexported_prost::prost_types")
        .compile_protos(&["protos/prost_path.proto"], &["protos"])
        .unwrap();
}
