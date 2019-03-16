#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(feature = "edition-2015")] {
        extern crate bytes;
        extern crate prost;
        extern crate protobuf;
        #[cfg(test)]
        extern crate prost_build;
        #[cfg(test)]
        extern crate tempfile;
    }
}

pub mod extern_paths;
pub mod packages;
pub mod unittest;

#[cfg(test)]
mod bootstrap;
#[cfg(test)]
mod debug;
#[cfg(test)]
mod message_encoding;
#[cfg(test)]
mod no_unused_results;

pub mod foo {
    pub mod bar_baz {
        include!(concat!(env!("OUT_DIR"), "/foo.bar_baz.rs"));
    }
}

pub mod nesting {
    include!(concat!(env!("OUT_DIR"), "/nesting.rs"));
}

pub mod recursive_oneof {
    include!(concat!(env!("OUT_DIR"), "/recursive_oneof.rs"));
}

/// This tests the custom attributes support by abusing docs.
///
/// Docs really are full-blown attributes. So we use them to ensure we can place them on everything
/// we need. If they aren't put onto something or allowed not to be there (by the generator),
/// compilation fails.
#[deny(missing_docs)]
pub mod custom_attributes {
    include!(concat!(env!("OUT_DIR"), "/foo.custom.attrs.rs"));
}

/// Also for testing custom attributes, but on oneofs.
///
/// Unfortunately, an OneOf field generates a companion module in the .rs file. There's no
/// reasonable way to place a doc comment on that, so we do the test with `derive(Ord)` and have it
/// in a separate file.
pub mod oneof_attributes {
    include!(concat!(env!("OUT_DIR"), "/foo.custom.one_of_attrs.rs"));
}

/// Issue https://github.com/danburkert/prost/issues/118
///
/// When a message contains an enum field with a default value, we
/// must ensure that the appropriate name conventions are used.
pub mod default_enum_value {
    include!(concat!(env!("OUT_DIR"), "/default_enum_value.rs"));
}

pub mod group_test {
    include!(concat!(env!("OUT_DIR"), "/group_test.rs"));
}

use std::error::Error;

use bytes::{Buf, IntoBuf};

use prost::Message;

pub enum RoundtripResult {
    /// The roundtrip succeeded.
    Ok(Vec<u8>),
    /// The data could not be decoded. This could indicate a bug in prost,
    /// or it could indicate that the input was bogus.
    DecodeError(prost::DecodeError),
    /// Re-encoding or validating the data failed.  This indicates a bug in `prost`.
    Error(Box<dyn Error + Send + Sync>),
}

impl RoundtripResult {
    /// Unwrap the roundtrip result.
    pub fn unwrap(self) -> Vec<u8> {
        match self {
            RoundtripResult::Ok(buf) => buf,
            RoundtripResult::DecodeError(error) => {
                panic!("failed to decode the roundtrip data: {}", error)
            }
            RoundtripResult::Error(error) => panic!("failed roundtrip: {}", error),
        }
    }

    /// Unwrap the roundtrip result. Panics if the result was a validation or re-encoding error.
    pub fn unwrap_error(self) -> Result<Vec<u8>, prost::DecodeError> {
        match self {
            RoundtripResult::Ok(buf) => Ok(buf),
            RoundtripResult::DecodeError(error) => Err(error),
            RoundtripResult::Error(error) => panic!("failed roundtrip: {}", error),
        }
    }
}

