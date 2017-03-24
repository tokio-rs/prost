//! This module defines the `Field` trait, which must be implemented by Protobuf
//! encodable values, as well as implementations for the built in types.
//!
//! The `Field` trait should not be used directly by users of the `proto` library.

use std::cmp::min;
use std::default;
use std::error;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};
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

fn invalid_data<E>(error: E) -> Error where E: Into<Box<error::Error + Send + Sync>> {
    Error::new(ErrorKind::InvalidData, error.into())
}
fn invalid_input<E>(error: E) -> Error where E: Into<Box<error::Error + Send + Sync>> {
    Error::new(ErrorKind::InvalidInput, error.into())
}

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
pub fn varint_len(value: u64) -> usize {
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

/*
#[inline]
pub fn skip_field(wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
    match wire_type {
        WireType::Varint => {
            <u64 as ScalarField>::read_from(r, limit)?;
        },
        WireType::SixtyFourBit => {
            <u64 as ScalarField<Fixed>>::read_from(r, limit)?;
        },
        WireType::ThirtyTwoBit => {
            <u32 as ScalarField<Fixed>>::read_from(r, limit)?;
        },
        WireType::LengthDelimited => {
            <Vec<u8> as ScalarField>::read_from(r, limit)?;
        },
    };
    Ok(())
}
*/

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
    varint_len((tag << 3) as u64)
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

    // Methods which must be implemented by every field type.

    /// Encodes a key and the field to the buffer.
    /// The buffer must have enough remaining space to hold the encoded key and field.
    #[inline]
    fn encode_with_key<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, Self::wire_type(), buf);
        self.encode(buf);
    }

    /// Decodes the field from the buffer.
    ///
    /// The tag is provided so that oneof fields can determine which variant to read.
    /// The wire type is provided so that repeated scalar fields can determine
    /// whether the field is packed or unpacked.
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf;

    /// Decodes the field from the buffer.
    ///
    /// This method should return the same value as returned by `decode` wrapped
    /// in `Some`, except in one circumstance: when reading an unknown
    /// enumeration value, in which case `None` will be returned.
    #[inline]
    fn decode_repeated<B>(tag: u32,
                          wire_type: WireType,
                          buf: &mut B)
                          -> Result<Option<Self>> where B: Buf {
        <Self as Field<E>>::decode(tag, wire_type, buf).map(|value| Some(value))
    }

    /// Decodes the field from the buffer, and merges the value into self.
    ///
    /// For scalar, enumeration, and oneof types, the default implementation
    /// can be used, which replaces the current value. Message, repeated, and
    /// map fields must override this in order to provide proper merge semantics.
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        *self = <Self as Field<E>>::decode(tag, wire_type, buf)?;
        Ok(())
    }

    /// Returns the length of the encoded field with the key.
    ///
    /// Types which may write multiple keys (like unpacked repeated fields)
    /// should overide this.
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        key_len(tag) + self.encoded_len()
    }

    // Internal methods.

    /// Encodes the field to the buffer.
    ///
    /// Must be implemented for scalar types.
    #[doc(hidden)]
    fn encode<B>(&self, buf: &mut B) where B: BufMut;

    /// Returns the length of the encoded field. The length of the key is not included.
    #[doc(hidden)]
    fn encoded_len(&self) -> usize;

    /// Returns the wire type of the field.
    ///
    /// This method must be implemented if the default `encode_with_key`
    /// implementation is not overriden. Otherise, the implementation of
    /// `wire_type` may panic.
    #[doc(hidden)]
    fn wire_type() -> WireType;
}

// bool
impl Field for bool {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_u8(if *self { 1u8 } else { 0u8 });
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if !buf.has_remaining() {
            return Err(invalid_input("failed to decode bool field: buffer underflow"));
        }
        match buf.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid_data("failed to decode bool field: invalid value")),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize { 1 }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// int32
