//! This module defines the `Field` trait, which must be implemented by Protobuf
//! encodable values, as well as implementations for the built in types.
//!
//! The `Field` trait should not be used directly by users of the `proto` library.

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

use Message;
use invalid_data;
use invalid_input;

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

/// Encodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline]
pub fn encode_key<B>(tag: u32, wire_type: WireType, buf: &mut B) where B: BufMut {
    debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);
    let key = (tag << 3) | wire_type as u32;
    encode_varint(key as u64, buf);
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

/// A type indicating that the default Protobuf encoding is used for a field.
pub enum Default {}
/// A type indicating that the integer field should use variable-width,
/// ZigZag encoded, signed encoding.
pub enum Signed {}
/// A type indicating that the integer field should use fixed-width encoding.
pub enum Fixed {}
/// A type indicating that a repeated field should use packed encoding.
pub enum Packed {}

/// A field type in a Protobuf message.
///
/// The `E` type parameter allows `Field` to be implemented multiple times for a
/// single type, in order to provide multiple encoding and decoding options for
/// a single Rust type. For instance, the Protobuf `fixed32` and `uint32` types
/// both correspond to the Rust `u32` type, so `u32` has two impls of `Field`
/// with different types for `E`, which correspond to `fixed32` and `uint32`.
pub trait Field<E=Default> : Sized {

    /// Encodes a key and the field to the buffer.
    /// The buffer must have enough remaining space to hold the encoded key and field.
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut;

    /// Decodes the field from the buffer, and merges the value into self.
    ///
    /// For scalar, enumeration, and oneof types, the default implementation
    /// can be used, which replaces the current value. Message, repeated, and
    /// map fields must override this in order to provide proper merge semantics.
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf;

    /// Returns the length of the encoded field.
    fn encoded_len(&self, tag: u32) -> usize;
}

/// A Protobuf field type which can be packed repeated.
pub trait PackedField<E=Default> : Sized {
    /// Encodes the scalar field to the buffer, without a key.
    /// The buffer must have enough remaining space to hold the encoded key and field.
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut;

    /// Decodes an instance of the field from the buffer.
    fn decode_raw<B>(buf: &mut B) -> Result<Self> where B: Buf;

    /// Returns the encoded length of the field, without a key.
    fn encoded_len_raw(self) -> usize;

    /// Returns the wire type of the scalar field.
    fn wire_type() -> WireType;
}

/// Marker trait for types which can be keys in a Protobuf map.
pub trait KeyField {}
impl KeyField for bool {}
impl KeyField for i32 {}
impl KeyField for i64 {}
impl KeyField for u32 {}
impl KeyField for u64 {}
impl KeyField for String {}

macro_rules! optional_field {
    ($ty: ty) => { optional_field!($ty, Default); };
    ($ty: ty, $e: ty) => {
        impl Field<$e> for Option<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                if let Some(ref f) = *self {
                    <$ty as Field<$e>>::encode(f, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                if self.is_none() {
                    *self = Some(default::Default::default());
                }
                <$ty as Field<$e>>::merge(self.as_mut().unwrap(), tag, wire_type, buf)
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                if let Some(ref f) = *self {
                    <$ty as Field<$e>>::encoded_len(f, tag)
                } else { 0 }
            }
        }
    };
}

macro_rules! packed_field {
    ($ty: ty) => { packed_field!($ty, Default); };
    ($ty: ty, $e: ty) => {
        impl Field<$e> for $ty {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                encode_key(tag, <$ty as PackedField<$e>>::wire_type(), buf);
                <$ty as PackedField<$e>>::encode_raw(*self, buf);
            }
            #[inline]
            fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type(<$ty as PackedField<$e>>::wire_type(), wire_type)?;
                *self = <$ty as PackedField<$e>>::decode_raw(buf)?;
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize { key_len(tag) + <$ty as PackedField<$e>>::encoded_len_raw(*self) }
        }
        optional_field!($ty, $e);

        impl Field<$e> for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                for value in self {
                    <$ty as Field<$e>>::encode(value, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                if wire_type == WireType::LengthDelimited {
                    // Packed repeated encoding.
                    let len = decode_varint(buf)?;
                    if len > buf.remaining() as u64 {
                        return Err(invalid_data("failed to decode packed repeated field: buffer underflow"));
                    }

                    let mut buf = buf.take(len as usize);
                    while buf.has_remaining() {
                        self.push(<$ty as PackedField<$e>>::decode_raw(&mut buf)?);
                    }
                } else {
                    // Default repeated encoding.
                    let mut value = default::Default::default();
                    <$ty as Field<$e>>::merge(&mut value, tag, wire_type, buf)?;
                    self.push(value);
                }
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                self.iter().map(|f| <$ty as Field<$e>>::encoded_len(f, tag)).sum()
            }
        }

        impl Field<(Packed, $e)> for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                if self.is_empty() { return; }
                encode_key(tag, WireType::LengthDelimited, buf);
                let len: usize = self.iter().cloned().map(<$ty as PackedField<$e>>::encoded_len_raw).sum();
                encode_varint(len as u64, buf);
                for &value in self {
                    <$ty as PackedField<$e>>::encode_raw(value, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                <Vec<$ty> as Field<$e>>::merge(self, tag, wire_type, buf)
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                if self.is_empty() { return 0; }
                let len: usize = self.iter().cloned().map(<$ty as PackedField<$e>>::encoded_len_raw).sum();
                key_len(tag) + encoded_len_varint(len as _) + len
            }
        }
    };
}

