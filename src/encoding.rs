//! Utility functions and types for encoding and decoding Protobuf types.

use std::cmp::min;
use std::error;
use std::io::{
    Result,
    Error,
    ErrorKind,
};
use std::str;
use std::u32;
use std::usize;

use bytes::{
    Buf,
    BufMut,
    LittleEndian,
    Take,
};

/// Returns an invalid data IO error wrapping the provided cause.
///
/// This should be used primarily when decoding a Protobuf type fails.
#[inline]
pub fn invalid_data<E>(error: E) -> Error where E: Into<Box<error::Error + Send + Sync>> {
    Error::new(ErrorKind::InvalidData, error.into())
}

/// Returns an invalid input IO error wrapping the provided cause.
///
/// This should be used primarily when encoding a Protobuf type fails due to
/// insufficient output buffer space.
#[inline]
pub fn invalid_input<E>(error: E) -> Error where E: Into<Box<error::Error + Send + Sync>> {
    Error::new(ErrorKind::InvalidInput, error.into())
}

/// Encodes an integer value into LEB128 variable length format, and writes it to the buffer.
/// The buffer must have enough remaining space (maximum 10 bytes).
#[inline]
pub fn encode_varint<B>(mut value: u64, buf: &mut B) where B: BufMut {
    let mut i;
    'outer: loop {
        i = 0;

        // bytes_mut is unsafe because it may return an uninitialized slice.
        // This use is safe because the slice is only written to, not read from.
        for byte in unsafe { buf.bytes_mut() } {
            i += 1;
            if value < 0x80 {
                *byte = value as u8;
                break 'outer;
            } else {
                *byte = ((value & 0x7F) | 0x80) as u8;
                value >>= 7;
            }
        }

        unsafe { buf.advance_mut(i); }
        assert!(buf.has_remaining_mut());
    }

    // advance_mut is unsafe because it could cause uninitialized memory to be
    // advanced over. This use is safe since each byte which is advanced over
    // has been written to in the previous loop.
    unsafe { buf.advance_mut(i); }
}

/// Decodes a LEB128-encoded variable length integer from the buffer.
#[inline]
pub fn decode_varint<B>(buf: &mut B) -> Result<u64> where B: Buf {
    let mut value = 0;
    let mut i = 0;
    'outer: loop {
        let bytes = buf.bytes();
        let len = bytes.len();

        for &byte in &bytes[..min(len, 10 - i)] {
            value |= ((byte & 0x7F) as u64) << (i * 7);
            i += 1;
            if byte <= 0x7F {
                break 'outer;
            }
        }

        if i == 10 {
            return Err(invalid_data("failed to decode varint: integer overflow"));
        }
        if !buf.has_remaining() {
            return Err(invalid_data("failed to decode varint: buffer underflow"));
        }
    }
    buf.advance(i);
    Ok(value)
}

/// Returns the encoded length of the value in LEB128 variable length format.
/// The returned value will be between 1 and 10, inclusive.
#[inline]
pub fn encoded_len_varint(value: u64) -> usize {
         if value < 1 <<  7 { 1 }
    else if value < 1 << 14 { 2 }
    else if value < 1 << 21 { 3 }
    else if value < 1 << 28 { 4 }
    else if value < 1 << 35 { 5 }
    else if value < 1 << 42 { 6 }
    else if value < 1 << 49 { 7 }
    else if value < 1 << 56 { 8 }
    else if value < 1 << 63 { 9 }
    else { 10 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum WireType {
    Varint = 0,
    SixtyFourBit = 1,
    LengthDelimited = 2,
    ThirtyTwoBit = 5
}

pub const MIN_TAG: u32 = 1;
pub const MAX_TAG: u32 = (1 << 29) - 1;

impl WireType {
    // TODO: impl TryFrom<u8> when stable.
    #[inline]
    pub fn try_from(val: u8) -> Result<WireType> {
        match val {
            0 => Ok(WireType::Varint),
            1 => Ok(WireType::SixtyFourBit),
            2 => Ok(WireType::LengthDelimited),
            5 => Ok(WireType::ThirtyTwoBit),
            _ => Err(invalid_data(format!("unknown wire type value: {}", val))),
        }
    }
}

/// Encodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline]
pub fn encode_key<B>(tag: u32, wire_type: WireType, buf: &mut B) where B: BufMut {
    debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);
    let key = (tag << 3) | wire_type as u32;
    encode_varint(key as u64, buf);
}

/// Decodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline]
pub fn decode_key<B>(buf: &mut B) -> Result<(u32, WireType)> where B: Buf {
    let key = decode_varint(buf)?;
    if key > u32::MAX as u64 {
        return Err(invalid_data("failed to decode field key: u8 overflow"));
    }
    let wire_type = WireType::try_from(key as u8 & 0x07)?;
    let tag = key as u32 >> 3;
    Ok((tag, wire_type))
}

/// Returns the width of an encoded Protobuf field key with the given tag.
/// The returned width will be between 1 and 5 bytes (inclusive).
#[inline]
pub fn key_len(tag: u32) -> usize {
    encoded_len_varint((tag << 3) as u64)
}

/// Checks that the expected wire type matches the actual wire type,
/// or returns an error result.
#[inline]
pub fn check_wire_type(expected: WireType, actual: WireType) -> Result<()> {
    if expected != actual {
        return Err(invalid_data(format!("illegal wire type: {:?} (expected {:?})", actual, expected)));
    }
    Ok(())
}

#[inline]
pub fn skip_field<B>(wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
    match wire_type {
        WireType::Varint => {
            decode_varint(buf)?;
        },
        WireType::SixtyFourBit => {
            if buf.remaining() < 8 {
                return Err(invalid_input("failed to skip field: buffer underflow"));
            }
            buf.advance(8);
        },
        WireType::ThirtyTwoBit => {
            if buf.remaining() < 4 {
                return Err(invalid_input("failed to skip field: buffer underflow"));
            }
            buf.advance(4);
        },
        WireType::LengthDelimited => {
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(invalid_input("failed to skip field: buffer underflow"));
            }
            buf.advance(len as usize);
        },
    };
    Ok(())
}

