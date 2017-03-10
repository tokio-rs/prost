//! Traits for encodable and decodable types.
//!
//! These traits should not be used directly by applications.

use std::cmp::min;
use std::default;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};
use std::u32;
use std::usize;
use std::collections::HashMap;
use std::hash::Hash;

use byteorder::{
    LittleEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use check_limit;
use Message;

#[derive(Debug, PartialEq)]
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
    pub fn try_from(val: u32) -> Result<WireType> {
        match val {
            0 => Ok(WireType::Varint),
            1 => Ok(WireType::SixtyFourBit),
            2 => Ok(WireType::LengthDelimited),
            5 => Ok(WireType::ThirtyTwoBit),
            _ => Err(Error::new(ErrorKind::InvalidData,
                                format!("illegal wire type value {}", val))),
        }
    }
}

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

#[inline]
pub fn read_key_from(r: &mut Read, limit: &mut usize) -> Result<(WireType, u32)> {
    let key = <u32 as ScalarField>::read_from(r, limit)?;
    let wire_type = WireType::try_from(key & 0x07)?;
    let tag = key >> 3;
    Ok((wire_type, tag))
}

#[inline]
fn write_key_to(tag: u32, wire_type: WireType, w: &mut Write) -> Result<()> {
    debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);
    let key = (tag << 3) | wire_type as u32;
    <u32 as ScalarField>::write_to(&key, w)
}

#[inline]
fn key_len(tag: u32) -> usize {
    let key = tag << 3;
    ScalarField::<Default>::wire_len(&key)
}

#[inline]
pub fn check_wire_type(expected: WireType, actual: WireType) -> Result<()> {
    if expected != actual {
        return Err(Error::new(ErrorKind::InvalidData,
                              format!("illegal wire type: {:?} (expected {:?})",
                                      actual, expected)));
    }
    Ok(())
}

/// A type indicating that the default Protobuf encoding is used for a field.
pub struct Default;
/// A type indicating that the integer field should use variable-width,
/// ZigZag encoded, signed encoding.
pub struct Signed;
/// A type indicating that the integer field should use fixed-width encoding.
pub struct Fixed;

/// A field type in a Protobuf message.
pub trait Field<E=Default> {

    /// Writes the field with the provided tag.
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()>;

    /// Reads the field, and merges it into self.
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()>;

    /// Returns the wire length of the field, including the provided tag.
    fn wire_len(&self, tag: u32) -> usize;
}

/// A scalar field.
pub trait ScalarField<E=Default>: Sized {

    /// Writes the field without a tag.
    fn write_to(&self, w: &mut Write) -> Result<()>;

    /// Reads an instance of the field.
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<Self>;

    /// Returns the wire type of the field.
    fn wire_type() -> WireType;

    /// Returns the wire length of the field without the tag.
    fn wire_len(&self) -> usize;
}

// This would be better as a blanket impl Field for ScalarField,
// but that runs afould of coherence.
macro_rules! scalar_field {
    ($ty:ty, $e:ty) => {
        impl Field<$e> for $ty {
            fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
                write_key_to(tag, <Self as ScalarField<$e>>::wire_type(), w)?;
                ScalarField::<$e>::write_to(self, w)
            }

            fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
                check_wire_type(<Self as ScalarField<$e>>::wire_type(), wire_type)?;
                *self = ScalarField::<$e>::read_from(r, limit)?;
                Ok(())
            }

            fn wire_len(&self, tag: u32) -> usize {
                key_len(tag) + ScalarField::<$e>::wire_len(self)
            }
        }

        impl Field<$e> for Vec<$ty> {
            fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
                match <$ty as ScalarField<$e>>::wire_type() {
                    WireType::Varint => {
                        write_key_to(tag, WireType::LengthDelimited, w)?;
                        let len: usize = self.iter().map(ScalarField::<$e>::wire_len).sum();
                        <u64 as ScalarField>::write_to(&(len as u64), w)?;
                        for value in self {
                            ScalarField::<$e>::write_to(value, w)?;
                        }
                    },
                    WireType::SixtyFourBit => {
                        write_key_to(tag, WireType::LengthDelimited, w)?;
                        let len = 8 * self.len() as u64;
                        <u64 as ScalarField>::write_to(&len, w)?;
                        for value in self {
                            ScalarField::<$e>::write_to(value, w)?;
                        }
                    },
                    WireType::ThirtyTwoBit => {
                        write_key_to(tag, WireType::LengthDelimited, w)?;
                        let len = 4 * self.len() as u64;
                        <u64 as ScalarField>::write_to(&len, w)?;
                        for value in self {
                            ScalarField::<$e>::write_to(value, w)?;
                        }
                    },
                    WireType::LengthDelimited => for value in self {
                        write_key_to(tag, WireType::LengthDelimited, w)?;
                        ScalarField::<$e>::write_to(value, w)?;
                    },
                }
                Ok(())
            }

            fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
                if wire_type == WireType::LengthDelimited && (<$ty as ScalarField<$e>>::wire_type() == WireType::Varint ||
                                                              <$ty as ScalarField<$e>>::wire_type() == WireType::SixtyFourBit ||
                                                              <$ty as ScalarField<$e>>::wire_type() == WireType::ThirtyTwoBit) {
                    // Packed encoding.
                    let len = <u64 as ScalarField>::read_from(r, limit)?;
                    if len > usize::MAX as u64 {
                        return Err(Error::new(ErrorKind::InvalidData,
                                              "packed length overflows usize"));
                    }
                    check_limit(len as usize, limit)?;
                    let mut remaining = len as usize;
                    while remaining > 0 {
                        self.push(ScalarField::<$e>::read_from(r, &mut remaining)?);
                    }
                } else {
                    // Normal encoding.
                    check_wire_type(<$ty as ScalarField<$e>>::wire_type(), wire_type)?;
                    self.push(ScalarField::<$e>::read_from(r, limit)?);
                }
                Ok(())
            }

            fn wire_len(&self, tag: u32) -> usize {
                let key_len = key_len(tag);
                match <$ty as ScalarField<$e>>::wire_type() {
                    WireType::Varint => {
                        let len: usize = self.iter().map(ScalarField::<$e>::wire_len).sum();
                        len + key_len
                    }
                    WireType::SixtyFourBit => key_len + 8 * self.len(),
                    WireType::ThirtyTwoBit => key_len + 4 * self.len(),
                    WireType::LengthDelimited => {
                        let len: usize = self.iter().map(ScalarField::<$e>::wire_len).sum();
                        key_len * self.len() + len
                    },
                }
            }
        }
    }
}

