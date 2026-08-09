//! Utility functions and types for encoding and decoding Protobuf types.
//!
//! This module contains the encoding and decoding primitives for Protobuf as described in
//! <https://protobuf.dev/programming-guides/encoding/>.
//!
//! This module is `pub`, but is only for prost internal use. The `prost-derive` crate needs access for its `Message` implementations.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem;
use core::str;

use ::bytes::{Buf, BufMut, Bytes};

use crate::error::DecodeErrorKind;
use crate::DecodeError;
use crate::Message;

pub mod varint;
pub use varint::{decode_varint, encode_varint, encoded_len_varint};

pub mod length_delimiter;
pub use length_delimiter::{
    decode_length_delimiter, encode_length_delimiter, length_delimiter_len,
};

pub mod wire_type;
pub use wire_type::{check_wire_type, WireType};

const SCOPE_LIMIT_BITS: u32 = 25;
const SCOPE_LIMIT_MASK: u32 = (1 << SCOPE_LIMIT_BITS) - 1;
// The all-ones scope value disables optional reservation when the boundary
// cannot fit alongside the recursion count.
const UNREPRESENTABLE_SCOPE_LIMIT: u32 = SCOPE_LIMIT_MASK;
#[cfg(not(feature = "no-recursion-limit"))]
const RECURSE_COUNT_SHIFT: u32 = SCOPE_LIMIT_BITS;
#[cfg(not(feature = "no-recursion-limit"))]
const _: () = assert!(crate::RECURSION_LIMIT < (1u32 << (u32::BITS - RECURSE_COUNT_SHIFT)));

/// Additional information passed to every decode/merge function.
///
/// The context should be passed by value and can be freely cloned. When passing
/// to a function which is decoding a nested object, then use `enter_recursion`.
// The low 32 bits hold the recursion count and length-delimited scope limit.
// The high 32 bits hold a generated repeated field's tag. Keeping the context
// transparent over a `u64` lets decode calls carry both pieces of state in one
// argument. Zero is not a valid protobuf tag, so it represents the untagged
// fallback used by hand-written Message implementations.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct DecodeContext(u64);

#[cfg(not(feature = "no-recursion-limit"))]
impl Default for DecodeContext {
    #[inline]
    fn default() -> DecodeContext {
        DecodeContext(u64::from(crate::RECURSION_LIMIT << RECURSE_COUNT_SHIFT))
    }
}

#[cfg(feature = "no-recursion-limit")]
impl Default for DecodeContext {
    #[inline]
    fn default() -> DecodeContext {
        DecodeContext(0)
    }
}

impl DecodeContext {
    /// Call this function before recursively decoding.
    ///
    /// There is no `exit` function since this function creates a new `DecodeContext`
    /// to be used at the next level of recursion. Continue to use the old context
    // at the previous level of recursion.
    #[cfg(not(feature = "no-recursion-limit"))]
    #[inline]
    pub(crate) fn enter_recursion(&self) -> DecodeContext {
        DecodeContext(u64::from((self.0 as u32) - (1 << RECURSE_COUNT_SHIFT)))
    }

    #[cfg(feature = "no-recursion-limit")]
    #[inline]
    pub(crate) fn enter_recursion(&self) -> DecodeContext {
        DecodeContext(u64::from(self.0 as u32))
    }

    #[inline]
    pub(crate) fn with_scope_limit(mut self, scope_limit: usize) -> DecodeContext {
        let scope_limit = if scope_limit < UNREPRESENTABLE_SCOPE_LIMIT as usize {
            scope_limit as u32
        } else {
            UNREPRESENTABLE_SCOPE_LIMIT
        };
        // Entering a length-delimited scope must not retain an outer field tag.
        self.0 = u64::from((self.0 as u32 & !SCOPE_LIMIT_MASK) | scope_limit);
        self
    }

