//! Protobuf scalar value encoding and decoding
//!
//! | .proto Type | Rust Type |
//! | ----------- | --------- |
//! | double      | f64       |
//! | float       | f32       |
//! | int32       | i32       |
//! | int64       | i64       |
//! | uint32      | u32       |
//! | uint64      | u64       |
//! | sint32      | i32       |
//! | sint64      | i64       |
//! | fixed32     | u32       |
//! | fixed64     | u64       |
//! | sfixed32    | i32       |
//! | sfixed64    | i64       |
//! | bool        | bool      |
//! | string      | &str      |
//! | bytes       | &[u8]     |

use std::cmp::min;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};
use std::u32;
use std::usize;

use byteorder::{
    LittleEndian,
    ReadBytesExt,
    WriteBytesExt,
};

/// Encodes a `double` value.
#[inline]
pub fn write_double<W>(w: &mut W, value: f64) -> Result<()> where W: Write {
    w.write_f64::<LittleEndian>(value)
}

/// Decodes a `double` value.
#[inline]
pub fn read_double<R>(r: &mut R) -> Result<f64> where R: Read {
    r.read_f64::<LittleEndian>()
}

/// Encodes a `float` value.
#[inline]
pub fn write_float<W>(w: &mut W, value: f32) -> Result<()> where W: Write {
    w.write_f32::<LittleEndian>(value)
}

/// Decodes a `float` value.
#[inline]
pub fn read_float<R>(r: &mut R) -> Result<f32> where R: Read {
    r.read_f32::<LittleEndian>()
}

/// Encodes an `int32` value (varint).
#[inline]
pub fn write_int32<W>(w: &mut W, value: i32) -> Result<()> where W: Write {
    write_uint32(w, value as u32)
}

/// Decodes an `int32` value (varint).
#[inline]
pub fn read_int32<R>(r: &mut R) -> Result<i32> where R: Read {
    read_uint32(r).map(|value| value as i32)
}

/// Encodes an `int64` value (varint).
#[inline]
pub fn write_int64<W>(w: &mut W, value: i64) -> Result<()> where W: Write {
    write_uint64(w, value as u64)
}

/// Decodes an `int64` value (varint).
#[inline]
pub fn read_int64<R>(r: &mut R) -> Result<i64> where R: Read {
    read_uint64(r).map(|value| value as i64)
}

/// Encodes a `uint32` value (varint).
#[inline]
pub fn write_uint32<W>(w: &mut W, value: u32) -> Result<()> where W: Write {
    write_uint64(w, value as u64)
}

/// Decodes a `uint32` value (varint).
#[inline]
pub fn read_uint32<R>(r: &mut R) -> Result<u32> where R: Read {
    read_uint64(r).and_then(|value| {
        if value > u32::MAX as u64 {
            Err(Error::new(ErrorKind::InvalidData, "uint32 overflow"))
        } else {
            Ok(value as u32)
        }
    })
}

/// Encodes a `uint64` value (varint).
#[inline]
pub fn write_uint64<W>(w: &mut W, mut value: u64) -> Result<()> where W: Write {
    let mut buf = &mut [0u8; 10];
    let mut i = 0;
    while (value & !0x7F) > 0 {
        buf[i] = ((value & 0x7F) | 0x80) as u8;
        value >>= 7;
        i += 1;
    }
    buf[i] = value as u8;
    w.write_all(&buf[..i+1])
}

/// Decodes a `uint64` value (varint).
#[inline]
pub fn read_uint64<R>(r: &mut R) -> Result<u64> where R: Read {
    let mut acc: u64 = 0;
    let buf = &mut [0u8; 1];
    for i in 0..10 {
        r.read_exact(buf)?;
        let b = buf[0];
        acc |= ((b & 0x7F) as u64) << (i * 7);
        if b <= 0x7F {
            return Ok(acc);
        }
    }
    Err(Error::new(ErrorKind::InvalidData, "uint64 overflow"))
}

/// Encodes an `int32` value (varint + zig zag).
#[inline]
pub fn write_sint32<W>(w: &mut W, value: i32) -> Result<()> where W: Write {
    write_int32(w, (value << 1) ^ (value >> 31))
}

/// Decodes an `int32` value (varint + zig zag).
#[inline]
pub fn read_sint32<R>(r: &mut R) -> Result<i32> where R: Read {
    read_int32(r).map(|value| (value >> 1) ^ (-(value & 1)))
}

