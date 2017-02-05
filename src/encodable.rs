//! Traits for encodable and decodable types.
//!
//! These traits should not be used directly by applications.

use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};

use Message;
use scalar;

#[derive(Debug)]
#[repr(u8)]
pub enum WireType {
    Varint = 0,
    SixtyFourBit = 1,
    LengthDelimited = 2,
    ThirtyTwoBit = 5
}

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
}

/// A fixed-width integer field type.
pub trait FixedField {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read;
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write;
    fn wire_type() -> WireType;
}

/// A variable-width signed integer field type with ZigZag encoding
pub trait SignedField {
    fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read;
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write;
    fn wire_type() -> WireType {
        WireType::Varint
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

field!(Field, bool, read_bool, write_bool, Varint);
field!(Field, i32, read_int32, write_int32, Varint);
field!(Field, i64, read_int64, write_int64, Varint);
field!(Field, u32, read_uint32, write_uint32, Varint);
field!(Field, u64, read_uint64, write_uint64, Varint);

field!(Field, f32, read_float, write_float, ThirtyTwoBit);
field!(Field, f64, read_double, write_double, SixtyFourBit);

field!(SignedField, i32, read_sint32, write_sint32, Varint);
field!(SignedField, i64, read_sint64, write_sint64, Varint);

field!(FixedField, i32, read_sfixed32, write_sfixed32, Varint);
field!(FixedField, i64, read_sfixed64, write_sfixed64, Varint);
field!(FixedField, u32, read_fixed32, write_fixed32, Varint);
field!(FixedField, u64, read_fixed64, write_fixed64, Varint);

macro_rules! varlen_field {
    ($ty:ty, $read_fn:ident, $write_fn:ident) => {
        impl Field for $ty {
            fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {
                *self = scalar::$read_fn(r)?;
                Ok(())
            }
            fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
                scalar::$write_fn(w, &self)
            }
            fn wire_type() -> WireType {
                WireType::Varint
            }
        }
    }
}

varlen_field!(Vec<u8>, read_bytes, write_bytes);
varlen_field!(String, read_string, write_string);

/*

impl SignedEncodable for i32 {

}
impl FixedEncodable for i32 {
}

impl SignedEncodable for i64 {}
impl FixedEncodable for i64 {}

impl FixedEncodable for u32 {}

impl FixedEncodable for u64 {}

impl Field for Vec<u8> {
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write { scalar::write_bytes(w, &self[..]) }
    fn merge_from<R>(&mut self, r: &mut R) -> Result<Vec<u8>> where R: Read {
        *self = scalar::read_bytes(r)?;
        Ok(())
    }
}

impl Field for String {
    fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write { scalar::write_string(w, &self[..]) }
    fn merge_from<R>(&mut self, r: &mut R) -> Result<String> where R: Read {
        *self = scalar::read_string(r)?;
        Ok(())
    }
}
*/
