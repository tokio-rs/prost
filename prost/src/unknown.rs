use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use bytes::{Buf, BufMut, Bytes};

use crate::encoding::{self, DecodeContext, WireType};
use crate::error::DecodeErrorKind;
use crate::{DecodeError, Message};

/// A set of unknown fields in a protobuf message.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct UnknownFieldList {
    /// A Map of unknown unique field tags, and the data within them
    fields: BTreeMap<u32, Vec<UnknownField>>,
}

/// An unknown field in a protobuf message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnknownField {
    /// An unknown field with the `Varint` wire type.
    Varint(u64),
    /// An unknown field with the `SixtyFourBit` wire type.
    SixtyFourBit(u64),
    /// An unknown field with the `LengthDelimited` wire type.
    LengthDelimited(Bytes),
    /// An unknown field with the group wire type.
    Group(UnknownFieldList),
    /// An unknown field with the `ThirtyTwoBit` wire type.
    ThirtyTwoBit(u32),
}

impl UnknownFieldList {
    /// Creates an empty [UnknownFieldList].
    pub fn new() -> Self {
        Default::default()
    }

    /// Gets an iterator over the fields contained in this set.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &UnknownField)> {
        self.fields
            .iter()
            .flat_map(|(tag, iter)| core::iter::repeat(*tag).zip(iter))
    }
}

impl Message for UnknownFieldList {
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized,
    {
        for (tag, field) in self.iter() {
            match field {
                UnknownField::Varint(value) => {
                    encoding::encode_key(tag, WireType::Varint, buf);
                    encoding::encode_varint(*value, buf);
                }
                UnknownField::SixtyFourBit(value) => {
                    encoding::encode_key(tag, WireType::SixtyFourBit, buf);
                    buf.put_u64_le(*value);
                }
                UnknownField::LengthDelimited(value) => {
                    encoding::bytes::encode(tag, value, buf);
                }
                UnknownField::Group(value) => {
                    encoding::group::encode(tag, value, buf);
                }
                UnknownField::ThirtyTwoBit(value) => {
                    encoding::encode_key(tag, WireType::ThirtyTwoBit, buf);
                    buf.put_u32_le(*value);
                }
            }
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        Self: Sized,
    {
        let field = match wire_type {
            WireType::Varint => {
                let value = encoding::decode_varint(buf)?;
                UnknownField::Varint(value)
            }
            WireType::SixtyFourBit => {
                if buf.remaining() < (u64::BITS / 8) as usize {
                    return Err(DecodeErrorKind::BufferUnderflow.into());
                }
                //https://protobuf.dev/programming-guides/encoding/
                let return_val = buf.get_u64_le();
                UnknownField::SixtyFourBit(return_val)
            }
            WireType::LengthDelimited => {
                let mut value = Bytes::default();
                encoding::bytes::merge(wire_type, &mut value, buf, ctx)?;
                UnknownField::LengthDelimited(value)
            }
            WireType::StartGroup => {
                let mut value = UnknownFieldList::default();
                encoding::group::merge(tag, wire_type, &mut value, buf, ctx)?;
                UnknownField::Group(value)
            }
            WireType::EndGroup => {
                return Err(DecodeErrorKind::UnexpectedEndGroupTag.into());
            }
            WireType::ThirtyTwoBit => {
                if buf.remaining() < (u32::BITS / 8) as usize {
                    return Err(DecodeErrorKind::BufferUnderflow.into());
                }
                //https://protobuf.dev/programming-guides/encoding/
                let return_val = buf.get_u32_le();
                UnknownField::ThirtyTwoBit(return_val)
            }
        };

        self.fields.entry(tag).or_default().push(field);
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.iter()
            .map(|(tag, field)| match field {
                UnknownField::Varint(value) => {
                    encoding::key_len(tag) + encoding::encoded_len_varint(*value)
                }
                UnknownField::SixtyFourBit(_) => encoding::key_len(tag) + 8,
                UnknownField::LengthDelimited(value) => encoding::bytes::encoded_len(tag, value),
                UnknownField::Group(value) => encoding::group::encoded_len(tag, value),
                UnknownField::ThirtyTwoBit(_) => encoding::key_len(tag) + 4,
            })
            .sum()
    }

    fn clear(&mut self) {
        self.fields.clear();
    }
}
