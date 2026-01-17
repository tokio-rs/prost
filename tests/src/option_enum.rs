include!(concat!(env!("OUT_DIR"), "/option_enum.rs"));

#[test]
fn test_enum_named_option_value() {
    let _ = FailMessage {
        result: Option::Hello.into(),
    };
}
