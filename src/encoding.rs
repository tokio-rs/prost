//! Utility functions and types for encoding and decoding Protobuf types.

use std::cmp::min;
use std::collections::HashMap;
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

use Message;

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
            return Err(invalid_data("varint overflow"));
        }
        if !buf.has_remaining() {
            return Err(invalid_data("buffer underflow"));
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
            _ => Err(invalid_data(format!("invalid wire type value: {}", val))),
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
        return Err(invalid_data("failed to decode field key: u32 overflow"));
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
        return Err(invalid_data(format!("invalid wire type: {:?} (expected {:?})", actual, expected)));
    }
    Ok(())
}

#[inline]
pub fn skip_field<B>(wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
    match wire_type {
        WireType::Varint => {
            decode_varint(buf).map_err(|error| {
                Error::new(error.kind(), format!("failed to skip varint field: {}", error))
            })?;
        },
        WireType::SixtyFourBit => {
            if buf.remaining() < 8 {
                return Err(invalid_data("failed to skip 64-bit field: buffer underflow"));
            }
            buf.advance(8);
        },
        WireType::ThirtyTwoBit => {
            if buf.remaining() < 4 {
                return Err(invalid_data("failed to skip 32-bit field: buffer underflow"));
            }
            buf.advance(4);
        },
        WireType::LengthDelimited => {
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(invalid_data("failed to skip length delimited field: buffer underflow"));
            }
            buf.advance(len as usize);
        },
    };
    Ok(())
}

macro_rules! encode_repeated {
    ($ty:ty, $encode:ident, $encode_repeated:ident) => (
         #[inline]
         pub fn $encode_repeated<B>(tag: u32, values: &Vec<$ty>, buf: &mut B) where B: BufMut {
             for value in values {
                 $encode(tag, value, buf);
             }
         }
    )
}

macro_rules! merge_repeated_numeric {
    ($ty:ty,
     $wire_type:expr,
     $merge:ident,
     $merge_repeated:ident) => (
        #[inline]
        pub fn $merge_repeated<B>(wire_type: WireType, values: &mut Vec<$ty>, buf: &mut Take<B>) -> Result<()> where B: Buf {
            if wire_type == WireType::LengthDelimited {
                let len = decode_varint(buf)?;
                if len > buf.remaining() as u64 {
                    return Err(invalid_data("buffer underflow"));
                }
                let len = len as usize;
                let limit = buf.limit();
                buf.set_limit(len);

                while buf.has_remaining() {
                let mut value = Default::default();
                $merge(wire_type, &mut value, buf)?;
                values.push(value);
                }
                buf.set_limit(limit - len);
            } else {
                check_wire_type($wire_type, wire_type)?;
                let mut value = Default::default();
                $merge(wire_type, &mut value, buf)?;
                values.push(value);
            }
            Ok(())
        }
    )
}

macro_rules! varint {
    ($ty:ty,
     $encode:ident,
     $merge:ident,
     $encode_repeated:ident,
     $merge_repeated:ident,
     $encode_packed:ident,
     $encoded_len:ident,
     $encoded_len_repeated:ident,
     $encoded_len_packed:ident) => (
        varint!($ty,
                $encode,
                $merge,
                $encode_repeated,
                $merge_repeated,
                $encode_packed,
                $encoded_len,
                $encoded_len_repeated,
                $encoded_len_packed,
                to_uint64(value) { *value as u64 },
                from_uint64(value) { value as $ty });
    );

    ($ty:ty,
     $encode:ident,
     $merge:ident,
     $encode_repeated:ident,
     $merge_repeated:ident,
     $encode_packed:ident,
     $encoded_len:ident,
     $encoded_len_repeated:ident,
     $encoded_len_packed:ident,
     to_uint64($to_uint64_value:ident) $to_uint64:expr,
     from_uint64($from_uint64_value:ident) $from_uint64:expr) => (

         #[inline]
         pub fn $encode<B>(tag: u32, $to_uint64_value: &$ty, buf: &mut B) where B: BufMut {
             encode_key(tag, WireType::Varint, buf);
             encode_varint($to_uint64, buf);
         }

         #[inline]
         pub fn $merge<B>(wire_type: WireType, value: &mut $ty, buf: &mut B) -> Result<()> where B: Buf {
             check_wire_type(WireType::Varint, wire_type)?;
             let $from_uint64_value = decode_varint(buf)?;
             *value = $from_uint64;
             Ok(())
         }

         encode_repeated!($ty, $encode, $encode_repeated);

         #[inline]
         pub fn $encode_packed<B>(tag: u32, values: &Vec<$ty>, buf: &mut B) where B: BufMut {
             if values.is_empty() { return; }

             encode_key(tag, WireType::LengthDelimited, buf);
             let len: usize = values.iter().map(|$to_uint64_value| {
                 encoded_len_varint($to_uint64)
             }).sum();
             encode_varint(len as u64, buf);

             for $to_uint64_value in values {
                 encode_varint($to_uint64, buf);
             }
         }

         merge_repeated_numeric!($ty, WireType::Varint, $merge, $merge_repeated);

         #[inline]
         pub fn $encoded_len(tag: u32, $to_uint64_value: &$ty) -> usize {
             key_len(tag) + encoded_len_varint($to_uint64)
         }

         #[inline]
         pub fn $encoded_len_repeated(tag: u32, values: &Vec<$ty>) -> usize {
             key_len(tag) * values.len() + values.iter().map(|$to_uint64_value| {
                 encoded_len_varint($to_uint64)
             }).sum::<usize>()
         }

         #[inline]
         pub fn $encoded_len_packed(tag: u32, values: &Vec<$ty>) -> usize {
             key_len(tag) + values.iter().map(|$to_uint64_value| {
                 encoded_len_varint($to_uint64)
             }).sum::<usize>()
         }
    );
}

varint!(bool,
        encode_bool,
        merge_bool,
        encode_repeated_bool,
        merge_repeated_bool,
        encode_packed_bool,
        encoded_len_bool,
        encoded_len_repeated_bool,
        encoded_len_packed_bool,
        to_uint64(value) if *value { 1u64 } else { 0u64 },
        from_uint64(value) value != 0);

varint!(i32,
        encode_int32,
        merge_int32,
        encode_repeated_int32,
        merge_repeated_int32,
        encode_packed_int32,
        encoded_len_int32,
        encoded_len_repeated_int32,
        encoded_len_packed_int32);

varint!(i64,
        encode_int64,
        merge_int64,
        encode_repeated_int64,
        merge_repeated_int64,
        encode_packed_int64,
        encoded_len_int64,
        encoded_len_repeated_int64,
        encoded_len_packed_int64);

varint!(u32,
        encode_uint32,
        merge_uint32,
        encode_repeated_uint32,
        merge_repeated_uint32,
        encode_packed_uint32,
        encoded_len_uint32,
        encoded_len_repeated_uint32,
        encoded_len_packed_uint32);

varint!(u64,
        encode_uint64,
        merge_uint64,
        encode_repeated_uint64,
        merge_repeated_uint64,
        encode_packed_uint64,
        encoded_len_uint64,
        encoded_len_repeated_uint64,
        encoded_len_packed_uint64);

