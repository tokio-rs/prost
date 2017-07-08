extern crate bytes;
extern crate prost;
#[macro_use] extern crate prost_derive;

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

use std::error::Error;
use std::io::Cursor;

use bytes::Buf;
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
pub fn roundtrip<M>(data: &[u8]) -> RoundtripResult where M: Message {
    // Try to decode a message from the data. If decoding fails, continue.
    let len = data.len();
    let all_types = match M::decode(&mut Buf::take(Cursor::new(data), len)) {
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

    let roundtrip = match M::decode(&mut Buf::take(Cursor::new(&buf1), encoded_len)) {
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
    eprintln!(" buf1: {:?}", buf.iter().map(|x| format!("0x{:x}", x)).collect::<Vec<_>>());
    eprintln!("a: {:?}\nb: {:?}", all_types, roundtrip);
    */

    if buf1 != buf2 {
        return RoundtripResult::Error("roundtripped encoded buffers do not match".into())
    }

    RoundtripResult::Ok(buf1)
}

#[cfg(test)]
mod tests {

    use protobuf_test_messages::proto3::TestAllTypes;
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

            // optional_value: nan
            &[0xe2,0x13,0x1b,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0x11,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x8,0xff,0xe]
        ];

        for msg in msgs {
            roundtrip::<TestAllTypes>(msg).unwrap();
        }
    }
}
