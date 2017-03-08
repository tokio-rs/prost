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
            u64::read_from(r, limit)?;
        },
        WireType::SixtyFourBit => {
            let mut value: u64 = 0;
            Field::merge_from(&mut value, wire_type, r, limit)?;
        },
        WireType::ThirtyTwoBit => {
            let mut value: u32 = 0;
            Field::merge_from(&mut value, wire_type, r, limit)?;
        },
        WireType::LengthDelimited => {
            let mut value = Vec::new();
            Field::merge_from(&mut value, wire_type, r, limit)?;
        },
    };
    Ok(())
}

#[inline]
fn write_varint(value: u64, w: &mut Write) -> Result<()> {
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

#[inline]
pub fn read_key_from(r: &mut Read, limit: &mut usize) -> Result<(WireType, u32)> {
    let key = u32::read_from(r, limit)?;
    let wire_type = WireType::try_from(key & 0x07)?;
    let tag = key >> 3;
    Ok((wire_type, tag))
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

#[inline]
pub fn check_wire_type(expected: WireType, actual: WireType) -> Result<()> {
    if expected != actual {
        return Err(Error::new(ErrorKind::InvalidData,
                              format!("illegal wire type: {:?} (expected {:?})",
                                      actual, expected)));
    }
    Ok(())
}

pub struct Default;
pub struct Signed;
pub struct Fixed;

/// A field type in a Protobuf message.
pub trait Field<F=Default> {

    /// Writes the field with the provided tag.
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()>;

    /// Reads the field, and merges it into self.
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()>;

    /// Returns the wire length of the field, including the provided tag.
    fn wire_len(&self, tag: u32) -> usize;
}

impl Field for bool {
    fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
        write_key_to(tag, WireType::Varint, w)?;
        let buf = if *self { [1u8] } else { [0u8] };
        w.write_all(&buf)
    }
    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        check_wire_type(WireType::Varint, wire_type)?;
        check_limit(1, limit)?;
        let buf = &mut [0u8];
        r.read_exact(buf)?;
        match buf[0] {
            0 => *self = false,
            1 => *self = true,
            _ => return Err(Error::new(ErrorKind::InvalidData, "invalid bool value")),
        }
        Ok(())
    }
    fn wire_len(&self) -> usize {
        1
    }
}
