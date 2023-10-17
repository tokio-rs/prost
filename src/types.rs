//! Protocol Buffers well-known wrapper types.
//!
//! This module provides implementations of `Message` for Rust standard library types which
//! correspond to a Protobuf well-known wrapper type. The remaining well-known types are defined in
//! the `prost-types` crate in order to avoid a cyclic dependency between `prost` and
//! `prost-build`.

use alloc::string::String;
use alloc::vec::Vec;

use ::bytes::{Buf, BufMut, Bytes};

use crate::{
    encoding::{
        bool, bytes, double, float, int32, int64, skip_field, string, uint32, uint64,
        DecodeContext, WireType,
    },
    DecodeError, Message, Name,
};

const PACKAGE: &str = "google.protobuf";

/// `google.protobuf.BoolValue`
impl Message for bool {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self {
            bool::encode(1, self, buf)
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
    {
        if tag == 1 {
            bool::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self {
            2
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = false;
    }
}

/// `google.protobuf.BoolValue`
impl Name for bool {
    const NAME: &'static str = "BoolValue";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.UInt32Value`
impl Message for u32 {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self != 0 {
            uint32::encode(1, self, buf)
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
    {
        if tag == 1 {
            uint32::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            uint32::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.UInt32Value`
impl Name for u32 {
    const NAME: &'static str = "UInt32Value";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.UInt64Value`
impl Message for u64 {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self != 0 {
            uint64::encode(1, self, buf)
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
    {
        if tag == 1 {
            uint64::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            uint64::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.UInt64Value`
impl Name for u64 {
    const NAME: &'static str = "UInt64Value";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.Int32Value`
impl Message for i32 {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self != 0 {
            int32::encode(1, self, buf)
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
    {
        if tag == 1 {
            int32::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            int32::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.Int32Value`
impl Name for i32 {
    const NAME: &'static str = "Int32Value";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.Int64Value`
impl Message for i64 {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self != 0 {
            int64::encode(1, self, buf)
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
    {
        if tag == 1 {
            int64::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0 {
            int64::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.Int64Value`
impl Name for i64 {
    const NAME: &'static str = "Int64Value";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.FloatValue`
impl Message for f32 {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self != 0.0 {
            float::encode(1, self, buf)
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
    {
        if tag == 1 {
            float::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0.0 {
            float::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = 0.0;
    }
}

/// `google.protobuf.FloatValue`
impl Name for f32 {
    const NAME: &'static str = "FloatValue";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.DoubleValue`
impl Message for f64 {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if *self != 0.0 {
            double::encode(1, self, buf)
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
    {
        if tag == 1 {
            double::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if *self != 0.0 {
            double::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        *self = 0.0;
    }
}

/// `google.protobuf.DoubleValue`
impl Name for f64 {
    const NAME: &'static str = "DoubleValue";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.StringValue`
impl Message for String {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if !self.is_empty() {
            string::encode(1, self, buf)
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
    {
        if tag == 1 {
            string::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if !self.is_empty() {
            string::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.StringValue`
impl Name for String {
    const NAME: &'static str = "StringValue";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.BytesValue`
impl Message for Vec<u8> {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if !self.is_empty() {
            bytes::encode(1, self, buf)
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
    {
        if tag == 1 {
            bytes::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if !self.is_empty() {
            bytes::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.BytesValue`
impl Name for Vec<u8> {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.BytesValue`
impl Message for Bytes {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        if !self.is_empty() {
            bytes::encode(1, self, buf)
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
    {
        if tag == 1 {
            bytes::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        if !self.is_empty() {
            bytes::encoded_len(1, self)
        } else {
            0
        }
    }
    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.BytesValue`
impl Name for Bytes {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// `google.protobuf.Empty`
impl Message for () {
    fn encode_raw<B>(&self, _buf: &mut B)
    where
        B: BufMut,
    {
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
    {
        skip_field(wire_type, tag, buf, ctx)
    }
    fn encoded_len(&self) -> usize {
        0
    }
    fn clear(&mut self) {}
}

/// `google.protobuf.Empty`
impl Name for () {
    const NAME: &'static str = "Empty";
    const PACKAGE: &'static str = PACKAGE;

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

/// Compute the type URL for the given `google.protobuf` type, using `type.googleapis.com` as the
/// authority for the URL.
fn type_url_for<T: Name>() -> String {
    format!("type.googleapis.com/{}.{}", T::PACKAGE, T::NAME)
}
