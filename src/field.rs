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

use encoding::*;
use invalid_data;
use invalid_input;
use Message;

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

macro_rules! scalar_optional_field {
    ($ty: ty, $e: ty) => {
        impl Field<$e> for Option<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                if let Some(ref f) = *self {
                    Field::<$e>::encode(f, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                if self.is_none() {
                    *self = Some(default::Default::default());
                }
                Field::<$e>::merge(self.as_mut().unwrap(), tag, wire_type, buf)
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                if let Some(ref f) = *self {
                    Field::<$e>::encoded_len(f, tag)
                } else { 0 }
            }
        }

    }

}

macro_rules! scalar_field {

    (32bit: $ty: ty, $e: ty, $encode: ident, $decode: ident) => {
        scalar_field!($ty, $e, WireType::ThirtyTwoBit, $encode, $decode, |_| { 4 });
    };

    (64bit: $ty: ty, $e: ty, $encode: ident, $decode: ident) => {
        scalar_field!($ty, $e, WireType::SixtyFourBit, $encode, $decode, |_| { 8 });
    };

    (varint: $ty: ty, $e: ty, $encode: ident, $decode: ident, $encoded_len: expr) => {
        scalar_field!($ty, $e, WireType::Varint, $encode, $decode, $encoded_len);
    };

    ($ty: ty, $e: ty, $wire_type: expr, $encode: ident, $decode: ident, $encoded_len: expr) => {
        impl Field<$e> for $ty {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                encode_key(tag, $wire_type, buf);
                $encode(*self, buf);
            }
            #[inline]
            fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type($wire_type, wire_type)?;
                *self = $decode(buf)?;
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                key_len(tag) + $encoded_len(*self)
            }
        }
        scalar_optional_field!($ty, $e);

        impl Field<$e> for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                // Packed repeated encoding.
                if self.is_empty() { return; }
                let len: usize = self.iter().map($encoded_len).sum();
                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(len as u64, buf);
                for value in self {
                    $encode(value, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                let field_wire_type = <F as Field<E>>::wire_type();
                if wire_type == WireType::LengthDelimited {
                    // Packed repeated encoding.
                    let len = decode_varint(buf)?;
                    if len > buf.remaining() as u64 {
                        return Err(invalid_data("failed to decode packed repeated field: buffer underflow"));
                    }

                    let mut buf = buf.take(len as usize);
                    while buf.has_remaining() {
                        if let Some(value) = Field::<E>::decode_repeated(tag, $wire_type, &mut buf)? {
                            self.push(value);
                        }
                    }
                } else {
                    // Default repeated encoding.
                    if let Some(value) = Field::<E>::decode_repeated(tag, field_wire_type, buf)? {
                        self.push(value);
                    }
                }
                Ok(())
            }
            #[inline]
            fn encoded_len_with_key(&self, tag: u32) -> usize {
                if self.is_empty() {
                    0
                } else if F::wire_type() == WireType::LengthDelimited {
                    // Default repeated encoding.
                    self.iter().map(|f| f.encoded_len_with_key(tag)).sum()
                } else {
                    // Packed repeated encoding.
                    let len: usize = self.iter().map(F::encoded_len).sum();
                    key_len(tag) + varint_len(len as u64) + len
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

    };

    (length_delimited: $ty: ty, $encode: ident, $merge: ident) => {
        impl Field for $ty {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                encode_key(tag, WireType::LengthDelimited, buf);
                $encode(self, buf);
            }
            #[inline]
            fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                $merge(self, buf)
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                key_len(tag) + self.len()
            }
        }
        scalar_optional_field!($ty, Default);
    };
}

scalar_field!(varint: bool, Default, encode_bool, decode_bool, encoded_len_bool);
scalar_field!(varint: i32, Default, encode_int32, decode_int32, encoded_len_int32);
scalar_field!(varint: i64, Default, encode_int64, decode_int64, encoded_len_int64);
scalar_field!(varint: u32, Default, encode_uint32, decode_uint32, encoded_len_uint32);
scalar_field!(varint: u64, Default, encode_uint64, decode_uint64, encoded_len_uint64);

scalar_field!(32bit: f32, Default, encode_float, decode_float);
scalar_field!(64bit: f64, Default, encode_double, decode_double);

scalar_field!(varint: i32, Signed, encode_int32, decode_sint32, encoded_len_sint32);
scalar_field!(varint: i64, Signed, encode_int64, decode_sint64, encoded_len_sint64);

scalar_field!(32bit: i32, Fixed, encode_sfixed32, decode_sfixed32);
scalar_field!(64bit: i64, Fixed, encode_sfixed64, decode_sfixed64);
scalar_field!(32bit: u32, Fixed, encode_fixed32, decode_fixed32);
scalar_field!(64bit: u64, Fixed, encode_fixed64, decode_fixed64);

scalar_field!(length_delimited: String, encode_string, merge_string);
scalar_field!(length_delimited: Vec<u8>, encode_bytes, merge_bytes);