    #[inline]
    pub(crate) fn scope_remaining(&self, buf_remaining: usize) -> Option<usize> {
        let scope_limit = (self.0 as u32) & SCOPE_LIMIT_MASK;
        if scope_limit == UNREPRESENTABLE_SCOPE_LIMIT {
            None
        } else {
            Some(buf_remaining.saturating_sub(scope_limit as usize))
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn with_field_tag(mut self, field_tag: u32) -> DecodeContext {
        self.0 = (self.0 & u64::from(u32::MAX)) | (u64::from(field_tag) << 32);
        self
    }

    #[inline]
    fn field_tag(&self) -> Option<u32> {
        let field_tag = (self.0 >> 32) as u32;
        (field_tag != 0).then_some(field_tag)
    }

    /// Checks whether the recursion limit has been reached in the stack of
    /// decodes described by the `DecodeContext` at `self.ctx`.
    ///
    /// Returns `Ok<()>` if it is ok to continue recursing.
    /// Returns `Err<DecodeError>` if the recursion limit has been reached.
    #[cfg(not(feature = "no-recursion-limit"))]
    #[inline]
    pub(crate) fn limit_reached(&self) -> Result<(), DecodeError> {
        if (self.0 as u32) >> RECURSE_COUNT_SHIFT == 0 {
            Err(DecodeErrorKind::RecursionLimitReached.into())
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "no-recursion-limit")]
    #[inline]
    pub(crate) fn limit_reached(&self) -> Result<(), DecodeError> {
        Ok(())
    }
}

pub const MIN_TAG: u32 = 1;
pub const MAX_TAG: u32 = (1 << 29) - 1;

/// Encodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline]
pub fn encode_key(tag: u32, wire_type: WireType, buf: &mut impl BufMut) {
    debug_assert!((MIN_TAG..=MAX_TAG).contains(&tag));
    let key = (tag << 3) | wire_type as u32;
    encode_varint(u64::from(key), buf);
}

/// Decodes a Protobuf field key, which consists of a wire type designator and
/// the field tag.
#[inline(always)]
pub fn decode_key(buf: &mut impl Buf) -> Result<(u32, WireType), DecodeError> {
    let key = decode_varint(buf)?;
    if key > u64::from(u32::MAX) {
        return Err(DecodeErrorKind::InvalidKey { key }.into());
    }
    let wire_type = WireType::try_from(key & 0x07)?;
    let tag = key as u32 >> 3;

    if tag < MIN_TAG {
        return Err(DecodeErrorKind::InvalidTag.into());
    }

    Ok((tag, wire_type))
}

/// Returns the width of an encoded Protobuf field key with the given tag.
/// The returned width will be between 1 and 5 bytes (inclusive).
#[inline]
pub const fn key_len(tag: u32) -> usize {
    encoded_len_varint((tag << 3) as u64)
}

/// Helper function which abstracts reading a length delimiter prefix followed
/// by decoding values until the length of bytes is exhausted.
pub fn merge_loop<T, M, B>(
    value: &mut T,
    buf: &mut B,
    ctx: DecodeContext,
    mut merge: M,
) -> Result<(), DecodeError>
where
    M: FnMut(&mut T, &mut B, DecodeContext) -> Result<(), DecodeError>,
    B: Buf,
{
    let len = decode_varint(buf)?;
    let remaining = buf.remaining();
    if len > remaining as u64 {
        return Err(DecodeErrorKind::BufferUnderflow.into());
    }

    let limit = remaining - len as usize;
    // Preserve this envelope's end for field decoders that inspect upcoming bytes.
    let ctx = ctx.with_scope_limit(limit);
    while buf.remaining() > limit {
        merge(value, buf, ctx)?;
    }

    if buf.remaining() != limit {
        return Err(DecodeErrorKind::DelimitedLengthExceeded.into());
    }
    Ok(())
}

pub fn skip_field(
    wire_type: WireType,
    tag: u32,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    ctx.limit_reached()?;
    let len = match wire_type {
        WireType::Varint => decode_varint(buf).map(|_| 0)?,
        WireType::ThirtyTwoBit => 4,
        WireType::SixtyFourBit => 8,
        WireType::LengthDelimited => decode_varint(buf)?,
        WireType::StartGroup => loop {
            let (inner_tag, inner_wire_type) = decode_key(buf)?;
            match inner_wire_type {
                WireType::EndGroup => {
                    if inner_tag != tag {
                        return Err(DecodeErrorKind::UnexpectedEndGroupTag.into());
                    }
                    break 0;
                }
                _ => skip_field(inner_wire_type, inner_tag, buf, ctx.enter_recursion())?,
            }
        },
        WireType::EndGroup => return Err(DecodeErrorKind::UnexpectedEndGroupTag.into()),
    };

    if len > buf.remaining() as u64 {
        return Err(DecodeErrorKind::BufferUnderflow.into());
    }

    buf.advance(len as usize);
    Ok(())
}

/// Helper macro which emits an `encode_repeated` function for the type.
macro_rules! encode_repeated {
    ($ty:ty) => {
        pub fn encode_repeated(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
            for value in values {
                encode(tag, value, buf);
            }
        }
    };
}

/// Helper macro which emits a `merge_repeated` function for the numeric type.
macro_rules! merge_repeated_numeric {
    ($ty:ty,
     $wire_type:expr,
     $merge:ident,
     $merge_repeated:ident) => {
        pub fn $merge_repeated(
            wire_type: WireType,
            values: &mut Vec<$ty>,
            buf: &mut impl Buf,
            ctx: DecodeContext,
        ) -> Result<(), DecodeError> {
            if wire_type == WireType::LengthDelimited {
                // Packed.
                merge_loop(values, buf, ctx, |values, buf, ctx| {
                    let mut value = Default::default();
                    $merge($wire_type, &mut value, buf, ctx)?;
                    values.push(value);
                    Ok(())
                })
            } else {
                // Unpacked.
                check_wire_type($wire_type, wire_type)?;
                let mut value = Default::default();
                $merge(wire_type, &mut value, buf, ctx)?;
                values.push(value);
                Ok(())
            }
        }
    };
}

/// Macro which emits a module containing a set of encoding functions for a
/// variable width numeric type.
macro_rules! varint {
    ($ty:ty,
     $proto_ty:ident) => (
        varint!($ty,
                $proto_ty,
                to_uint64(value) { *value as u64 },
                from_uint64(value) { value as $ty });
    );

    ($ty:ty,
     $proto_ty:ident,
     to_uint64($to_uint64_value:ident) $to_uint64:expr,
     from_uint64($from_uint64_value:ident) $from_uint64:expr) => (

         pub mod $proto_ty {
            use crate::encoding::*;

            pub fn encode(tag: u32, $to_uint64_value: &$ty, buf: &mut impl BufMut) {
                encode_key(tag, WireType::Varint, buf);
                encode_varint($to_uint64, buf);
            }

            pub fn merge(wire_type: WireType, value: &mut $ty, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
                check_wire_type(WireType::Varint, wire_type)?;
                let $from_uint64_value = decode_varint(buf)?;
                *value = $from_uint64;
                Ok(())
            }

            encode_repeated!($ty);

            pub fn encode_packed(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
                if values.is_empty() { return; }

                encode_key(tag, WireType::LengthDelimited, buf);
                let len: usize = values.iter().map(|$to_uint64_value| {
                    encoded_len_varint($to_uint64)
                }).sum();
                encode_varint(len as u64, buf);

                for $to_uint64_value in values {
                    encode_varint($to_uint64, buf);
                }
            }

            merge_repeated_numeric!($ty, WireType::Varint, merge, merge_repeated);

            #[inline]
            pub fn encoded_len(tag: u32, $to_uint64_value: &$ty) -> usize {
                key_len(tag) + encoded_len_varint($to_uint64)
            }

            #[inline]
            pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
                key_len(tag) * values.len() + values.iter().map(|$to_uint64_value| {
                    encoded_len_varint($to_uint64)
                }).sum::<usize>()
            }

            #[inline]
            pub fn encoded_len_packed(tag: u32, values: &[$ty]) -> usize {
                if values.is_empty() {
                    0
                } else {
                    let len = values.iter()
                                    .map(|$to_uint64_value| encoded_len_varint($to_uint64))
                                    .sum::<usize>();
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }

            #[cfg(test)]
            mod test {
                use proptest::prelude::*;

                use crate::encoding::$proto_ty::*;
                use crate::encoding::test::{
                    check_collection_type,
                    check_type,
                };

                proptest! {
                    #[test]
                    fn check(value: $ty, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, WireType::Varint,
                                   encode, merge, encoded_len)?;
                    }
                    #[test]
                    fn check_repeated(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_collection_type(value, tag, WireType::Varint,
                                              encode_repeated, merge_repeated,
                                              encoded_len_repeated)?;
                    }
                    #[test]
                    fn check_packed(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, WireType::LengthDelimited,
                                   encode_packed, merge_repeated,
                                   encoded_len_packed)?;
                    }
                }
            }
         }

    );
}
varint!(bool, bool,
        to_uint64(value) u64::from(*value),
        from_uint64(value) value != 0);
varint!(i32, int32);
varint!(i64, int64);
varint!(u32, uint32);
varint!(u64, uint64);
varint!(i32, sint32,
to_uint64(value) {
    ((value << 1) ^ (value >> 31)) as u32 as u64
},
from_uint64(value) {
    let value = value as u32;
    ((value >> 1) as i32) ^ (-((value & 1) as i32))
});
varint!(i64, sint64,
to_uint64(value) {
    ((value << 1) ^ (value >> 63)) as u64
},
from_uint64(value) {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
});