impl Field for i32 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(*self as u64, buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        decode_varint(buf).map(|value| value as _)
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(*self as u64) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// int64
impl Field for i64 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(*self as u64, buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        decode_varint(buf).map(|value| value as _)
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(*self as u64) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// uint32
impl Field for u32 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(*self as u64, buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        decode_varint(buf).map(|value| value as _)
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(*self as u64) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// uint64
impl Field for u64 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(*self, buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        decode_varint(buf)
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(*self) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// float
impl Field for f32 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_f32::<LittleEndian>(*self);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode float field: buffer underflow"));
        }
        Ok(buf.get_f32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(&self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}

// double
impl Field for f64 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_f64::<LittleEndian>(*self);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode double field: buffer underflow"));
        }
        Ok(buf.get_f64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(&self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}

// sint32
impl Field<Signed> for i32 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(((*self << 1) ^ (*self >> 31)) as u64, buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        decode_varint(buf).map(|value| {
            let value = value as i32;
            (value >> 1) ^ -(value & 1)
        })
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(((*self << 1) ^ (*self >> 31)) as u64) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// sint64
impl Field<Signed> for i64 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(((*self << 1) ^ (*self >> 63)) as u64, buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        decode_varint(buf).map(|value| {
            let value = value as i64;
            (value >> 1) ^ -(value & 1)
        })
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(((*self << 1) ^ (*self >> 63)) as u64) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}

// fixed32
impl Field<Fixed> for u32 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_u32::<LittleEndian>(*self);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode fixed32 field: buffer underflow"));
        }
        Ok(buf.get_u32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(&self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}

// fixed64
impl Field<Fixed> for u64 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_u64::<LittleEndian>(*self);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode fixed64 field: buffer underflow"));
        }
        Ok(buf.get_u64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(&self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}

// sfixed32
impl Field<Fixed> for i32 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_i32::<LittleEndian>(*self);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode sfixed32 field: buffer underflow"));
        }
        Ok(buf.get_i32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(&self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}

// sfixed64
impl Field<Fixed> for i64 {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_i64::<LittleEndian>(*self);
    }
    #[inline]
    fn decode<B>(_tag: u32, _wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode sfixed64 field: buffer underflow"));
        }
        Ok(buf.get_i64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(&self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}

// bytes
impl Field for Vec<u8> {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(self.len() as u64, buf);
        buf.put_slice(&self[..]);
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut v = Vec::new();
        Field::merge(&mut v, tag, wire_type, buf)?;
        Ok(v)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, _wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        let len = decode_varint(buf)?;
        if (buf.remaining() as u64) < len {
            return Err(invalid_input("failed to decode bytes field: buffer underflow"));
        }
        let len = len as usize;
        self.clear();
        self.extend_from_slice(&buf.bytes()[..len]);
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(self.len() as u64) + self.len() }
    #[inline]
    fn wire_type() -> WireType { WireType::LengthDelimited }
}

// string
impl Field for String {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        encode_varint(self.len() as u64, buf);
        buf.put_slice(self.as_bytes());
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut s = String::new();
        s.merge(tag, wire_type, buf)?;
        Ok(s)
    }
    #[inline]
    fn merge<B>(&mut self,
                     tag: u32,
                     _wire_type: WireType,
                     buf: &mut B) -> Result<()> where B: Buf {
        let len = decode_varint(buf)?;
        if (buf.remaining() as u64) < len {
            return Err(invalid_input("failed to decode string field: buffer underflow"));
        }
        let len = len as usize;

        self.clear();
        self.push_str(str::from_utf8(&buf.bytes()[..len]).map_err(|_| {
            invalid_data("failed to decode string field: data is not UTF-8 encoded")
        })?);
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self) -> usize { varint_len(self.len() as u64) + self.len() }
    #[inline]
    fn wire_type() -> WireType { WireType::LengthDelimited }
}

// optional
//
// All methods are overriden in case the underlying type has an overriden impl.
impl <F, E> Field<E> for Option<F> where F: Field<E> {
    #[inline]
    fn encode_with_key<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        if let Some(ref f) = *self {
            f.encode_with_key(tag, buf);
        }
    }
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        if let Some(ref f) = *self {
            f.encode(buf);
        }
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        <F as Field<E>>::decode(tag, wire_type, buf).map(|f| Some(f))
    }
    #[inline]
    fn decode_repeated<B>(tag: u32,
                          wire_type: WireType,
                          buf: &mut B)
                          -> Result<Option<Self>> where B: Buf {
        <F as Field<E>>::decode_repeated(tag, wire_type, buf).map(|f| Some(f))
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        match *self {
            Some(ref mut f) => f.merge(tag, wire_type, buf)?,
            None => *self = Self::decode(tag, wire_type, buf)?,
        }
        Ok(())
    }
    #[inline]
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        self.as_ref().map(|f| f.encoded_len_with_key(tag)).unwrap_or(0)
    }
    fn encoded_len(&self) -> usize {
        // Implement encoded_len_with_key instead, because there are a variable
        // number of keys to encode.
        unimplemented!()
    }
    fn wire_type() -> WireType {
        // encode_with_key is overriden, so this will not be called.
        unimplemented!()
    }
}

// repeated
impl <F, E> Field<(Default, E)> for Vec<F> where F: Field<E> {
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        // encode_with_key is overriden, so this will not be called.
        unimplemented!()
    }
    #[inline]
    fn encode_with_key<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        for value in self {
            Field::<E>::encode_with_key(value, tag, buf);
        }
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut vec = Vec::new();
        Field::<(Default, E)>::merge(&mut vec, tag, wire_type, buf)?;
        Ok(vec)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        let field_wire_type = <F as Field<E>>::wire_type();
        if wire_type == WireType::LengthDelimited && (field_wire_type == WireType::Varint ||
                                                      field_wire_type == WireType::SixtyFourBit ||
                                                      field_wire_type == WireType::ThirtyTwoBit) {
            // Packed encoding.
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(invalid_data("failed to decode packed repeated field: buffer underflow"));
            }

            let mut buf = buf.take(len as usize);
            while buf.has_remaining() {
                if let Some(value) = Field::<E>::decode_repeated(tag, field_wire_type, &mut buf)? {
                    self.push(value);
                }
            }
        } else {
            // Normal encoding.
            if let Some(value) = Field::<E>::decode_repeated(tag, field_wire_type, buf)? {
                self.push(value);
            }
        }
        Ok(())
    }
    #[inline]
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        self.iter().map(|f| f.encoded_len_with_key(tag)).sum()
    }
    fn encoded_len(&self) -> usize {
        // Implement encoded_len_with_key instead, because there are a variable
        // number of keys to encode.
        unimplemented!()
    }
    #[inline]
    fn wire_type() -> WireType {
        <F as Field<E>>::wire_type()
    }
}

