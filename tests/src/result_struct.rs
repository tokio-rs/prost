include!(concat!(env!("OUT_DIR"), "/result_struct.rs"));

#[test]
fn test_result_named_result_value() {
    let _ = Result {
        msg: "Can I create?".into(),
    };
}