/// Macro which emits a module containing a set of encoding functions for a
/// fixed width numeric type.
macro_rules! fixed_width {
    ($ty:ty,
     $width:expr,
     $wire_type:expr,
     $proto_ty:ident,
     $put:ident,
     $get:ident) => {
        pub mod $proto_ty {
            use crate::encoding::*;

            pub fn encode(tag: u32, value: &$ty, buf: &mut impl BufMut) {
                encode_key(tag, $wire_type, buf);
                buf.$put(*value);
            }

            pub fn merge(
                wire_type: WireType,
                value: &mut $ty,
                buf: &mut impl Buf,
                _ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                check_wire_type($wire_type, wire_type)?;
                if buf.remaining() < $width {
                    return Err(DecodeErrorKind::BufferUnderflow.into());
                }
                *value = buf.$get();
                Ok(())
            }

            encode_repeated!($ty);

            pub fn encode_packed(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
                if values.is_empty() {
                    return;
                }

                encode_key(tag, WireType::LengthDelimited, buf);
                let len = values.len() as u64 * $width;
                encode_varint(len as u64, buf);

                for value in values {
                    buf.$put(*value);
                }
            }

            merge_repeated_numeric!($ty, $wire_type, merge, merge_repeated);

            #[inline]
            pub fn encoded_len(tag: u32, _: &$ty) -> usize {
                key_len(tag) + $width
            }

            #[inline]
            pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
                (key_len(tag) + $width) * values.len()
            }

            #[inline]
            pub fn encoded_len_packed(tag: u32, values: &[$ty]) -> usize {
                if values.is_empty() {
                    0
                } else {
                    let len = $width * values.len();
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }

            #[cfg(test)]
            mod test {
                use proptest::prelude::*;

                use super::super::test::{check_collection_type, check_type};
                use super::*;

                proptest! {
                    #[test]
                    fn check(value: $ty, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, $wire_type,
                                   encode, merge, encoded_len)?;
                    }
                    #[test]
                    fn check_repeated(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_collection_type(value, tag, $wire_type,
                                              encode_repeated, merge_repeated,
                                              encoded_len_repeated)?;
                    }
                    #[test]
                    fn check_packed(value: Vec<$ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_type(value, tag, WireType::LengthDelimited,
                                   encode_packed, merge_repeated,
                                   encoded_len_packed)?;
                    }
                }
            }
        }
    };
}
fixed_width!(
    f32,
    4,
    WireType::ThirtyTwoBit,
    float,
    put_f32_le,
    get_f32_le
);
fixed_width!(
    f64,
    8,
    WireType::SixtyFourBit,
    double,
    put_f64_le,
    get_f64_le
);
fixed_width!(
    u32,
    4,
    WireType::ThirtyTwoBit,
    fixed32,
    put_u32_le,
    get_u32_le
);
fixed_width!(
    u64,
    8,
    WireType::SixtyFourBit,
    fixed64,
    put_u64_le,
    get_u64_le
);
fixed_width!(
    i32,
    4,
    WireType::ThirtyTwoBit,
    sfixed32,
    put_i32_le,
    get_i32_le
);
fixed_width!(
    i64,
    8,
    WireType::SixtyFourBit,
    sfixed64,
    put_i64_le,
    get_i64_le
);

/// Macro which emits encoding functions for a length-delimited type.
macro_rules! length_delimited {
    ($ty:ty) => {
        encode_repeated!($ty);

        pub fn merge_repeated(
            wire_type: WireType,
            values: &mut Vec<$ty>,
            buf: &mut impl Buf,
            ctx: DecodeContext,
        ) -> Result<(), DecodeError> {
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let mut value = Default::default();
            merge(wire_type, &mut value, buf, ctx)?;
            values.push(value);
            Ok(())
        }

        #[inline]
        #[allow(clippy::ptr_arg)]
        pub fn encoded_len(tag: u32, value: &$ty) -> usize {
            key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
        }

        #[inline]
        pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
            key_len(tag) * values.len()
                + values
                    .iter()
                    .map(|value| encoded_len_varint(value.len() as u64) + value.len())
                    .sum::<usize>()
        }
    };
}

pub mod string {
    use super::*;

    pub fn encode(tag: u32, value: &String, buf: &mut impl BufMut) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        buf.put_slice(value.as_bytes());
    }

    pub fn merge(
        wire_type: WireType,
        value: &mut String,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        // ## Unsafety
        //
        // `string::merge` reuses `bytes::merge`, with an additional check of utf-8
        // well-formedness. If the utf-8 is not well-formed, or if any other error occurs, then the
        // string is cleared, so as to avoid leaking a string field with invalid data.
        //
        // This implementation uses the unsafe `String::as_mut_vec` method instead of the safe
        // alternative of temporarily swapping an empty `String` into the field, because it results
        // in up to 10% better performance on the protobuf message decoding benchmarks.
        //
        // It's required when using `String::as_mut_vec` that invalid utf-8 data not be leaked into
        // the backing `String`. To enforce this, even in the event of a panic in `bytes::merge` or
        // in the buf implementation, a drop guard is used.
        unsafe {
            struct DropGuard<'a>(&'a mut Vec<u8>);
            impl Drop for DropGuard<'_> {
                #[inline]
                fn drop(&mut self) {
                    self.0.clear();
                }
            }

            let drop_guard = DropGuard(value.as_mut_vec());
            bytes::merge_one_copy(wire_type, drop_guard.0, buf, ctx)?;
            match str::from_utf8(drop_guard.0) {
                Ok(_) => {
                    // Success; do not clear the bytes.
                    mem::forget(drop_guard);
                    Ok(())
                }
                Err(_) => Err(DecodeErrorKind::InvalidString.into()),
            }
        }
    }

    length_delimited!(String);

    #[cfg(test)]
    mod test {
        use proptest::prelude::*;

        use super::super::test::{check_collection_type, check_type};
        use super::*;

        proptest! {
            #[test]
            fn check(value: String, tag in MIN_TAG..=MAX_TAG) {
                super::test::check_type(value, tag, WireType::LengthDelimited,
                                        encode, merge, encoded_len)?;
            }
            #[test]
            fn check_repeated(value: Vec<String>, tag in MIN_TAG..=MAX_TAG) {
                super::test::check_collection_type(value, tag, WireType::LengthDelimited,
                                                   encode_repeated, merge_repeated,
                                                   encoded_len_repeated)?;
            }
        }
    }
}

pub trait BytesAdapter: sealed::BytesAdapter {}

mod sealed {
    use super::{Buf, BufMut};

    pub trait BytesAdapter: Default + Sized + 'static {
        fn len(&self) -> usize;

        /// Replace contents of this buffer with the contents of another buffer.
        fn replace_with(&mut self, buf: impl Buf);

        /// Appends this buffer to the (contents of) other buffer.
        fn append_to(&self, buf: &mut impl BufMut);

        fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }
}

