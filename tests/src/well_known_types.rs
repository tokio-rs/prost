use prost::MessageDescriptor;
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

#[test]
fn test_message_descriptor() {
    let msg = Foo {
        null: ::prost_types::NullValue::NullValue.into(),
        timestamp: Some(::prost_types::Timestamp {
            seconds: 99,
            nanos: 42,
        }),
    };

    assert_eq!(msg.message_name(), "Foo");
    assert_eq!(msg.package_name(), "well_known_types");
    assert_eq!(msg.type_url(), "type.googleapis.com/well_known_types.Foo");
}