varint!(i32,
        encode_sint32,
        merge_sint32,
        encode_repeated_sint32,
        merge_repeated_sint32,
        encode_packed_sint32,
        encoded_len_sint32,
        encoded_len_repeated_sint32,
        encoded_len_packed_sint32,
        to_uint64(value) {
            ((value << 1) ^ (value >> 31)) as u64
        },
        from_uint64(value) {
            let value = value as u32;
            ((value >> 1) as i32) ^ (-((value & 1) as i32))
        });

varint!(i64,
        encode_sint64,
        merge_sint64,
        encode_repeated_sint64,
        merge_repeated_sint64,
        encode_packed_sint64,
        encoded_len_sint64,
        encoded_len_repeated_sint64,
        encoded_len_packed_sint64,
        to_uint64(value) {
            ((value << 1) ^ (value >> 63)) as u64
        },
        from_uint64(value) {
            ((value >> 1) as i64) ^ (-((value & 1) as i64))
        });

macro_rules! fixed_width {
    ($ty:ty,
     $width:expr,
     $wire_type:expr,
     $encode:ident,
     $merge:ident,
     $encode_repeated:ident,
     $merge_repeated:ident,
     $encode_packed:ident,
     $encoded_len:ident,
     $encoded_len_repeated:ident,
     $encoded_len_packed:ident,
     $put:ident,
     $get:ident) => (

         #[inline]
         pub fn $encode<B>(tag: u32, value: &$ty, buf: &mut B) where B: BufMut {
             encode_key(tag, $wire_type, buf);
             buf.$put::<LittleEndian>(*value);
         }

         #[inline]
         pub fn $merge<B>(wire_type: WireType, value: &mut $ty, buf: &mut B) -> Result<()> where B: Buf {
             check_wire_type(WireType::Varint, wire_type)?;
             if buf.remaining() < 4 {
                 return Err(invalid_data("buffer underflow"));
             }
             *value = buf.$get::<LittleEndian>();
             Ok(())
         }

         encode_repeated!($ty, $encode, $encode_repeated);

         #[inline]
         pub fn $encode_packed<B>(tag: u32, values: &Vec<$ty>, buf: &mut B) where B: BufMut {
             if values.is_empty() { return; }

             encode_key(tag, WireType::LengthDelimited, buf);
             let len = values.len() as u64 * $width;
             encode_varint(len as u64, buf);

             for value in values {
                 buf.$put::<LittleEndian>(*value);
             }
         }

         #[inline]
         pub fn $merge_repeated<B>(wire_type: WireType, values: &mut Vec<$ty>, buf: &mut Take<B>) -> Result<()> where B: Buf {
             if wire_type == WireType::LengthDelimited {
                 let len = decode_varint(buf)?;
                 if len > buf.remaining() as u64 {
                     return Err(invalid_data("buffer underflow"));
                 }
                 let len = len as usize;
                 let limit = buf.limit();
                 buf.set_limit(len);

                 while buf.has_remaining() {
                    let mut value = Default::default();
                    $merge(wire_type, &mut value, buf)?;
                    values.push(value);
                 }
                 buf.set_limit(limit - len);
             } else {
                 check_wire_type(WireType::Varint, wire_type)?;
                 let mut value = Default::default();
                 $merge(wire_type, &mut value, buf)?;
                 values.push(value);
             }
             Ok(())
         }

         #[inline]
         pub fn $encoded_len(tag: u32, _: &$ty) -> usize {
             key_len(tag) + $width
         }

         #[inline]
         pub fn $encoded_len_repeated(tag: u32, values: &Vec<$ty>) -> usize {
             (key_len(tag) + $width) * values.len()
         }

         #[inline]
         pub fn $encoded_len_packed(tag: u32, values: &Vec<$ty>) -> usize {
             key_len(tag) + $width * values.len()
         }
    );
}

fixed_width!(f32,
             4,
             WireType::ThirtyTwoBit,
             encode_float,
             merge_float,
             encode_repeated_float,
             merge_repeated_float,
             encode_packed_float,
             encoded_len_float,
             encoded_len_repeated_float,
             encoded_len_packed_float,
             put_f32,
             get_f32);

fixed_width!(f64,
             8,
             WireType::SixtyFourBit,
             encode_double,
             merge_double,
             encode_repeated_double,
             merge_repeated_double,
             encode_packed_double,
             encoded_len_double,
             encoded_len_repeated_double,
             encoded_len_packed_double,
             put_f64,
             get_f64);

fixed_width!(u32,
             4,
             WireType::ThirtyTwoBit,
             encode_fixed32,
             merge_fixed32,
             encode_repeated_fixed32,
             merge_repeated_fixed32,
             encode_packed_fixed32,
             encoded_len_fixed32,
             encoded_len_repeated_fixed32,
             encoded_len_packed_fixed32,
             put_u32,
             get_u32);

fixed_width!(u64,
             8,
             WireType::SixtyFourBit,
             encode_fixed64,
             merge_fixed64,
             encode_repeated_fixed64,
             merge_repeated_fixed64,
             encode_packed_fixed64,
             encoded_len_fixed64,
             encoded_len_repeated_fixed64,
             encoded_len_packed_fixed64,
             put_u64,
             get_u64);

fixed_width!(i32,
             4,
             WireType::ThirtyTwoBit,
             encode_sfixed32,
             merge_sfixed32,
             encode_repeated_sfixed32,
             merge_repeated_sfixed32,
             encode_packed_sfixed32,
             encoded_len_sfixed32,
             encoded_len_repeated_sfixed32,
             encoded_len_packed_sfixed32,
             put_i32,
             get_i32);

fixed_width!(i64,
             8,
             WireType::SixtyFourBit,
             encode_sfixed64,
             merge_sfixed64,
             encode_repeated_sfixed64,
             merge_repeated_sfixed64,
             encode_packed_sfixed64,
             encoded_len_sfixed64,
             encoded_len_repeated_sfixed64,
             encoded_len_packed_sfixed64,
             put_i64,
             get_i64);

macro_rules! length_delimited {
    ($ty:ty,
     $merge:ident,
     $merge_repeated:ident,
     $encoded_len:ident,
     $encoded_len_repeated:ident) => (
         #[inline]
         pub fn $merge_repeated<B>(wire_type: WireType, values: &mut Vec<$ty>, buf: &mut Take<B>) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let mut value = Default::default();
                $merge(wire_type, &mut value, buf)?;
                values.push(value);
                Ok(())
         }

         #[inline]
         pub fn $encoded_len(tag: u32, value: &$ty) -> usize {
             key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
         }

         #[inline]
         pub fn $encoded_len_repeated(tag: u32, values: &Vec<$ty>) -> usize {
             key_len(tag) * values.len() + values.iter().map(|value| {
                 encoded_len_varint(value.len() as u64) + value.len()
             }).sum::<usize>()
         }
    )
}

