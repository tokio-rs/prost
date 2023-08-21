//! Tests for skipping the default Debug implementation.

use prost::alloc::{format, string::String};
use std::fmt;

/// A special case with a tuple struct
#[test]
fn tuple_struct_custom_debug() {
    #[derive(Clone, PartialEq, prost::Message)]
    #[prost(skip_debug)]
    struct NewType(#[prost(enumeration = "BasicEnumeration", tag = "5")] i32);
    impl fmt::Debug for NewType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(f, "NewType(custom_debug)")?;
        }
    }
    assert_eq!(
        format!("{:?}", NewType(BasicEnumeration::TWO as i32)),
        "NewType(custom_debug)"
    );
    assert_eq!(format!("{:?}", NewType(42)), "NewType(42)");
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, prost::Oneof)]
#[prost(skip_debug)]
pub enum OneofWithEnumNoDebug {
    #[prost(int32, tag = "8")]
    Int(i32),
    #[prost(string, tag = "9")]
    String(String),
    #[prost(enumeration = "BasicEnumeration", tag = "10")]
    Enumeration(i32),
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(skip_debug)]
struct MessageWithOneofCustomDebug {
    #[prost(oneof = "OneofWithEnumCustomDebug", tags = "8, 9, 10")]
    of: Option<OneofWithEnumCustomDebug>,
}

impl fmt::Debug for MessageWithOneofCustomDebug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MessageWithOneOfCustomDebug {..}")
    }
}

/// Enumerations inside oneofs
#[test]
fn oneof_with_enum_custom_Debug() {
    let msg = MessageWithOneofCustomDebug {
        of: Some(OneofWithEnumCustomDebug::Enumeration(
            BasicEnumeration::TWO as i32,
        )),
    };
    assert_eq!(format!("{:?}", msg), "MessageWithOneofCustomDebug {..}");
}
