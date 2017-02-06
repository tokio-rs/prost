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

#[derive(Debug)]
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

/// A valid field type in a Protobuf message.
pub trait Field {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read;
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write;
    fn wire_type() -> WireType;
    fn wire_len(&self) -> usize;

    fn write_with_tag_to<W>(&self, tag: u32, w: &mut W) -> Result<()> where W: Write {
        debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);

        let key = (tag << 3) | <Self as Field>::wire_type() as u32;
        Field::write_to(&key, w)?;
        Field::write_to(self, w)
    }
}

/// A fixed-width little-endian encoded integer field type.
pub trait FixedField {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read;
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write;
    fn wire_type() -> WireType;
    fn wire_len(&self) -> usize;

    fn write_with_tag_to<W>(&self, tag: u32, w: &mut W) -> Result<()> where W: Write {
        debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);

        let key = (tag << 3) | <Self as FixedField>::wire_type() as u32;
        Field::write_to(&key, w)?;
        FixedField::write_to(self, w)
    }
}

/// A variable-width, ZigZag-encoded, signed integer field type.
pub trait SignedField : Field {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read;
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write;
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize;

    fn write_with_tag_to<W>(&self, tag: u32, w: &mut W) -> Result<()> where W: Write {
        debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);

        let key = (tag << 3) | <Self as SignedField>::wire_type() as u32;
        Field::write_to(&key, w)?;
        SignedField::write_to(self, w)
    }
}

macro_rules! field {
    ($field_type:ident, $ty:ty, $read_fn:ident, $write_fn:ident, $wire_type:ident) => {
        impl $field_type for $ty {
            fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
                *self = scalar::$read_fn(r)?;
                Ok(())
            }
            fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
                scalar::$write_fn(w, *self)
            }
            fn wire_type() -> WireType {
                WireType::$wire_type
            }
        }
    }
}

// bool
impl Field for bool {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        let buf = &mut [0u8];
        r.read_exact(buf)?;
        match buf[0] {
            0 => *self = false,
            1 => *self = true,
            _ => return Err(Error::new(ErrorKind::InvalidData, "invalid bool value")),
        }
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        let buf = if *self { [1u8] } else { [0u8] };
        w.write_all(&buf)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        1
    }
}

// int32
impl Field for i32 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        let mut value: u32 = 0;
        Field::merge_from(&mut value, r)?;
        *self = value as _;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&(*self as u32), w)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&(*self as u32))
    }
}

// int64
impl Field for i64 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        let mut value: u64 = 0;
        Field::merge_from(&mut value, r)?;
        *self = value as _;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&(*self as u64), w)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&(*self as u64))
    }
}

// uint32
impl Field for u32 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        let mut value: u64 = 0;
        Field::merge_from(&mut value, r)?;
        if value > u32::MAX as u64 {
            Err(Error::new(ErrorKind::InvalidData, "uint32 overflow"))
        } else {
            *self = value as _;
            Ok(())
        }
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&(*self as u64), w)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&(*self as u64))
    }
}

// uint64
impl Field for u64 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = 0;
        let buf = &mut [0u8; 1];
        for i in 0..10 {
            r.read_exact(buf)?;
            let b = buf[0];
            *self |= ((b & 0x7F) as u64) << (i * 7);
            if b <= 0x7F {
                return Ok(());
            }
        }
        Err(Error::new(ErrorKind::InvalidData, "uint64 overflow"))
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
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
impl Field for f32 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = r.read_f32::<LittleEndian>()?;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        w.write_f32::<LittleEndian>(*self)
    }
    fn wire_type() -> WireType {
        WireType::ThirtyTwoBit
    }
    fn wire_len(&self) -> usize {
        4
    }
}

// double
impl Field for f64 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = r.read_f64::<LittleEndian>()?;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        w.write_f64::<LittleEndian>(*self)
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
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        Field::merge_from(self, r)?;
        *self = (*self >> 1) ^ (-(*self & 1));
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&((*self << 1) ^ (*self >> 31)), w)
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&((*self << 1) ^ (*self >> 31)))
    }
}

// sint64
impl SignedField for i64 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        Field::merge_from(self, r)?;
        *self = (*self >> 1) ^ (-(*self & 1));
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&((*self << 1) ^ (*self >> 63)), w)
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&((*self << 1) ^ (*self >> 63)))
    }
}

// fixed32
impl FixedField for u32 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = r.read_u32::<LittleEndian>()?;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        w.write_u32::<LittleEndian>(*self)
    }
    fn wire_type() -> WireType {
        WireType::ThirtyTwoBit
    }
    fn wire_len(&self) -> usize {
        4
    }
}

