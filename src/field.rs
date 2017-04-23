//! This module defines the `Field` trait, which must be implemented by Protobuf
//! encodable values, as well as implementations for the built in types.
//!
//! The `Field` trait should not be used directly by users of the `proto` library.

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
};

use Message;
use encoding::*;

/// A field in a Protobuf message.
///
/// There are two major categories of fields: named types, and compound fields.
/// Named types include numerics like `int32`, length-delimited types like
/// `string` and messages, as well as enumerations. Compound fields include
/// repeated fields, optional fields, map fields, and oneof fields.
///
/// Named types are represented by a sub-trait, `Type`, and compound fields are implemented as
/// blanket impls.
///
/// The `E` type parameter is necessary in order to allow concrete Rust types
/// to implement `Field` multiple times. This is useful for numeric types which
/// can have multiple encodings, and repeated numeric fields which can be packed
/// or unpacked.
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

/// A marker trait for fields which are standalone, named types.
///
/// There are three major categories of types, organized by similarity in
/// encoding:
///
///  * numerics (`int32`, `float`, etc.)
///  * length-delimited types (`string`, `message`s)
///  * enumerations
///
/// Unlike fields in general, `Type` instances can be `optional`, `repeated`,
/// have a default value, and can be values in a map field.
///
/// There is a blanket implementation of `Field` for all concrete `Option<Type>`
/// instances. A blanket implementation of `Field` is not provided for
/// `Vec<Type>` because each class of types (numeric, length-delimited, and
/// enumeration) encode repeated fields slightly differently.
pub trait Type<E = Default> : Field<E> + default::Default {}

impl <T, E> Field<E> for Option<T> where T: Type<E> {
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        if let Some(ref f) = *self {
            <T as Field<E>>::encode(f, tag, buf);
        }
    }
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        if self.is_none() {
            *self = Some(default::Default::default());
        }
        <T as Field<E>>::merge(self.as_mut().unwrap(), tag, wire_type, buf)
    }
    fn encoded_len(&self, tag: u32) -> usize {
        self.as_ref().map(|value| <T as Field<E>>::encoded_len(value, tag)).unwrap_or(0)
    }
}

/// Marker trait for types which can be keys in a Protobuf map.
pub trait KeyType {}
impl KeyType for bool {}
impl KeyType for i32 {}
impl KeyType for i64 {}
impl KeyType for u32 {}
impl KeyType for u64 {}
impl KeyType for String {}

// Map
impl <K, V, EK, EV> Field<(EK, EV)> for HashMap<K, V>
where K: Eq + Hash + KeyType + Type<EK>,
      V: Type {

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
where K: Eq + Hash + KeyType + Type,
      V: Type {
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

struct Enumeration;
impl <T> Field<Enumeration> for Vec<T> where T: Type {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        unimplemented!()
    }
    #[inline]
    fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        unimplemented!()
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        unimplemented!()
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