/// Encodes an `int64` value (varint + zig zag).
#[inline]
pub fn write_sint64<W>(w: &mut W, value: i64) -> Result<()> where W: Write {
    write_int64(w, (value << 1) ^ (value >> 63))
}

/// Decodes an `int64` value (varint + zig zag).
#[inline]
pub fn read_sint64<R>(r: &mut R) -> Result<i64> where R: Read {
    read_int64(r).map(|value| (value >> 1) ^ (-(value & 1)))
}

/// Encodes a `uint32` value (fixed-width).
#[inline]
pub fn write_fixed32<W>(w: &mut W, value: u32) -> Result<()> where W: Write {
    w.write_u32::<LittleEndian>(value)
}

/// Decodes a `uint32` value (fixed-width).
#[inline]
pub fn read_fixed32<R>(r: &mut R) -> Result<u32> where R: Read {
    r.read_u32::<LittleEndian>()
}

/// Encodes a `uint64` value (fixed-width).
#[inline]
pub fn write_fixed64<W>(w: &mut W, value: u64) -> Result<()> where W: Write {
    w.write_u64::<LittleEndian>(value)
}

/// Decodes a `uint64` value (fixed-width).
#[inline]
pub fn read_fixed64<R>(r: &mut R) -> Result<u64> where R: Read {
    r.read_u64::<LittleEndian>()
}

/// Encodes an `int32` value (fixed-width).
#[inline]
pub fn write_sfixed32<W>(w: &mut W, value: i32) -> Result<()> where W: Write {
    w.write_i32::<LittleEndian>(value)
}

/// Decodes an `int32` value (fixed-width).
#[inline]
pub fn read_sfixed32<R>(r: &mut R) -> Result<i32> where R: Read {
    r.read_i32::<LittleEndian>()
}

/// Encodes an `int64` value (fixed-width).
#[inline]
pub fn write_sfixed64<W>(w: &mut W, value: i64) -> Result<()> where W: Write {
    w.write_i64::<LittleEndian>(value)
}

/// Decodes an `int64` value (fixed-width).
#[inline]
pub fn read_sfixed64<R>(r: &mut R) -> Result<i64> where R: Read {
    r.read_i64::<LittleEndian>()
}

/// Encodes a `bool` value.
#[inline]
pub fn write_bool<W>(w: &mut W, value: bool) -> Result<()> where W: Write {
    let buf = if value { [1u8] } else { [0u8] };
    w.write_all(&buf)
}

/// Encodes a `bool` value.
#[inline]
pub fn read_bool<R>(r: &mut R) -> Result<bool> where R: Read {
    let buf = &mut [0u8];
    r.read_exact(buf)?;
    match buf[0] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::new(ErrorKind::InvalidData, "invalid bool value")),
    }
}

/// Encodes a `string` value.
#[inline]
pub fn write_string<W>(w: &mut W, value: &str) -> Result<()> where W: Write {
    write_bytes(w, value.as_bytes())
}

/// Decodes a `string` value.
#[inline]
pub fn read_string<R>(r: &mut R) -> Result<String> where R: Read {
    use std::error::Error as StdError;
    read_bytes(r).and_then(|bytes| {
        String::from_utf8(bytes).map_err(|error| Error::new(ErrorKind::InvalidData,
                                                            error.description()))
    })
}

/// Encodes a `bytes` value.
#[inline]
pub fn write_bytes<W>(w: &mut W, value: &[u8]) -> Result<()> where W: Write {
    write_uint64(w, value.len() as u64)?;
    w.write_all(value)
}