// bool
impl PackedField for bool {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut {
        buf.put_u8(if self { 1u8 } else { 0u8 });
    }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<bool> where B: Buf {
        if !buf.has_remaining() {
            return Err(invalid_input("failed to decode bool: buffer underflow"));
        }
        match buf.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid_data("failed to decode bool: invalid value")),
        }
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 1 }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
packed_field!(bool);

// int32
impl PackedField for i32 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<i32> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len_raw(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
packed_field!(i32);

// int64
impl PackedField for i64 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<i64> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len_raw(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType {
        WireType::Varint
    }
}
packed_field!(i64);

// uint32
impl PackedField for u32 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<u32> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len_raw(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
packed_field!(u32);

// uint64
impl PackedField for u64 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<u64> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len_raw(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
packed_field!(u64);

// float
impl PackedField for f32 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { buf.put_f32::<LittleEndian>(self) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<f32> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode float: buffer underflow"));
        }
        Ok(buf.get_f32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}
packed_field!(f32);

// double
impl PackedField for f64 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { buf.put_f64::<LittleEndian>(self) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<f64> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode double: buffer underflow"));
        }
        Ok(buf.get_f64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}
packed_field!(f64);

// sint32
impl PackedField<Signed> for i32 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut {
        encode_varint(((self << 1) ^ (self >> 31)) as u64, buf)
    }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<i32> where B: Buf {
        decode_varint(buf).map(|value| {
            let value = value as i32;
            (value >> 1) ^ -(value & 1)
        })
    }
    #[inline]
    fn encoded_len_raw(self) -> usize {
        encoded_len_varint(((self << 1) ^ (self >> 31)) as u64)
    }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
packed_field!(i32, Signed);

// sint64
impl PackedField<Signed> for i64 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut {
        encode_varint(((self << 1) ^ (self >> 63)) as u64, buf)
    }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<i64> where B: Buf {
        decode_varint(buf).map(|value| {
            let value = value as i64;
            (value >> 1) ^ -(value & 1)
        })
    }
    #[inline]
    fn encoded_len_raw(self) -> usize {
        encoded_len_varint(((self << 1) ^ (self >> 63)) as u64)
    }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
packed_field!(i64, Signed);

// fixed32
impl PackedField<Fixed> for u32 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { buf.put_u32::<LittleEndian>(self) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<u32> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode fixed32: buffer underflow"));
        }
        Ok(buf.get_u32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}
packed_field!(u32, Fixed);

// fixed64
impl PackedField<Fixed> for u64 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { buf.put_u64::<LittleEndian>(self) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<u64> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode fixed64: buffer underflow"));
        }
        Ok(buf.get_u64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}
packed_field!(u64, Fixed);

// sfixed32
impl PackedField<Fixed> for i32 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { buf.put_i32::<LittleEndian>(self) }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<i32> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode sfixed32: buffer underflow"));
        }
        Ok(buf.get_i32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}
packed_field!(i32, Fixed);

// sfixed64
impl PackedField<Fixed> for i64 {
    #[inline]
    fn encode_raw<B>(self, buf: &mut B) where B: BufMut { buf.put_i64::<LittleEndian>(self); }
    #[inline]
    fn decode_raw<B>(buf: &mut B) -> Result<i64> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode sfixed64 field: buffer underflow"));
        }
        Ok(buf.get_i64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len_raw(self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}
packed_field!(i64, Fixed);

macro_rules! repeated_length_delimited_field {
    ($ty: ty) => {
        impl Field for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                for value in self {
                    <$ty as Field>::encode(value, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let mut value = default::Default::default();
                <$ty as Field>::merge(&mut value, tag, WireType::LengthDelimited, buf)?;
                self.push(value);
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                self.iter().map(|f| f.encoded_len(tag)).sum()
            }
        }
    };
}

// bytes
impl Field for Vec<u8> {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(self.len() as u64, buf);
        buf.put_slice(self);
    }
    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if (buf.remaining() as u64) < len {
            return Err(invalid_input("failed to decode bytes: buffer underflow"));
        }
        let len = len as usize;
        self.clear();
        self.extend_from_slice(&buf.bytes()[..len]);
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        let len = self.len();
        key_len(tag) + encoded_len_varint(len as u64) + len
    }
}
optional_field!(Vec<u8>);
repeated_length_delimited_field!(Vec<u8>);

// string
impl Field for String {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(self.len() as u64, buf);
        buf.put_slice(self.as_bytes());
    }
    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if (buf.remaining() as u64) < len {
            return Err(invalid_input("failed to decode string: buffer underflow"));
        }
        let len = len as usize;
        self.clear();
        self.push_str(str::from_utf8(&buf.bytes()[..len]).map_err(|_| {
            invalid_data("failed to decode string: data is not UTF-8 encoded")
        })?);
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        let len = self.len();
        key_len(tag) + encoded_len_varint(len as u64) + len
    }
}
optional_field!(String);
repeated_length_delimited_field!(String);

