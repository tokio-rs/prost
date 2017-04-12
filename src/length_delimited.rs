use std::default;
use std::io::Result;
use std::str;

use bytes::{
    Buf,
    BufMut,
    Take,
};

use encoding::*;
use field::{
    Field,
    Type,
};

/// A length-delimited scalar Protobuf field type.
pub trait LengthDelimited : default::Default {
    /// Encodes the length-delimited type to the buffer, without the length delimiter.
    /// The buffer must have enough remaining space to hold the encoded type.
    fn encode<B>(&self, buf: &mut B) where B: BufMut;

    /// Decodes the length-delimited type from the buffer, and merges the value into self.
    fn merge<B>(&mut self, buf: Take<&mut B>) -> Result<()> where B: Buf;

    /// Returns the length of the encoded field, without the length delimiter.
    fn encoded_len(&self) -> usize;
}

// Provides Field, Type, and repeated Field impls for length-delimited types.
macro_rules! length_delimited_field {
    ($ty: ty) => {
        impl Field for $ty {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(LengthDelimited::encoded_len(self) as u64, buf);
                LengthDelimited::encode(self, buf);
            }
            #[inline]
            fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)?;
                if (buf.remaining() as u64) < len {
                    return Err(invalid_input("failed to decode length-delimited field: buffer underflow"));
                }
                LengthDelimited::merge(self, buf.take(len as usize))
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                let len = LengthDelimited::encoded_len(self);
                key_len(tag) + encoded_len_varint(len as u64) + len
            }
        }
        impl Type for $ty {}

        impl Field for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                for value in self {
                    Field::encode(value, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let mut value = default::Default::default();
                Field::merge(&mut value, tag, WireType::LengthDelimited, buf)?;
                self.push(value);
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                self.iter().map(|value| Field::encoded_len(value, tag)).sum()
            }
        }
    };
}

// string
impl LengthDelimited for String {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_slice(self.as_bytes());
    }
    #[inline]
    fn merge<B>(&mut self, mut buf: Take<&mut B>) -> Result<()> where B: Buf {
        self.clear();
        self.push_str(str::from_utf8(buf.bytes()).map_err(|_| {
            invalid_data("failed to decode string: data is not UTF-8 encoded")
        })?);
        let len = buf.remaining();
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        self.len()
    }
}
length_delimited_field!(String);

// bytes
impl LengthDelimited for Vec<u8> {
    #[inline]
    fn encode<B>(&self, buf: &mut B) where B: BufMut {
        buf.put_slice(self);
    }
    #[inline]
    fn merge<B>(&mut self, mut buf: Take<&mut B>) -> Result<()> where B: Buf {
        self.clear();
        self.extend_from_slice(buf.bytes());
        let len = buf.remaining();
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        self.len()
    }
}
length_delimited_field!(Vec<u8>);