// bool
scalar_field!(bool, Default);
impl ScalarField for bool {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        let buf = if *self { [1u8] } else { [0u8] };
        w.write_all(&buf)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<bool> {
        check_limit(1, limit)?;
        let buf = &mut [0u8];
        r.read_exact(buf)?;
        match buf[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::new(ErrorKind::InvalidData, "invalid bool value")),
        }
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        1
    }
}

// int32
scalar_field!(i32, Default);
impl ScalarField for i32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::<Default>::write_to(&(*self as u32), w)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<i32> {
        <u32 as ScalarField>::read_from(r, limit).map(|value| value as _)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        ScalarField::<Default>::wire_len(&(*self as u32))
    }
}

// int64
scalar_field!(i64, Default);
impl ScalarField for i64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        <u64 as ScalarField>::write_to(&(*self as u64), w)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<i64> {
        <u64 as ScalarField>::read_from(r, limit).map(|value| value as _)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        <u64 as ScalarField>::wire_len(&(*self as u64))
    }
}

// uint32
scalar_field!(u32, Default);
impl ScalarField for u32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        <u64 as ScalarField>::write_to(&(*self as u64), w)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<u32> {
        <u64 as ScalarField>::read_from(r, limit).and_then(|value| {
            if value > u32::MAX as u64 {
                Err(Error::new(ErrorKind::InvalidData, "uint32 overflow"))
            } else {
                Ok(value as _)
            }
        })
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        <u64 as ScalarField>::wire_len(&(*self as u64))
    }
}

// uint64
scalar_field!(u64, Default);
impl ScalarField for u64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        let mut value = *self;
        let mut buf = &mut [0u8; 10];
        let mut i = 0;
        while value >= 0x80 {
            buf[i] = ((value & 0x7F) | 0x80) as u8;
            value >>= 7;
            i += 1;
        }
        buf[i] = value as u8;
        w.write_all(&buf[..i+1])
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<u64> {
        let mut value = 0;
        let buf = &mut [0u8; 1];
        for i in 0..min(10, *limit) {
            r.read_exact(buf)?;
            let b = buf[0];
            value |= ((b & 0x7F) as u64) << (i * 7);
            if b <= 0x7F {
                *limit -= i + 1;
                return Ok(value);
            }
        }
        if *limit < 9 {
            Err(Error::new(ErrorKind::InvalidData, "read limit exceeded"))
        } else {
            Err(Error::new(ErrorKind::InvalidData, "varint overflow"))
        }
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        if *self < 1 <<  7 { return 1; }
        if *self < 1 << 14 { return 2; }
        if *self < 1 << 21 { return 3; }
        if *self < 1 << 28 { return 4; }
        if *self < 1 << 35 { return 5; }
        if *self < 1 << 42 { return 6; }
        if *self < 1 << 49 { return 7; }
        if *self < 1 << 56 { return 8; }
        if *self < 1 << 63 { return 9; }
        10
    }
}

