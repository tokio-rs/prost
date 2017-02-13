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
fn write_key_to(tag: u32, wire_type: WireType, w: &mut Write) -> Result<()> {
    debug_assert!(tag >= MIN_TAG && tag <= MAX_TAG);
    let key = (tag << 3) | wire_type as u32;
    Field::write_to(&key, w)
}

#[inline]
fn key_len(tag: u32) -> usize {
    let key = tag << 3;
    Field::wire_len(&key)
}

/// A field type in a Protobuf message.
pub trait Field {

    /// Writes the field with the provided tag.
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()>;

    /// Reads the field, and merges it into this.
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()>;

    /// Returns the wire length of the field, including the provided tag.
    fn wire_len(&self, tag: u32) -> usize;
}

/// A scalar fixed-width little-endian encoded integer field type.
pub trait FixedField {
    fn write_to(&self, w: &mut Write) -> Result<()>;
    fn merge_from(&mut self, r: &mut Read) -> Result<()>;
    fn wire_type() -> WireType;
    fn wire_len(&self) -> usize;

    fn write_with_tag_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, Self::wire_type(), w)?;
        self.write_to(w)
    }
}

/// A scalar, variable-width, ZigZag-encoded, signed integer field type.
pub trait SignedField {
    fn write_to(&self, w: &mut Write) -> Result<()>;
    fn merge_from(&mut self, r: &mut Read) -> Result<()>;
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize;

    fn write_with_tag_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, Self::wire_type(), w)?;
        self.write_to(w)
    }
}

/// A scalar field.
trait ScalarField {

    /// Writes the field without a tag.
    fn write_to(&self, w: &mut Write) -> Result<()>;

    /// Reads an instance of the field and merges it into this.
    fn merge_from(&mut self, r: &mut Read) -> Result<()>;

    /// Returns the wire type of the field.
    fn wire_type() -> WireType;

    /// Returns the wire length of the field without the tag.
    fn wire_len(&self) -> usize;
}

impl <F> Field for F where F: ScalarField {
    fn write_to(&mut self, tag: u32, r: &mut Read) -> Result<()> {
        write_key_to(tag, ScalarField::wire_type(self), r)?;
        ScalarField::write_to(self, r)
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        if wire_type != Self::wire_type() {
            return Err(Error::new(ErrorKind::InvalidData,
                                  format!("illegal wire type: {:?} (expected {:?})",
                                          wire_type, Self::wire_type())));
        }
        ScalarField::merge_from(r)
    }

    fn wire_len(&self, tag: u32) -> usize {
        key_len(tag) + ScalarField::wire_len(self)
    }
}

// bool
impl ScalarField for bool {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        let buf = &mut [0u8];
        r.read_exact(buf)?;
        match buf[0] {
            0 => *self = false,
            1 => *self = true,
            _ => return Err(Error::new(ErrorKind::InvalidData, "invalid bool value")),
        }
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
impl ScalarField for i32 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        let mut value: u32 = 0;
        ScalarField::merge_from(&mut value, r)?;
        *self = value as _;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(*self as u32), w)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(*self as u32))
    }
}

// int64
impl ScalarField for i64 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        let mut value: u64 = 0;
        ScalarField::merge_from(&mut value, r)?;
        *self = value as _;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(*self as u64), w)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(*self as u64))
    }
}

// uint32
impl ScalarField for u32 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        let mut value: u64 = 0;
        ScalarField::merge_from(&mut value, r)?;
        if value > u32::MAX as u64 {
            Err(Error::new(ErrorKind::InvalidData, "uint32 overflow"))
        } else {
            *self = value as _;
            Ok(())
        }
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(*self as u64), w)
    }
    fn wire_type() -> WireType {
        WireType::Varint
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(*self as u64))
    }
}

// uint64
impl ScalarField for u64 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
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
impl ScalarField for f32 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        *self = r.read_f32::<LittleEndian>()?;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
impl ScalarField for f64 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        *self = r.read_f64::<LittleEndian>()?;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        ScalarField::merge_from(self, r)?;
        *self = (*self >> 1) ^ (-(*self & 1));
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&((*self << 1) ^ (*self >> 31)), w)
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&((*self << 1) ^ (*self >> 31)))
    }
}

