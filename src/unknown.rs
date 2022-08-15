use alloc::boxed::Box;
use alloc::vec::Vec;

use core::fmt::Debug;
use core::usize;

use bytes::{Buf, BufMut};

use crate::encoding::{
    decode_key, encode_varint, encoded_len_varint, message, DecodeContext, WireType,
};
use crate::DecodeError;
use crate::EncodeError;
use crate::Message;

pub struct UnknownFields {}

impl Debug for UnknownFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("UnknownFields")
    }
}

impl Clone for UnknownFields {
    fn clone(&self) -> Self {
        Self {}
    }
}

// TODO(jason): give a better partial eq implementation
impl PartialEq for UnknownFields {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for UnknownFields {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Default for UnknownFields {
    fn default() -> Self {
        Self {}
    }
}

impl Message for UnknownFields {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        // TODO(jason)
        // (**self).encode_raw(buf)
    }
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        // TODO(jason)
        // (**self).merge_field(tag, wire_type, buf, ctx)
        Ok(())
    }
    fn encoded_len(&self) -> usize {
        // (**self).encoded_len()
        // TODO(jason)
        0
    }
    fn clear(&mut self) {
        // TODO(jason)
        // (**self).clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _MESSAGE_IS_OBJECT_SAFE: Option<&dyn Message> = None;
}
