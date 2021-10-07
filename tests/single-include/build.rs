use prost_build::Config;

fn main() {
    Config::new()
        .include_file("lib.rs")
        .compile_protos(&["protos/search.proto"], &["protos"])
        .unwrap();

    Config::new()
        .out_dir("src/outdir")
        .include_file("mod.rs")
        .compile_protos(&["protos/outdir.proto"], &["protos"])
        .unwrap();
}
