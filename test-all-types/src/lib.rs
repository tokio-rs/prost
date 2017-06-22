#![feature(float_bits_conv)]

extern crate bytes;
extern crate proto;
#[macro_use] extern crate proto_derive;

pub mod protobuf_test_messages {
    #[allow(non_snake_case)]
    pub mod proto3 {
        include!(concat!(env!("OUT_DIR"), "/proto3.rs"));
    }
}

pub mod google {
    pub mod protobuf {
        include!(concat!(env!("OUT_DIR"), "/protobuf.rs"));
    }
}

use std::collections::HashMap;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Result,
};

use bytes::Buf;
use proto::Message;

use protobuf_test_messages::proto3;

pub enum RoundtripResult {
    /// The roundtrip succeeded.
    Ok(Vec<u8>),
    /// The data could not be decoded. This could indicate a bug in the 'proto'
    /// library, or it could indicate that the input was bogus.
    DecodeError(Error),
    /// Re-encoding or validating the data failed.  This indicates a bug in the
    /// 'proto' library.
    Error(Error),
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
    pub fn unwrap_error(self) -> Result<Vec<u8>> {
        match self {
            RoundtripResult::Ok(buf) => Ok(buf),
            RoundtripResult::DecodeError(error) => Err(error),
            RoundtripResult::Error(error) => panic!("failed roundtrip: {}", error),
        }
    }
}

/// Tests round-tripping a proto3 `TestAllTypes` message.
pub fn test_all_types_proto3_roundtrip(data: &[u8]) -> RoundtripResult {
    // Try to decode a message from the data. If decoding fails, continue.
    let len = data.len();
    let all_types = match proto3::TestAllTypes::decode(&mut Buf::take(Cursor::new(data), len)) {
        Ok(all_types) => all_types,
        Err(error) => return RoundtripResult::DecodeError(error),
    };
    let encoded_len = all_types.encoded_len();

    // TODO: Reenable this once sign-extension in negative int32s is figured out.
    //assert!(encoded_len <= len, "encoded_len: {}, len: {}, all_types: {:?}",
                                //encoded_len, len, all_types);

    let mut buf = Vec::new();
    if let Err(error) = all_types.encode(&mut buf) {
        return RoundtripResult::Error(error);
    }
    assert_eq!(encoded_len, buf.len());

    let roundtrip = match proto3::TestAllTypes::decode(&mut Buf::take(Cursor::new(&buf), encoded_len)) {
        Ok(roundtrip) => roundtrip,
        Err(error) => return RoundtripResult::Error(error),
    };

    /*
    // Useful for debugging:
    eprintln!(" data: {:?}", data.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!(" buf: {:?}", buf.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!("a: {:?}\nb: {:?}", all_types, roundtrip);
    */

    if !all_types_proto3_eq(all_types, roundtrip) {
        return RoundtripResult::Error(Error::new(ErrorKind::Other,
                                                 "roundtrip value does not equal original"));
    }

    RoundtripResult::Ok(buf)
}

