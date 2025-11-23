use crate::check_message;

mod should_compile_successfully {
    #![deny(unused_results)]
    include!(concat!(env!("OUT_DIR"), "/no_unused_results.rs"));
}

#[test]
fn roundtrip() {
    let msg = should_compile_successfully::Test {
        dummy_field: "dummy".into(),
    };
    check_message(&msg);
}
