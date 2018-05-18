mod should_compile_successfully {
    #![deny(unused_results)]
    include!(concat!(env!("OUT_DIR"), "/no_unused_results.rs"));
}

#[test]
fn dummy() {}
