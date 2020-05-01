use prost::{Message, UnknownField};

mod unknown_fields {
    include!(concat!(env!("OUT_DIR"), "/unknown_fields.rs"));
}

#[test]
fn test_access_unknown_field() {
    let message = unknown_fields::MessageWithExtraFields {
        normal_field: "normal".to_string(),
        extra_field: "extra".to_string(),
    };
    let mut encoded = Vec::new();
    message.encode(&mut encoded).unwrap();
    let message = unknown_fields::MessageWithUnknownFields::decode(&encoded[..])
        .expect("Could not decode as MessageWithUnknownFields");

    assert_eq!(
        message.protobuf_unknown_fields,
        vec![UnknownField {
            tag: 2,
            value: b"extra".to_vec()
        }]
    );
}

#[test]
fn test_serialize_unknown_field() {
    let message = unknown_fields::MessageWithUnknownFields {
        normal_field: "normal".to_string(),
        protobuf_unknown_fields: vec![UnknownField {
            tag: 2,
            value: b"extra".to_vec(),
        }],
    };
    let mut encoded = Vec::new();
    message.encode(&mut encoded).unwrap();
    let message = unknown_fields::MessageWithExtraFields::decode(&encoded[..])
        .expect("Could not decode as MessageWithExtraFields");

    assert_eq!(message.extra_field, "extra");
}

#[test]
fn test_access_repeated_unknown_field() {
    let message = unknown_fields::MessageWithRepeatedExtraFields {
        normal_field: "normal".to_string(),
        extra_field: vec![
            "repeated".to_string(),
            "extra".to_string(),
            "repeated".to_string(),
        ],
    };
    let mut encoded = Vec::new();
    message.encode(&mut encoded).unwrap();
    let message = unknown_fields::MessageWithUnknownFields::decode(&encoded[..])
        .expect("Could not decode as MessageWithUnknownFields");

    assert_eq!(
        message.protobuf_unknown_fields,
        vec![
            UnknownField {
                tag: 2,
                value: b"repeated".to_vec(),
            },
            UnknownField {
                tag: 2,
                value: b"extra".to_vec(),
            },
            UnknownField {
                tag: 2,
                value: b"repeated".to_vec(),
            },
        ]
    );
}
