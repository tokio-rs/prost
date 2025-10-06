include!(concat!(env!("OUT_DIR"), "/option_struct.rs"));

#[test]
fn test_struct_named_option_value() {
    let _ = Option {
        msg: "Can I create?".into(),
    };
}
