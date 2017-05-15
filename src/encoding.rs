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
                return Err(invalid_input("failed to skip 64-bit field: buffer underflow"));
            }
            buf.advance(8);
        },
        WireType::ThirtyTwoBit => {
            if buf.remaining() < 4 {
                return Err(invalid_input("failed to skip 32-bit field: buffer underflow"));
            }
            buf.advance(4);
        },
        WireType::LengthDelimited => {
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(invalid_input("failed to skip length delimited field: buffer underflow"));
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
                    return Err(invalid_input("buffer underflow"));
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
        encoded_len_sint3,
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
        encoded_len_sint6,
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
                 return Err(invalid_input("buffer underflow"));
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
                     return Err(invalid_input("buffer underflow"));
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
encode_repeated!(Vec<u8>, encode_bytes, encode_repeated_bytes);
length_delimited!(Vec<u8>, merge_bytes, merge_repeated_bytes, encoded_len_bytes, encoded_len_repeated_bytes);

#[inline]
pub fn encode_enum<E, B>(tag: u32, value: E, buf: &mut B) where B: BufMut, E: Into<i32>  {
    encode_int32(tag, &value.into(), buf)
}


#[inline]
pub fn merge_enum<E, B>(wire_type: WireType, value: &mut E, buf: &mut B) -> Result<()>
where B: Buf,
      E: From<i32> {
    let mut i = 0i32;
    merge_int32(wire_type, &mut i, buf)?;
    *value = E::from(i);
    Ok(())
}

/*
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
            return Err(invalid_input("buffer underflow"));
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
*/
