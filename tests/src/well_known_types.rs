include!(concat!(env!("OUT_DIR"), "/well_known_types.rs"));

#[test]
fn test_well_known_types() {
    let msg = Foo {
        null: ::prost_types::NullValue::NullValue.into(),
        timestamp: Some(::prost_types::Timestamp {
            seconds: 99,
            nanos: 42,
        }),
        double: Some(42.0_f64),
        float: Some(42.0_f32),
        int64: Some(42_i64),
        uint64: Some(42_u64),
        int32: Some(42_i32),
        uint32: Some(42_u32),
        bool: Some(false),
        string: Some("value".to_owned()),
        bytes: Some(b"value".to_vec()),
    };

    crate::check_message(&msg);
}
