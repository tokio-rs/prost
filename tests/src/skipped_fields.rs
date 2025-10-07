//! Tests for skipping fields when using prost-derive.

use crate::alloc::string::ToString;
use crate::check_serialize_equivalent;
use alloc::collections::BTreeMap;
use alloc::string::String;
use prost::Message;

/// A struct with the same data as another, but with a skipped field, should be equal when encoded.
#[test]
fn skipped_field_serial_equality() {
    #[derive(Clone, PartialEq, prost::Message)]
    struct TypeWithoutSkippedField {
        #[prost(string, tag = "1")]
        value: String,
    }

    fn create_hashmap() -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), "value".to_string());
        map
    }

    #[derive(Clone, PartialEq, prost::Message)]
    struct TypeWithSkippedField {
        #[prost(string, tag = "1")]
        value: String,
        #[prost(skip, default = "create_hashmap")]
        pub temp_data: BTreeMap<String, String>, // This field will be skipped
    }

    let a = TypeWithoutSkippedField {
        value: "hello".to_string(),
    };
    let b = TypeWithSkippedField {
        value: "hello".to_string(),
        temp_data: create_hashmap(),
    };

    // Encoded forms should be equal
    check_serialize_equivalent(&a, &b);

    // Decoded forms should be equal, with the skipped field initialized using the default attribute
    let decoded = TypeWithSkippedField::decode(a.encode_to_vec().as_slice()).unwrap();
    assert_eq!(b, decoded);
}
