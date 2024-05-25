mod should_compile_successfully {
    include!(concat!(env!("OUT_DIR"), "/no_shadowed_types.rs"));
}

#[test]
fn dummy() {}
