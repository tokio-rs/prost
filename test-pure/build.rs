fn main() {
    let mut parser = protobuf_parse::Parser::new();

    parser.input("proto/foo.proto");
    parser.include("proto");

    parser.pure();

    let fds = parser.file_descriptor_set().unwrap();

    let tmp = std::env::temp_dir();
    let fds_path = tmp.join("protobuf-pure-fds.bin");
    let mut file = std::fs::File::create(fds_path.clone()).unwrap();

    use protobuf::Message;
    fds.write_to_writer(&mut file).unwrap();

    prost_build::Config::new()
        .file_descriptor_set_path(fds_path)
        .skip_protoc_run()
        .compile_protos(&["proto/foo.proto"], &["proto"])
        .unwrap()
}
