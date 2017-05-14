use std::fmt::Debug;
use std::io::Result;
use std::usize;

use bytes::{
    Buf,
    BufMut,
    Take,
};

use encoding::*;

/// A Protocol Buffers message.
pub trait Message: Debug + Default /*+ PartialEq + PartialOrd*/ + Send + Sync {

    /// Encodes the message, and writes it to the buffer. An error will be
    /// returned if the buffer does not have sufficient capacity.
    fn encode<B>(&self, buf: &mut B) -> Result<()> where B: BufMut {
        let len = self.encoded_len();
        if len > buf.remaining_mut() {
            return Err(invalid_input("failed to encode message: insufficient buffer capacity"));
        }

        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message, and writes it with a length-delimiter prefix to
    /// the buffer. An error will be returned if the buffer does not have
    /// sufficient capacity.
    fn encode_length_delimited<B>(&self, buf: &mut B) -> Result<()> where B: BufMut {
        let len = self.encoded_len();
        if len + encoded_len_varint(len as u64) > buf.remaining_mut() {
            return Err(invalid_input("failed to encode message: insufficient buffer capacity"));
        }
        encode_varint(len as u64, buf);
        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message, writing it to the buffer.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Prefer using `Message::encode`.
    #[doc(hidden)]
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut;

    /// Decodes an instance of the message from the buffer.
    /// The entire buffer will be consumed.
    fn decode<B>(buf: &mut Take<B>) -> Result<Self> where B: Buf, Self: Default {
        let mut message = Self::default();
        Self::merge(&mut message, buf).map(|_| message)
    }

    /// Decodes a length-delimited instance of the message from the buffer.
    fn decode_length_delimited<B>(buf: &mut B) -> Result<Self> where B: Buf, Self: Default {
        let mut message = Self::default();
        message.merge_length_delimited(buf)?;
        Ok(message)
    }

    /// Decodes an instance of the message from the buffer, and merges
    /// it into `self`. The entire buffer will be consumed.
    fn merge<B>(&mut self, buf: &mut Take<B>) -> Result<()> where B: Buf;

    /// Decodes a length-delimited instance of the message from the
    /// buffer, and merges it into `self`.
    fn merge_length_delimited<B>(&mut self, buf: &mut B) -> Result<()> where B: Buf {
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(invalid_input("failed to merge message: buffer underflow"));
        }
        self.merge(&mut buf.take(len as usize))
    }

    /// The encoded length of the message without a length delimiter.
    fn encoded_len(&self) -> usize;
}

impl <M> Message for Box<M> where M: Message {
    #[inline]
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut {
        (**self).encode_raw(buf)
    }
    #[inline]
    fn merge<B>(&mut self, buf: &mut Take<B>) -> Result<()> where B: Buf {
        (**self).merge(buf)
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }
}
