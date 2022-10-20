//! Types used for dynamically encoding to and merging from encoded protobuf binary data by delegating to the `crate::encoding` module.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use bytes::{Buf, BufMut};

use crate::bytes::buf::UninitSlice;
use crate::encoding::{DecodeContext, WireType};
use crate::{DecodeError, Message};

/// A type that can decode a value from a buffer of protobuf binary data and 'merge' it into the type's data.
pub trait Merge: Send + Sync {
    fn merge(
        &mut self,
        proto_int_type: ProtoIntType,
        wire_type: WireType,
        buf: &mut MergeBuffer,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;
}

/// A type that can encode a value to a buffer of protobuf binary data.
pub trait Encode {
    fn encode(&self, proto_int_type: ProtoIntType, tag: u32, buf: &mut EncodeBuffer);
    fn encoded_len(&self, proto_int_type: ProtoIntType, tag: u32) -> usize;
}

/// Additional information needed to know how to parse the additional protobuf int types on Merge impls for each integer type.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProtoIntType {
    Default,
    Sint,
    Fixed,
}

impl Default for ProtoIntType {
    fn default() -> Self {
        ProtoIntType::Default
    }
}

/// Hides ref to an unsized dyn Buf so we can call into it from a sized context by impl Buf on our container.
pub struct MergeBuffer<'a> {
    buf: &'a mut dyn Buf,
}

impl<'a> MergeBuffer<'a> {
    pub fn new(buf: &'a mut dyn Buf) -> Self {
        Self { buf }
    }
}

impl Buf for MergeBuffer<'_> {
    fn remaining(&self) -> usize {
        self.buf.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.buf.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.buf.advance(cnt)
    }
}

/// Hides ref to an unsized dyn Buf so we can call into it from a sized context by impl Buf on our container.
pub struct EncodeBuffer<'a> {
    buf: &'a mut dyn BufMut,
}

impl<'a> EncodeBuffer<'a> {
    pub fn new(buf: &'a mut dyn BufMut) -> Self {
        Self { buf }
    }
}

unsafe impl BufMut for EncodeBuffer<'_> {
    fn remaining_mut(&self) -> usize {
        self.buf.remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.buf.advance_mut(cnt)
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        self.buf.chunk_mut()
    }
}

fn sint_decode_error(ty: &str) -> Result<(), DecodeError> {
    Err(DecodeError::new(format!(
        "Cannot decode proto 'sint' type as a '{}'",
        ty
    )))
}

fn sint_encode_error() {
    debug_assert!(
        false,
        "Cannot encode proto 'sint' type as an unsigned type."
    );
}

macro_rules! impl_generics {
    ($ty:ty, $merge:ident, $encode:ident, $encoded_len:ident, $proto_default:ident, $proto_fixed:ident, $proto_sint:ident, $uses_sint:expr) => {
        impl Merge for $ty {
            fn merge(
                &mut self,
                proto_int_type: ProtoIntType,
                wire_type: WireType,
                buf: &mut MergeBuffer,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                match proto_int_type {
                    ProtoIntType::Default => {
                        crate::encoding::$proto_default::$merge(wire_type, self, buf, ctx)
                    }
                    ProtoIntType::Sint => {
                        if ($uses_sint) {
                            crate::encoding::$proto_sint::$merge(wire_type, self, buf, ctx)
                        } else {
                            sint_decode_error(stringify!($ty))
                        }
                    }
                    ProtoIntType::Fixed => {
                        crate::encoding::$proto_fixed::$merge(wire_type, self, buf, ctx)
                    }
                }
            }
        }

        impl Encode for $ty {
            fn encode(&self, proto_int_type: ProtoIntType, tag: u32, buf: &mut EncodeBuffer) {
                match proto_int_type {
                    ProtoIntType::Default => {
                        crate::encoding::$proto_default::$encode(tag, self, buf)
                    }
                    ProtoIntType::Sint => {
                        if ($uses_sint) {
                            crate::encoding::$proto_sint::$encode(tag, self, buf)
                        } else {
                            sint_encode_error();
                        }
                    }
                    ProtoIntType::Fixed => crate::encoding::$proto_fixed::$encode(tag, self, buf),
                }
            }

            fn encoded_len(&self, proto_int_type: ProtoIntType, tag: u32) -> usize {
                match proto_int_type {
                    ProtoIntType::Default => {
                        crate::encoding::$proto_default::$encoded_len(tag, self)
                    }
                    ProtoIntType::Sint => {
                        if ($uses_sint) {
                            crate::encoding::$proto_sint::$encoded_len(tag, self)
                        } else {
                            sint_encode_error();
                            0
                        }
                    }
                    ProtoIntType::Fixed => crate::encoding::$proto_fixed::$encoded_len(tag, self),
                }
            }
        }
    };
}

