use alloc::collections::btree_map::{self, BTreeMap};
use alloc::vec::Vec;
use core::slice;

use bytes::{Buf, BufMut, Bytes};

use crate::encoding::{self, DecodeContext, WireType};
use crate::{DecodeError, Message};

/// A set of unknown fields in a protobuf message.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct UnknownFieldSet {
    fields: BTreeMap<u32, Vec<UnknownField>>,
}

/// An unknown field in a protobuf message.
#[derive(Debug, Clone, PartialEq)]
pub enum UnknownField {
    /// An unknown field with the `Varint` wire type.
    Varint(u64),
    /// An unknown field with the `SixtyFourBit` wire type.
    SixtyFourBit([u8; 8]),
    /// An unknown field with the `LengthDelimited` wire type.
    LengthDelimited(Bytes),
    /// An unknown field with the group wire type.
    Group(UnknownFieldSet),
    /// An unknown field with the `ThirtyTwoBit` wire type.
    ThirtyTwoBit([u8; 4]),
}

/// An iterator over the fields of an [UnknownFieldSet].
#[derive(Debug)]
pub struct UnknownFieldIter<'a> {
    tags_iter: btree_map::Iter<'a, u32, Vec<UnknownField>>,
    current_tag: Option<(u32, slice::Iter<'a, UnknownField>)>,
}

impl UnknownFieldSet {
    /// Creates an empty [UnknownFieldSet].
    pub fn new() -> Self {
        Default::default()
    }

    /// Gets an iterator over the fields contained in this set.
    pub fn iter(&self) -> UnknownFieldIter<'_> {
        UnknownFieldIter {
            tags_iter: self.fields.iter(),
            current_tag: None,
        }
    }
}

impl<'a> Iterator for UnknownFieldIter<'a> {
    type Item = (u32, &'a UnknownField);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((tag, iter)) = &mut self.current_tag {
                if let Some(value) = iter.next() {
                    return Some((*tag, value));
                } else {
                    self.current_tag = None;
                }
            }
            if let Some((tag, values)) = self.tags_iter.next() {
                self.current_tag = Some((*tag, values.iter()));
            } else {
                return None;
            }
        }
    }
}

impl Message for UnknownFieldSet {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for (&tag, fields) in &self.fields {
            for field in fields {
                match field {
                    UnknownField::Varint(value) => {
                        encoding::encode_key(tag, WireType::Varint, buf);
                        encoding::encode_varint(*value, buf);
                    }
                    UnknownField::SixtyFourBit(value) => {
                        encoding::encode_key(tag, WireType::SixtyFourBit, buf);
                        buf.put_slice(value);
                    }
                    UnknownField::LengthDelimited(value) => {
                        encoding::bytes::encode(tag, value, buf);
                    }
                    UnknownField::Group(value) => {
                        encoding::group::encode(tag, value, buf);
                    }
                    UnknownField::ThirtyTwoBit(value) => {
                        encoding::encode_key(tag, WireType::ThirtyTwoBit, buf);
                        buf.put_slice(value);
                    }
                }
            }
        }
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        let field = match wire_type {
            WireType::Varint => {
                let value = encoding::decode_varint(buf)?;
                UnknownField::Varint(value)
            }
            WireType::SixtyFourBit => {
                let mut value = [0; 8];
                if buf.remaining() < value.len() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                buf.copy_to_slice(&mut value);
                UnknownField::SixtyFourBit(value)
            }
            WireType::LengthDelimited => {
                let mut value = Bytes::default();
                encoding::bytes::merge(wire_type, &mut value, buf, ctx)?;
                UnknownField::LengthDelimited(value)
            }
            WireType::StartGroup => {
                let mut value = UnknownFieldSet::default();
                encoding::group::merge(tag, wire_type, &mut value, buf, ctx)?;
                UnknownField::Group(value)
            }
            WireType::EndGroup => {
                return Err(DecodeError::new("unexpected end group tag"));
            }
            WireType::ThirtyTwoBit => {
                let mut value = [0; 4];
                if buf.remaining() < value.len() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                buf.copy_to_slice(&mut value);
                UnknownField::ThirtyTwoBit(value)
            }
        };

        self.fields.entry(tag).or_default().push(field);
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for (&tag, fields) in &self.fields {
            for field in fields {
                len += match field {
                    UnknownField::Varint(value) => {
                        encoding::key_len(tag) + encoding::encoded_len_varint(*value)
                    }
                    UnknownField::SixtyFourBit(value) => encoding::key_len(tag) + value.len(),
                    UnknownField::LengthDelimited(value) => {
                        encoding::bytes::encoded_len(tag, value)
                    }
                    UnknownField::Group(value) => encoding::group::encoded_len(tag, value),
                    UnknownField::ThirtyTwoBit(value) => encoding::key_len(tag) + value.len(),
                };
            }
        }
        len
    }

    fn clear(&mut self) {
        self.fields.clear();
    }
}