impl BytesAdapter for Bytes {}

impl sealed::BytesAdapter for Bytes {
    fn len(&self) -> usize {
        Buf::remaining(self)
    }

    fn replace_with(&mut self, mut buf: impl Buf) {
        *self = buf.copy_to_bytes(buf.remaining());
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.clone())
    }
}

impl BytesAdapter for Vec<u8> {}

impl sealed::BytesAdapter for Vec<u8> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn replace_with(&mut self, buf: impl Buf) {
        self.clear();
        self.reserve(buf.remaining());
        self.put(buf);
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.as_slice())
    }
}

pub mod bytes {
    use crate::error::DecodeErrorKind;

    use super::*;

    pub fn encode(tag: u32, value: &impl BytesAdapter, buf: &mut impl BufMut) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        value.append_to(buf);
    }

    pub fn merge(
        wire_type: WireType,
        value: &mut impl BytesAdapter,
        buf: &mut impl Buf,
        _ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeErrorKind::BufferUnderflow.into());
        }
        let len = len as usize;

        // Clear the existing value. This follows from the following rule in the encoding guide[1]:
        //
        // > Normally, an encoded message would never have more than one instance of a non-repeated
        // > field. However, parsers are expected to handle the case in which they do. For numeric
        // > types and strings, if the same field appears multiple times, the parser accepts the
        // > last value it sees.
        //
        // [1]: https://protobuf.dev/programming-guides/encoding/#last-one-wins
        //
        // This is intended for A and B both being Bytes so it is zero-copy.
        // Some combinations of A and B types may cause a double-copy,
        // in which case merge_one_copy() should be used instead.
        value.replace_with(buf.copy_to_bytes(len));
        Ok(())
    }

    pub(super) fn merge_one_copy(
        wire_type: WireType,
        value: &mut impl BytesAdapter,
        buf: &mut impl Buf,
        _ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeErrorKind::BufferUnderflow.into());
        }
        let len = len as usize;

        // If we must copy, make sure to copy only once.
        value.replace_with(buf.take(len));
        Ok(())
    }

    length_delimited!(impl BytesAdapter);

    #[cfg(test)]
    mod test {
        use proptest::prelude::*;

        use super::super::test::{check_collection_type, check_type};
        use super::*;

        proptest! {
            #[test]
            fn check_vec(value: Vec<u8>, tag in MIN_TAG..=MAX_TAG) {
                super::test::check_type::<Vec<u8>, Vec<u8>>(value, tag, WireType::LengthDelimited,
                                                            encode, merge, encoded_len)?;
            }

            #[test]
            fn check_bytes(value: Vec<u8>, tag in MIN_TAG..=MAX_TAG) {
                let value = Bytes::from(value);
                super::test::check_type::<Bytes, Bytes>(value, tag, WireType::LengthDelimited,
                                                        encode, merge, encoded_len)?;
            }

            #[test]
            fn check_repeated_vec(value: Vec<Vec<u8>>, tag in MIN_TAG..=MAX_TAG) {
                super::test::check_collection_type(value, tag, WireType::LengthDelimited,
                                                   encode_repeated, merge_repeated,
                                                   encoded_len_repeated)?;
            }

            #[test]
            fn check_repeated_bytes(value: Vec<Vec<u8>>, tag in MIN_TAG..=MAX_TAG) {
                let value = value.into_iter().map(Bytes::from).collect();
                super::test::check_collection_type(value, tag, WireType::LengthDelimited,
                                                   encode_repeated, merge_repeated,
                                                   encoded_len_repeated)?;
            }
        }
    }
}

pub mod message {
    use super::*;

    // Keep both wire lookahead and requested reservations bounded.
    const MAX_REPEATED_MESSAGE_RESERVE_BYTES: usize = 4 * 1024;
    const MAX_REPEATED_MESSAGE_LOOKAHEAD_BYTES: usize = 2 * MAX_REPEATED_MESSAGE_RESERVE_BYTES;

    pub fn encode<M>(tag: u32, msg: &M, buf: &mut impl BufMut)
    where
        M: Message,
    {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(msg.encoded_len() as u64, buf);
        msg.encode_raw(buf);
    }