/// Test that a pair of `TestAllTypes` messages are bit-for-bit equivalent.
fn all_types_proto3_eq(mut a: proto3::TestAllTypes,
                       mut b: proto3::TestAllTypes) -> bool {
    use proto3::test_all_types::OneofField::*;


    // First, check that all floating point fields are bit-for-bit equivalent.
    fn float_eq(a: f32, b: f32) -> bool {
        a == b || a.to_bits() == b.to_bits()
    }
    fn double_eq(a: f64, b: f64) -> bool {
        a == b || a.to_bits() == b.to_bits()
    }

    // optional_[float,double]
    if !float_eq(a.optional_float, b.optional_float) { return false; }
    a.optional_float = 0.0;
    b.optional_float = 0.0;
    if !double_eq(a.optional_double, b.optional_double) { return false; }
    a.optional_double = 0.0;
    b.optional_double = 0.0;

    // repeated_[float,double]
    if a.repeated_float.len() != b.repeated_float.len() ||
       a.repeated_float.iter().zip(b.repeated_float.iter()).any(|(&a, &b)| !float_eq(a, b)) {
        return false;
    }
    a.repeated_float.clear();
    b.repeated_float.clear();
    if a.repeated_double.len() != b.repeated_double.len() ||
       a.repeated_double.iter().zip(b.repeated_double.iter()).any(|(&a, &b)| !double_eq(a, b)) {
        return false;
    }
    a.repeated_double.clear();
    b.repeated_double.clear();

    // map_int32_[float,double]
    let mut a_entries = a.map_int32_float.into_iter().collect::<Vec<_>>();
    let mut b_entries = b.map_int32_float.into_iter().collect::<Vec<_>>();
    a_entries.sort_by_key(|&(k, _)| k);
    b_entries.sort_by_key(|&(k, _)| k);
    if a_entries.len() != b_entries.len() ||
        a_entries.iter().zip(b_entries.iter()).any(|(&(ak, av), &(bk, bv))| ak != bk || !float_eq(av, bv)) {
        return false;
    }
    a.map_int32_float = HashMap::new();
    b.map_int32_float = HashMap::new();
    let mut a_entries = a.map_int32_double.into_iter().collect::<Vec<_>>();
    let mut b_entries = b.map_int32_double.into_iter().collect::<Vec<_>>();
    a_entries.sort_by_key(|&(k, _)| k);
    b_entries.sort_by_key(|&(k, _)| k);
    if a_entries.len() != b_entries.len() ||
        a_entries.iter().zip(b_entries.iter()).any(|(&(ak, av), &(bk, bv))| ak != bk || !double_eq(av, bv)) {
        return false;
    }
    a.map_int32_double = HashMap::new();
    b.map_int32_double = HashMap::new();

    // optional_[float,double]_wrapper
    if match (a.optional_float_wrapper.take(), b.optional_float_wrapper.take()) {
        (Some(a), Some(b)) => !float_eq(a.value, b.value),
        (None, None) => false,
        _ => true,
    } { return false; }
    if match (a.optional_double_wrapper.take(), b.optional_double_wrapper.take()) {
        (Some(a), Some(b)) => !double_eq(a.value, b.value),
        (None, None) => false,
        _ => true,
    } { return false; }

    // repeated_[float,double]_wrapper
    if a.repeated_float_wrapper.len() != b.repeated_float_wrapper.len() ||
       a.repeated_float_wrapper.iter().zip(b.repeated_float_wrapper.iter()).any(|(a, b)| !float_eq(a.value, b.value)) {
        return false;
    }
    a.repeated_float_wrapper.clear();
    b.repeated_float_wrapper.clear();
    if a.repeated_double_wrapper.len() != b.repeated_double_wrapper.len() ||
       a.repeated_double_wrapper.iter().zip(b.repeated_double_wrapper.iter()).any(|(a, b)| !double_eq(a.value, b.value)) {
        return false;
    }
    a.repeated_double_wrapper.clear();
    b.repeated_double_wrapper.clear();

    // oneof_[float,double]
    if let (&Some(OneofFloat(av)), &Some(OneofFloat(bv))) = (&a.oneof_field, &b.oneof_field) {
        if !float_eq(av, bv) {
            return false;
        }
        a.oneof_field = None;
        b.oneof_field = None;
    }
    if let (&Some(OneofDouble(av)), &Some(OneofDouble(bv))) = (&a.oneof_field, &b.oneof_field) {
        if !double_eq(av, bv) {
            return false;
        }
        a.oneof_field = None;
        b.oneof_field = None;
    }

    // Next, compare all (co)recursive nested fields.

    fn optional_all_types_proto3_eq(a: Option<Box<proto3::TestAllTypes>>,
                                    b: Option<Box<proto3::TestAllTypes>>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => all_types_proto3_eq(*a, *b),
            (None, None) => true,
            _ => false,
        }
    }

    // recursive_message
    if !optional_all_types_proto3_eq(a.recursive_message.take(), b.recursive_message.take()) {
        return false;
    }

    // optional_nested_message
    if !optional_all_types_proto3_eq(a.optional_nested_message.take().and_then(|mut a| a.corecursive.take()),
                                     b.optional_nested_message.take().and_then(|mut b| b.corecursive.take())) {
        return false;
    }

    // repeated_nested_message
    if a.repeated_nested_message.len() != b.repeated_nested_message.len() ||
       a.repeated_nested_message.iter_mut().map(|msg| msg.corecursive.take())
        .zip(b.repeated_nested_message.iter_mut().map(|msg| msg.corecursive.take()))
        .any(|(a, b)| !optional_all_types_proto3_eq(a, b)) {
        return false;
    }

    // map_string_nested_message
    let mut a_entries = a.map_string_nested_message.iter_mut().map(|(k, ref mut v)| (k.clone(), v.corecursive.take())).collect::<Vec<_>>();
    let mut b_entries = b.map_string_nested_message.iter_mut().map(|(k, ref mut v)| (k.clone(), v.corecursive.take())).collect::<Vec<_>>();
    a_entries.sort_by_key(|&(ref k, _)| k.clone());
    b_entries.sort_by_key(|&(ref k, _)| k.clone());
    if a_entries.len() != b_entries.len() ||
       a_entries.into_iter().zip(b_entries.into_iter()).any(|((ak, av), (bk, bv))| ak != bk || !optional_all_types_proto3_eq(av, bv)) {
        return false;
    }

    // oneof_nested_message
    fn get_nested_message(oneof: &mut Option<proto3::test_all_types::OneofField>) -> Option<Box<proto3::TestAllTypes>> {
        match *oneof {
            Some(OneofNestedMessage(ref mut msg)) => msg.corecursive.take(),
            _ => None,
        }
    }
    if !optional_all_types_proto3_eq(get_nested_message(&mut a.oneof_field),
                                     get_nested_message(&mut b.oneof_field)) {
        return false;
    }

    (a == b)
}

#[cfg(test)]
mod tests {

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
            &[0xb1,0x7,0xf6,0x3d,0xf5,0xff,0x27,0x3d,0xf5,0xff],

            // optional_float: -0.0
            &[0xdd,0x0,0x0,0x0,0x0,0x80],
        ];

        for msg in msgs {
            test_all_types_proto3_roundtrip(msg).unwrap();
        }
    }
}