// float
scalar_field!(f32, Default);
impl ScalarField for f32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_f32::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<f32> {
        check_limit(4, limit)?;
        r.read_f32::<LittleEndian>()
    }
    fn wire_type() -> WireType {
        WireType::ThirtyTwoBit
    }
    fn wire_len(&self) -> usize {
        4
    }
}

// double
scalar_field!(f64, Default);
impl ScalarField for f64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_f64::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<f64> {
        check_limit(8, limit)?;
        r.read_f64::<LittleEndian>()
    }
    fn wire_type() -> WireType {
        WireType::SixtyFourBit
    }
    fn wire_len(&self) -> usize {
        8
    }
}

// sint32
scalar_field!(i32, Signed);
impl ScalarField<Signed> for i32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        <i32 as ScalarField>::write_to(&((*self << 1) ^ (*self >> 31)), w)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<i32> {
        let value = <i32 as ScalarField>::read_from(r, limit)?;
        Ok((value >> 1) ^ (-(value & 1)))
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        <i32 as ScalarField>::wire_len(&((*self << 1) ^ (*self >> 31)))
    }
}

// sint64
scalar_field!(i64, Signed);
impl ScalarField<Signed> for i64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        <i64 as ScalarField>::write_to(&((*self << 1) ^ (*self >> 63)), w)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<i64> {
        let value = <i64 as ScalarField>::read_from(r, limit)?;
        Ok((value >> 1) ^ (-(value & 1)))
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        <i64 as ScalarField>::wire_len(&((*self << 1) ^ (*self >> 63)))
    }
}

// fixed32
scalar_field!(u32, Fixed);
impl ScalarField<Fixed> for u32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_u32::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<u32> {
        check_limit(4, limit)?;
        r.read_u32::<LittleEndian>()
    }
    fn wire_type() -> WireType {
        WireType::ThirtyTwoBit
    }
    fn wire_len(&self) -> usize {
        4
    }
}

// fixed64
scalar_field!(u64, Fixed);
impl ScalarField<Fixed> for u64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_u64::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<u64> {
        check_limit(8, limit)?;
        r.read_u64::<LittleEndian>()
    }
    fn wire_type() -> WireType {
        WireType::SixtyFourBit
    }
    fn wire_len(&self) -> usize {
        8
    }
}

// sfixed32
scalar_field!(i32, Fixed);
impl ScalarField<Fixed> for i32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_i32::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<i32> {
        check_limit(4, limit)?;
        r.read_i32::<LittleEndian>()
    }
    fn wire_type() -> WireType {
        WireType::ThirtyTwoBit
    }
    fn wire_len(&self) -> usize {
        4
    }
}

// sfixed64
scalar_field!(i64, Fixed);
impl ScalarField<Fixed> for i64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_i64::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<i64> {
        check_limit(8, limit)?;
        r.read_i64::<LittleEndian>()
    }
    fn wire_type() -> WireType {
        WireType::SixtyFourBit
    }
    fn wire_len(&self) -> usize {
        8
    }
}

// bytes
scalar_field!(Vec<u8>, Default);
impl ScalarField for Vec<u8> {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        <u64 as ScalarField>::write_to(&(self.len() as u64), w)?;
        w.write_all(self)
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<Vec<u8>> {
        let len = <u64 as ScalarField>::read_from(r, limit)?;
        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidData, "length overflows usize"));
        }
        let len = len as usize;
        check_limit(len, limit)?;

        let mut value = Vec::with_capacity(len);
        let read_len = r.take(len as u64).read_to_end(&mut value)?;

        if read_len == len {
            Ok(value)
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire field"))
        }
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        <u64 as ScalarField>::wire_len(&(self.len() as u64)) + self.len()
    }
}

// string
scalar_field!(String, Default);
impl ScalarField for String {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        <u64 as ScalarField>::write_to(&(self.len() as u64), w)?;
        w.write_all(self.as_bytes())
    }
    fn read_from(r: &mut Read, limit: &mut usize) -> Result<String> {
        String::from_utf8(Vec::<u8>::read_from(r, limit)?).map_err(|_| {
            Error::new(ErrorKind::InvalidData, "string does not contain valid UTF-8")
        })
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        <u64 as ScalarField>::wire_len(&(self.len() as u64)) + self.len()
    }
}


// Message
impl <M> Field for Option<M> where M: Message + default::Default {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        if let Some(ref m) = *self {
            write_key_to(tag, WireType::LengthDelimited, w)?;
            m.write_length_delimited_to(w)?;
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        if self.is_none() {
            *self = Some(M::default());
        }
        self.as_mut().unwrap().merge_length_delimited_from(r, limit)
    }

    fn wire_len(&self, tag: u32) -> usize {
        match *self {
            Some(ref m) => {
                let len = Message::wire_len(m);
                key_len(tag) + <u64 as ScalarField>::wire_len(&(len as u64)) + len
            },
            None => 0,
        }
    }
}