// Map
impl <K, V, EK, EV> Field<(EK, EV)> for HashMap<K, V>
where K: Eq + Hash + KeyField + Field<EK> + default::Default,
      V: Field<EV>  + default::Default {

    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        for (key, value) in self {
            encode_key(tag, WireType::LengthDelimited, buf);
            let len = key.encoded_len(1) + value.encoded_len(2);
            encode_varint(len as u64, buf);

            key.encode(1, buf);
            value.encode(2, buf);
        }
    }
    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(invalid_data("failed to decode map entry: buffer underflow"));
        }
        let mut buf = buf.take(len as usize);

        let mut key = K::default();
        let mut value = V::default();

        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            match tag {
                1 => key.merge(tag, wire_type, &mut buf)?,
                2 => value.merge(tag, wire_type, &mut buf)?,
                _ => return Err(invalid_data(format!("failed to decode map entry: unexpected field ({:?}, {:?})",
                                                     tag, wire_type))),
            }
        }
        self.insert(key, value);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        let key_len = key_len(tag);
        self.iter().map(|(key, value)| {
            let len = key.encoded_len(1) + value.encoded_len(2);
            key_len + encoded_len_varint(len as u64) + len
        }).sum()
    }
}

impl <K, V> Field for HashMap<K, V>
where K: Eq + Hash + KeyField + Field + default::Default,
      V: Field + default::Default {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        <HashMap<K, V> as Field<(Default, Default)>>::encode(self, tag, buf)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        <HashMap<K, V> as Field<(Default, Default)>>::merge(self, tag, wire_type, buf)
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        <HashMap<K, V> as Field<(Default, Default)>>::encoded_len(self, tag)
    }
}

impl <M> Field for M where M: Message + default::Default {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, WireType::LengthDelimited, buf);
        self.encode_length_delimited(buf).expect("failed to encode message");
    }

    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        self.merge_length_delimited(buf)
    }

    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        key_len(tag) + self.encoded_len()
    }
}

impl <M> Field for Option<M> where M: Message + default::Default {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        if let Some(ref f) = *self {
            Field::encode(f, tag, buf);
        }
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        if self.is_none() {
            *self = Some(default::Default::default());
        }
        Field::merge(self.as_mut().unwrap(), tag, wire_type, buf)
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        if let Some(ref f) = *self {
            Field::encoded_len(f, tag)
        } else { 0 }
    }
}

impl <M> Field for Vec<M> where M: Message + default::Default {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        for value in self {
            Field::encode(value, tag, buf);
        }
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut value = default::Default::default();
        Field::merge(&mut value, tag, WireType::LengthDelimited, buf)?;
        self.push(value);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        self.iter().map(|f| Field::encoded_len(f, tag)).sum()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use bytes::{
        Buf,
        Bytes,
        BytesMut,
        IntoBuf,
    };
    use quickcheck::TestResult;

    use super::*;

