include!(concat!(env!("OUT_DIR"), "/optional_enum.rs"));

#[test]
fn test_optional_enum_value() {
    let msg = Message { v: None };
    assert_eq!(msg.v, None);
    assert_eq!(
        core::any::type_name_of_val(&msg.v()),
        core::any::type_name::<Option<Variant>>()
    );
}
