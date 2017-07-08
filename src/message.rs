use std::fmt::Debug;
use std::usize;

use bytes::{
    Buf,
    BufMut,
    Take,
};

use DecodeError;
use EncodeError;
use encoding::*;

/// A Protocol Buffers message.
pub trait Message: Debug + Default + PartialEq + Send + Sync {

    /// Encodes the message to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode<B>(&self, buf: &mut B) -> Result<(), EncodeError> where B: BufMut {
        let required = self.encoded_len();
        let remaining = buf.remaining_mut();
        if required > buf.remaining_mut() {
            return Err(EncodeError::new(required, remaining));
        }

        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message with a length-delimiter to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode_length_delimited<B>(&self, buf: &mut B) -> Result<(), EncodeError> where B: BufMut {
        let len = self.encoded_len();
        let required = len + encoded_len_varint(len as u64);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining))
        }
        encode_varint(len as u64, buf);
        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message to a buffer.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut;

    /// Decodes an instance of the message from a buffer.
    ///
    /// The entire buffer will be consumed.
    fn decode<B>(buf: &mut Take<B>) -> Result<Self, DecodeError> where B: Buf, Self: Default {
        let mut message = Self::default();
        Self::merge(&mut message, buf).map(|_| message)
    }

    /// Decodes a length-delimited instance of the message from the buffer.
    fn decode_length_delimited<B>(buf: &mut B) -> Result<Self, DecodeError> where B: Buf, Self: Default {
        let mut message = Self::default();
        message.merge_length_delimited(buf)?;
        Ok(message)
    }

    /// Decodes an instance of the message from a buffer, and merges it into `self`.
    ///
    /// The entire buffer will be consumed.
    fn merge<B>(&mut self, buf: &mut Take<B>) -> Result<(), DecodeError> where B: Buf;

    /// Decodes a length-delimited instance of the message from buffer, and
    /// merges it into `self`.
    fn merge_length_delimited<B>(&mut self, buf: &mut B) -> Result<(), DecodeError> where B: Buf {
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeError::new("buffer underflow"))
        }
        self.merge(&mut buf.take(len as usize))
    }

    /// Returns the encoded length of the message without a length delimiter.
    fn encoded_len(&self) -> usize;
}

impl <M> Message for Box<M> where M: Message {
    #[inline]
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut {
        (**self).encode_raw(buf)
    }
    #[inline]
    fn merge<B>(&mut self, buf: &mut Take<B>) -> Result<(), DecodeError> where B: Buf {
        (**self).merge(buf)
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }
}