    // Creates a checker function for each field trait. Necessary to create as a macro as opposed
    // to taking the field trait as a parameter, because Field, SignedField, and FixedField don't
    // share a common super trait.
    fn check_field<T, E>(value: T, tag: u32) -> TestResult where T: Debug + default::Default + PartialEq + Field<E> {
        if tag > MAX_TAG || tag < MIN_TAG {
            return TestResult::discard()
        }

        let expected_len = value.encoded_len(tag);

        let mut buf = BytesMut::with_capacity(expected_len);
        value.encode(tag, &mut buf);

        let mut buf = buf.freeze().into_buf();

        if buf.remaining() != expected_len {
            return TestResult::error(format!("encoded_len wrong; expected: {}, actual: {}",
                                              expected_len, buf.remaining()));
        }

        if !buf.has_remaining() {
            // Short circuit for empty optional values or empty repeated values.
            return TestResult::passed();
        }

        let (decoded_tag, wire_type) = match decode_key(&mut buf) {
            Ok(key) => key,
            Err(error) => return TestResult::error(format!("{:?}", error)),
        };

        if tag != decoded_tag {
            return TestResult::error(
                format!("decoded tag does not match; expected: {}, actual: {}",
                        tag, decoded_tag));
        }

        match wire_type {
            WireType::SixtyFourBit if buf.remaining() != 8 => {
                return TestResult::error(
                    format!("64bit wire type illegal remaining: {}, tag: {}",
                            buf.remaining(), tag));
            },
            WireType::ThirtyTwoBit if buf.remaining() != 4 => {
                return TestResult::error(
                    format!("32bit wire type illegal remaining: {}, tag: {}",
                            buf.remaining(), tag));
            },
            _ => (),
        }

        let mut roundtrip_value = T::default();
        if let Err(error) = roundtrip_value.merge(tag, wire_type, &mut buf) {
            return TestResult::error(format!("{:?}", error));
        };

        if buf.has_remaining() {
            return TestResult::error(format!("expected buffer to be empty: {}", buf.remaining()));
        }

        if value == roundtrip_value {
            TestResult::passed()
        } else {
            TestResult::failed()
        }
    }

    quickcheck! {
        fn bool(value: bool, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn double(value: f64, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn float(value: f32, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn int32(value: i32, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn int64(value: i64, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn uint32(value: u32, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn uint64(value: u64, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn bytes(value: Vec<u8>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn string(value: String, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn sint32(value: i32, tag: u32) -> TestResult {
            check_field::<_, Signed>(value, tag)
        }
        fn sint64(value: i64, tag: u32) -> TestResult {
            check_field::<_, Signed>(value, tag)
        }
        fn fixed32(value: u32, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }
        fn fixed64(value: u64, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }
        fn sfixed32(value: i32, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }
        fn sfixed64(value: i64, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }

        fn optional_bool(value: Option<bool>, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn optional_double(value: Option<f64>, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn optional_float(value: Option<f32>, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn optional_int32(value: Option<i32>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn optional_int64(value: Option<i64>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn optional_uint32(value: Option<u32>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn optional_uint64(value: Option<u64>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn optional_bytes(value: Option<Vec<u8>>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn optional_string(value: Option<String>, tag: u32) -> TestResult {
            check_field::<_, Default>(value, tag)
        }
        fn optional_sint32(value: Option<i32>, tag: u32) -> TestResult {
            check_field::<_, Signed>(value, tag)
        }
        fn optional_sint64(value: Option<i64>, tag: u32) -> TestResult {
            check_field::<_, Signed>(value, tag)
        }
        fn optional_fixed32(value: Option<u32>, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }
        fn optional_fixed64(value: Option<u64>, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }
        fn optional_sfixed32(value: Option<i32>, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }
        fn optional_sfixed64(value: Option<i64>, tag: u32) -> TestResult {
            check_field::<_, Fixed>(value, tag)
        }

        fn packed_bool(value: Vec<bool>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_double(value: Vec<f64>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_float(value: Vec<f32>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_int32(value: Vec<i32>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_int64(value: Vec<i64>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_uint32(value: Vec<u32>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_uint64(value: Vec<u64>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Default)>(value, tag)
        }
        fn packed_sint32(value: Vec<i32>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Signed)>(value, tag)
        }
        fn packed_sint64(value: Vec<i64>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Signed)>(value, tag)
        }
        fn packed_fixed32(value: Vec<u32>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Fixed)>(value, tag)
        }
        fn packed_fixed64(value: Vec<u64>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Fixed)>(value, tag)
        }
        fn packed_sfixed32(value: Vec<i32>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Fixed)>(value, tag)
        }
        fn packed_sfixed64(value: Vec<i64>, tag: u32) -> TestResult {
            check_field::<_, (Packed, Fixed)>(value, tag)
        }
    }

    #[test]
    fn varint() {
        fn check(value: u64, encoded: &[u8]) {
            let mut buf = Vec::new();

            encode_varint(value, &mut buf);

            assert_eq!(buf, encoded);

            let roundtrip_value = decode_varint(&mut Bytes::from(encoded).into_buf()).expect("decoding failed");
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