#[inline]
pub fn encode_string<B>(tag: u32, value: &String, buf: &mut B) where B: BufMut {
    encode_key(tag, WireType::LengthDelimited, buf);
    buf.put_slice(value.as_bytes());
}
#[inline]
pub fn merge_string<B>(wire_type: WireType, value: &mut String, buf: &mut Take<B>) -> Result<()> where B: Buf {
    unsafe {
        // String::as_mut_vec is unsafe because it doesn't check that the bytes
        // inserted into it the resulting vec are valid UTF-8. We check
        // explicitly in order to ensure this is safe.
        merge_bytes(wire_type, value.as_mut_vec(), buf)?;
        str::from_utf8(value.as_bytes()).map_err(|_| {
            invalid_data("failed to decode string: data is not UTF-8 encoded")
        })?;
    }
    Ok(())
}
encode_repeated!(String, encode_string, encode_repeated_string);
length_delimited!(String, merge_string, merge_repeated_string, encoded_len_string, encoded_len_repeated_string);

#[inline]
pub fn encode_bytes<B>(tag: u32, value: &Vec<u8>, buf: &mut B) where B: BufMut {
    encode_key(tag, WireType::LengthDelimited, buf);
    buf.put_slice(value);
}
#[inline]
pub fn merge_bytes<B>(wire_type: WireType, value: &mut Vec<u8>, buf: &mut Take<B>) -> Result<()> where B: Buf {
    check_wire_type(WireType::LengthDelimited, wire_type)?;
    let len = decode_varint(buf)?;
    if (buf.remaining() as u64) < len {
        return Err(invalid_data("buffer underflow"));
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
encode_repeated!(Vec<u8>, encode_bytes, encode_repeated_bytes);
length_delimited!(Vec<u8>, merge_bytes, merge_repeated_bytes, encoded_len_bytes, encoded_len_repeated_bytes);

// Generates methods to encode, merge, and get the encoded length of a map.
macro_rules! map {
    ($key_ty:ty,
     $val_ty:ty,
     $encode:ident,
     $merge:ident,
     $encoded_len:ident,
     $key_encode:ident,
     $key_merge:ident,
     $key_encoded_len:ident,
     $val_encode:ident,
     $val_merge:ident,
     $val_encoded_len:ident) => (

         #[inline]
         pub fn $encode<B>(tag: u32,
                           values: &HashMap<$key_ty, $val_ty>,
                           buf: &mut B) where B: BufMut {
            for (key, val) in values {
                let skip_key = key == &<$key_ty as Default>::default();
                let skip_val = val == &<$val_ty as Default>::default();

                let len = (if skip_key { 0 } else { $key_encoded_len(1, key) }) +
                          (if skip_val { 0 } else { $val_encoded_len(2, val) });

                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(len as u64, buf);
                if !skip_key {
                    $key_encode(1, key, buf);
                }
                if !skip_val {
                    $val_encode(2, val, buf);
                }
            }
         }

         #[inline]
         pub fn $merge<B>(values: &mut HashMap<$key_ty, $val_ty>,
                          buf: &mut Take<B>) -> Result<()> where B: Buf {
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(invalid_data("buffer underflow"));
            }
            let len = len as usize;
            let limit = buf.limit();
            buf.set_limit(len);

            let mut key = Default::default();
            let mut val = Default::default();

            while buf.has_remaining() {
                let (tag, wire_type) = decode_key(buf)?;
                match tag {
                    1 => $key_merge(wire_type, &mut key, buf)?,
                    2 => $val_merge(wire_type, &mut val, buf)?,
                    _ => (),
                }
            }

            values.insert(key, val);
            buf.set_limit(limit - len);
            Ok(())
         }

         #[inline]
         pub fn $encoded_len(tag: u32,
                             values: &HashMap<$key_ty, $val_ty>) -> usize {
             key_len(tag) * values.len() + values.iter().map(|(key, val)| {
                (if key == &<$key_ty as Default>::default() { 0 } else { $key_encoded_len(1, key) }) +
                (if val == &<$val_ty as Default>::default() { 0 } else { $val_encoded_len(2, val) })
             }).sum::<usize>()
         }
    );
}

// This differs from map_<$key_ty>_int32 in one way: the enumeration can have
// a default value other than 0. This is an extremely subtle edge condition that
// only happens with proto2.
macro_rules! enumeration_map {
    ($key_ty:ty,
     $encode:ident,
     $merge:ident,
     $encoded_len:ident,
     $key_encode:ident,
     $key_merge:ident,
     $key_encoded_len:ident) => (

         #[inline]
         pub fn $encode<B>(default_val: i32,
                           tag: u32,
                           values: &HashMap<$key_ty, i32>,
                           buf: &mut B) where B: BufMut {
            for (key, val) in values {
                let skip_key = key == &<$key_ty as Default>::default();
                let skip_val = val == &default_val;

                let len = (if skip_key { 0 } else { $key_encoded_len(1, key) }) +
                          (if skip_val { 0 } else { encoded_len_int32(2, val) });

                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(len as u64, buf);
                if !skip_key {
                    $key_encode(1, key, buf);
                }
                if !skip_val {
                    encode_int32(2, val, buf);
                }
            }
         }

         #[inline]
         pub fn $merge<B>(default_val: i32,
                          values: &mut HashMap<$key_ty, i32>,
                          buf: &mut Take<B>) -> Result<()> where B: Buf {
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(invalid_data("buffer underflow"));
            }
            let len = len as usize;
            let limit = buf.limit();
            buf.set_limit(len);

            let mut key = Default::default();
            let mut val = default_val;

            while buf.has_remaining() {
                let (tag, wire_type) = decode_key(buf)?;
                match tag {
                    1 => $key_merge(wire_type, &mut key, buf)?,
                    2 => merge_int32(wire_type, &mut val, buf)?,
                    _ => (),
                }
            }

            values.insert(key, val);
            buf.set_limit(limit - len);
            Ok(())
         }

         #[inline]
         pub fn $encoded_len(default_val: i32,
                             tag: u32,
                             values: &HashMap<$key_ty, i32>) -> usize {
             key_len(tag) * values.len() + values.iter().map(|(key, val)| {
                (if key == &<$key_ty as Default>::default() { 0 } else { $key_encoded_len(1, key) }) +
                (if val == &default_val { 0 } else { encoded_len_int32(2, val) })
             }).sum::<usize>()
         }
    );
}

