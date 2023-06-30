include!(concat!(env!("OUT_DIR"), "/string_types.rs"));

#[test]
fn test_string_types() {
    // Successful compilation means the types and generated code are correct
    let _ = StringTypes {
        is_string: ::prost::alloc::string::String::default(),
        is_boxed_str: ::prost::alloc::boxed::Box::<str>::default(),
    };
}
