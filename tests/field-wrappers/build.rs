use prost_build::Config;

fn main() {
    let wrapper_fields = [
        ".MyMessage.int",
        ".MyMessage.optional_int",
        ".MyMessage.repeated_int",
        ".MyMessage.packed_int",
        ".MyMessage.str",
        ".MyMessage.optional_str",
        ".MyMessage.repeated_str",
        ".MyMessage.packed_str",
        ".MyMessage.payload",
        ".MyMessage.optional_payload",
        ".MyMessage.repeated_payload",
        ".MyMessage.map_payload",
        ".MyMessage.group",
        ".MyMessage.optional_group",
        ".MyMessage.repeated_group",
        ".MyMessage.oneof_field",
        ".MyMessage.my_enum",
        ".MyMessage.optional_my_enum",
        ".MyMessage.repeated_my_enum",
        ".MyMessage.packed_my_enum",
        ".MyMessage.default_int",
        ".MyMessage.default_float",
        ".MyMessage.default_string",
    ];

    Config::new()
        .arc(wrapper_fields)
        .default_package_filename("wrappers_arc")
        .btree_map(&[".MyMessage.map_payload"])
        .out_dir("src")
        .compile_protos(&["protos/wrappers.proto"], &["protos"])
        .unwrap();

    Config::new()
        .r#box(wrapper_fields)
        .default_package_filename("wrappers_box")
        .btree_map(&[".MyMessage.map_payload"])
        .out_dir("src")
        .compile_protos(&["protos/wrappers.proto"], &["protos"])
        .unwrap();
}