/// Tests round-tripping a message type. The message should be compiled with `BTreeMap` fields,
/// otherwise the comparison may fail due to inconsistent `HashMap` entry encoding ordering.
pub fn roundtrip<M>(data: &[u8]) -> RoundtripResult
where
    M: Message + Default,
{
    // Try to decode a message from the data. If decoding fails, continue.
    let all_types = match M::decode(data) {
        Ok(all_types) => all_types,
        Err(error) => return RoundtripResult::DecodeError(error),
    };

    let encoded_len = all_types.encoded_len();

    // TODO: Reenable this once sign-extension in negative int32s is figured out.
    // assert!(encoded_len <= data.len(), "encoded_len: {}, len: {}, all_types: {:?}",
    //         encoded_len, data.len(), all_types);

    let mut buf1 = Vec::new();
    if let Err(error) = all_types.encode(&mut buf1) {
        return RoundtripResult::Error(error.into());
    }
    if encoded_len != buf1.len() {
        return RoundtripResult::Error(
            format!(
                "expected encoded len ({}) did not match actual encoded len ({})",
                encoded_len,
                buf1.len()
            )
            .into(),
        );
    }

    let roundtrip = match M::decode(&buf1) {
        Ok(roundtrip) => roundtrip,
        Err(error) => return RoundtripResult::Error(error.into()),
    };

    let mut buf2 = Vec::new();
    if let Err(error) = roundtrip.encode(&mut buf2) {
        return RoundtripResult::Error(error.into());
    }

    /*
    // Useful for debugging:
    eprintln!(" data: {:?}", data.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!(" buf1: {:?}", buf1.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!("a: {:?}\nb: {:?}", all_types, roundtrip);
    */

    if buf1 != buf2 {
        return RoundtripResult::Error("roundtripped encoded buffers do not match".into());
    }

    RoundtripResult::Ok(buf1)
}

/// Generic rountrip serialization check for messages.
pub fn check_message<M>(msg: &M)
where
    M: Message + Default + PartialEq,
{
    let expected_len = msg.encoded_len();

    let mut buf = Vec::with_capacity(18);
    msg.encode(&mut buf).unwrap();
    assert_eq!(expected_len, buf.len());

    let mut buf = buf.into_buf();
    let roundtrip = M::decode(&mut buf).unwrap();

    if buf.has_remaining() {
        panic!(format!("expected buffer to be empty: {}", buf.remaining()));
    }

    assert_eq!(msg, &roundtrip);
}

/// Serialize from A should equal Serialize from B
pub fn check_serialize_equivalent<M, N>(msg_a: &M, msg_b: &N)
where
    M: Message + Default + PartialEq,
    N: Message + Default + PartialEq,
{
    let mut buf_a = Vec::new();
    msg_a.encode(&mut buf_a).unwrap();
    let mut buf_b = Vec::new();
    msg_b.encode(&mut buf_b).unwrap();
    assert_eq!(buf_a, buf_b);
}

#[cfg(test)]
mod tests {

    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use protobuf::test_messages::proto3::TestAllTypesProto3;

    #[test]
    fn test_all_types_proto3() {
        // Some selected encoded messages, mostly collected from failed fuzz runs.
        let msgs: &[&[u8]] = &[
            &[0x28, 0x28, 0x28, 0xFF, 0xFF, 0xFF, 0xFF, 0x68],
            &[0x92, 0x01, 0x00, 0x92, 0xF4, 0x01, 0x02, 0x00, 0x00],
            &[0x5d, 0xff, 0xff, 0xff, 0xff, 0x28, 0xff, 0xff, 0x21],
            &[0x98, 0x04, 0x02, 0x08, 0x0B, 0x98, 0x04, 0x02, 0x08, 0x02],
            // optional_int32: -1
            &[0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x08],
            // repeated_bool: [true, true]
            &[0xDA, 0x02, 0x02, 0x2A, 0x03],
            // oneof_double: nan
            &[0xb1, 0x07, 0xf6, 0x3d, 0xf5, 0xff, 0x27, 0x3d, 0xf5, 0xff],
            // optional_float: -0.0
            &[0xdd, 0x00, 0x00, 0x00, 0x00, 0x80],
            // optional_value: nan
            &[
                0xE2, 0x13, 0x1B, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
                0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0x08, 0xFF, 0x0E,
            ],
        ];

        for msg in msgs {
            roundtrip::<TestAllTypesProto3>(msg).unwrap();
        }
    }

