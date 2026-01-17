include!(concat!(env!("OUT_DIR"), "/result_enum.rs"));

#[test]
fn test_enum_named_result_value() {
    let _ = FailMessage {
        result: Result::Hello.into(),
    };
}