// The following block is generated with the snippet:
//
// ```rust
// let key_types = &[
//     ("i32", "int32"),
//     ("i64", "int64"),
//     ("u32", "uint32"),
//     ("u64", "uint64"),
//     ("i32", "sint32"),
//     ("i64", "sint64"),
//     ("u32", "fixed32"),
//     ("u64", "fixed64"),
//     ("i32", "sfixed32"),
//     ("i64", "sfixed64"),
//     ("bool", "bool"),
//     ("String", "string"),
// ];
//
// let val_types = &[
//     ("f32", "float"),
//     ("f64", "double"),
//     ("i32", "int32"),
//     ("i64", "int64"),
//     ("u32", "uint32"),
//     ("u64", "uint64"),
//     ("i32", "sint32"),
//     ("i64", "sint64"),
//     ("u32", "fixed32"),
//     ("u64", "fixed64"),
//     ("i32", "sfixed32"),
//     ("i64", "sfixed64"),
//     ("bool", "bool"),
//     ("String", "string"),
//     ("Vec<u8>", "bytes"),
// ];
//
// for &(ref key_rust_ty, ref key_pb_ty) in key_types {
//     for &(ref val_rust_ty, ref val_pb_ty) in val_types {
//         println!("map!({key_rust_ty}, {val_rust_ty}, encode_map_{key_pb_ty}_{val_pb_ty}, merge_map_{key_pb_ty}_{val_pb_ty}, encoded_len_map_{key_pb_ty}_{val_pb_ty},
//  encode_{key_pb_ty}, merge_{key_pb_ty}, encoded_len_{key_pb_ty},
//  encode_{val_pb_ty}, merge_{val_pb_ty}, encoded_len_{val_pb_ty});",
//      key_rust_ty=key_rust_ty,
//      val_rust_ty=val_rust_ty,
//      key_pb_ty=key_pb_ty,
//      val_pb_ty=val_pb_ty);
//   }
// }
// for &(ref key_rust_ty, ref key_pb_ty) in key_types {
//     println!("enumeration_map!({key_rust_ty}, encode_map_{key_pb_ty}_enumeration, merge_map_{key_pb_ty}_enumeration, encoded_len_map_{key_pb_ty}_enumeration,
//              encode_{key_pb_ty}, merge_{key_pb_ty}, encoded_len_{key_pb_ty});",
//      key_rust_ty=key_rust_ty,
//      key_pb_ty=key_pb_ty);
// }
// ```

map!(i32, f32, encode_map_int32_float, merge_map_int32_float, encoded_len_map_int32_float,
     encode_int32, merge_int32, encoded_len_int32,
     encode_float, merge_float, encoded_len_float);
map!(i32, f64, encode_map_int32_double, merge_map_int32_double, encoded_len_map_int32_double,
     encode_int32, merge_int32, encoded_len_int32,
     encode_double, merge_double, encoded_len_double);
map!(i32, i32, encode_map_int32_int32, merge_map_int32_int32, encoded_len_map_int32_int32,
     encode_int32, merge_int32, encoded_len_int32,
     encode_int32, merge_int32, encoded_len_int32);
map!(i32, i64, encode_map_int32_int64, merge_map_int32_int64, encoded_len_map_int32_int64,
     encode_int32, merge_int32, encoded_len_int32,
     encode_int64, merge_int64, encoded_len_int64);
