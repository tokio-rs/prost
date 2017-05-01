use std::default;
use std::io::Result;

use bytes::{
    Buf,
    BufMut,
    LittleEndian,
    Take,
};

use field::{Field, Type};
use encoding::*;

/// A numeric scalar Protobuf field type.
///
/// The `E` type parameter allows `Numeric` to be implemented multiple times for a
/// single type, in order to provide multiple encoding and decoding options for
/// a single Rust type. For instance, the Protobuf `fixed32` and `uint32` types
/// both correspond to the Rust `u32` type, so `u32` has two impls of `Numeric`
/// with different types for `E`, which correspond to `fixed32` and `uint32`.
/// Repeated numeric fields can optionally use packed encoding, which is
/// controlled using the `Packed` type.
pub trait Numeric<E=Default> : default::Default {
    /// Encodes the scalar field to the buffer, without a key.
    /// The buffer must have enough remaining space to hold the encoded key and field.
    fn encode<B>(self, buf: &mut B) where B: BufMut;

    /// Decodes an instance of the field from the buffer.
    fn decode<B>(buf: &mut B) -> Result<Self> where B: Buf;

    /// Returns the encoded length of the field, without a key.
    fn encoded_len(self) -> usize;

    /// Returns the wire type of the numeric scalar field.
    fn wire_type() -> WireType;
}

// Provides Field, Type, and repeated Field impls for numeric types.
macro_rules! numeric_field {
    ($ty: ty) => { numeric_field!($ty, Default); };
    ($ty: ty, $e: ty) => {
        impl Field<$e> for $ty {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                encode_key(tag, <$ty as Numeric<$e>>::wire_type(), buf);
                <$ty as Numeric<$e>>::encode(*self, buf);
            }
            #[inline]
            fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut Take<B>) -> Result<()> where B: Buf {
                check_wire_type(<$ty as Numeric<$e>>::wire_type(), wire_type)?;
                *self = <$ty as Numeric<$e>>::decode(buf)?;
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize { key_len(tag) + <$ty as Numeric<$e>>::encoded_len(*self) }
        }
        impl Type<$e> for $ty {
            fn empty() -> $ty {
                ::std::default::Default::default()
            }
        }

        impl Field<$e> for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                for value in self {
                    <$ty as Field<$e>>::encode(value, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut Take<B>) -> Result<()> where B: Buf {
                if wire_type == WireType::LengthDelimited {
                    // Packed repeated encoding.
                    let len = decode_varint(buf)?;
                    if len > buf.remaining() as u64 {
                        return Err(invalid_data("failed to decode packed repeated field: buffer underflow"));
                    }

                    let limit = buf.limit();
                    buf.set_limit(len as usize);
                    while buf.has_remaining() {
                        self.push(<$ty as Numeric<$e>>::decode(buf)?);
                    }
                    buf.set_limit(limit - len as usize);
                } else {
                    // Default repeated encoding.
                    let mut value = default::Default::default();
                    <$ty as Field<$e>>::merge(&mut value, tag, wire_type, buf)?;
                    self.push(value);
                }
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                self.iter().map(|f| <$ty as Field<$e>>::encoded_len(f, tag)).sum()
            }
        }

        impl Field<(Packed, $e)> for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                if self.is_empty() { return; }
                encode_key(tag, WireType::LengthDelimited, buf);
                let len: usize = self.iter().cloned().map(<$ty as Numeric<$e>>::encoded_len).sum();
                encode_varint(len as u64, buf);
                for &value in self {
                    <$ty as Numeric<$e>>::encode(value, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut Take<B>) -> Result<()> where B: Buf {
                <Vec<$ty> as Field<$e>>::merge(self, tag, wire_type, buf)
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                if self.is_empty() { return 0; }
                let len: usize = self.iter().cloned().map(<$ty as Numeric<$e>>::encoded_len).sum();
                key_len(tag) + encoded_len_varint(len as _) + len
            }
        }
    };
}

