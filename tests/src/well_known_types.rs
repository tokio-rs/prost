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
        double: Some(42.0_f64),
        float: Some(42.0_f32),
        int64: Some(42_i64),
        uint64: Some(42_u64),
        int32: Some(42_i32),
        uint32: Some(42_u32),
        bool: Some(false),
        string: Some("value".into()),
        bytes: Some(b"value".to_vec()),
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
        double: None,
        float: None,
        int64: None,
        uint64: None,
        int32: None,
        uint32: None,
        bool: None,
        string: None,
        bytes: None,
    };

    assert_eq!(msg.message_name(), "Foo");
    assert_eq!(msg.package_name(), "well_known_types");
    assert_eq!(msg.type_url(), "type.googleapis.com/well_known_types.Foo");
}

#[cfg(feature = "std")]
#[test]
fn test_timestamp() {
    use std::collections::HashSet;

    let timestamp = ::prost_types::Timestamp {
        seconds: 100,
        nanos: 42,
    };

    let mut non_normalized_timestamp = ::prost_types::Timestamp {
        seconds: 99,
        nanos: 1_000_000_042,
    };

    let mut hashset = HashSet::new();
    assert!(hashset.insert(timestamp.clone()));
    assert!(
        hashset.insert(non_normalized_timestamp.clone()),
        "hash for non-normalized different and should be inserted"
    );

    assert_ne!(
        timestamp, non_normalized_timestamp,
        "non-nomarlized timestamp considered different"
    );
    non_normalized_timestamp.normalize();
    assert_eq!(
        timestamp, non_normalized_timestamp,
        "normalized timestamp matches"
    );

    let mut hashset = HashSet::new();
    assert!(hashset.insert(timestamp.clone()));
    assert!(
        !hashset.insert(non_normalized_timestamp),
        "hash for normalized should match and not inserted"
    );
}