// packed repeated
impl <F, E> Field<(Packed, E)> for Vec<F> where F: Field<E> {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        let len: usize = self.iter().map(<F as Field<E>>::encoded_len).sum();
        encode_varint(len as u64, buf);
        for value in self {
            F::encode(value, buf);
        }
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut vec = Vec::new();
        Field::<(Default, E)>::merge(&mut vec, tag, wire_type, buf)?;
        Ok(vec)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        Field::<(Default, E)>::merge(self, tag, wire_type, buf)
    }
    #[inline]
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        if self.is_empty() {
            0
        } else {
            let len: usize = self.iter().map(F::encoded_len).sum();
            varint_len(len as u64) + len
        }
    }
    fn encoded_len(&self) -> usize {
        // Implement encoded_len_with_key instead, because there are a variable
        // number of keys to encode.
        unimplemented!()
    }
    #[inline]
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
}

// Message
impl <M> Field for M where M: Message + default::Default {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        // This should never happen, since we check lengths upfront.
        self.encode_length_delimited(buf).expect("failed to encode message: buffer underflow")
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        <M as Message>::decode_length_delimited(buf)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        <M as Message>::merge_length_delimited(self, buf)
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        <M as Message>::encoded_len(self)
    }
    #[inline]
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
}

// Trait for types which can be keys in a Protobuf map.
pub trait Key {}
impl Key for i32 {}
impl Key for i64 {}
impl Key for u32 {}
impl Key for u64 {}
impl Key for bool {}
impl Key for String {}

