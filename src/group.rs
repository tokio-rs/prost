use std::fmt::Debug;
use std::usize;

use ::bytes::{Buf, BufMut, IntoBuf};

use crate::encoding::*;
use crate::DecodeError;
use crate::EncodeError;

/// A Protocol Buffers group.
pub trait Group: Debug + Send + Sync {
    /// Encodes the group to a buffer.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Meant to be used only by `Group` implementations.
    #[doc(hidden)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized;

    /// Decodes a field from a buffer, and merges it into `self`.
    ///
    /// Meant to be used only by `Group` implementations.
    #[doc(hidden)]
    fn merge_field<B>(&mut self, buf: &mut B) -> Result<bool, DecodeError>
    where
        B: Buf,
        Self: Sized;

    /// Returns the encoded length of the group without a length delimiter.
    fn encoded_len(&self) -> usize;

    /// Encodes the group to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode<B>(&self, buf: &mut B) -> Result<(), EncodeError>
    where
        B: BufMut,
        Self: Sized,
    {
        let required = self.encoded_len();
        let remaining = buf.remaining_mut();
        if required > buf.remaining_mut() {
            return Err(EncodeError::new(required, remaining));
        }

        self.encode_raw(buf);
        Ok(())
    }

    /// Decodes an instance of the group from a buffer.
    ///
    /// The entire buffer will be consumed.
    fn decode<B>(buf: B) -> Result<Self, DecodeError>
    where
        B: IntoBuf,
        Self: Default,
    {
        let mut group = Self::default();
        Self::merge(&mut group, &mut buf.into_buf()).map(|_| group)
    }

    /// Decodes an instance of the group from a buffer, and merges it into `self`.
    ///
    /// The entire buffer will be consumed.
    fn merge<B>(&mut self, buf: B) -> Result<(), DecodeError>
    where
        B: IntoBuf,
        Self: Sized,
    {
        let mut buf = buf.into_buf();
        while buf.has_remaining() {
            self.merge_field(&mut buf)?;
        }
        Ok(())
    }

    /// Clears the group, resetting all fields to their default.
    fn clear(&mut self);
}

impl<M> Group for Box<M>
where
    M: Group,
{
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        (**self).encode_raw(buf)
    }
    fn merge_field<B>(&mut self, buf: &mut B) -> Result<bool, DecodeError>
    where
        B: Buf,
    {
        (**self).merge_field(buf)
    }
    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }
    fn clear(&mut self) {
        (**self).clear()
    }
}
