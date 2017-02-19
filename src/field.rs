//! Traits for encodable and decodable types.
//!
//! These traits should not be used directly by applications.

use std::cmp::min;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};
use std::u32;
use std::usize;

use byteorder::{
    LittleEndian,
    ReadBytesExt,
    WriteBytesExt,
};

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

#[inline]
pub fn read_key_from(r: &mut Read) -> Result<(WireType, u32)> {
    let key = u32::read_from(r)?;
    let wire_type = WireType::try_from(key & 0x07)?;
    let tag = key >> 3;
    Ok((wire_type, tag))
}

#[inline]
fn write_key_to(tag: u32, wire_type: WireType, w: &mut Write) -> Result<()> {
    debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);
    let key = (tag << 3) | wire_type as u32;
    ScalarField::write_to(&key, w)
}

#[inline]
fn key_len(tag: u32) -> usize {
    let key = tag << 3;
    ScalarField::wire_len(&key)
}

#[inline]
fn check_wire_type(expected: WireType, actual: WireType) -> Result<()> {
    if expected != actual {
        return Err(Error::new(ErrorKind::InvalidData,
                              format!("illegal wire type: {:?} (expected {:?})",
                                      actual, expected)));
    }
    Ok(())
}

/// A field type in a Protobuf message.
pub trait Field {

    /// Writes the field with the provided tag.
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()>;

    /// Reads the field, and merges it into self.
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()>;

    /// Returns the wire length of the field, including the provided tag.
    fn wire_len(&self, tag: u32) -> usize;
}

/// A scalar fixed-width little-endian encoded integer field type.
pub trait FixedField {
    /// Writes the fixed-size field with the provided tag.
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()>;

    /// Reads the fixed-size field, and merges it into self.
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()>;

    /// Returns the wire length of the fixed-size field, including the provided tag.
    fn wire_len(&self, tag: u32) -> usize;
}

/// A scalar, variable-width, ZigZag-encoded, signed integer field type.
pub trait SignedField {
    /// Writes the signed field with the provided tag.
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()>;

    /// Reads the signed field, and merges it into self.
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()>;

    /// Returns the wire length of the signed field, including the provided tag.
    fn wire_len(&self, tag: u32) -> usize;
}

/// A scalar field.
pub trait ScalarField: Sized {

    /// Writes the field without a tag.
    fn write_to(&self, w: &mut Write) -> Result<()>;

    /// Reads an instance of the field.
    fn read_from(r: &mut Read) -> Result<Self>;

    /// Returns the wire type of the field.
    fn wire_type() -> WireType;

    /// Returns the wire length of the field without the tag.
    fn wire_len(&self) -> usize;
}

// This would be better as a blanket impl Field for ScalarField, but that run afoul of coherence
// issues.
macro_rules! scalar_field {
    ($ty:ty) => {
        impl Field for $ty {
            fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
                write_key_to(tag, <Self as ScalarField>::wire_type(), w)?;
                ScalarField::write_to(self, w)
            }

            fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
                check_wire_type(<Self as ScalarField>::wire_type(), wire_type)?;
                *self = ScalarField::read_from(r)?;
                Ok(())
            }

            fn wire_len(&self, tag: u32) -> usize {
                key_len(tag) + ScalarField::wire_len(self)
            }
        }
    }
}

// bool
scalar_field!(bool);
impl ScalarField for bool {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        let buf = if *self { [1u8] } else { [0u8] };
        w.write_all(&buf)
    }
    fn read_from(r: &mut Read) -> Result<bool> {
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
scalar_field!(i32);
impl ScalarField for i32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(*self as u32), w)
    }
    fn read_from(r: &mut Read) -> Result<i32> {
        u32::read_from(r).map(|value| value as _)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(*self as u32))
    }
}

// int64
scalar_field!(i64);
impl ScalarField for i64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(*self as u64), w)
    }
    fn read_from(r: &mut Read) -> Result<i64> {
        u64::read_from(r).map(|value| value as _)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(*self as u64))
    }
}