/// Decodes a `bytes` value.
#[inline]
pub fn read_bytes<R>(r: &mut R) -> Result<Vec<u8>> where R: Read {
    let len = read_uint64(r)?;

    if len > usize::MAX as u64 {
        return Err(Error::new(ErrorKind::InvalidData, "length overflows usize"));
    }
    let len = len as usize;

    // Cap at 4KiB to avoid over-allocating when the length field is bogus.
    let mut value = Vec::with_capacity(min(4096, len));
    let read_len = r.take(len as u64).read_to_end(&mut value)?;

    if read_len == len {
        Ok(value)
    } else {
        Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire string"))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use std::fmt::Debug;
    use std::io::{
        Cursor,
        Result,
    };

    use quickcheck::TestResult;

    fn check_roundtrip<E, D, T>(mut encode: E, mut decode: D, value: T) -> TestResult
    where E: FnMut(&mut Vec<u8>, T) -> Result<()>,
          D: FnMut(&mut Cursor<Vec<u8>>) -> Result<T>,
          T: Clone + Debug + PartialEq {
        let mut buf = Vec::new();
        if let Err(error) = encode(&mut buf, value.clone()) {
            return TestResult::error(format!("{:?}", error));
        };

        let roundtrip_value = match decode(&mut Cursor::new(buf)) {
            Ok(value) => value,
            Err(error) => return TestResult::error(format!("{:?}", error)),
        };
        if value == roundtrip_value {
            TestResult::passed()
        } else {
            TestResult::failed()
        }
    }


    quickcheck! {
        fn double_roundtrip(value: f64) -> TestResult {
            check_roundtrip(write_double, read_double, value)
        }
        fn float_roundtrip(value: f32) -> TestResult {
            check_roundtrip(write_float, read_float, value)
        }
        fn int32_roundtrip(value: i32) -> TestResult {
            check_roundtrip(write_int32, read_int32, value)
        }
        fn int64_roundtrip(value: i64) -> TestResult {
            check_roundtrip(write_int64, read_int64, value)
        }
        fn uint32_roundtrip(value: u32) -> TestResult {
            check_roundtrip(write_uint32, read_uint32, value)
        }
        fn uint64_roundtrip(value: u64) -> TestResult {
            check_roundtrip(write_uint64, read_uint64, value)
        }
        fn sint32_roundtrip(value: i32) -> TestResult {
            check_roundtrip(write_sint32, read_sint32, value)
        }
        fn sint64_roundtrip(value: i64) -> TestResult {
            check_roundtrip(write_sint64, read_sint64, value)
        }
        fn fixed32_roundtrip(value: u32 ) -> TestResult {
            check_roundtrip(write_fixed32, read_fixed32, value)
        }
        fn fixed64_roundtrip(value: u64 ) -> TestResult {
            check_roundtrip(write_fixed64, read_fixed64, value)
        }
        fn sfixed32_roundtrip(value: i32) -> TestResult {
            check_roundtrip(write_sfixed32, read_sfixed32, value)
        }
        fn sfixed64_roundtrip(value: i64) -> TestResult {
            check_roundtrip(write_sfixed64, read_sfixed64, value)
        }
        fn bool_roundtrip(value: bool) -> TestResult {
            check_roundtrip(write_bool, read_bool, value)
        }
        fn string_roundtrip(value: String) -> TestResult {
            let mut buf = Vec::new();
            if let Err(error) = write_string(&mut buf, &value) {
                return TestResult::error(format!("{:?}", error));
            };

            let roundtrip_value = match read_string(&mut Cursor::new(buf)) {
                Ok(value) => value,
                Err(error) => return TestResult::error(format!("{:?}", error)),
            };
            if value == roundtrip_value {
                TestResult::passed()
            } else {
                TestResult::failed()
            }
        }
        fn bytes_roundtrip(value: Vec<u8>) -> TestResult {
            let mut buf = Vec::new();
            if let Err(error) = write_bytes(&mut buf, &value) {
                return TestResult::error(format!("{:?}", error));
            };

            let roundtrip_value = match read_bytes(&mut Cursor::new(buf)) {
                Ok(value) => value,
                Err(error) => return TestResult::error(format!("{:?}", error)),
            };
            if value == roundtrip_value {
                TestResult::passed()
            } else {
                TestResult::failed()
            }
        }
    }


    #[test]
    fn test_varint() {
        fn check(value: u64, encoded: &[u8]) {
            let mut buf = Vec::new();
            write_uint64(&mut buf, value.clone()).expect("encoding failed");

            assert_eq!(&buf[..], encoded);

            let roundtrip_value = read_uint64(&mut Cursor::new(buf)).expect("decoding failed");
            assert_eq!(value, roundtrip_value);
        }

        check(0, &[0b0000_0000]);
        check(1, &[0b0000_0001]);

        check(127, &[0b0111_1111]);
        check(128, &[0b1000_0000, 0b0000_0001]);

        check(300, &[0b1010_1100, 0b0000_0010]);

        check(16_383, &[0b1111_1111, 0b0111_1111]);
        check(16_384, &[0b1000_0000, 0b1000_0000, 0b0000_0001]);
    }
}