#[inline]
pub fn encode_bool<B>(value: bool, buf: &mut B) where B: BufMut { buf.put_u8(if value { 1u8 } else { 0u8 }); }
#[inline]
pub fn decode_bool<B>(buf: &mut B) -> Result<bool> where B: Buf { decode_varint(buf).map(|value| value != 0) }

#[inline]
pub fn encode_int32<B>(value: i32, buf: &mut B) where B: BufMut { encode_varint(value as _, buf) }
#[inline]
pub fn decode_int32<B>(buf: &mut B) -> Result<i32> where B: Buf { decode_varint(buf).map(|value| value as _) }
#[inline]
pub fn encoded_len_int32(value: i32) -> usize { encoded_len_varint(value as _) }

#[inline]
pub fn encode_int64<B>(value: i64, buf: &mut B) where B: BufMut { encode_varint(value as _, buf) }
#[inline]
pub fn decode_int64<B>(buf: &mut B) -> Result<i64> where B: Buf { decode_varint(buf).map(|value| value as _) }
#[inline]
pub fn encoded_len_int64(value: i64) -> usize { encoded_len_varint(value as _) }

#[inline]
pub fn encode_uint32<B>(value: u32, buf: &mut B) where B: BufMut { encode_varint(value as _, buf) }
#[inline]
pub fn decode_uint32<B>(buf: &mut B) -> Result<u32> where B: Buf { decode_varint(buf).map(|value| value as _) }
#[inline]
pub fn encoded_len_uint32(value: u32) -> usize { encoded_len_varint(value as _) }

#[inline]
pub fn encode_uint64<B>(value: u64, buf: &mut B) where B: BufMut { encode_varint(value as _, buf) }
#[inline]
pub fn decode_uint64<B>(buf: &mut B) -> Result<u64> where B: Buf { decode_varint(buf).map(|value| value as _) }
#[inline]
pub fn encoded_len_uint64(value: u64) -> usize { encoded_len_varint(value as _) }

#[inline]
pub fn encode_float<B>(value: f32, buf: &mut B) where B: BufMut { buf.put_f32::<LittleEndian>(value) }
#[inline]
pub fn decode_float<B>(buf: &mut B) -> Result<f32> where B: Buf {
    if buf.remaining() < 4 {
        return Err(invalid_input("failed to decode float: buffer underflow"));
    }
    Ok(buf.get_f32::<LittleEndian>())
}

#[inline]
pub fn encode_double<B>(value: f64, buf: &mut B) where B: BufMut { buf.put_f64::<LittleEndian>(value) }
#[inline]
pub fn decode_double<B>(buf: &mut B) -> Result<f64> where B: Buf {
    if buf.remaining() < 8 {
        return Err(invalid_input("failed to decode double: buffer underflow"));
    }
    Ok(buf.get_f64::<LittleEndian>())
}