/*

// repeated
impl <F, E> Field<E> for Vec<F> where F: Field<E> {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        if F::wire_type() == WireType::LengthDelimited {
            // Default repeated encoding.
            for value in self {
                Field::<E>::encode(value, tag, buf);
            }
        } else {
            // Packed repeated encoding.
            if self.is_empty() { return; }
            let len: usize = self.iter().map(<F as Field<E>>::encoded_len).sum();
            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(len as u64, buf);
            for value in self {
                F::encode(value, buf);
            }
        }
    }
    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut vec = Vec::new();
        Self::merge(&mut vec, tag, wire_type, buf)?;
        Ok(vec)
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        let field_wire_type = <F as Field<E>>::wire_type();
        if wire_type == WireType::LengthDelimited && field_wire_type != WireType::LengthDelimited {
            // Packed repeated encoding.
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
            // Default repeated encoding.
            if let Some(value) = Field::<E>::decode_repeated(tag, field_wire_type, buf)? {
                self.push(value);
            }
        }
        Ok(())
    }
    #[inline]
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        if self.is_empty() {
            0
        } else if F::wire_type() == WireType::LengthDelimited {
            // Default repeated encoding.
            self.iter().map(|f| f.encoded_len_with_key(tag)).sum()
        } else {
            // Packed repeated encoding.
            let len: usize = self.iter().map(F::encoded_len).sum();
            key_len(tag) + varint_len(len as u64) + len
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
*/

/// Marker trait for types which can use packed encoding in repeated fields.
pub trait Packed {}
impl Packed for bool {}
impl Packed for i32 {}
impl Packed for i64 {}
impl Packed for u32 {}
impl Packed for u64 {}
impl Packed for f32 {}
impl Packed for f64 {}

// packed repeated
/*
impl <F, E> Field<(PackedRepeated, E)> for Vec<F> where F: Field<E> + Packed {
    fn encode<B>(&self, _buf: &mut B) where B: BufMut {
        unimplemented!()
    }

    #[inline]
    fn encode_with_key<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        if self.is_empty() { return; }
        let len: usize = self.iter().map(<F as Field<E>>::encoded_len).sum();
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(len as u64, buf);
        for value in self {
            F::encode(value, buf);
        }
    }

    #[inline]
    fn decode<B>(tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        let mut vec = Vec::new();
        <Vec<F> as Field<(Default, E)>>::merge(&mut vec, tag, wire_type, buf)?;
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
            key_len(tag) + varint_len(len as u64) + len
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
*/

/*
impl <F, E> Field<E> for F where F: RepeatableField<E> {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, <Self as RepeatableField<E>>::wire_type(), buf);
        <Self as RepeatableField<E>>::encode(buf);
    }
    #[inline]
    fn decode<B>(_tag: u32, wire_type: WireType, buf: &mut B) -> Result<Self> where B: Buf {
        check_wire_type(<Self as RepeatableField<E>>::wire_type(), wire_type)?;
        <Self as RepeatableField<E>>::decode().map(Option::unwrap_or_default)
    }
    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(<Self as RepeatableField<E>>::wire_type(), wire_type)?;
        <Self as RepeatableField<E>>::merge(self, buf)
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        key_len(tag) + <Self as RepeatableField<E>>::encoded_len()
    }
}

// Message
impl <M> RepeatableField for M where M: Message + default::Default {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        // This should never happen, since we check lengths upfront.
        self.encode_length_delimited(buf).expect("failed to encode message: buffer underflow")
    }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<Self> where B: Buf {
        M::decode(buf).map(Option::Some)
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        let len = <M as Message>::encoded_len(self);
        varint_len(len) + len
    }
    #[inline]
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
}
*/

// Trait for types which can be keys in a Protobuf map.
pub trait Key {}
impl Key for bool {}
impl Key for i32 {}
impl Key for i64 {}
impl Key for u32 {}
impl Key for u64 {}
impl Key for String {}

/*
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
    fn encoded_len_with_key(&self, tag: u32) -> usize {
        let key_len = key_len(tag);
        self.iter().map(|(key, value)| {
            let len = key.encoded_len_with_key(1) + value.encoded_len_with_key(2);
            key_len + varint_len(len as u64) + len
        }).sum()
    }

    fn encode<B>(&self, _buf: &mut B) where B: BufMut {
        unimplemented!()
    }
    fn encoded_len(&self) -> usize {
        unimplemented!()
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
}
*/

/*
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

    fn encode<B>(&self, _buf: &mut B) where B: BufMut {
        unimplemented!()
    }
    fn encoded_len(&self) -> usize {
        unimplemented!()
    }
    fn wire_type() -> WireType {
        unimplemented!()
    }
}
*/

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

        let expected_len = value.encoded_len_with_key(tag);

        let mut buf = BytesMut::with_capacity(expected_len);
        value.encode_with_key(tag, &mut buf);
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

        let roundtrip_value = match T::decode(tag, wire_type, &mut buf) {
            Ok(value) => value,
            Err(error) => return TestResult::error(format!("{:?}", error)),
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
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_double(value: Vec<f64>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_float(value: Vec<f32>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_int32(value: Vec<i32>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_int64(value: Vec<i64>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_uint32(value: Vec<u32>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_uint64(value: Vec<u64>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Default)>(value, tag)
        }
        fn packed_sint32(value: Vec<i32>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Signed)>(value, tag)
        }
        fn packed_sint64(value: Vec<i64>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Signed)>(value, tag)
        }
        fn packed_fixed32(value: Vec<u32>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Fixed)>(value, tag)
        }
        fn packed_fixed64(value: Vec<u64>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Fixed)>(value, tag)
        }
        fn packed_sfixed32(value: Vec<i32>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Fixed)>(value, tag)
        }
        fn packed_sfixed64(value: Vec<i64>, tag: u32) -> TestResult {
            check_field::<_, (PackedRepeated, Fixed)>(value, tag)
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