// sint64
impl SignedField for i64 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        ScalarField::merge_from(self, r)?;
        *self = (*self >> 1) ^ (-(*self & 1));
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&((*self << 1) ^ (*self >> 63)), w)
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&((*self << 1) ^ (*self >> 63)))
    }
}

// fixed32
impl FixedField for u32 {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        *self = r.read_u32::<LittleEndian>()?;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        *self = r.read_u64::<LittleEndian>()?;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        *self = r.read_i32::<LittleEndian>()?;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        *self = r.read_i64::<LittleEndian>()?;
        Ok(())
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
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
impl ScalarField for Vec<u8> {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        let mut len: u64 = 0;
        ScalarField::merge_from(&mut len, r)?;

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
            Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire binary field"))
        }
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(self.len() as u64), w)?;
        w.write_all(self)
    }
    fn wire_type() -> WireType {
        WireType::LengthDelimited
    }
    fn wire_len(&self) -> usize {
        ScalarField::wire_len(&(self.len() as u64)) + self.len()
    }
}

// string
impl ScalarField for String {
    fn merge_from(&mut self, r: &mut Read) -> Result<()> {
        let mut len: u64 = 0;
        ScalarField::merge_from(&mut len, r)?;

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
            Err(Error::new(ErrorKind::UnexpectedEof, "unable to read entire string field"))
        }
    }
    fn write_to(&self, w: &mut Write) -> Result<()> {
        ScalarField::write_to(&(self.len() as u64), w)?;
        w.write_all(self.as_bytes())
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
            Field::write_to(&len, w)?;
            Message::write_to(m, w)?;
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        if self.is_none() {
            *self = Some(M::default());
        }
        Message::merge_from(self.as_mut().unwrap(), r)
    }

    fn wire_len(&self, tag: u32) -> usize {
        match *self {
            Some(ref m) => key_len(tag) + Message::wire_len(m),
            None => 0,
        }
    }
}

impl <T> Field for Vec<T> where T: ScalarField {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        match T::wire_type() {
            WireType::Varint => {
                write_key_to(tag, WireType::LengthDelimited, w)?;
                let len: usize = self.iter().map(Field::wire_len).sum();
                Field::write_to(&(len as u64), w)?;
                for value in self {
                    value.write_to(w)?;
                }
            },
            WireType::SixtyFourBit => {
                write_key_to(tag, WireType::LengthDelimited, w)?;
                let len = 8 * self.len() as u64;
                Field::write_to(&len, w)?;
                for value in self {
                    value.write_to(w)?;
                }
            },
            WireType::ThirtyTwoBit => {
                write_key_to(tag, WireType::LengthDelimited, w)?;
                let len = 4 * self.len() as u64;
                Field::write_to(&len, w)?;
                for value in self {
                    value.write_to(w)?;
                }
            },
            WireType::LengthDelimited => {
                for value in self {
                    write_key_to(tag, WireType::LengthDelimited, w)?;
                    value.write_to(w)?;
                }
            },
        }
        Ok(())
    }

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read) -> Result<()> {
        match (wire_type, T::wire_type()) {
            // Packed encoding.
            (WireType::LengthDelimited, WireType::Varint) => {

            },
            (WireType::LengthDelimited, WireType::SixtyFourBit) => {

            },
            (WireType::LengthDelimited, WireType::ThirtyTwoBit) => {

            },

            // Unpacked encoding.
            (WireType::Varint, WireType::Varint) => {
            },
            (WireType::SixtyFourBit, WireType::SixtyFourBit) => {
            },
            (WireType::ThirtyTwoBit, WireType::ThirtyTwoBit) => {
            },
            (WireType::LengthDelimited, WireType::LengthDelimited) => {
            },
            (wire_type, expected) => {
                return Err(Error::new(ErrorKind::InvalidData,
                                      format!("illegal wire type for repeated field: {:?} (expected {:?})",
                                              wire_type, expected)));
            }
        }
        Ok(())
    }

    fn wire_len(&self, tag: u32) -> usize {
        let key_len = key_len(tag);
        match T::wire_type() {
            WireType::Varint => {
                let len: usize = self.iter().map(Field::wire_len).sum();
                len + key_len
            }
            WireType::SixtyFourBit => key_len + 8 * self.len(),
            WireType::ThirtyTwoBit => key_len + 4 * self.len(),
            WireType::LengthDelimited => key_len * self.len() + self.iter().map(ScalarField::wire_len).sum(),
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