    #[test]
    fn test_ident_conversions() {
        let msg = foo::bar_baz::FooBarBaz {
            foo_bar_baz: 42,
            fuzz_busters: vec![foo::bar_baz::foo_bar_baz::FuzzBuster {
                t: BTreeMap::<i32, foo::bar_baz::FooBarBaz>::new(),
                nested_self: None,
            }],
            p_i_e: 0,
        };

        // Test enum ident conversion.
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::Foo;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::Bar;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::FooBar;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::FuzzBuster;
        let _ = foo::bar_baz::foo_bar_baz::StrawberryRhubarbPie::NormalRustEnumCase;

        let mut buf = Vec::new();
        msg.encode(&mut buf).expect("encode");
        roundtrip::<foo::bar_baz::FooBarBaz>(&buf).unwrap();
    }

    #[test]
    fn test_custom_type_attributes() {
        // We abuse the ident conversion protobuf for the custom attribute additions. We placed
        // `Ord` on the FooBarBaz (which is not implemented by ordinary messages).
        let mut set1 = BTreeSet::new();
        let msg1 = foo::bar_baz::FooBarBaz::default();
        set1.insert(msg1);
        // Similar, but for oneof fields
        let mut set2 = BTreeSet::new();
        let msg2 = oneof_attributes::Msg::default();
        set2.insert(msg2.field);
    }

    #[test]
    fn test_nesting() {
        use crate::nesting::{A, B};
        let _ = A {
            a: Some(Box::new(A::default())),
            repeated_a: Vec::<A>::new(),
            map_a: BTreeMap::<i32, A>::new(),
            b: Some(Box::new(B::default())),
            repeated_b: Vec::<B>::new(),
            map_b: BTreeMap::<i32, B>::new(),
        };
    }

    #[test]
    fn test_recursive_oneof() {
        use crate::recursive_oneof::{a, A, B, C};
        let _ = A {
            kind: Some(a::Kind::B(Box::new(B {
                a: Some(Box::new(A {
                    kind: Some(a::Kind::C(C {})),
                })),
            }))),
        };
    }

    #[test]
    fn test_default_enum() {
        let msg = default_enum_value::Test::default();
        assert_eq!(msg.privacy_level_1(), default_enum_value::PrivacyLevel::One);
        assert_eq!(
            msg.privacy_level_3(),
            default_enum_value::PrivacyLevel::PrivacyLevelThree
        );
        assert_eq!(
            msg.privacy_level_4(),
            default_enum_value::PrivacyLevel::PrivacyLevelprivacyLevelFour
        );
    }

    #[test]
    fn test_group() {
        // optional group
        let msg1_bytes = &[0x0B, 0x10, 0x20, 0x0C];

        let mut msg1 = group_test::Test1::default();
        msg1.groupa = Some(group_test::test1::GroupA { i2: Some(32) });

        let mut bytes = Vec::new();
        msg1.encode(&mut bytes).unwrap();
        assert_eq!(&bytes, msg1_bytes);

        // skip group while decoding
        let data = &[0x0B,
                     0x30, 0x01, // unused int32
                     0x2B, 0x30, 0xFF, 0x01, 0x2C, // unused group
                     0x10, 0x20, // f3, 32
                     0x0C];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(data);
        assert_eq!(group_test::Test1::decode(&bytes), Ok(msg1));

        // repeated group
        let msg2_bytes = &[0x20, 0x40, 0x2B, 0x30, 0xFF, 0x01, 0x2C,
                           0x2B, 0x30, 0x01, 0x2C, 0x38, 0x64];

        let mut msg2 = group_test::Test2::default();
        msg2.i14 = Some(64);
        msg2.groupb.push(group_test::test2::GroupB { i16: Some(255) });
        msg2.groupb.push(group_test::test2::GroupB { i16: Some(1) });
        msg2.i17 = Some(100);

        let mut bytes = Vec::new();
        msg2.encode(&mut bytes).unwrap();
        assert_eq!(&bytes, msg2_bytes);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(msg2_bytes);
        assert_eq!(group_test::Test2::decode(&bytes), Ok(msg2));
    }
}