// fixed64
impl FixedField for u64 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = r.read_u64::<LittleEndian>()?;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        w.write_u64::<LittleEndian>(*self)
    }
    fn wire_type() -> WireType {
        WireType::SixtyFourBit
    }
    fn wire_len(&self) -> usize {
        8
    }
}

// sfixed32
impl FixedField for i32 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = r.read_i32::<LittleEndian>()?;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        w.write_i32::<LittleEndian>(*self)
    }
    fn wire_type() -> WireType {
        WireType::ThirtyTwoBit
    }
    fn wire_len(&self) -> usize {
        4
    }
}

// sfixed64
impl FixedField for i64 {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        *self = r.read_i64::<LittleEndian>()?;
        Ok(())
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        w.write_i64::<LittleEndian>(*self)
    }
    fn wire_type() -> WireType {
        WireType::SixtyFourBit
    }
    fn wire_len(&self) -> usize {
        8
    }
}

// bytes
impl Field for Vec<u8> {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        let mut len: u64 = 0;
        Field::merge_from(&mut len, r)?;

        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidData, "length overflows usize"));
        }
        let len = len as usize;

        self.clear();

        // Cap at 4KiB to avoid over-allocating when the length field is bogus.
        self.reserve(min(4096, len));
        let read_len = r.take(len as u64).read_to_end(self)?;

        if read_len == len {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire string"))
        }
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&(self.len() as u64), w)?;
        w.write_all(self)
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&(self.len() as u64)) + self.len()
    }
}

// string
impl Field for String {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
        let mut len: u64 = 0;
        Field::merge_from(&mut len, r)?;

        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidData, "length overflows usize"));
        }
        let len = len as usize;

        self.clear();

        // Cap at 4KiB to avoid over-allocating when the length field is bogus.
        self.reserve(min(4096, len));
        let read_len = r.take(len as u64).read_to_string(self)?;

        if read_len == len {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire string"))
        }
    }
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
        Field::write_to(&(self.len() as u64), w)?;
        w.write_all(self.as_bytes())
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        Field::wire_len(&(self.len() as u64)) + self.len()
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
            fn $check_fn<T>(value: T) -> TestResult where T: Debug + Default + PartialEq + $field_type {
                let mut buf = Vec::new();
                if let Err(error) = <T as $field_type>::write_to(&value, &mut buf) {
                    return TestResult::error(format!("write_to failed: {:?}", error));
                };

                let expected_len = <T as $field_type>::wire_len(&value);
                if expected_len != buf.len() {
                    return TestResult::error(format!("wire_len wrong; expected: {}, actual: {}",
                                                     expected_len, buf.len()));
                }

                match <T as $field_type>::wire_type() {
                    WireType::SixtyFourBit if buf.len() != 8 => {
                        return TestResult::error(format!("64bit wire type illegal wire_len: {}",
                                                         buf.len()));
                    },
                    WireType::ThirtyTwoBit if buf.len() != 4 => {
                        return TestResult::error(format!("64bit wire type illegal wire_len: {}",
                                                         buf.len()));
                    },
                    _ => (),
                }

                let mut roundtrip_value = T::default();
                if let Err(error) = $field_type::merge_from(&mut roundtrip_value, &mut Cursor::new(buf)) {
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
        fn bool(value: bool) -> TestResult {
            check_field(value)
        }
        fn double(value: f64) -> TestResult {
            check_field(value)
        }
        fn float(value: f32) -> TestResult {
            check_field(value)
        }
        fn int32(value: i32) -> TestResult {
            check_field(value)
        }
        fn int64(value: i64) -> TestResult {
            check_field(value)
        }
        fn uint32(value: u32) -> TestResult {
            check_field(value)
        }
        fn uint64(value: u64) -> TestResult {
            check_field(value)
        }
        fn bytes(value: Vec<u8>) -> TestResult {
            check_field(value)
        }
        fn string(value: String) -> TestResult {
            check_field(value)
        }
        fn sint32(value: i32) -> TestResult {
            check_signed_field(value)
        }
        fn sint64(value: i64) -> TestResult {
            check_signed_field(value)
        }
        fn fixed32(value: u32 ) -> TestResult {
            check_fixed_field(value)
        }
        fn fixed64(value: u64 ) -> TestResult {
            check_fixed_field(value)
        }
        fn sfixed32(value: i32) -> TestResult {
            check_fixed_field(value)
        }
        fn sfixed64(value: i64) -> TestResult {
            check_fixed_field(value)
        }
    }

    #[test]
    fn varint() {
        fn check(value: u64, encoded: &[u8]) {
            let mut buf = Vec::new();
            Field::write_to(&value, &mut buf).expect("encoding failed");

            assert_eq!(buf, encoded);

            let mut roundtrip_value: u64 = 0;
            Field::merge_from(&mut roundtrip_value, &mut Cursor::new(buf)).expect("decoding failed");
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
