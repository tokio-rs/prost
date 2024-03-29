//! Tests for skipping the default Debug implementation.

use std::fmt;

use prost::alloc::format;
#[cfg(not(feature = "std"))]
use prost::alloc::string::String;
use prost::OpenEnum;

use crate::custom_debug::{msg, AnEnum, Msg};
use crate::message_encoding::BasicEnumeration;

/// A special case with a tuple struct
#[test]
fn tuple_struct_custom_debug() {
    #[derive(Clone, PartialEq, prost::Message)]
    #[prost(skip_debug)]
    struct NewType(
        #[prost(enumeration = "BasicEnumeration", tag = "5")] OpenEnum<BasicEnumeration>,
    );
    impl fmt::Debug for NewType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("NewType(custom_debug)")
        }
    }
    assert_eq!(
        format!("{:?}", NewType(BasicEnumeration::TWO.into())),
        "NewType(custom_debug)"
    );
    assert_eq!(
        format!("{:?}", NewType(OpenEnum::from_raw(42))),
        "NewType(custom_debug)"
    );
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, prost::Oneof)]
#[prost(skip_debug)]
pub enum OneofWithEnumCustomDebug {
    #[prost(int32, tag = "8")]
    Int(i32),
    #[prost(string, tag = "9")]
    String(String),
    #[prost(enumeration = "BasicEnumeration", tag = "10")]
    Enumeration(OpenEnum<BasicEnumeration>),
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(skip_debug)]
struct MessageWithOneofCustomDebug {
    #[prost(oneof = "OneofWithEnumCustomDebug", tags = "8, 9, 10")]
    of: Option<OneofWithEnumCustomDebug>,
}

impl fmt::Debug for MessageWithOneofCustomDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MessageWithOneofCustomDebug {..}")
    }
}

/// Enumerations inside oneofs
#[test]
fn oneof_with_enum_custom_debug() {
    let msg = MessageWithOneofCustomDebug {
        of: Some(OneofWithEnumCustomDebug::Enumeration(
            BasicEnumeration::TWO.into(),
        )),
    };
    assert_eq!(format!("{:?}", msg), "MessageWithOneofCustomDebug {..}");
}

/// Generated protobufs
#[test]
fn test_proto_msg_custom_debug() {
    let msg = Msg {
        a: 0,
        b: "".to_string(),
        c: Some(msg::C::D(AnEnum::A.into())),
    };
    assert_eq!(format!("{:?}", msg), "Msg {..}");
}
