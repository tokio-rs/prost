//! Runtime library code for storing unknown fields.

use encoding::{
    bytes,
    decode_varint,
    encode_key,
    encode_varint,
    fixed32,
    fixed64,
    uint64,
    WireType,
};
use bytes::{ Buf, BufMut };
use DecodeError;

/// A set of Protobuf fields that were not recognized during decoding.
///
/// Every Message struct should have an UnknownFieldSet member. This is how
/// messages make sure to not discard unknown data in a decode/encode cycle,
/// which is required by the Protobuf spec.
#[derive(Clone, Debug, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct UnknownFieldSet {
    // The actual data of this struct is wrapped in a Box to ensure that
    // this struct uses only one machine word of memory unless there are
    // unknown fields to store.
    //
    // If the Option is non-empty, the Vec is also non-empty.
    data: Option<Box<Vec<UnknownField>>>,
}

impl UnknownFieldSet {
    /// Adds a field to the UnknownFieldSet. Takes the tag, the wire type and
    /// a buffer that points to where the field itself (excluding the key is).
    ///
    /// Mutates the provided buffer to point to after the unknown field ends.
    #[doc(hidden)]  // Not for external use.
    pub fn skip_unknown_field<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B)
            -> Result<(), DecodeError>  where B : Buf {
        self.push(UnknownField::parse(tag, wire_type, buf)?);
        Ok(())
    }

    fn push(&mut self, field: UnknownField) {
        match self.data {
            Some(ref mut vec) => vec.push(field),
            None => self.data = Some(Box::new(vec![field])),
        }
    }

    #[doc(hidden)]  // Not for external use.
    pub fn encode<B>(&self, buf: &mut B) where B : BufMut {
        match self.data {
            Some(ref vec) => {
                for field in vec.iter() {
                    field.encode(buf);
                }
            },
            None => {},
        }
    }

    #[doc(hidden)]  // Not for external use.
    pub fn encoded_len(&self) -> usize {
        match self.data {
            Some(ref vec) =>
                vec.iter().map(|ref field| field.encoded_len()).sum(),
            None => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UnknownField {
    tag: u32,
    data: UnknownFieldData,
}

impl UnknownField {
    /// Parses an unknown field. Takes the tag, the wire type and a buffer that
    /// points to where the field itself (excluding the key is). Returns the
    /// parsed UnknownField.
    fn parse<B>(tag: u32, wire_type: WireType, buf: &mut B)
            -> Result<UnknownField, DecodeError> where B : Buf {
        let data = match wire_type {
            WireType::Varint =>
                decode_varint(buf).map(|val| UnknownFieldData::Varint(val))?,
            WireType::ThirtyTwoBit => {
                if buf.remaining() < 4 {
                    return Err(DecodeError::new("buffer underflow"));
                }
                UnknownFieldData::ThirtyTwoBit(buf.get_u32_le())
            },
            WireType::SixtyFourBit => {
                if buf.remaining() < 8 {
                    return Err(DecodeError::new("buffer underflow"));
                }
                UnknownFieldData::SixtyFourBit(buf.get_u64_le())
            }
            WireType::LengthDelimited => {
                let mut field_buf = Vec::new();
                ::encoding::bytes::merge(wire_type, &mut field_buf, buf)?;
                UnknownFieldData::LengthDelimited(field_buf)
            }
        };
        Ok(UnknownField{ tag, data })
    }

    fn encode<B>(&self, buf: &mut B) where B : BufMut {
        match &self.data {
            UnknownFieldData::Varint(value) => {
                encode_key(self.tag, WireType::Varint, buf);
                encode_varint(*value, buf);
            },
            UnknownFieldData::SixtyFourBit(value) => {
                encode_key(self.tag, WireType::SixtyFourBit, buf);
                buf.put_u64_le(*value);
            },
            UnknownFieldData::LengthDelimited(value) => {
                encode_key(self.tag, WireType::LengthDelimited, buf);
                encode_varint(value.len() as u64, buf);
                buf.put_slice(value);
            },
            UnknownFieldData::ThirtyTwoBit(value) => {
                encode_key(self.tag, WireType::ThirtyTwoBit, buf);
                buf.put_u32_le(*value);
            },
        }
    }

    fn encoded_len(&self) -> usize {
        match &self.data {
            UnknownFieldData::Varint(value) => {
                uint64::encoded_len(self.tag, value)
            },
            UnknownFieldData::SixtyFourBit(value) => {
                fixed64::encoded_len(self.tag, value)
            },
            UnknownFieldData::LengthDelimited(value) => {
                bytes::encoded_len(self.tag, value)
            },
            UnknownFieldData::ThirtyTwoBit(value) => {
                fixed32::encoded_len(self.tag, value)
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum UnknownFieldData {
    Varint(u64),
    SixtyFourBit(u64),
    LengthDelimited(Vec<u8>),
    ThirtyTwoBit(u32),
}
