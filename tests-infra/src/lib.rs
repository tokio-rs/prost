#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use core::default::Default;
use core::cmp::PartialEq;
use core::fmt::Display;

#[macro_use]
extern crate cfg_if;

use prost;
use prost::Message;

cfg_if! {
    if #[cfg(feature = "edition-2015")] {
        extern crate bytes;
        extern crate prost;
        extern crate prost_types;
        extern crate protobuf;
        #[cfg(test)]
        extern crate prost_build;
        #[cfg(test)]
        extern crate tempfile;
        #[cfg(test)]
        extern crate tests_infra;
    }
}

pub enum RoundtripResult {
    /// The roundtrip succeeded.
    Ok(Vec<u8>),
    /// The data could not be decoded. This could indicate a bug in prost,
    /// or it could indicate that the input was bogus.
    DecodeError(prost::DecodeError),
    /// Re-encoding or validating the data failed.  This indicates a bug in `prost`.
    Error(String),
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
        return RoundtripResult::Error(to_string(&error));
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

    let roundtrip = match M::decode(&buf1[..]) {
        Ok(roundtrip) => roundtrip,
        Err(error) => return RoundtripResult::Error(to_string(&error)),
    };

    let mut buf2 = Vec::new();
    if let Err(error) = roundtrip.encode(&mut buf2) {
        return RoundtripResult::Error(to_string(&error));
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

    use bytes::Buf;
    let buf = (&buf[..]).to_bytes();
    let roundtrip = M::decode(buf).unwrap();

    // FIXME(chris)
    // if buf.has_remaining() {
    //    panic!("expected buffer to be empty: {}", buf.remaining());
    // }

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

// helper
fn to_string<T: Display>(obj: &T) -> String {
    format!("{}", obj)
}
