#![cfg(test)]

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use prost::Message;
use protobuf::test_messages::proto3::TestAllTypesProto3;

#[test]
fn test_decode_error_invalid_wire_type() {
    let msg = [0x36].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: invalid wire type value: 6"
    );
}

#[test]
fn test_decode_error_invalid_varint() {
    let msg = [0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.optional_int32: invalid varint"
    );
}

#[test]
fn test_decode_error_multiple_levels() {
    use protobuf::test_messages::proto3::ForeignMessage;
    let msg = TestAllTypesProto3 {
        recursive_message: Some(Box::new(TestAllTypesProto3 {
            optional_foreign_message: Some(ForeignMessage { c: -1 }),
            ..Default::default()
        })),
        ..Default::default()
    };
    let mut buf = msg.encode_to_vec();

    // Last byte is part of varint value `-1`. Set it to an invalid value.
    assert_eq!(buf.last().unwrap(), &0x01);
    *buf.last_mut().unwrap() = 0xFF;

    assert_eq!(
        TestAllTypesProto3::decode(buf.as_slice())
            .unwrap_err()
            .to_string(),
        "failed to decode Protobuf message: ForeignMessage.c: TestAllTypesProto3.optional_foreign_message: TestAllTypesProto3.recursive_message: invalid varint"
    );
}

#[cfg(not(target_pointer_width = "64"))]
#[test]
fn test_decode_error_length_delimiter_too_large() {
    assert!((usize::MAX as u64) < u64::MAX);

    let msg = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01].as_slice();
    assert_eq!(
        prost::decode_length_delimiter(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: length delimiter exceeds maximum usize value"
    );
}

#[test]
fn test_decode_error_recursion_limit_reached() {
    let recursve_message = {
        let mut msg = TestAllTypesProto3::default();
        for _ in 0..101 {
            msg = TestAllTypesProto3 {
                recursive_message: Some(Box::new(msg)),
                ..Default::default()
            };
        }
        msg
    };

    let buf = recursve_message.encode_to_vec();
    assert_eq!(
        TestAllTypesProto3::decode(buf.as_slice())
            .unwrap_err()
            .to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: TestAllTypesProto3.recursive_message: recursion limit reached"
    );
}

#[test]
fn test_decode_error_invalid_key_value() {
    let msg = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: invalid key value: 1125899906842623"
    );
}

#[test]
fn test_decode_error_invalid_tag() {
    let msg = [0x00].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode(msg).unwrap_err().to_string(),
        "failed to decode Protobuf message: invalid tag value: 0"
    );
}

#[test]
fn test_decode_error_unexpected_wire_type() {
    let mut buf = [0x00].as_slice();
    let mut msg = TestAllTypesProto3::default();
    let ctx = prost::encoding::DecodeContext::new(None);
    assert_eq!(
        msg.merge_field(1, prost::encoding::WireType::LengthDelimited, &mut buf, ctx)
            .unwrap_err()
            .to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.optional_int32: invalid wire type: LengthDelimited (expected Varint)"
    );
}

#[test]
fn test_decode_error_map_invalid_wire_type() {
    use alloc::collections::BTreeMap;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
    enum Color {
        Red = 0,
    }

    #[derive(Clone, PartialEq, Message)]
    struct Maps {
        #[prost(btree_map = "string, int32", tag = "1")]
        scalar: BTreeMap<String, i32>,
        #[prost(btree_map = "string, message", tag = "2")]
        message: BTreeMap<String, Maps>,
        #[prost(btree_map = "string, enumeration(Color)", tag = "3")]
        enumeration: BTreeMap<String, i32>,
    }

    // Each field uses wire type Varint, followed by bytes that would otherwise
    // be read as a length-delimited `{"a": 7}` map entry.
    for (key, field) in [(0x08, "scalar"), (0x10, "message"), (0x18, "enumeration")] {
        let msg = [key, 0x05, 0x0a, 0x01, 0x61, 0x10, 0x07];
        assert_eq!(
            Maps::decode(msg.as_slice()).unwrap_err().to_string(),
            alloc::format!(
                "failed to decode Protobuf message: Maps.{field}: invalid wire type: Varint (expected LengthDelimited)"
            )
        );
    }
}

#[test]
fn test_decode_error_buffer_underflow() {
    let msg = [0x12].as_slice();
    assert_eq!(
        TestAllTypesProto3::decode_length_delimited(msg)
            .unwrap_err()
            .to_string(),
        "failed to decode Protobuf message: buffer underflow"
    );
}

#[test]
fn test_decode_error_invalid_string() {
    let msg = TestAllTypesProto3 {
        optional_string: "Hello".to_string(),
        ..Default::default()
    };
    let mut buf = msg.encode_to_vec();

    // Last byte is part of string value `o`. Set it to an invalid value.
    assert_eq!(buf.last().unwrap(), &b'o');
    *buf.last_mut().unwrap() = 0xA0;

    assert_eq!(
        TestAllTypesProto3::decode(buf.as_slice())
            .unwrap_err()
            .to_string(),
        "failed to decode Protobuf message: TestAllTypesProto3.optional_string: invalid string value: data is not UTF-8 encoded"
    );
}

#[test]
fn test_decode_error_any() {
    use prost_types::{Any, Timestamp};

    let msg = Any {
        type_url: "non-existing-url".to_string(),
        value: Vec::new(),
    };

    assert_eq!(
        msg.to_msg::<Timestamp>().unwrap_err().to_string(),
        "failed to decode Protobuf message: unexpected type URL.type_url: expected type URL: \"type.googleapis.com/google.protobuf.Timestamp\" (got: \"non-existing-url\")"
    );
}
