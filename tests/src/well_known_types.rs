include!(concat!(env!("OUT_DIR"), "/well_known_types.rs"));

#[test]
fn test_well_known_types() {
    let msg = Foo {
        null: ::prost_types::NullValue::NullValue.into(),
        timestamp: Some(::prost_types::Timestamp {
            seconds: 99,
            nanos: 42,
        }),
    };

    crate::check_message(&msg);
}