#[inline]
pub fn encode_sint32<B>(value: i32, buf: &mut B) where B: BufMut {
    encode_varint(((value << 1) ^ (value >> 31)) as u64, buf)
}
#[inline]
pub fn decode_sint32<B>(buf: &mut B) -> Result<i32> where B: Buf {
    decode_varint(buf).map(|value| {
        let value = value as u32;
        ((value >> 1) as i32) ^ (-((value & 1) as i32))
    })
}
#[inline]
pub fn encoded_len_sint32(value: i32) -> usize {
    encoded_len_varint(((value << 1) ^ (value >> 31)) as u64)
}

#[inline]
pub fn encode_sint64<B>(value: i64, buf: &mut B) where B: BufMut {
    encode_varint(((value << 1) ^ (value >> 63)) as u64, buf)
}
#[inline]
pub fn decode_sint64<B>(buf: &mut B) -> Result<i64> where B: Buf {
    decode_varint(buf).map(|value| {
        ((value >> 1) as i64) ^ (-((value & 1) as i64))
    })
}
#[inline]
pub fn encoded_len_sint64(value: i64) -> usize {
    encoded_len_varint(((value << 1) ^ (value >> 63)) as u64)
}

#[inline]
pub fn encode_fixed32<B>(value: u32, buf: &mut B) where B: BufMut { buf.put_u32::<LittleEndian>(value) }
#[inline]
pub fn decode_fixed32<B>(buf: &mut B) -> Result<u32> where B: Buf {
    if buf.remaining() < 4 {
        return Err(invalid_input("failed to decode fixed32: buffer underflow"));
    }
    Ok(buf.get_u32::<LittleEndian>())
}

#[inline]
pub fn encode_fixed64<B>(value: u64, buf: &mut B) where B: BufMut { buf.put_u64::<LittleEndian>(value) }
#[inline]
pub fn decode_fixed64<B>(buf: &mut B) -> Result<u64> where B: Buf {
    if buf.remaining() < 8 {
        return Err(invalid_input("failed to decode fixed64: buffer underflow"));
    }
    Ok(buf.get_u64::<LittleEndian>())
}

#[inline]
pub fn encode_sfixed32<B>(value: i32, buf: &mut B) where B: BufMut { buf.put_i32::<LittleEndian>(value) }
#[inline]
pub fn decode_sfixed32<B>(buf: &mut B) -> Result<i32> where B: Buf {
    if buf.remaining() < 4 {
        return Err(invalid_input("failed to decode sfixed32: buffer underflow"));
    }
    Ok(buf.get_i32::<LittleEndian>())
}

#[inline]
pub fn encode_sfixed64<B>(value: i64, buf: &mut B) where B: BufMut { buf.put_i64::<LittleEndian>(value); }
#[inline]
pub fn decode_sfixed64<B>(buf: &mut B) -> Result<i64> where B: Buf {
    if buf.remaining() < 8 {
        return Err(invalid_input("failed to decode sfixed64 field: buffer underflow"));
    }
    Ok(buf.get_i64::<LittleEndian>())
}

#[inline]
pub fn encode_string<B>(value: &str, buf: &mut B) where B: BufMut {
    buf.put_slice(value.as_bytes());
}
#[inline]
pub fn merge_string<B>(value: &mut String, buf: &mut Take<B>) -> Result<()> where B: Buf {
    unsafe {
        // String::as_mut_vec is unsafe because it doesn't check that the bytes
        // inserted into it the resulting vec are valid UTF-8. We check
        // explicitly in order to ensure this is safe.
        merge_bytes(value.as_mut_vec(), buf)?;
        str::from_utf8(value.as_bytes()).map_err(|_| {
            invalid_data("failed to decode string: data is not UTF-8 encoded")
        })?;
    }
    Ok(())
}

#[inline]
pub fn encode_bytes<B>(value: &[u8], buf: &mut B) where B: BufMut {
    buf.put_slice(value);
}
#[inline]
pub fn merge_bytes<B>(value: &mut Vec<u8>, buf: &mut Take<B>) -> Result<()> where B: Buf {
    let len = decode_varint(buf)?;
    if (buf.remaining() as u64) < len {
        return Err(invalid_input("failed to decode length-delimited field: buffer underflow"));
    }
    let limit = buf.limit();
    buf.set_limit(len as usize);
    value.clear();
    value.reserve_exact(len as usize);
    while buf.has_remaining() {
        let len = {
            let bytes = buf.bytes();
            value.extend_from_slice(bytes);
            bytes.len()
        };
        buf.advance(len);
    }
    buf.set_limit(limit - len as usize);
    Ok(())
}