//                             ty           encoding functions                                      proto_int_type           uses_sint
// int
#[rustfmt::skip] impl_generics!(i32,         merge, encode, encoded_len,                             int32, sfixed32, sint32, true);
#[rustfmt::skip] impl_generics!(Vec<i32>,    merge_repeated, encode_repeated, encoded_len_repeated,  int32, sfixed32, sint32, true);
#[rustfmt::skip] impl_generics!(i64,         merge, encode, encoded_len,                             int64, sfixed64, sint64, true);
#[rustfmt::skip] impl_generics!(Vec<i64>,    merge_repeated, encode_repeated, encoded_len_repeated,  int64, sfixed64, sint64, true);
// uint
#[rustfmt::skip] impl_generics!(u32,         merge, encode, encoded_len,                             uint32, fixed32, uint32, false);
#[rustfmt::skip] impl_generics!(Vec<u32>,    merge_repeated, encode_repeated, encoded_len_repeated,  uint32, fixed32, uint32, false);
#[rustfmt::skip] impl_generics!(u64,         merge, encode, encoded_len,                             uint64, fixed64, uint64, false);
#[rustfmt::skip] impl_generics!(Vec<u64>,    merge_repeated, encode_repeated, encoded_len_repeated,  uint64, fixed64, uint64, false);
// Other.
#[rustfmt::skip] impl_generics!(f64,         merge, encode, encoded_len,                             double, double, double, true);
#[rustfmt::skip] impl_generics!(Vec<f64>,    merge_repeated, encode_repeated, encoded_len_repeated,  double, double, double, true);
#[rustfmt::skip] impl_generics!(f32,         merge, encode, encoded_len,                             float, float, float, true);
#[rustfmt::skip] impl_generics!(Vec<f32>,    merge_repeated, encode_repeated, encoded_len_repeated,  float, float, float, true);
#[rustfmt::skip] impl_generics!(bool,        merge, encode, encoded_len,                             bool, bool, bool, true);
#[rustfmt::skip] impl_generics!(Vec<bool>,   merge_repeated, encode_repeated, encoded_len_repeated,  bool, bool, bool, true);
#[rustfmt::skip] impl_generics!(String,      merge, encode, encoded_len,                             string, string, string, true);
#[rustfmt::skip] impl_generics!(Vec<String>, merge_repeated, encode_repeated, encoded_len_repeated,  string, string, string, true);
#[rustfmt::skip] impl_generics!(Vec<u8>,     merge, encode, encoded_len,                             bytes, bytes, bytes, true);
#[rustfmt::skip] impl_generics!(Vec<Vec<u8>>,merge_repeated, encode_repeated, encoded_len_repeated,  bytes, bytes, bytes, true);

// Unfortunately, because all primitives also impl Message, a blanket impl Merge for Vec<M>
// also catches Vec<i32>, Vec<u32> etc.
//
// Instead we use another trait implemented on only _derived_ message types that knows how to
// forward to the appropriate merge_repeated.
//
// Primitive vecs are implemented directly for Vec<$ty> in the macros above.
pub trait MergeRepeated {
    fn merge_repeated(
        vec: &mut Vec<Self>,
        wire_type: WireType,
        buf: &mut MergeBuffer,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        Self: Sized;
}
pub trait EncodeRepeated {
    fn encode_repeated(
        vec: &[Self],
        proto_int_type: ProtoIntType,
        tag: u32,
        buf: &mut EncodeBuffer,
    ) where
        Self: Sized;

    fn encoded_len_repeated(vec: &[Self], proto_int_type: ProtoIntType, tag: u32) -> usize
    where
        Self: Sized;
}

impl<M> Merge for Vec<M>
where
    M: Message + MergeRepeated,
{
    fn merge(
        &mut self,
        _: ProtoIntType,
        wire_type: WireType,
        buf: &mut MergeBuffer,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        <M as MergeRepeated>::merge_repeated(self, wire_type, buf, ctx)
    }
}
impl<M> Encode for Vec<M>
where
    M: Message + EncodeRepeated,
{
    fn encode(&self, proto_int_type: ProtoIntType, tag: u32, buf: &mut EncodeBuffer) {
        <M as EncodeRepeated>::encode_repeated(self, proto_int_type, tag, buf)
    }

    fn encoded_len(&self, proto_int_type: ProtoIntType, tag: u32) -> usize {
        <M as EncodeRepeated>::encoded_len_repeated(self, proto_int_type, tag)
    }
}