// uint32
scalar_field!(u32);
impl ScalarField for u32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(*self as u64), w)
    }
    fn read_from(r: &mut Read) -> Result<u32> {
        u64::read_from(r).and_then(|value| {
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
        ScalarField::wire_len(&(*self as u64))
    }
}

// uint64
scalar_field!(u64);
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
    fn read_from(r: &mut Read) -> Result<u64> {
        let mut value = 0;
        let buf = &mut [0u8; 1];
        for i in 0..10 {
            r.read_exact(buf)?;
            let b = buf[0];
            value |= ((b & 0x7F) as u64) << (i * 7);
            if b <= 0x7F {
                return Ok(value);
            }
        }
        Err(Error::new(ErrorKind::InvalidData, "uint64 overflow"))
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
scalar_field!(f32);
impl ScalarField for f32 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_f32::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read) -> Result<f32> {
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
scalar_field!(f64);
impl ScalarField for f64 {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        w.write_f64::<LittleEndian>(*self)
    }
    fn read_from(r: &mut Read) -> Result<f64> {
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
impl SignedField for i32 {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        Field::write_to(&((*self << 1) ^ (*self >> 31)), tag, w)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        Field::merge_from(self, wire_type, r)?;
        *self = (*self >> 1) ^ (-(*self & 1));
        Ok(())
    }
    fn wire_len(&self, tag: u32) -> usize {
        Field::wire_len(&((*self << 1) ^ (*self >> 31)), tag)
    }
}

// sint64
impl SignedField for i64 {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        Field::write_to(&((*self << 1) ^ (*self >> 63)), tag, w)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        Field::merge_from(self, wire_type, r)?;
        *self = (*self >> 1) ^ (-(*self & 1));
        Ok(())
    }
    fn wire_len(&self, tag: u32) -> usize {
        Field::wire_len(&((*self << 1) ^ (*self >> 63)), tag)
    }
}

// fixed32
impl FixedField for u32 {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, WireType::ThirtyTwoBit, w)?;
        w.write_u32::<LittleEndian>(*self)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        check_wire_type(wire_type, WireType::ThirtyTwoBit)?;
        *self = r.read_u32::<LittleEndian>()?;
        Ok(())
    }
    fn wire_len(&self, tag: u32) -> usize {
        key_len(tag) + 4
    }
}

// fixed64
impl FixedField for u64 {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, WireType::SixtyFourBit, w)?;
        w.write_u64::<LittleEndian>(*self)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        check_wire_type(wire_type, WireType::SixtyFourBit)?;
        *self = r.read_u64::<LittleEndian>()?;
        Ok(())
    }
    fn wire_len(&self, tag: u32) -> usize {
        key_len(tag) + 8
    }
}

// sfixed32
impl FixedField for i32 {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, WireType::ThirtyTwoBit, w)?;
        w.write_i32::<LittleEndian>(*self)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        check_wire_type(wire_type, WireType::ThirtyTwoBit)?;
        *self = r.read_i32::<LittleEndian>()?;
        Ok(())
    }
    fn wire_len(&self, tag: u32) -> usize {
        key_len(tag) + 4
    }
}

// sfixed64
impl FixedField for i64 {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, WireType::SixtyFourBit, w)?;
        w.write_i64::<LittleEndian>(*self)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        check_wire_type(wire_type, WireType::SixtyFourBit)?;
        *self = r.read_i64::<LittleEndian>()?;
        Ok(())
    }
    fn wire_len(&self, tag: u32) -> usize {
        key_len(tag) + 8
    }
}

// bytes
scalar_field!(Vec<u8>);
impl ScalarField for Vec<u8> {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(self.len() as u64), w)?;
        w.write_all(self)
    }
    fn read_from(r: &mut Read) -> Result<Vec<u8>> {
        let len = u64::read_from(r)?;
        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidData, "length overflows usize"));
        }
        let len = len as usize;

        // Cap at 4KiB to avoid over-allocating when the length field is bogus.
        let mut value = Vec::with_capacity(min(4096, len));
        let read_len = r.take(len as u64).read_to_end(&mut value)?;

        if read_len == len {
            Ok(value)
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire binary field"))
        }
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(self.len() as u64)) + self.len()
    }
}

// string
scalar_field!(String);
impl ScalarField for String {
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(self.len() as u64), w)?;
        w.write_all(self.as_bytes())
    }
    fn read_from(r: &mut Read) -> Result<String> {
        String::from_utf8(Vec::<u8>::read_from(r)?).map_err(|_| {
            Error::new(ErrorKind::InvalidData, "string does not contain valid UTF-8")
        })
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(self.len() as u64)) + self.len()
    }
}

// message
impl <M> Field for Option<M> where M: Message + Default {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        if let Some(ref m) = *self {
            write_key_to(tag, WireType::LengthDelimited, w)?;
            let len = m.wire_len() as u64;
            ScalarField::write_to(&len, w)?;
            Message::write_to(m, w)?;
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        if self.is_none() {
            *self = Some(M::default());
        }
        Message::merge_from(self.as_mut().unwrap(), r)
    }

    fn wire_len(&self, tag: u32) -> usize {
        match *self {
            Some(ref m) => {
                let len = Message::wire_len(m);
                key_len(tag) + ScalarField::wire_len(&(len as u64)) + len
            },
            None => 0,
        }
    }
}

