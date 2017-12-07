extern crate bytes;
extern crate prost;
extern crate prost_types;

#[macro_use] extern crate prost_derive;

#[cfg(test)] extern crate tempdir;
#[cfg(test)] extern crate prost_build;

pub mod packages;
pub mod unittest;

#[cfg(test)] mod bootstrap;
#[cfg(test)] mod debug;
#[cfg(test)] mod message_encoding;

pub mod protobuf_test_messages {
    pub mod proto2 {
        include!(concat!(env!("OUT_DIR"), "/protobuf_test_messages.proto2.rs"));
    }
    pub mod proto3 {
        include!(concat!(env!("OUT_DIR"), "/protobuf_test_messages.proto3.rs"));
    }
}

pub mod google {
    pub mod protobuf {
        include!(concat!(env!("OUT_DIR"), "/google.protobuf.rs"));
    }
}

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
    Error(Box<Error + Send + Sync>),
}

impl RoundtripResult {
    /// Unwrap the roundtrip result.
    pub fn unwrap(self) -> Vec<u8> {
        match self {
            RoundtripResult::Ok(buf) => buf,
            RoundtripResult::DecodeError(error) => panic!("failed to decode the roundtrip data: {}", error),
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
pub fn roundtrip<M>(data: &[u8]) -> RoundtripResult where M: Message + Default {
    // Try to decode a message from the data. If decoding fails, continue.
    let all_types = match M::decode(data) {
        Ok(all_types) => all_types,
        Err(error) => return RoundtripResult::DecodeError(error),
    };

    let encoded_len = all_types.encoded_len();

    // TODO: Reenable this once sign-extension in negative int32s is figured out.
    //assert!(encoded_len <= len, "encoded_len: {}, len: {}, all_types: {:?}",
                                //encoded_len, len, all_types);

    let mut buf1 = Vec::new();
    if let Err(error) = all_types.encode(&mut buf1) {
        return RoundtripResult::Error(error.into());
    }
    if encoded_len != buf1.len() {
        return RoundtripResult::Error(
            format!("expected encoded len ({}) did not match actual encoded len ({})",
                    encoded_len, buf1.len()).into());
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
        return RoundtripResult::Error("roundtripped encoded buffers do not match".into())
    }

    RoundtripResult::Ok(buf1)
}

/// Generic rountrip serialization check for messages.
pub fn check_message<M>(msg: &M) where M: Message + Default + PartialEq {
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

#[cfg(test)]
mod tests {

    use std::collections::BTreeMap;

    use protobuf_test_messages::proto3::TestAllTypesProto3;
    use super::*;

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
            &[0xE2, 0x13, 0x1B, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
              0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
              0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x08, 0xFF, 0x0E],
        ];

        for msg in msgs {
            roundtrip::<TestAllTypesProto3>(msg).unwrap();
        }
    }

    #[test]
    fn test_ident_conversions() {
        let msg = foo::bar_baz::FooBarBaz {
            foo_bar_baz: 42,
            fuzz_busters: vec![
                foo::bar_baz::foo_bar_baz::FuzzBuster {
                    t: BTreeMap::<i32, foo::bar_baz::FooBarBaz>::new(),
                    nested_self: None,
                },
            ],
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
    fn test_custom_container_attributes() {
        // We abuse the ident conversion protobuf for the custom attribute additions. We placed
        // `Eq` on the FooBarBaz (which is not implemented by ordinary messages).
        let msg1 = foo::bar_baz::FooBarBaz::default();
        let msg2 = foo::bar_baz::FooBarBaz::default();
        // This uses Eq, which wouldn't compile if the attribute didn't work
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_custom_field_attributes() {
        let input = include_str!(concat!(env!("OUT_DIR"), "/foo.bar_baz.rs"));
        assert!(input.contains("// Testing comment"));
    }

    #[test]
    fn test_nesting() {
        use nesting::{A, B};
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
        use recursive_oneof::{a, A, B, C};
        let _ = A {
            kind: Some(a::Kind::B(Box::new(B {
                a: Some(Box::new(A {
                    kind: Some(a::Kind::C(C {}))
                }))
            })))
        };
    }
}
