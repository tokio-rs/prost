include!(concat!(env!("OUT_DIR"), "/optional_enum.rs"));

#[test]
fn test_optional_enum_value() {
    let mut msg = Message::default();
    assert_eq!(msg.v, None);
    assert_eq!(msg.v(), None);
    assert_eq!(msg.v().unwrap_or_default(), Variant::Default);

    let msg = Message { v: None };
    assert_eq!(msg.v, None);
    assert_eq!(msg.v(), None);
    assert_eq!(msg.v().unwrap_or_default(), Variant::Default);

    let msg = Message {
        v: Some(Variant::Default as i32),
    };
    assert_eq!(msg.v, Some(Variant::Default as i32));
    assert_eq!(msg.v(), Some(Variant::Default));
    assert_eq!(msg.v().unwrap_or_default(), Variant::Default);

    let msg = Message {
        v: Some(Variant::NotDefault as i32),
    };
    assert_eq!(msg.v, Some(Variant::NotDefault as i32));
    assert_eq!(msg.v(), Some(Variant::NotDefault));
    assert_eq!(msg.v().unwrap_or_default(), Variant::NotDefault);
}