map!(i32, u32, encode_map_int32_uint32, merge_map_int32_uint32, encoded_len_map_int32_uint32,
     encode_int32, merge_int32, encoded_len_int32,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(i32, u64, encode_map_int32_uint64, merge_map_int32_uint64, encoded_len_map_int32_uint64,
     encode_int32, merge_int32, encoded_len_int32,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(i32, i32, encode_map_int32_sint32, merge_map_int32_sint32, encoded_len_map_int32_sint32,
     encode_int32, merge_int32, encoded_len_int32,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(i32, i64, encode_map_int32_sint64, merge_map_int32_sint64, encoded_len_map_int32_sint64,
     encode_int32, merge_int32, encoded_len_int32,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(i32, u32, encode_map_int32_fixed32, merge_map_int32_fixed32, encoded_len_map_int32_fixed32,
     encode_int32, merge_int32, encoded_len_int32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(i32, u64, encode_map_int32_fixed64, merge_map_int32_fixed64, encoded_len_map_int32_fixed64,
     encode_int32, merge_int32, encoded_len_int32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(i32, i32, encode_map_int32_sfixed32, merge_map_int32_sfixed32, encoded_len_map_int32_sfixed32,
     encode_int32, merge_int32, encoded_len_int32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(i32, i64, encode_map_int32_sfixed64, merge_map_int32_sfixed64, encoded_len_map_int32_sfixed64,
     encode_int32, merge_int32, encoded_len_int32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(i32, bool, encode_map_int32_bool, merge_map_int32_bool, encoded_len_map_int32_bool,
     encode_int32, merge_int32, encoded_len_int32,
     encode_bool, merge_bool, encoded_len_bool);
map!(i32, String, encode_map_int32_string, merge_map_int32_string, encoded_len_map_int32_string,
     encode_int32, merge_int32, encoded_len_int32,
     encode_string, merge_string, encoded_len_string);
map!(i32, Vec<u8>, encode_map_int32_bytes, merge_map_int32_bytes, encoded_len_map_int32_bytes,
     encode_int32, merge_int32, encoded_len_int32,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(i64, f32, encode_map_int64_float, merge_map_int64_float, encoded_len_map_int64_float,
     encode_int64, merge_int64, encoded_len_int64,
     encode_float, merge_float, encoded_len_float);
map!(i64, f64, encode_map_int64_double, merge_map_int64_double, encoded_len_map_int64_double,
     encode_int64, merge_int64, encoded_len_int64,
     encode_double, merge_double, encoded_len_double);
map!(i64, i32, encode_map_int64_int32, merge_map_int64_int32, encoded_len_map_int64_int32,
     encode_int64, merge_int64, encoded_len_int64,
     encode_int32, merge_int32, encoded_len_int32);
map!(i64, i64, encode_map_int64_int64, merge_map_int64_int64, encoded_len_map_int64_int64,
     encode_int64, merge_int64, encoded_len_int64,
     encode_int64, merge_int64, encoded_len_int64);
map!(i64, u32, encode_map_int64_uint32, merge_map_int64_uint32, encoded_len_map_int64_uint32,
     encode_int64, merge_int64, encoded_len_int64,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(i64, u64, encode_map_int64_uint64, merge_map_int64_uint64, encoded_len_map_int64_uint64,
     encode_int64, merge_int64, encoded_len_int64,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(i64, i32, encode_map_int64_sint32, merge_map_int64_sint32, encoded_len_map_int64_sint32,
     encode_int64, merge_int64, encoded_len_int64,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(i64, i64, encode_map_int64_sint64, merge_map_int64_sint64, encoded_len_map_int64_sint64,
     encode_int64, merge_int64, encoded_len_int64,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(i64, u32, encode_map_int64_fixed32, merge_map_int64_fixed32, encoded_len_map_int64_fixed32,
     encode_int64, merge_int64, encoded_len_int64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(i64, u64, encode_map_int64_fixed64, merge_map_int64_fixed64, encoded_len_map_int64_fixed64,
     encode_int64, merge_int64, encoded_len_int64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(i64, i32, encode_map_int64_sfixed32, merge_map_int64_sfixed32, encoded_len_map_int64_sfixed32,
     encode_int64, merge_int64, encoded_len_int64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(i64, i64, encode_map_int64_sfixed64, merge_map_int64_sfixed64, encoded_len_map_int64_sfixed64,
     encode_int64, merge_int64, encoded_len_int64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(i64, bool, encode_map_int64_bool, merge_map_int64_bool, encoded_len_map_int64_bool,
     encode_int64, merge_int64, encoded_len_int64,
     encode_bool, merge_bool, encoded_len_bool);
map!(i64, String, encode_map_int64_string, merge_map_int64_string, encoded_len_map_int64_string,
     encode_int64, merge_int64, encoded_len_int64,
     encode_string, merge_string, encoded_len_string);
map!(i64, Vec<u8>, encode_map_int64_bytes, merge_map_int64_bytes, encoded_len_map_int64_bytes,
     encode_int64, merge_int64, encoded_len_int64,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(u32, f32, encode_map_uint32_float, merge_map_uint32_float, encoded_len_map_uint32_float,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_float, merge_float, encoded_len_float);
map!(u32, f64, encode_map_uint32_double, merge_map_uint32_double, encoded_len_map_uint32_double,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_double, merge_double, encoded_len_double);
map!(u32, i32, encode_map_uint32_int32, merge_map_uint32_int32, encoded_len_map_uint32_int32,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_int32, merge_int32, encoded_len_int32);
map!(u32, i64, encode_map_uint32_int64, merge_map_uint32_int64, encoded_len_map_uint32_int64,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_int64, merge_int64, encoded_len_int64);
map!(u32, u32, encode_map_uint32_uint32, merge_map_uint32_uint32, encoded_len_map_uint32_uint32,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(u32, u64, encode_map_uint32_uint64, merge_map_uint32_uint64, encoded_len_map_uint32_uint64,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(u32, i32, encode_map_uint32_sint32, merge_map_uint32_sint32, encoded_len_map_uint32_sint32,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(u32, i64, encode_map_uint32_sint64, merge_map_uint32_sint64, encoded_len_map_uint32_sint64,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(u32, u32, encode_map_uint32_fixed32, merge_map_uint32_fixed32, encoded_len_map_uint32_fixed32,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(u32, u64, encode_map_uint32_fixed64, merge_map_uint32_fixed64, encoded_len_map_uint32_fixed64,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(u32, i32, encode_map_uint32_sfixed32, merge_map_uint32_sfixed32, encoded_len_map_uint32_sfixed32,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(u32, i64, encode_map_uint32_sfixed64, merge_map_uint32_sfixed64, encoded_len_map_uint32_sfixed64,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(u32, bool, encode_map_uint32_bool, merge_map_uint32_bool, encoded_len_map_uint32_bool,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_bool, merge_bool, encoded_len_bool);
map!(u32, String, encode_map_uint32_string, merge_map_uint32_string, encoded_len_map_uint32_string,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_string, merge_string, encoded_len_string);
map!(u32, Vec<u8>, encode_map_uint32_bytes, merge_map_uint32_bytes, encoded_len_map_uint32_bytes,
     encode_uint32, merge_uint32, encoded_len_uint32,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(u64, f32, encode_map_uint64_float, merge_map_uint64_float, encoded_len_map_uint64_float,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_float, merge_float, encoded_len_float);
map!(u64, f64, encode_map_uint64_double, merge_map_uint64_double, encoded_len_map_uint64_double,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_double, merge_double, encoded_len_double);
map!(u64, i32, encode_map_uint64_int32, merge_map_uint64_int32, encoded_len_map_uint64_int32,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_int32, merge_int32, encoded_len_int32);
map!(u64, i64, encode_map_uint64_int64, merge_map_uint64_int64, encoded_len_map_uint64_int64,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_int64, merge_int64, encoded_len_int64);
map!(u64, u32, encode_map_uint64_uint32, merge_map_uint64_uint32, encoded_len_map_uint64_uint32,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(u64, u64, encode_map_uint64_uint64, merge_map_uint64_uint64, encoded_len_map_uint64_uint64,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(u64, i32, encode_map_uint64_sint32, merge_map_uint64_sint32, encoded_len_map_uint64_sint32,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(u64, i64, encode_map_uint64_sint64, merge_map_uint64_sint64, encoded_len_map_uint64_sint64,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(u64, u32, encode_map_uint64_fixed32, merge_map_uint64_fixed32, encoded_len_map_uint64_fixed32,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(u64, u64, encode_map_uint64_fixed64, merge_map_uint64_fixed64, encoded_len_map_uint64_fixed64,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(u64, i32, encode_map_uint64_sfixed32, merge_map_uint64_sfixed32, encoded_len_map_uint64_sfixed32,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(u64, i64, encode_map_uint64_sfixed64, merge_map_uint64_sfixed64, encoded_len_map_uint64_sfixed64,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(u64, bool, encode_map_uint64_bool, merge_map_uint64_bool, encoded_len_map_uint64_bool,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_bool, merge_bool, encoded_len_bool);
map!(u64, String, encode_map_uint64_string, merge_map_uint64_string, encoded_len_map_uint64_string,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_string, merge_string, encoded_len_string);
map!(u64, Vec<u8>, encode_map_uint64_bytes, merge_map_uint64_bytes, encoded_len_map_uint64_bytes,
     encode_uint64, merge_uint64, encoded_len_uint64,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(i32, f32, encode_map_sint32_float, merge_map_sint32_float, encoded_len_map_sint32_float,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_float, merge_float, encoded_len_float);
map!(i32, f64, encode_map_sint32_double, merge_map_sint32_double, encoded_len_map_sint32_double,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_double, merge_double, encoded_len_double);
map!(i32, i32, encode_map_sint32_int32, merge_map_sint32_int32, encoded_len_map_sint32_int32,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_int32, merge_int32, encoded_len_int32);
map!(i32, i64, encode_map_sint32_int64, merge_map_sint32_int64, encoded_len_map_sint32_int64,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_int64, merge_int64, encoded_len_int64);
map!(i32, u32, encode_map_sint32_uint32, merge_map_sint32_uint32, encoded_len_map_sint32_uint32,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(i32, u64, encode_map_sint32_uint64, merge_map_sint32_uint64, encoded_len_map_sint32_uint64,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(i32, i32, encode_map_sint32_sint32, merge_map_sint32_sint32, encoded_len_map_sint32_sint32,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(i32, i64, encode_map_sint32_sint64, merge_map_sint32_sint64, encoded_len_map_sint32_sint64,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(i32, u32, encode_map_sint32_fixed32, merge_map_sint32_fixed32, encoded_len_map_sint32_fixed32,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(i32, u64, encode_map_sint32_fixed64, merge_map_sint32_fixed64, encoded_len_map_sint32_fixed64,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(i32, i32, encode_map_sint32_sfixed32, merge_map_sint32_sfixed32, encoded_len_map_sint32_sfixed32,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(i32, i64, encode_map_sint32_sfixed64, merge_map_sint32_sfixed64, encoded_len_map_sint32_sfixed64,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(i32, bool, encode_map_sint32_bool, merge_map_sint32_bool, encoded_len_map_sint32_bool,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_bool, merge_bool, encoded_len_bool);
map!(i32, String, encode_map_sint32_string, merge_map_sint32_string, encoded_len_map_sint32_string,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_string, merge_string, encoded_len_string);
map!(i32, Vec<u8>, encode_map_sint32_bytes, merge_map_sint32_bytes, encoded_len_map_sint32_bytes,
     encode_sint32, merge_sint32, encoded_len_sint32,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(i64, f32, encode_map_sint64_float, merge_map_sint64_float, encoded_len_map_sint64_float,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_float, merge_float, encoded_len_float);
map!(i64, f64, encode_map_sint64_double, merge_map_sint64_double, encoded_len_map_sint64_double,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_double, merge_double, encoded_len_double);
map!(i64, i32, encode_map_sint64_int32, merge_map_sint64_int32, encoded_len_map_sint64_int32,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_int32, merge_int32, encoded_len_int32);
map!(i64, i64, encode_map_sint64_int64, merge_map_sint64_int64, encoded_len_map_sint64_int64,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_int64, merge_int64, encoded_len_int64);
map!(i64, u32, encode_map_sint64_uint32, merge_map_sint64_uint32, encoded_len_map_sint64_uint32,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(i64, u64, encode_map_sint64_uint64, merge_map_sint64_uint64, encoded_len_map_sint64_uint64,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(i64, i32, encode_map_sint64_sint32, merge_map_sint64_sint32, encoded_len_map_sint64_sint32,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(i64, i64, encode_map_sint64_sint64, merge_map_sint64_sint64, encoded_len_map_sint64_sint64,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(i64, u32, encode_map_sint64_fixed32, merge_map_sint64_fixed32, encoded_len_map_sint64_fixed32,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(i64, u64, encode_map_sint64_fixed64, merge_map_sint64_fixed64, encoded_len_map_sint64_fixed64,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(i64, i32, encode_map_sint64_sfixed32, merge_map_sint64_sfixed32, encoded_len_map_sint64_sfixed32,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(i64, i64, encode_map_sint64_sfixed64, merge_map_sint64_sfixed64, encoded_len_map_sint64_sfixed64,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(i64, bool, encode_map_sint64_bool, merge_map_sint64_bool, encoded_len_map_sint64_bool,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_bool, merge_bool, encoded_len_bool);
map!(i64, String, encode_map_sint64_string, merge_map_sint64_string, encoded_len_map_sint64_string,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_string, merge_string, encoded_len_string);
map!(i64, Vec<u8>, encode_map_sint64_bytes, merge_map_sint64_bytes, encoded_len_map_sint64_bytes,
     encode_sint64, merge_sint64, encoded_len_sint64,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(u32, f32, encode_map_fixed32_float, merge_map_fixed32_float, encoded_len_map_fixed32_float,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_float, merge_float, encoded_len_float);
map!(u32, f64, encode_map_fixed32_double, merge_map_fixed32_double, encoded_len_map_fixed32_double,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_double, merge_double, encoded_len_double);
map!(u32, i32, encode_map_fixed32_int32, merge_map_fixed32_int32, encoded_len_map_fixed32_int32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_int32, merge_int32, encoded_len_int32);
map!(u32, i64, encode_map_fixed32_int64, merge_map_fixed32_int64, encoded_len_map_fixed32_int64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_int64, merge_int64, encoded_len_int64);
map!(u32, u32, encode_map_fixed32_uint32, merge_map_fixed32_uint32, encoded_len_map_fixed32_uint32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(u32, u64, encode_map_fixed32_uint64, merge_map_fixed32_uint64, encoded_len_map_fixed32_uint64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(u32, i32, encode_map_fixed32_sint32, merge_map_fixed32_sint32, encoded_len_map_fixed32_sint32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(u32, i64, encode_map_fixed32_sint64, merge_map_fixed32_sint64, encoded_len_map_fixed32_sint64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(u32, u32, encode_map_fixed32_fixed32, merge_map_fixed32_fixed32, encoded_len_map_fixed32_fixed32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(u32, u64, encode_map_fixed32_fixed64, merge_map_fixed32_fixed64, encoded_len_map_fixed32_fixed64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(u32, i32, encode_map_fixed32_sfixed32, merge_map_fixed32_sfixed32, encoded_len_map_fixed32_sfixed32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(u32, i64, encode_map_fixed32_sfixed64, merge_map_fixed32_sfixed64, encoded_len_map_fixed32_sfixed64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(u32, bool, encode_map_fixed32_bool, merge_map_fixed32_bool, encoded_len_map_fixed32_bool,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_bool, merge_bool, encoded_len_bool);
map!(u32, String, encode_map_fixed32_string, merge_map_fixed32_string, encoded_len_map_fixed32_string,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_string, merge_string, encoded_len_string);
map!(u32, Vec<u8>, encode_map_fixed32_bytes, merge_map_fixed32_bytes, encoded_len_map_fixed32_bytes,
     encode_fixed32, merge_fixed32, encoded_len_fixed32,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(u64, f32, encode_map_fixed64_float, merge_map_fixed64_float, encoded_len_map_fixed64_float,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_float, merge_float, encoded_len_float);
map!(u64, f64, encode_map_fixed64_double, merge_map_fixed64_double, encoded_len_map_fixed64_double,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_double, merge_double, encoded_len_double);
map!(u64, i32, encode_map_fixed64_int32, merge_map_fixed64_int32, encoded_len_map_fixed64_int32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_int32, merge_int32, encoded_len_int32);
map!(u64, i64, encode_map_fixed64_int64, merge_map_fixed64_int64, encoded_len_map_fixed64_int64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_int64, merge_int64, encoded_len_int64);
map!(u64, u32, encode_map_fixed64_uint32, merge_map_fixed64_uint32, encoded_len_map_fixed64_uint32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(u64, u64, encode_map_fixed64_uint64, merge_map_fixed64_uint64, encoded_len_map_fixed64_uint64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(u64, i32, encode_map_fixed64_sint32, merge_map_fixed64_sint32, encoded_len_map_fixed64_sint32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(u64, i64, encode_map_fixed64_sint64, merge_map_fixed64_sint64, encoded_len_map_fixed64_sint64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(u64, u32, encode_map_fixed64_fixed32, merge_map_fixed64_fixed32, encoded_len_map_fixed64_fixed32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(u64, u64, encode_map_fixed64_fixed64, merge_map_fixed64_fixed64, encoded_len_map_fixed64_fixed64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(u64, i32, encode_map_fixed64_sfixed32, merge_map_fixed64_sfixed32, encoded_len_map_fixed64_sfixed32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(u64, i64, encode_map_fixed64_sfixed64, merge_map_fixed64_sfixed64, encoded_len_map_fixed64_sfixed64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(u64, bool, encode_map_fixed64_bool, merge_map_fixed64_bool, encoded_len_map_fixed64_bool,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_bool, merge_bool, encoded_len_bool);
map!(u64, String, encode_map_fixed64_string, merge_map_fixed64_string, encoded_len_map_fixed64_string,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_string, merge_string, encoded_len_string);
map!(u64, Vec<u8>, encode_map_fixed64_bytes, merge_map_fixed64_bytes, encoded_len_map_fixed64_bytes,
     encode_fixed64, merge_fixed64, encoded_len_fixed64,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(i32, f32, encode_map_sfixed32_float, merge_map_sfixed32_float, encoded_len_map_sfixed32_float,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_float, merge_float, encoded_len_float);
map!(i32, f64, encode_map_sfixed32_double, merge_map_sfixed32_double, encoded_len_map_sfixed32_double,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_double, merge_double, encoded_len_double);
map!(i32, i32, encode_map_sfixed32_int32, merge_map_sfixed32_int32, encoded_len_map_sfixed32_int32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_int32, merge_int32, encoded_len_int32);
map!(i32, i64, encode_map_sfixed32_int64, merge_map_sfixed32_int64, encoded_len_map_sfixed32_int64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_int64, merge_int64, encoded_len_int64);
map!(i32, u32, encode_map_sfixed32_uint32, merge_map_sfixed32_uint32, encoded_len_map_sfixed32_uint32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(i32, u64, encode_map_sfixed32_uint64, merge_map_sfixed32_uint64, encoded_len_map_sfixed32_uint64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(i32, i32, encode_map_sfixed32_sint32, merge_map_sfixed32_sint32, encoded_len_map_sfixed32_sint32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(i32, i64, encode_map_sfixed32_sint64, merge_map_sfixed32_sint64, encoded_len_map_sfixed32_sint64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(i32, u32, encode_map_sfixed32_fixed32, merge_map_sfixed32_fixed32, encoded_len_map_sfixed32_fixed32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(i32, u64, encode_map_sfixed32_fixed64, merge_map_sfixed32_fixed64, encoded_len_map_sfixed32_fixed64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(i32, i32, encode_map_sfixed32_sfixed32, merge_map_sfixed32_sfixed32, encoded_len_map_sfixed32_sfixed32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(i32, i64, encode_map_sfixed32_sfixed64, merge_map_sfixed32_sfixed64, encoded_len_map_sfixed32_sfixed64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(i32, bool, encode_map_sfixed32_bool, merge_map_sfixed32_bool, encoded_len_map_sfixed32_bool,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_bool, merge_bool, encoded_len_bool);
map!(i32, String, encode_map_sfixed32_string, merge_map_sfixed32_string, encoded_len_map_sfixed32_string,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_string, merge_string, encoded_len_string);
map!(i32, Vec<u8>, encode_map_sfixed32_bytes, merge_map_sfixed32_bytes, encoded_len_map_sfixed32_bytes,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(i64, f32, encode_map_sfixed64_float, merge_map_sfixed64_float, encoded_len_map_sfixed64_float,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_float, merge_float, encoded_len_float);
map!(i64, f64, encode_map_sfixed64_double, merge_map_sfixed64_double, encoded_len_map_sfixed64_double,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_double, merge_double, encoded_len_double);
map!(i64, i32, encode_map_sfixed64_int32, merge_map_sfixed64_int32, encoded_len_map_sfixed64_int32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_int32, merge_int32, encoded_len_int32);
map!(i64, i64, encode_map_sfixed64_int64, merge_map_sfixed64_int64, encoded_len_map_sfixed64_int64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_int64, merge_int64, encoded_len_int64);
map!(i64, u32, encode_map_sfixed64_uint32, merge_map_sfixed64_uint32, encoded_len_map_sfixed64_uint32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(i64, u64, encode_map_sfixed64_uint64, merge_map_sfixed64_uint64, encoded_len_map_sfixed64_uint64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(i64, i32, encode_map_sfixed64_sint32, merge_map_sfixed64_sint32, encoded_len_map_sfixed64_sint32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(i64, i64, encode_map_sfixed64_sint64, merge_map_sfixed64_sint64, encoded_len_map_sfixed64_sint64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(i64, u32, encode_map_sfixed64_fixed32, merge_map_sfixed64_fixed32, encoded_len_map_sfixed64_fixed32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(i64, u64, encode_map_sfixed64_fixed64, merge_map_sfixed64_fixed64, encoded_len_map_sfixed64_fixed64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(i64, i32, encode_map_sfixed64_sfixed32, merge_map_sfixed64_sfixed32, encoded_len_map_sfixed64_sfixed32,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(i64, i64, encode_map_sfixed64_sfixed64, merge_map_sfixed64_sfixed64, encoded_len_map_sfixed64_sfixed64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(i64, bool, encode_map_sfixed64_bool, merge_map_sfixed64_bool, encoded_len_map_sfixed64_bool,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_bool, merge_bool, encoded_len_bool);
map!(i64, String, encode_map_sfixed64_string, merge_map_sfixed64_string, encoded_len_map_sfixed64_string,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_string, merge_string, encoded_len_string);
map!(i64, Vec<u8>, encode_map_sfixed64_bytes, merge_map_sfixed64_bytes, encoded_len_map_sfixed64_bytes,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(bool, f32, encode_map_bool_float, merge_map_bool_float, encoded_len_map_bool_float,
     encode_bool, merge_bool, encoded_len_bool,
     encode_float, merge_float, encoded_len_float);
map!(bool, f64, encode_map_bool_double, merge_map_bool_double, encoded_len_map_bool_double,
     encode_bool, merge_bool, encoded_len_bool,
     encode_double, merge_double, encoded_len_double);
map!(bool, i32, encode_map_bool_int32, merge_map_bool_int32, encoded_len_map_bool_int32,
     encode_bool, merge_bool, encoded_len_bool,
     encode_int32, merge_int32, encoded_len_int32);
map!(bool, i64, encode_map_bool_int64, merge_map_bool_int64, encoded_len_map_bool_int64,
     encode_bool, merge_bool, encoded_len_bool,
     encode_int64, merge_int64, encoded_len_int64);
map!(bool, u32, encode_map_bool_uint32, merge_map_bool_uint32, encoded_len_map_bool_uint32,
     encode_bool, merge_bool, encoded_len_bool,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(bool, u64, encode_map_bool_uint64, merge_map_bool_uint64, encoded_len_map_bool_uint64,
     encode_bool, merge_bool, encoded_len_bool,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(bool, i32, encode_map_bool_sint32, merge_map_bool_sint32, encoded_len_map_bool_sint32,
     encode_bool, merge_bool, encoded_len_bool,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(bool, i64, encode_map_bool_sint64, merge_map_bool_sint64, encoded_len_map_bool_sint64,
     encode_bool, merge_bool, encoded_len_bool,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(bool, u32, encode_map_bool_fixed32, merge_map_bool_fixed32, encoded_len_map_bool_fixed32,
     encode_bool, merge_bool, encoded_len_bool,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(bool, u64, encode_map_bool_fixed64, merge_map_bool_fixed64, encoded_len_map_bool_fixed64,
     encode_bool, merge_bool, encoded_len_bool,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(bool, i32, encode_map_bool_sfixed32, merge_map_bool_sfixed32, encoded_len_map_bool_sfixed32,
     encode_bool, merge_bool, encoded_len_bool,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(bool, i64, encode_map_bool_sfixed64, merge_map_bool_sfixed64, encoded_len_map_bool_sfixed64,
     encode_bool, merge_bool, encoded_len_bool,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(bool, bool, encode_map_bool_bool, merge_map_bool_bool, encoded_len_map_bool_bool,
     encode_bool, merge_bool, encoded_len_bool,
     encode_bool, merge_bool, encoded_len_bool);
map!(bool, String, encode_map_bool_string, merge_map_bool_string, encoded_len_map_bool_string,
     encode_bool, merge_bool, encoded_len_bool,
     encode_string, merge_string, encoded_len_string);
map!(bool, Vec<u8>, encode_map_bool_bytes, merge_map_bool_bytes, encoded_len_map_bool_bytes,
     encode_bool, merge_bool, encoded_len_bool,
     encode_bytes, merge_bytes, encoded_len_bytes);
map!(String, f32, encode_map_string_float, merge_map_string_float, encoded_len_map_string_float,
     encode_string, merge_string, encoded_len_string,
     encode_float, merge_float, encoded_len_float);
map!(String, f64, encode_map_string_double, merge_map_string_double, encoded_len_map_string_double,
     encode_string, merge_string, encoded_len_string,
     encode_double, merge_double, encoded_len_double);
map!(String, i32, encode_map_string_int32, merge_map_string_int32, encoded_len_map_string_int32,
     encode_string, merge_string, encoded_len_string,
     encode_int32, merge_int32, encoded_len_int32);
map!(String, i64, encode_map_string_int64, merge_map_string_int64, encoded_len_map_string_int64,
     encode_string, merge_string, encoded_len_string,
     encode_int64, merge_int64, encoded_len_int64);
map!(String, u32, encode_map_string_uint32, merge_map_string_uint32, encoded_len_map_string_uint32,
     encode_string, merge_string, encoded_len_string,
     encode_uint32, merge_uint32, encoded_len_uint32);
map!(String, u64, encode_map_string_uint64, merge_map_string_uint64, encoded_len_map_string_uint64,
     encode_string, merge_string, encoded_len_string,
     encode_uint64, merge_uint64, encoded_len_uint64);
map!(String, i32, encode_map_string_sint32, merge_map_string_sint32, encoded_len_map_string_sint32,
     encode_string, merge_string, encoded_len_string,
     encode_sint32, merge_sint32, encoded_len_sint32);
map!(String, i64, encode_map_string_sint64, merge_map_string_sint64, encoded_len_map_string_sint64,
     encode_string, merge_string, encoded_len_string,
     encode_sint64, merge_sint64, encoded_len_sint64);
map!(String, u32, encode_map_string_fixed32, merge_map_string_fixed32, encoded_len_map_string_fixed32,
     encode_string, merge_string, encoded_len_string,
     encode_fixed32, merge_fixed32, encoded_len_fixed32);
map!(String, u64, encode_map_string_fixed64, merge_map_string_fixed64, encoded_len_map_string_fixed64,
     encode_string, merge_string, encoded_len_string,
     encode_fixed64, merge_fixed64, encoded_len_fixed64);
map!(String, i32, encode_map_string_sfixed32, merge_map_string_sfixed32, encoded_len_map_string_sfixed32,
     encode_string, merge_string, encoded_len_string,
     encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
map!(String, i64, encode_map_string_sfixed64, merge_map_string_sfixed64, encoded_len_map_string_sfixed64,
     encode_string, merge_string, encoded_len_string,
     encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
map!(String, bool, encode_map_string_bool, merge_map_string_bool, encoded_len_map_string_bool,
     encode_string, merge_string, encoded_len_string,
     encode_bool, merge_bool, encoded_len_bool);
map!(String, String, encode_map_string_string, merge_map_string_string, encoded_len_map_string_string,
     encode_string, merge_string, encoded_len_string,
     encode_string, merge_string, encoded_len_string);
map!(String, Vec<u8>, encode_map_string_bytes, merge_map_string_bytes, encoded_len_map_string_bytes,
     encode_string, merge_string, encoded_len_string,
     encode_bytes, merge_bytes, encoded_len_bytes);

enumeration_map!(i32, encode_map_int32_enumeration, merge_map_int32_enumeration, encoded_len_map_int32_enumeration,
                 encode_int32, merge_int32, encoded_len_int32);
enumeration_map!(i64, encode_map_int64_enumeration, merge_map_int64_enumeration, encoded_len_map_int64_enumeration,
                 encode_int64, merge_int64, encoded_len_int64);
enumeration_map!(u32, encode_map_uint32_enumeration, merge_map_uint32_enumeration, encoded_len_map_uint32_enumeration,
                 encode_uint32, merge_uint32, encoded_len_uint32);
enumeration_map!(u64, encode_map_uint64_enumeration, merge_map_uint64_enumeration, encoded_len_map_uint64_enumeration,
                 encode_uint64, merge_uint64, encoded_len_uint64);
enumeration_map!(i32, encode_map_sint32_enumeration, merge_map_sint32_enumeration, encoded_len_map_sint32_enumeration,
                 encode_sint32, merge_sint32, encoded_len_sint32);
enumeration_map!(i64, encode_map_sint64_enumeration, merge_map_sint64_enumeration, encoded_len_map_sint64_enumeration,
                 encode_sint64, merge_sint64, encoded_len_sint64);
enumeration_map!(u32, encode_map_fixed32_enumeration, merge_map_fixed32_enumeration, encoded_len_map_fixed32_enumeration,
                 encode_fixed32, merge_fixed32, encoded_len_fixed32);
enumeration_map!(u64, encode_map_fixed64_enumeration, merge_map_fixed64_enumeration, encoded_len_map_fixed64_enumeration,
                 encode_fixed64, merge_fixed64, encoded_len_fixed64);
enumeration_map!(i32, encode_map_sfixed32_enumeration, merge_map_sfixed32_enumeration, encoded_len_map_sfixed32_enumeration,
                 encode_sfixed32, merge_sfixed32, encoded_len_sfixed32);
enumeration_map!(i64, encode_map_sfixed64_enumeration, merge_map_sfixed64_enumeration, encoded_len_map_sfixed64_enumeration,
                 encode_sfixed64, merge_sfixed64, encoded_len_sfixed64);
enumeration_map!(bool, encode_map_bool_enumeration, merge_map_bool_enumeration, encoded_len_map_bool_enumeration,
                 encode_bool, merge_bool, encoded_len_bool);
enumeration_map!(String, encode_map_string_enumeration, merge_map_string_enumeration, encoded_len_map_string_enumeration,
                 encode_string, merge_string, encoded_len_string);

pub fn encode_message<M, B>(msg: &M, tag: u32, buf: &mut B)
where M: Message,
      B: BufMut {
    encode_key(tag, WireType::LengthDelimited, buf);
    encode_varint(msg.encoded_len() as u64, buf);
    msg.encode_raw(buf);
}


pub fn merge_message<M, B>(msg: &mut M, buf: &mut Take<B>) -> Result<()>
where M: Message,
      B: Buf {
    let len = decode_varint(buf)?;
    if len > buf.remaining() as u64 {
        return Err(invalid_data("buffer underflow"));
    }

    let len = len as usize;
    let limit = buf.limit();
    buf.set_limit(len);
    msg.merge(buf)?;
    buf.set_limit(limit - len);
    Ok(())
}

pub fn encode_repeated_message<M, B>(tag: u32, messages: &[M], buf: &mut B)
where M: Message,
      B: BufMut {
    for msg in messages {
        encode_message(msg, tag, buf);
    }
}

pub fn merge_repeated_message<M, B>(messages: &mut Vec<M>, buf: &mut Take<B>) -> Result<()>
where M: Message,
      B: Buf {
    let mut msg = M::default();
    merge_message(&mut msg, buf)?;
    messages.push(msg);
    Ok(())
}

pub fn encoded_len_message<M>(tag: u32, msg: &M) -> usize where M: Message {
    key_len(tag) + msg.encoded_len()
}

pub fn encoded_len_repeated_message<M>(tag: u32, messages: &[M]) -> usize where M: Message {
    key_len(tag) * messages.len() + messages.iter().map(Message::encoded_len).sum::<usize>()
}
