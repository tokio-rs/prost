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
    return Ok(value);
}

/// Returns the encoded length of the value in LEB128 variable length format.
/// The returned value will be between 1 and 10, inclusive.
#[inline]
pub fn encoded_len_varint(value: u64) -> usize {
    if value < 1 <<  7 { 1 } else
    if value < 1 << 14 { 2 } else
    if value < 1 << 21 { 3 } else
    if value < 1 << 28 { 4 } else
    if value < 1 << 35 { 5 } else
    if value < 1 << 42 { 6 } else
    if value < 1 << 49 { 7 } else
    if value < 1 << 56 { 8 } else
    if value < 1 << 63 { 9 }
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
            _ => return Err(invalid_data(format!("unknown wire type value: {}", val))),
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

/// A type indicating that the default encoding is used for a field.
pub enum Plain {}
/// A type indicating that the integer field should use variable-width,
/// ZigZag, signed encoding.
pub enum Signed {}
/// A type indicating that the integer field should use fixed-width encoding.
pub enum Fixed {}
/// A type indicating that a repeated numeric field should use packed encoding.
pub enum Packed {}