// Boxed Message
impl <M> Field for Box<Option<M>> where M: Message + default::Default {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        Field::write_to(&**self, tag, w)
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        Field::merge_from(&mut **self, wire_type, r, limit)
    }

    fn wire_len(&self, tag: u32) -> usize {
        Field::wire_len(&**self, tag)
    }
}

// Repeated Message
impl <M> Field for Vec<M> where M: Message + default::Default {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        for message in self {
            write_key_to(tag, WireType::LengthDelimited, w)?;
            message.write_length_delimited_to(w)?;
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut m = M::default();
        m.merge_length_delimited_from(r, limit)?;
        self.push(m);
        Ok(())
    }

    fn wire_len(&self, tag: u32) -> usize {
        let len: usize = self.iter().map(Message::wire_len).sum();
        key_len(tag) * self.len() + len
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

    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        for (key, value) in self.iter() {
            write_key_to(tag, WireType::LengthDelimited, w)?;

            let len = Field::<EK>::wire_len(key, 1) + Field::<EV>::wire_len(value, 2);
            <u64 as ScalarField>::write_to(&(len as u64), w)?;

            Field::<EK>::write_to(key, 1, w)?;
            Field::<EV>::write_to(value, 2, w)?;
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = <u64 as ScalarField>::read_from(r, limit)?;
        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "map length overflows usize"));
        }
        check_limit(len as usize, limit)?;

        let mut key = None;
        let mut value = None;

        let mut limit = len as usize;
        while limit > 0 {
            let (wire_type, tag) = read_key_from(r, &mut limit)?;
            match tag {
                1 => {
                    let mut k = K::default();
                    <K as Field<EK>>::merge_from(&mut k, wire_type, r, &mut limit)?;
                    key = Some(k);
                },
                2 => {
                    let mut v = V::default();
                    <V as Field<EV>>::merge_from(&mut v, wire_type, r, &mut limit)?;
                    value = Some(v);
                },
                _ => return Err(Error::new(ErrorKind::InvalidData,
                                           format!("map entry contains unexpected field; tag: {:?}, wire type: {:?}",
                                                   tag, wire_type))),
            }
        }

        match (key, value) {
            (Some(key), Some(value)) => {
                self.insert(key, value);
            },
            (Some(_), None) => return Err(Error::new(ErrorKind::InvalidData,
                                                     "map entry is missing a key")),
            (None, Some(_)) => return Err(Error::new(ErrorKind::InvalidData,
                                                     "map entry is missing a value")),
            (None, None) => return Err(Error::new(ErrorKind::InvalidData,
                                                  "map entry is missing a key and a value")),
        }

        Ok(())
    }

    fn wire_len(&self, tag: u32) -> usize {
        self.iter().fold(key_len(tag), |acc, (key, value)| {
            acc + Field::<EK>::wire_len(key, 1) + Field::<EV>::wire_len(value, 2)
        })
    }
}

impl <K, V> Field<Default> for HashMap<K, V>
where K: default::Default + Eq + Hash + Key + Field<Default>,
      V: default::Default + Field<Default> {

    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        <HashMap<K, V> as Field<(Default, Default)>>::write_to(self, tag, w)
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        <HashMap<K, V> as Field<(Default, Default)>>::merge_from(self, wire_type, r, limit)
    }

    fn wire_len(&self, tag: u32) -> usize {
        <HashMap<K, V> as Field<(Default, Default)>>::wire_len(self, tag)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use std::fmt::Debug;
    use std::io::Cursor;

    use quickcheck::TestResult;

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

        let expected_len = <T as Field<E>>::wire_len(&value, tag);
        if expected_len != buf.len() {
            return TestResult::error(format!("wire_len wrong; expected: {}, actual: {}",
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
                    format!("64bit wire type illegal wire_len: {}, tag: {}",
                            encoded_len, tag));
            },
            WireType::ThirtyTwoBit if encoded_len != 4 => {
                return TestResult::error(
                    format!("32bit wire type illegal wire_len: {}, tag: {}",
                            encoded_len, tag));
            },
            _ => (),
        }

        let mut roundtrip_value = T::default();
        if let Err(error) = <T as Field<E>>::merge_from(&mut roundtrip_value,
                                                        wire_type,
                                                        &mut cursor,
                                                        &mut encoded_len) {
            return TestResult::error(format!("merge_from failed: {:?}", error));
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
            <u64 as ScalarField>::write_to(&value, &mut buf).expect("encoding failed");

            assert_eq!(buf, encoded);

            let mut limit = encoded.len();
            let roundtrip_value = <u64 as ScalarField>::read_from(&mut Cursor::new(buf), &mut limit).expect("decoding failed");
            assert_eq!(value, roundtrip_value);
            assert_eq!(limit, 0);
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
