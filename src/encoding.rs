use std::cmp::min;
use std::default;
use std::io::Result;
use std::str;
use std::u32;
use std::usize;
use std::collections::HashMap;
use std::hash::Hash;

use bytes::{
    Buf,
    BufMut,
    LittleEndian,
};

use invalid_input;
use invalid_data;

/// Encodes an integer value into LEB128 variable length format, and writes it to the buffer.
/// The buffer must have enough remaining space (maximum 10 bytes).
#[inline]
pub fn encode_varint<B>(mut value: u64, buf: &mut B) where B: BufMut {
    let mut i = 0;
    'outer: loop {
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

pub fn encode_bool<B>(value: bool, buf: &mut B) where B: BufMut {
    buf.put_u8(if value { 1u8 } else { 0u8 });
}
pub fn decode_bool<B>(buf: &mut B) -> Result<bool> where B: Buf {
    if !buf.has_remaining() {
        return Err(invalid_input("failed to decode bool: buffer underflow"));
    }
    match buf.get_u8() {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid_data("failed to decode bool: invalid value")),
    }
}
pub fn encoded_len_bool(_value: bool) -> usize {
    1
}

pub fn encode_int32<B>(value: i32, buf: &mut B) where B: BufMut {
    encode_varint(value as u64, buf);
}
pub fn decode_int32<B>(buf: &mut B) -> Result<i32> where B: Buf {
    decode_varint(buf).map(|value| value as _)
}
pub fn encoded_len_int32(value: i32) -> usize {
    encoded_len_varint(value as u64)
}

pub fn encode_int64<B>(value: i64, buf: &mut B) where B: BufMut {
    encode_varint(value as u64, buf);
}
pub fn decode_int64<B>(buf: &mut B) -> Result<i64> where B: Buf {
    decode_varint(buf).map(|value| value as _)
}
pub fn encoded_len_int64(value: i64) -> usize {
    encoded_len_varint(value as u64)
}

pub fn encode_uint32<B>(value: u32, buf: &mut B) where B: BufMut {
    encode_varint(value as u64, buf);
}
pub fn decode_uint32<B>(buf: &mut B) -> Result<u32> where B: Buf {
    decode_varint(buf).map(|value| value as _)
}
pub fn encoded_len_uint32(value: u32) -> usize {
    encoded_len_varint(value as u64)
}

pub fn encode_uint64<B>(value: u64, buf: &mut B) where B: BufMut {
    encode_varint(value, buf);
}
pub fn decode_uint64<B>(buf: &mut B) -> Result<u64> where B: Buf {
    decode_varint(buf)
}
pub fn encoded_len_uint64(value: u64) -> usize {
    encoded_len_varint(value)
}

pub fn encode_float<B>(value: f32, buf: &mut B) where B: BufMut {
    buf.put_f32::<LittleEndian>(value);
}
pub fn decode_float<B>(buf: &mut B) -> Result<f32> where B: Buf {
    if buf.remaining() < 4 {
        return Err(invalid_input("failed to decode float: buffer underflow"));
    }
    Ok(buf.get_f32::<LittleEndian>())
}

pub fn encode_double<B>(value: f64, buf: &mut B) where B: BufMut {
    buf.put_f64::<LittleEndian>(value);
}
pub fn decode_double<B>(buf: &mut B) -> Result<f64> where B: Buf {
    if buf.remaining() < 8 {
        return Err(invalid_input("failed to decode double: buffer underflow"));
    }
    Ok(buf.get_f64::<LittleEndian>())
}

pub fn encode_sint32<B>(value: i32, buf: &mut B) where B: BufMut {
    encode_varint(((value << 1) ^ (value >> 31)) as u64, buf);
}
pub fn decode_sint32<B>(buf: &mut B) -> Result<i32> where B: Buf {
    decode_varint(buf).map(|value| {
        let value = value as i32;
        (value >> 1) ^ -(value & 1)
    })
}
pub fn encoded_len_sint32(value: i32) -> usize {
    encoded_len_varint(((value << 1) ^ (value >> 31)) as u64)
}

pub fn encode_sint64<B>(value: i64, buf: &mut B) where B: BufMut {
    encode_varint(((value << 1) ^ (value >> 63)) as u64, buf);
}
pub fn decode_sint64<B>(buf: &mut B) -> Result<i64> where B: Buf {
    decode_varint(buf).map(|value| {
        let value = value as i64;
        (value >> 1) ^ -(value & 1)
    })
}
pub fn encoded_len_sint64(value: i64) -> usize {
    encoded_len_varint(((value << 1) ^ (value >> 63)) as u64)
}

pub fn encode_fixed32<B>(value: u32, buf: &mut B) where B: BufMut {
    buf.put_u32::<LittleEndian>(value);
}
pub fn decode_fixed32<B>(buf: &mut B) -> Result<u32> where B: Buf {
    if buf.remaining() < 4 {
        return Err(invalid_input("failed to decode fixed32: buffer underflow"));
    }
    Ok(buf.get_u32::<LittleEndian>())
}

pub fn encode_fixed64<B>(value: u64, buf: &mut B) where B: BufMut {
    buf.put_u64::<LittleEndian>(value);
}
pub fn decode_fixed64<B>(buf: &mut B) -> Result<u64> where B: Buf {
    if buf.remaining() < 8 {
        return Err(invalid_input("failed to decode fixed64: buffer underflow"));
    }
    Ok(buf.get_u64::<LittleEndian>())
}

pub fn encode_sfixed32<B>(value: i32, buf: &mut B) where B: BufMut {
    buf.put_i32::<LittleEndian>(value);
}
pub fn decode_sfixed32<B>(buf: &mut B) -> Result<i32> where B: Buf {
    if buf.remaining() < 4 {
        return Err(invalid_input("failed to decode sfixed32: buffer underflow"));
    }
    Ok(buf.get_i32::<LittleEndian>())
}

pub fn encode_sfixed64<B>(value: i64, buf: &mut B) where B: BufMut {
    buf.put_i64::<LittleEndian>(value);
}
pub fn decode_sfixed64<B>(buf: &mut B) -> Result<i64> where B: Buf {
    if buf.remaining() < 8 {
        return Err(invalid_input("failed to decode sfixed64 field: buffer underflow"));
    }
    Ok(buf.get_i64::<LittleEndian>())
}

pub fn encode_bytes<B>(value: &[u8], buf: &mut B) where B: BufMut {
    encode_varint(value.len() as u64, buf);
    buf.put_slice(value);
}
pub fn merge_bytes<B>(value: &mut Vec<u8>, buf: &mut B) -> Result<()> where B: Buf {
    let len = decode_varint(buf)?;
    if (buf.remaining() as u64) < len {
        return Err(invalid_input("failed to decode bytes: buffer underflow"));
    }
    let len = len as usize;
    value.clear();
    value.extend_from_slice(&buf.bytes()[..len]);
    buf.advance(len);
    Ok(())
}

pub fn encode_string<B>(value: &str, buf: &mut B) where B: BufMut {
    encode_bytes(value.as_bytes(), buf);
}
pub fn merge_string<B>(value: &mut String, buf: &mut B) -> Result<()> where B: Buf {
    let len = decode_varint(buf)?;
    if (buf.remaining() as u64) < len {
        return Err(invalid_input("failed to decode string: buffer underflow"));
    }
    let len = len as usize;
    value.clear();
    value.push_str(str::from_utf8(&buf.bytes()[..len]).map_err(|_| {
        invalid_data("failed to decode string: data is not UTF-8 encoded")
    })?);
    buf.advance(len);
    Ok(())
}