    pub fn merge<M, B>(
        wire_type: WireType,
        msg: &mut M,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Message,
        B: Buf,
    {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        ctx.limit_reached()?;
        merge_loop(
            msg,
            buf,
            ctx.enter_recursion(),
            |msg: &mut M, buf: &mut B, ctx| {
                let (tag, wire_type) = decode_key(buf)?;
                msg.merge_field(tag, wire_type, buf, ctx)
            },
        )
    }

    pub fn encode_repeated<M>(tag: u32, messages: &[M], buf: &mut impl BufMut)
    where
        M: Message,
    {
        for msg in messages {
            encode(tag, msg, buf);
        }
    }

    #[inline]
    pub fn merge_repeated<M>(
        wire_type: WireType,
        messages: &mut Vec<M>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Message + Default,
    {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut msg = M::default();
        let remaining = buf.remaining();
        let scope_remaining = ctx.scope_remaining(remaining);
        merge(WireType::LengthDelimited, &mut msg, buf, ctx)?;

        // Repeated fields are typically serialized consecutively. Count only
        // complete, same-tag fields in the current chunk so unrelated later
        // fields cannot cause a reservation. Hand-written Message
        // implementations do not provide a field tag and retain the old path.
        // Preserve the established hot-path shape for repeated-message decoding.
        #[allow(clippy::collapsible_if)]
        if let (Some(tag), Some(scope_remaining)) = (ctx.field_tag(), scope_remaining) {
            if messages.len() == messages.capacity() {
                let limit = repeated_message_reserve_limit::<M>();
                if limit > 0 {
                    let value_len = remaining.saturating_sub(buf.remaining());
                    let scope_remaining = scope_remaining.saturating_sub(value_len);
                    let count =
                        consecutive_repeated_messages_in_scope(tag, buf, limit, scope_remaining);
                    if count > 0 {
                        // Reserve for the decoded message and the following fields.
                        messages.reserve(count + 1);
                    }
                }
            }
        }

        messages.push(msg);
        Ok(())
    }

    fn repeated_message_reserve_limit<M>() -> usize {
        let max_messages = MAX_REPEATED_MESSAGE_RESERVE_BYTES / core::mem::size_of::<M>().max(1);

        // The reservation must also include the message already decoded above.
        max_messages.saturating_sub(1)
    }

    fn consecutive_repeated_messages_in_scope(
        tag: u32,
        buf: &impl Buf,
        limit: usize,
        scope_remaining: usize,
    ) -> usize {
        // A `Buf` may be fragmented, so only inspect fields entirely contained
        // in the current chunk without advancing the caller's buffer.
        let key = (u64::from(tag) << 3) | u64::from(WireType::LengthDelimited as u8);
        let chunk = buf.chunk();
        let mut chunk = &chunk[..chunk
            .len()
            .min(scope_remaining)
            .min(MAX_REPEATED_MESSAGE_LOOKAHEAD_BYTES)];
        let mut count = 0usize;

        // The common canonical one-byte envelope can advance without decoding
        // either varint. Leave all other inputs to the generic loop below.
        if key < 0x80 {
            let key = key as u8;
            while count < limit && chunk.len() >= 2 && chunk[0] == key && chunk[1] < 0x80 {
                let len = usize::from(chunk[1]);
                let field = &chunk[2..];
                if len > field.len() {
                    break;
                }

                chunk = &field[len..];
                // `count < limit <= usize::MAX` above makes this addition safe.
                count += 1;
            }
        }

        while count < limit && !chunk.is_empty() {
            let mut field = chunk;
            if key < 0x80 && field[0] == key as u8 {
                field = &field[1..];
            } else {
                let decoded_key = match decode_varint(&mut field) {
                    Ok(decoded_key) => decoded_key,
                    Err(_) => break,
                };
                if decoded_key != key {
                    break;
                }
            }

            let len = if let Some((&byte, remaining)) = field.split_first() {
                if byte < 0x80 {
                    field = remaining;
                    usize::from(byte)
                } else {
                    let len = match decode_varint(&mut field) {
                        Ok(len) => len,
                        Err(_) => break,
                    };
                    match len.try_into() {
                        Ok(len) => len,
                        Err(_) => break,
                    }
                }
            } else {
                break;
            };
            if len > field.len() {
                break;
            }

            chunk = &field[len..];
            count = match count.checked_add(1) {
                Some(count) => count,
                None => break,
            };
        }

        count
    }

    #[inline]
    pub fn encoded_len<M>(tag: u32, msg: &M) -> usize
    where
        M: Message,
    {
        let len = msg.encoded_len();
        key_len(tag) + encoded_len_varint(len as u64) + len
    }

    #[inline]
    pub fn encoded_len_repeated<M>(tag: u32, messages: &[M]) -> usize
    where
        M: Message,
    {
        key_len(tag) * messages.len()
            + messages
                .iter()
                .map(Message::encoded_len)
                .map(|len| len + encoded_len_varint(len as u64))
                .sum::<usize>()
    }

    #[cfg(test)]
    mod tests {
        use bytes::{Buf, Bytes};

        use super::{
            consecutive_repeated_messages_in_scope, decode_key, merge_repeated,
            repeated_message_reserve_limit, DecodeContext, MAX_REPEATED_MESSAGE_LOOKAHEAD_BYTES,
        };
        use crate::alloc::vec::Vec;
        use crate::Message;

        #[derive(Clone, PartialEq, crate::Message)]
        #[prost(prost_path = "crate")]
        struct ScopedValue {
            #[prost(bool, tag = "1")]
            value: bool,
        }

        #[derive(Clone, PartialEq, crate::Message)]
        #[prost(prost_path = "crate")]
        struct ScopedInner {
            #[prost(message, repeated, tag = "2")]
            values: Vec<ScopedValue>,
        }

        #[derive(Clone, PartialEq, crate::Message)]
        #[prost(prost_path = "crate")]
        struct ScopedOuter {
            #[prost(message, optional, tag = "1")]
            inner: Option<ScopedInner>,
            #[prost(message, repeated, tag = "2")]
            values: Vec<ScopedValue>,
        }

        #[derive(Clone, PartialEq, crate::Message)]
        #[prost(prost_path = "crate")]
        struct RepeatedThenBytes {
            #[prost(message, repeated, tag = "1")]
            values: Vec<ScopedValue>,
            #[prost(bytes = "vec", tag = "2")]
            payload: Vec<u8>,
        }

        fn consecutive_repeated_messages(tag: u32, buf: &impl Buf, limit: usize) -> usize {
            consecutive_repeated_messages_in_scope(tag, buf, limit, usize::MAX)
        }

        #[test]
        fn field_tag_is_opt_in() {
            let ctx = DecodeContext::default();
            assert_eq!(None, ctx.field_tag());
            assert_eq!(Some(1), ctx.with_field_tag(1).field_tag());
            assert_eq!(None, ctx.with_field_tag(0).field_tag());
        }

        #[test]
        fn recursion_clears_the_field_tag_and_preserves_the_scope() {
            let ctx = DecodeContext::default()
                .with_scope_limit(3)
                .with_field_tag(1)
                .enter_recursion();

            assert_eq!(None, ctx.field_tag());
            assert_eq!(Some(2), ctx.scope_remaining(5));
        }

        #[test]
        fn out_of_range_scope_disables_lookahead() {
            let ctx = DecodeContext::default().with_scope_limit(usize::MAX);

            assert_eq!(None, ctx.scope_remaining(usize::MAX));
        }

        #[test]
        fn counts_contiguous_repeated_messages() {
            let buf = &b"\x0a\x00\x0a\x02\x08\x01\x12\x00"[..];

            assert_eq!(2, consecutive_repeated_messages(1, &buf, usize::MAX));
        }

        #[test]
        fn counts_multibyte_tags() {
            let buf = &b"\x82\x01\x00\x82\x01\x00"[..];

            assert_eq!(2, consecutive_repeated_messages(16, &buf, usize::MAX));
        }

        #[test]
        fn counts_noncanonical_keys_and_lengths() {
            let buf = &b"\x8a\x00\x80\x00"[..];

            assert_eq!(1, consecutive_repeated_messages(1, &buf, usize::MAX));
        }

        #[test]
        fn stops_at_a_different_tag_or_incomplete_value() {
            let different_tag = &b"\x0a\x00\x12\x00\x0a\x00"[..];
            let incomplete_value = &b"\x0a\x00\x0a\x02\x08"[..];

            assert_eq!(
                1,
                consecutive_repeated_messages(1, &different_tag, usize::MAX)
            );
            assert_eq!(
                1,
                consecutive_repeated_messages(1, &incomplete_value, usize::MAX)
            );
        }

        #[test]
        fn stops_at_the_reservation_and_lookahead_limits() {
            let limited = &b"\x0a\x00\x0a\x00\x0a\x00"[..];
            assert_eq!(2, consecutive_repeated_messages(1, &limited, 2));

            let mut buf = Vec::new();
            for _ in 0..=MAX_REPEATED_MESSAGE_LOOKAHEAD_BYTES / 2 {
                buf.extend_from_slice(b"\x0a\x00");
            }
            assert_eq!(
                MAX_REPEATED_MESSAGE_LOOKAHEAD_BYTES / 2,
                consecutive_repeated_messages(1, &buf.as_slice(), usize::MAX)
            );
        }

        #[test]
        fn stops_at_the_message_boundary_and_chunk_boundary() {
            let message = &b"\x0a\x00\x0a\x00"[..];
            assert_eq!(
                1,
                consecutive_repeated_messages_in_scope(1, &message, usize::MAX, 2)
            );

            let buf = Bytes::from_static(b"\x0a\x00").chain(Bytes::from_static(b"\x0a\x00"));
            assert_eq!(1, consecutive_repeated_messages(1, &buf, usize::MAX));
        }

        #[test]
        fn bounds_reservation_by_message_size() {
            assert_eq!(1_023, repeated_message_reserve_limit::<[u8; 4]>());
            assert_eq!(7, repeated_message_reserve_limit::<[u8; 512]>());
            assert_eq!(0, repeated_message_reserve_limit::<[u8; 4_096]>());
        }

        #[test]
        fn reserves_through_merge_repeated() {
            let value = ScopedValue { value: true };
            let expected_values: Vec<_> = (0..100).map(|_| value.clone()).collect();
            let encoded = ScopedInner {
                values: expected_values.clone(),
            }
            .encode_to_vec();
            // Hand-written Message implementations do not provide a field tag
            // and continue to call the unchanged four-argument API.
            let mut untagged_buf = encoded.as_slice();
            let mut untagged_values: Vec<ScopedValue> = Vec::new();
            while untagged_buf.has_remaining() {
                let (_, wire_type) = decode_key(&mut untagged_buf).unwrap();
                merge_repeated(
                    wire_type,
                    &mut untagged_values,
                    &mut untagged_buf,
                    DecodeContext::default(),
                )
                .unwrap();
            }
            assert_eq!(untagged_values, expected_values);

            let mut buf = encoded.as_slice();
            let mut values: Vec<ScopedValue> = Vec::new();
            while buf.has_remaining() {
                let (tag, wire_type) = decode_key(&mut buf).unwrap();
                assert_eq!(2, tag);
                merge_repeated(
                    wire_type,
                    &mut values,
                    &mut buf,
                    DecodeContext::default().with_field_tag(tag),
                )
                .unwrap();
            }
            assert_eq!(values, expected_values);
            assert!(values.capacity() < 128);

            let decoded = ScopedInner::decode(encoded.as_slice()).unwrap();
            assert_eq!(decoded.values, expected_values);
            assert!(decoded.values.capacity() < 128);
        }

        #[test]
        fn does_not_reserve_for_an_unrelated_later_field() {
            let message = RepeatedThenBytes {
                values: Vec::from([ScopedValue { value: true }]),
                payload: (0..10_000).map(|_| 0).collect(),
            };

            let decoded = RepeatedThenBytes::decode(message.encode_to_vec().as_slice()).unwrap();
            assert_eq!(decoded, message);
            assert!(decoded.values.capacity() < 16);
        }

        #[test]
        fn does_not_reserve_past_a_nested_message() {
            let value = ScopedValue { value: true };
            let message = ScopedOuter {
                inner: Some(ScopedInner {
                    values: Vec::from([value.clone()]),
                }),
                values: (0..2_048).map(|_| value.clone()).collect(),
            };

            let outer_value_count = message.values.len();
            let encoded = message.encode_to_vec();
            let decoded = ScopedOuter::decode(encoded.as_slice()).unwrap();
            assert_eq!(decoded, message);
            assert!(decoded.inner.unwrap().values.capacity() < outer_value_count / 4);
        }
    }
}

pub mod group {
    use crate::error::DecodeErrorKind;

    use super::*;

    pub fn encode<M>(tag: u32, msg: &M, buf: &mut impl BufMut)
    where
        M: Message,
    {
        encode_key(tag, WireType::StartGroup, buf);
        msg.encode_raw(buf);
        encode_key(tag, WireType::EndGroup, buf);
    }

    pub fn merge<M>(
        tag: u32,
        wire_type: WireType,
        msg: &mut M,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Message,
    {
        check_wire_type(WireType::StartGroup, wire_type)?;

        ctx.limit_reached()?;
        loop {
            let (field_tag, field_wire_type) = decode_key(buf)?;
            if field_wire_type == WireType::EndGroup {
                if field_tag != tag {
                    return Err(DecodeErrorKind::UnexpectedEndGroupTag.into());
                }
                return Ok(());
            }

            M::merge_field(msg, field_tag, field_wire_type, buf, ctx.enter_recursion())?;
        }
    }

    pub fn encode_repeated<M>(tag: u32, messages: &[M], buf: &mut impl BufMut)
    where
        M: Message,
    {
        for msg in messages {
            encode(tag, msg, buf);
        }
    }

    pub fn merge_repeated<M>(
        tag: u32,
        wire_type: WireType,
        messages: &mut Vec<M>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        M: Message + Default,
    {
        check_wire_type(WireType::StartGroup, wire_type)?;
        let mut msg = M::default();
        merge(tag, WireType::StartGroup, &mut msg, buf, ctx)?;
        messages.push(msg);
        Ok(())
    }

    #[inline]
    pub fn encoded_len<M>(tag: u32, msg: &M) -> usize
    where
        M: Message,
    {
        2 * key_len(tag) + msg.encoded_len()
    }

    #[inline]
    pub fn encoded_len_repeated<M>(tag: u32, messages: &[M]) -> usize
    where
        M: Message,
    {
        2 * key_len(tag) * messages.len() + messages.iter().map(Message::encoded_len).sum::<usize>()
    }
}

/// Rust doesn't have a `Map` trait, so macros are currently the best way to be
/// generic over `HashMap` and `BTreeMap`.
macro_rules! map {
    ($map_ty:ident) => {
        use crate::encoding::*;
        use core::hash::Hash;

        /// Generic protobuf map encode function.
        pub fn encode<K, V, B, KE, KL, VE, VL>(
            key_encode: KE,
            key_encoded_len: KL,
            val_encode: VE,
            val_encoded_len: VL,
            tag: u32,
            values: &$map_ty<K, V>,
            buf: &mut B,
        ) where
            K: Default + Eq + Hash + Ord,
            V: Default + PartialEq,
            B: BufMut,
            KE: Fn(u32, &K, &mut B),
            KL: Fn(u32, &K) -> usize,
            VE: Fn(u32, &V, &mut B),
            VL: Fn(u32, &V) -> usize,
        {
            encode_with_default(
                key_encode,
                key_encoded_len,
                val_encode,
                val_encoded_len,
                &V::default(),
                tag,
                values,
                buf,
            )
        }

        /// Generic protobuf map merge function.
        pub fn merge<K, V, B, KM, VM>(
            key_merge: KM,
            val_merge: VM,
            values: &mut $map_ty<K, V>,
            buf: &mut B,
            ctx: DecodeContext,
        ) -> Result<(), DecodeError>
        where
            K: Default + Eq + Hash + Ord,
            V: Default,
            B: Buf,
            KM: Fn(WireType, &mut K, &mut B, DecodeContext) -> Result<(), DecodeError>,
            VM: Fn(WireType, &mut V, &mut B, DecodeContext) -> Result<(), DecodeError>,
        {
            merge_with_default(key_merge, val_merge, V::default(), values, buf, ctx)
        }

        /// Generic protobuf map encode function.
        pub fn encoded_len<K, V, KL, VL>(
            key_encoded_len: KL,
            val_encoded_len: VL,
            tag: u32,
            values: &$map_ty<K, V>,
        ) -> usize
        where
            K: Default + Eq + Hash + Ord,
            V: Default + PartialEq,
            KL: Fn(u32, &K) -> usize,
            VL: Fn(u32, &V) -> usize,
        {
            encoded_len_with_default(key_encoded_len, val_encoded_len, &V::default(), tag, values)
        }

        /// Generic protobuf map encode function with an overridden value default.
        ///
        /// This is necessary because enumeration values can have a default value other
        /// than 0 in proto2.
        pub fn encode_with_default<K, V, B, KE, KL, VE, VL>(
            key_encode: KE,
            key_encoded_len: KL,
            val_encode: VE,
            val_encoded_len: VL,
            val_default: &V,
            tag: u32,
            values: &$map_ty<K, V>,
            buf: &mut B,
        ) where
            K: Default + Eq + Hash + Ord,
            V: PartialEq,
            B: BufMut,
            KE: Fn(u32, &K, &mut B),
            KL: Fn(u32, &K) -> usize,
            VE: Fn(u32, &V, &mut B),
            VL: Fn(u32, &V) -> usize,
        {
            for (key, val) in values.iter() {
                let skip_key = key == &K::default();
                let skip_val = val == val_default;

                let len = (if skip_key { 0 } else { key_encoded_len(1, key) })
                    + (if skip_val { 0 } else { val_encoded_len(2, val) });

                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(len as u64, buf);
                if !skip_key {
                    key_encode(1, key, buf);
                }
                if !skip_val {
                    val_encode(2, val, buf);
                }
            }
        }

        /// Generic protobuf map merge function with an overridden value default.
        ///
        /// This is necessary because enumeration values can have a default value other
        /// than 0 in proto2.
        pub fn merge_with_default<K, V, B, KM, VM>(
            key_merge: KM,
            val_merge: VM,
            val_default: V,
            values: &mut $map_ty<K, V>,
            buf: &mut B,
            ctx: DecodeContext,
        ) -> Result<(), DecodeError>
        where
            K: Default + Eq + Hash + Ord,
            B: Buf,
            KM: Fn(WireType, &mut K, &mut B, DecodeContext) -> Result<(), DecodeError>,
            VM: Fn(WireType, &mut V, &mut B, DecodeContext) -> Result<(), DecodeError>,
        {
            let mut key = Default::default();
            let mut val = val_default;
            ctx.limit_reached()?;
            merge_loop(
                &mut (&mut key, &mut val),
                buf,
                ctx.enter_recursion(),
                |&mut (ref mut key, ref mut val), buf, ctx| {
                    let (tag, wire_type) = decode_key(buf)?;
                    match tag {
                        1 => key_merge(wire_type, key, buf, ctx),
                        2 => val_merge(wire_type, val, buf, ctx),
                        _ => skip_field(wire_type, tag, buf, ctx),
                    }
                },
            )?;
            values.insert(key, val);

            Ok(())
        }

        /// Generic protobuf map encode function with an overridden value default.
        ///
        /// This is necessary because enumeration values can have a default value other
        /// than 0 in proto2.
        pub fn encoded_len_with_default<K, V, KL, VL>(
            key_encoded_len: KL,
            val_encoded_len: VL,
            val_default: &V,
            tag: u32,
            values: &$map_ty<K, V>,
        ) -> usize
        where
            K: Default + Eq + Hash + Ord,
            V: PartialEq,
            KL: Fn(u32, &K) -> usize,
            VL: Fn(u32, &V) -> usize,
        {
            key_len(tag) * values.len()
                + values
                    .iter()
                    .map(|(key, val)| {
                        let len = (if key == &K::default() {
                            0
                        } else {
                            key_encoded_len(1, key)
                        }) + (if val == val_default {
                            0
                        } else {
                            val_encoded_len(2, val)
                        });
                        encoded_len_varint(len as u64) + len
                    })
                    .sum::<usize>()
        }
    };
}

#[cfg(feature = "std")]
pub mod hash_map {
    use std::collections::HashMap;
    map!(HashMap);
}

pub mod btree_map {
    map!(BTreeMap);
}

#[cfg(test)]
mod test {
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;
    use core::borrow::Borrow;
    use core::fmt::Debug;

    use ::bytes::BytesMut;
    use proptest::{prelude::*, test_runner::TestCaseResult};

    use super::*;

    pub fn check_type<T, B>(
        value: T,
        tag: u32,
        wire_type: WireType,
        encode: fn(u32, &B, &mut BytesMut),
        merge: fn(WireType, &mut T, &mut Bytes, DecodeContext) -> Result<(), DecodeError>,
        encoded_len: fn(u32, &B) -> usize,
    ) -> TestCaseResult
    where
        T: Debug + Default + PartialEq + Borrow<B>,
        B: ?Sized,
    {
        prop_assume!((MIN_TAG..=MAX_TAG).contains(&tag));

        let expected_len = encoded_len(tag, value.borrow());

        let mut buf = BytesMut::with_capacity(expected_len);
        encode(tag, value.borrow(), &mut buf);

        let mut buf = buf.freeze();

        prop_assert_eq!(
            buf.remaining(),
            expected_len,
            "encoded_len wrong; expected: {}, actual: {}",
            expected_len,
            buf.remaining()
        );

        if !buf.has_remaining() {
            // Short circuit for empty packed values.
            return Ok(());
        }

        let (decoded_tag, decoded_wire_type) =
            decode_key(&mut buf).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(
            tag,
            decoded_tag,
            "decoded tag does not match; expected: {}, actual: {}",
            tag,
            decoded_tag
        );

        prop_assert_eq!(
            wire_type,
            decoded_wire_type,
            "decoded wire type does not match; expected: {:?}, actual: {:?}",
            wire_type,
            decoded_wire_type,
        );

        match wire_type {
            WireType::SixtyFourBit if buf.remaining() != 8 => Err(TestCaseError::fail(format!(
                "64bit wire type illegal remaining: {}, tag: {}",
                buf.remaining(),
                tag
            ))),
            WireType::ThirtyTwoBit if buf.remaining() != 4 => Err(TestCaseError::fail(format!(
                "32bit wire type illegal remaining: {}, tag: {}",
                buf.remaining(),
                tag
            ))),
            _ => Ok(()),
        }?;

        let mut roundtrip_value = T::default();
        merge(
            wire_type,
            &mut roundtrip_value,
            &mut buf,
            DecodeContext::default(),
        )
        .map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert!(
            !buf.has_remaining(),
            "expected buffer to be empty, remaining: {}",
            buf.remaining()
        );

        prop_assert_eq!(value, roundtrip_value);

        Ok(())
    }

    pub fn check_collection_type<T, B, E, M, L>(
        value: T,
        tag: u32,
        wire_type: WireType,
        encode: E,
        mut merge: M,
        encoded_len: L,
    ) -> TestCaseResult
    where
        T: Debug + Default + PartialEq + Borrow<B>,
        B: ?Sized,
        E: FnOnce(u32, &B, &mut BytesMut),
        M: FnMut(WireType, &mut T, &mut Bytes, DecodeContext) -> Result<(), DecodeError>,
        L: FnOnce(u32, &B) -> usize,
    {
        prop_assume!((MIN_TAG..=MAX_TAG).contains(&tag));

        let expected_len = encoded_len(tag, value.borrow());

        let mut buf = BytesMut::with_capacity(expected_len);
        encode(tag, value.borrow(), &mut buf);

        let mut buf = buf.freeze();

        prop_assert_eq!(
            buf.remaining(),
            expected_len,
            "encoded_len wrong; expected: {}, actual: {}",
            expected_len,
            buf.remaining()
        );

        let mut roundtrip_value = Default::default();
        while buf.has_remaining() {
            let (decoded_tag, decoded_wire_type) =
                decode_key(&mut buf).map_err(|error| TestCaseError::fail(error.to_string()))?;

            prop_assert_eq!(
                tag,
                decoded_tag,
                "decoded tag does not match; expected: {}, actual: {}",
                tag,
                decoded_tag
            );

            prop_assert_eq!(
                wire_type,
                decoded_wire_type,
                "decoded wire type does not match; expected: {:?}, actual: {:?}",
                wire_type,
                decoded_wire_type
            );

            merge(
                wire_type,
                &mut roundtrip_value,
                &mut buf,
                DecodeContext::default(),
            )
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        }

        prop_assert_eq!(value, roundtrip_value);

        Ok(())
    }

    #[test]
    fn string_merge_invalid_utf8() {
        let mut s = String::new();
        let buf = b"\x02\x80\x80";

        let r = string::merge(
            WireType::LengthDelimited,
            &mut s,
            &mut &buf[..],
            DecodeContext::default(),
        );
        r.expect_err("must be an error");
        assert!(s.is_empty());
    }

    /// This big bowl o' macro soup generates an encoding property test for each combination of map
    /// type, scalar map key, and value type.
    /// TODO: these tests take a long time to compile, can this be improved?
    #[cfg(feature = "std")]
    macro_rules! map_tests {
        (keys: $keys:tt,
         vals: $vals:tt) => {
            mod hash_map {
                map_tests!(@private HashMap, hash_map, $keys, $vals);
            }
            mod btree_map {
                map_tests!(@private BTreeMap, btree_map, $keys, $vals);
            }
        };

        (@private $map_type:ident,
                  $mod_name:ident,
                  [$(($key_ty:ty, $key_proto:ident)),*],
                  $vals:tt) => {
            $(
                mod $key_proto {
                    use std::collections::$map_type;

                    use proptest::prelude::*;

                    use crate::encoding::*;
                    use crate::encoding::test::check_collection_type;

                    map_tests!(@private $map_type, $mod_name, ($key_ty, $key_proto), $vals);
                }
            )*
        };

        (@private $map_type:ident,
                  $mod_name:ident,
                  ($key_ty:ty, $key_proto:ident),
                  [$(($val_ty:ty, $val_proto:ident)),*]) => {
            $(
                proptest! {
                    #[test]
                    fn $val_proto(values: $map_type<$key_ty, $val_ty>, tag in MIN_TAG..=MAX_TAG) {
                        check_collection_type(values, tag, WireType::LengthDelimited,
                                              |tag, values, buf| {
                                                  $mod_name::encode($key_proto::encode,
                                                                    $key_proto::encoded_len,
                                                                    $val_proto::encode,
                                                                    $val_proto::encoded_len,
                                                                    tag,
                                                                    values,
                                                                    buf)
                                              },
                                              |wire_type, values, buf, ctx| {
                                                  check_wire_type(WireType::LengthDelimited, wire_type)?;
                                                  $mod_name::merge($key_proto::merge,
                                                                   $val_proto::merge,
                                                                   values,
                                                                   buf,
                                                                   ctx)
                                              },
                                              |tag, values| {
                                                  $mod_name::encoded_len($key_proto::encoded_len,
                                                                         $val_proto::encoded_len,
                                                                         tag,
                                                                         values)
                                              })?;
                    }
                }
             )*
        };
    }

    #[cfg(feature = "std")]
    map_tests!(keys: [
        (i32, int32),
        (i64, int64),
        (u32, uint32),
        (u64, uint64),
        (i32, sint32),
        (i64, sint64),
        (u32, fixed32),
        (u64, fixed64),
        (i32, sfixed32),
        (i64, sfixed64),
        (bool, bool),
        (String, string)
    ],
    vals: [
        (f32, float),
        (f64, double),
        (i32, int32),
        (i64, int64),
        (u32, uint32),
        (u64, uint64),
        (i32, sint32),
        (i64, sint64),
        (u32, fixed32),
        (u64, fixed64),
        (i32, sfixed32),
        (i64, sfixed64),
        (bool, bool),
        (String, string),
        (Vec<u8>, bytes)
    ]);

    #[test]
    /// `decode_varint` accepts a `Buf`, which can be multiple concatenated buffers.
    /// This test ensures that future optimizations don't break the
    /// `decode_varint` for non-continuous memory.
    fn split_varint_decoding() {
        let mut test_values = Vec::<u64>::with_capacity(10 * 2);
        test_values.push(128);
        for i in 2..9 {
            test_values.push((1 << (7 * i)) - 1);
            test_values.push(1 << (7 * i));
        }

        for v in test_values {
            let mut buf = BytesMut::with_capacity(10);
            encode_varint(v, &mut buf);
            let half_len = buf.len() / 2;
            let len = buf.len();
            // this weird sequence here splits the buffer into two instances of Bytes
            // which we then stitch together with `bytes::buf::Buf::chain`
            // which ensures the varint bytes are not in a single chunk
            let b2 = buf.split_off(half_len);
            let mut c = buf.chain(b2);

            // make sure all the bytes are inside
            assert_eq!(c.remaining(), len);
            // make sure the first chunk is split as we expected
            assert_eq!(c.chunk().len(), half_len);
            assert_eq!(v, decode_varint(&mut c).unwrap());
        }
    }
}