// bool
impl Numeric for bool {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut {
        buf.put_u8(if self { 1u8 } else { 0u8 });
    }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<bool> where B: Buf {
        if !buf.has_remaining() {
            return Err(invalid_data("failed to decode bool: buffer underflow"));
        }
        match buf.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(invalid_data(format!("failed to decode bool: invalid value: {}", b))),
        }
    }
    #[inline]
    fn encoded_len(self) -> usize { 1 }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
numeric_field!(bool);

// int32
impl Numeric for i32 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<i32> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
numeric_field!(i32);

// int64
impl Numeric for i64 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<i64> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType {
        WireType::Varint
    }
}
numeric_field!(i64);

// uint32
impl Numeric for u32 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<u32> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
numeric_field!(u32);

// uint64
impl Numeric for u64 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { encode_varint(self as _, buf) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<u64> where B: Buf { decode_varint(buf).map(|value| value as _) }
    #[inline]
    fn encoded_len(self) -> usize { encoded_len_varint(self as _) }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
numeric_field!(u64);

// float
impl Numeric for f32 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { buf.put_f32::<LittleEndian>(self) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<f32> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode float: buffer underflow"));
        }
        Ok(buf.get_f32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}
numeric_field!(f32);

// double
impl Numeric for f64 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { buf.put_f64::<LittleEndian>(self) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<f64> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode double: buffer underflow"));
        }
        Ok(buf.get_f64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}
numeric_field!(f64);

// sint32
impl Numeric<Signed> for i32 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut {
        encode_varint(((self << 1) ^ (self >> 31)) as u64, buf)
    }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<i32> where B: Buf {
        decode_varint(buf).map(|value| {
            let value = value as i32;
            (value >> 1) ^ -(value & 1)
        })
    }
    #[inline]
    fn encoded_len(self) -> usize {
        encoded_len_varint(((self << 1) ^ (self >> 31)) as u64)
    }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
numeric_field!(i32, Signed);

// sint64
impl Numeric<Signed> for i64 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut {
        encode_varint(((self << 1) ^ (self >> 63)) as u64, buf)
    }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<i64> where B: Buf {
        decode_varint(buf).map(|value| {
            let value = value as i64;
            (value >> 1) ^ -(value & 1)
        })
    }
    #[inline]
    fn encoded_len(self) -> usize {
        encoded_len_varint(((self << 1) ^ (self >> 63)) as u64)
    }
    #[inline]
    fn wire_type() -> WireType { WireType::Varint }
}
numeric_field!(i64, Signed);

// fixed32
impl Numeric<Fixed> for u32 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { buf.put_u32::<LittleEndian>(self) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<u32> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode fixed32: buffer underflow"));
        }
        Ok(buf.get_u32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}
numeric_field!(u32, Fixed);

// fixed64
impl Numeric<Fixed> for u64 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { buf.put_u64::<LittleEndian>(self) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<u64> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode fixed64: buffer underflow"));
        }
        Ok(buf.get_u64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}
numeric_field!(u64, Fixed);

// sfixed32
impl Numeric<Fixed> for i32 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { buf.put_i32::<LittleEndian>(self) }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<i32> where B: Buf {
        if buf.remaining() < 4 {
            return Err(invalid_input("failed to decode sfixed32: buffer underflow"));
        }
        Ok(buf.get_i32::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(self) -> usize { 4 }
    #[inline]
    fn wire_type() -> WireType { WireType::ThirtyTwoBit }
}
numeric_field!(i32, Fixed);

// sfixed64
impl Numeric<Fixed> for i64 {
    #[inline]
    fn encode<B>(self, buf: &mut B) where B: BufMut { buf.put_i64::<LittleEndian>(self); }
    #[inline]
    fn decode<B>(buf: &mut B) -> Result<i64> where B: Buf {
        if buf.remaining() < 8 {
            return Err(invalid_input("failed to decode sfixed64 field: buffer underflow"));
        }
        Ok(buf.get_i64::<LittleEndian>())
    }
    #[inline]
    fn encoded_len(self) -> usize { 8 }
    #[inline]
    fn wire_type() -> WireType { WireType::SixtyFourBit }
}
numeric_field!(i64, Fixed);