impl <T> Field for Vec<T> where T: ScalarField {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        match T::wire_type() {
            WireType::Varint => {
                write_key_to(tag, WireType::LengthDelimited, w)?;
                let len: usize = self.iter().map(ScalarField::wire_len).sum();
                ScalarField::write_to(&(len as u64), w)?;
                for value in self {
                    value.write_to(w)?;
                }
            },
            WireType::SixtyFourBit => {
                write_key_to(tag, WireType::LengthDelimited, w)?;
                let len = 8 * self.len() as u64;
                ScalarField::write_to(&len, w)?;
                for value in self {
                    value.write_to(w)?;
                }
            },
            WireType::ThirtyTwoBit => {
                write_key_to(tag, WireType::LengthDelimited, w)?;
                let len = 4 * self.len() as u64;
                ScalarField::write_to(&len, w)?;
                for value in self {
                    value.write_to(w)?;
                }
            },
            WireType::LengthDelimited => for value in self {
                write_key_to(tag, T::wire_type(), w)?;
                ScalarField::write_to(value, w)?;
            },
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        if wire_type == WireType::LengthDelimited && (T::wire_type() == WireType::Varint ||
                                                      T::wire_type() == WireType::SixtyFourBit ||
                                                      T::wire_type() == WireType::ThirtyTwoBit) {
            // Packed encoding.
            let len = u64::read_from(r)?;
            let mut take = r.take(len);
            while take.limit() > 0 {
                self.push(T::read_from(&mut take)?);
            }
        } else {
            // Normal encoding.
            check_wire_type(T::wire_type(), wire_type)?;
            self.push(ScalarField::read_from(r)?);
        }
        Ok(())
    }

    fn wire_len(&self, tag: u32) -> usize {
        let key_len = key_len(tag);
        match T::wire_type() {
            WireType::Varint => {
                let len: usize = self.iter().map(ScalarField::wire_len).sum();
                len + key_len
            }
            WireType::SixtyFourBit => key_len + 8 * self.len(),
            WireType::ThirtyTwoBit => key_len + 4 * self.len(),
            WireType::LengthDelimited => {
                let len: usize = self.iter().map(ScalarField::wire_len).sum();
                key_len * self.len() + len
            },
        }
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
    macro_rules! check_fn {
        ($check_fn:ident, $field_type:ident) => {
            fn $check_fn<T>(value: T, tag: u32) -> TestResult where T: Debug + Default + PartialEq + $field_type {
                if tag > MAX_TAG || tag < MIN_TAG {
                    return TestResult::discard()
                }

                let mut buf = Vec::new();
                if let Err(error) = <T as $field_type>::write_to(&value, tag, &mut buf) {
                    return TestResult::error(format!("write_to failed: {:?}", error));
                };

                let expected_len = <T as $field_type>::wire_len(&value, tag);
                if expected_len != buf.len() {
                    return TestResult::error(format!("wire_len wrong; expected: {}, actual: {}",
                                                     expected_len, buf.len()));
                }

                let encoded_len = buf.len();
                let mut cursor = Cursor::new(buf);
                let (wire_type, decoded_tag) = match read_key_from(&mut cursor) {
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
                    WireType::SixtyFourBit if encoded_len != key_len(tag) + 8 => {
                        return TestResult::error(
                            format!("64bit wire type illegal wire_len: {}, tag: {}",
                                    encoded_len, tag));
                    },
                    WireType::ThirtyTwoBit if encoded_len != key_len(tag) + 4 => {
                        return TestResult::error(
                            format!("64bit wire type illegal wire_len: {}, tag: {}",
                                    encoded_len, tag));
                    },
                    _ => (),
                }

                let mut roundtrip_value = T::default();
                if let Err(error) = $field_type::merge_from(&mut roundtrip_value,
                                                            wire_type,
                                                            &mut cursor) {
                    return TestResult::error(format!("merge_from failed: {:?}", error));
                };

                if value == roundtrip_value {
                    TestResult::passed()
                } else {
                    TestResult::failed()
                }
            }
        }
    }

    check_fn!(check_field, Field);
    check_fn!(check_signed_field, SignedField);
    check_fn!(check_fixed_field, FixedField);

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
            check_field(value, tag)
        }
        fn int64(value: i64, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn uint32(value: u32, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn uint64(value: u64, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn bytes(value: Vec<u8>, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn string(value: String, tag: u32) -> TestResult {
            check_field(value, tag)
        }
        fn sint32(value: i32, tag: u32) -> TestResult {
            check_signed_field(value, tag)
        }
        fn sint64(value: i64, tag: u32) -> TestResult {
            check_signed_field(value, tag)
        }
        fn fixed32(value: u32, tag: u32) -> TestResult {
            check_fixed_field(value, tag)
        }
        fn fixed64(value: u64, tag: u32) -> TestResult {
            check_fixed_field(value, tag)
        }
        fn sfixed32(value: i32, tag: u32) -> TestResult {
            check_fixed_field(value, tag)
        }
        fn sfixed64(value: i64, tag: u32) -> TestResult {
            check_fixed_field(value, tag)
        }
    }

    #[test]
    fn varint() {
        fn check(value: u64, encoded: &[u8]) {
            let mut buf = Vec::new();
            <u64 as ScalarField>::write_to(&value, &mut buf).expect("encoding failed");

            assert_eq!(buf, encoded);

            let roundtrip_value = u64::read_from(&mut Cursor::new(buf)).expect("decoding failed");
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
