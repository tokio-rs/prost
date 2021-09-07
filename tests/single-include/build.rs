use prost_build::Config;

fn main() {
    Config::new()
        .include_file("lib.rs")
        .compile_protos(&["protos/search.proto"], &["protos"])
        .unwrap();
}