// Map
impl <K, V, EK, EV> Field<(EK, EV)> for HashMap<K, V>
where K: default::Default + Eq + Hash + Key + Field<EK>,
      V: default::Default + Field<EV> {

    #[inline]
    fn encode_with_key<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        for (key, value) in self {
            encode_key(tag, WireType::LengthDelimited, buf);
            let len = key.encoded_len_with_key(1) + value.encoded_len_with_key(2);
            encode_varint(len as u64, buf);

            key.encode_with_key(1, buf);
            value.encode_with_key(2, buf);
        }
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut map = HashMap::new();
        map.merge(tag, wire_type, buf)?;
        Ok(map)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
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
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        self.iter().map(|(key, value)| {
            let len = key.encoded_len_with_key(1) + value.encoded_len_with_key(2);
            varint_len(len as u64) + len
        }).sum()
    }

    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        unimplemented!()
    }
    fn encoded_len(&self) -> usize {
        unimplemented!()
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
}

impl <K, V> Field<Default> for HashMap<K, V>
where K: default::Default + Eq + Hash + Key + Field<Default>,
      V: default::Default + Field<Default> {

    #[inline]
    fn encode_with_key<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        <HashMap<K, V> as Field<(Default, Default)>>::encode_with_key(self, tag, buf)
    }

    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        <HashMap<K, V> as Field<(Default, Default)>>::decode(tag, wire_type, buf)
    }

    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        <HashMap<K, V> as Field<(Default, Default)>>::merge(self, tag, wire_type, buf)
    }

    #[inline]
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        <HashMap<K, V> as Field<(Default, Default)>>::encoded_len_with_key(self, tag)
    }

    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        unimplemented!()
    }
    fn encoded_len(&self) -> usize {
        unimplemented!()
    }
    fn wire_type() -> WireType {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;
    use std::io::Cursor;

    use bytes::{Bytes, IntoBuf};

    use quickcheck::TestResult;

    use super::*;

    // Creates a checker function for each field trait. Necessary to create as a macro as opposed
    // to taking the field trait as a parameter, because Field, SignedField, and FixedField don't
    // share a common super trait.
    fn check_field<T, E>(value: T, tag: u32) -> TestResult where T: Debug + default::Default + PartialEq + Field<E> {
        if tag > MAX_TAG || tag < MIN_TAG {
            return TestResult::discard()
        }

        let mut buf = Vec::new();
        if let Err(error) = <T as Field<E>>::write_to(&value, tag, &mut buf) {
            return TestResult::error(format!("write_to failed: {:?}", error));
        };

        let expected_len = <T as Field<E>>::encoded_len(&value, tag);
        if expected_len != buf.len() {
            return TestResult::error(format!("encoded_len wrong; expected: {}, actual: {}",
                                                expected_len, buf.len()));
        }

        let mut encoded_len = buf.len();
        let mut cursor = Cursor::new(buf);
        let (wire_type, decoded_tag) = match read_key_from(&mut cursor, &mut encoded_len) {
            Ok(key) => key,
            Err(error) => return TestResult::error(format!("failed to read key: {:?}",
                                                            error)),
        };

        if tag != decoded_tag {
            return TestResult::error(
                format!("decoded tag does not match; expected: {}, actual: {}",
                        tag, decoded_tag));
        }

        match wire_type {
            WireType::SixtyFourBit if encoded_len != 8 => {
                return TestResult::error(
                    format!("64bit wire type illegal encoded_len: {}, tag: {}",
                            encoded_len, tag));
            },
            WireType::ThirtyTwoBit if encoded_len != 4 => {
                return TestResult::error(
                    format!("32bit wire type illegal encoded_len: {}, tag: {}",
                            encoded_len, tag));
            },
            _ => (),
        }

        let mut roundtrip_value = T::default();
        if let Err(error) = <T as Field<E>>::merge(&mut roundtrip_value,
                                                   tag,
                                                   wire_type,
                                                   &mut cursor,
                                                   &mut encoded_len) {
            return TestResult::error(format!("merge failed: {:?}", error));
        };

        if encoded_len != 0 {
            return TestResult::error(format!("expected read limit to be 0: {}",
                                                encoded_len));
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
